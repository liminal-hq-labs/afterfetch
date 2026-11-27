// Linux: the system calls afterfetch makes (and only those), and the `Platform` that reads the facts from `/etc`, `/proc`, `/sys` and the
// environment. The parsing lives in the core; this file reads the bytes and hands them over.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::format::{c_str, format_terminal};
use afterfetch_core::parse::cpuinfo::CpuInfo;
use afterfetch_core::parse::lines::LineFeed;
use afterfetch_core::parse::meminfo::MemInfo;
use afterfetch_core::parse::os_release::OsRelease;
use afterfetch_core::parse::{basename, model, passwd, uptime};
use afterfetch_core::{Buf, FactKey, Facts, Platform, TerminalInfo, default_logo, depth, run};

use crate::arch::{self, nr};
use crate::rt::{Args, c_bytes};
use crate::term::{FdSink, Out, locale_is_utf8};

/// Every system call the program makes. `syscalls.allow` lists the same names, and `scripts/check-unix.sh` keeps the two equal.
#[derive(Clone, Copy)]
enum Sys {
	Read,
	Write,
	OpenAt,
	Close,
	Ioctl,
	Uname,
	Sysinfo,
	Getuid,
	ExitGroup,
}

impl Sys {
	const fn number(self) -> usize {
		match self {
			Sys::Read => nr::READ,
			Sys::Write => nr::WRITE,
			Sys::OpenAt => nr::OPENAT,
			Sys::Close => nr::CLOSE,
			Sys::Ioctl => nr::IOCTL,
			Sys::Uname => nr::UNAME,
			Sys::Sysinfo => nr::SYSINFO,
			Sys::Getuid => nr::GETUID,
			Sys::ExitGroup => nr::EXIT_GROUP,
		}
	}
}

fn call(sys: Sys, first: usize, second: usize, third: usize) -> isize {
	// SAFETY: every caller in this file passes pointers to live buffers of the length it passes beside them.
	unsafe { arch::syscall3(sys.number(), first, second, third) }
}

const EINTR: isize = -4;
const AT_FDCWD: usize = -100isize as usize;
const O_RDONLY: usize = 0;
const O_CLOEXEC: usize = 0o2000000;

/// Ends the program.
pub fn exit(code: u8) -> ! {
	call(Sys::ExitGroup, usize::from(code), 0, 0);
	loop {
		core::hint::spin_loop();
	}
}

/// Writes all of `bytes` to `fd`, retrying after a signal and a short write. False when the write fails.
pub fn write_all(fd: i32, mut bytes: &[u8]) -> bool {
	while !bytes.is_empty() {
		let written = call(Sys::Write, fd as usize, bytes.as_ptr() as usize, bytes.len());
		if written == EINTR {
			continue;
		}
		if written <= 0 {
			return false;
		}
		bytes = &bytes[written as usize..];
	}
	true
}

/// Reads up to `buffer.len()` bytes; 0 at the end of the file or on an error.
fn read(fd: i32, buffer: &mut [u8]) -> usize {
	loop {
		let count = call(Sys::Read, fd as usize, buffer.as_mut_ptr() as usize, buffer.len());
		if count != EINTR {
			return count.max(0) as usize;
		}
	}
}

/// Opens `path` (NUL-terminated) for reading.
fn open(path: &[u8]) -> Option<i32> {
	debug_assert!(path.last() == Some(&0));
	let fd = call(Sys::OpenAt, AT_FDCWD, path.as_ptr() as usize, O_RDONLY | O_CLOEXEC);
	(fd >= 0).then_some(fd as i32)
}

fn close(fd: i32) {
	call(Sys::Close, fd as usize, 0, 0);
}

/// Reads the file at `path` (NUL-terminated) in chunks, passing each line to `handle`. False when it cannot be opened.
fn read_lines(path: &[u8], handle: &mut impl FnMut(&[u8])) -> bool {
	let Some(fd) = open(path) else { return false };
	let mut chunk = [0u8; 1024];
	let mut feed = LineFeed::<192>::new();
	loop {
		let count = read(fd, &mut chunk);
		if count == 0 {
			break;
		}
		feed.feed(&chunk[..count], handle);
	}
	feed.finish(handle);
	close(fd);
	true
}

/// Reads the start of the small file at `path` into `buffer`, returning the bytes read (0 when it cannot be opened).
fn read_small(path: &[u8], buffer: &mut [u8]) -> usize {
	let Some(fd) = open(path) else { return 0 };
	let count = read(fd, buffer);
	close(fd);
	count
}

// The kernel fills the whole structure; only some fields are read.
#[allow(dead_code)]
#[repr(C)]
struct Utsname {
	sysname: [u8; 65],
	nodename: [u8; 65],
	release: [u8; 65],
	version: [u8; 65],
	machine: [u8; 65],
	domainname: [u8; 65],
}

fn uname() -> Option<Utsname> {
	let mut names = Utsname {
		sysname: [0; 65],
		nodename: [0; 65],
		release: [0; 65],
		version: [0; 65],
		machine: [0; 65],
		domainname: [0; 65],
	};
	(call(Sys::Uname, &raw mut names as usize, 0, 0) == 0).then_some(names)
}

/// The kernel's `struct sysinfo`, with room to spare at the end. The kernel fills it all; only some fields are read.
#[allow(dead_code)]
#[repr(C)]
struct Sysinfo {
	uptime: isize,
	loads: [usize; 3],
	total_ram: usize,
	free_ram: usize,
	shared_ram: usize,
	buffer_ram: usize,
	total_swap: usize,
	free_swap: usize,
	processes: u16,
	padding: u16,
	total_high: usize,
	free_high: usize,
	memory_unit: u32,
	tail: [u8; 64],
}

fn sysinfo() -> Option<Sysinfo> {
	let mut info = Sysinfo {
		uptime: 0,
		loads: [0; 3],
		total_ram: 0,
		free_ram: 0,
		shared_ram: 0,
		buffer_ram: 0,
		total_swap: 0,
		free_swap: 0,
		processes: 0,
		padding: 0,
		total_high: 0,
		free_high: 0,
		memory_unit: 0,
		tail: [0; 64],
	};
	(call(Sys::Sysinfo, &raw mut info as usize, 0, 0) == 0).then_some(info)
}

fn getuid() -> u32 {
	call(Sys::Getuid, 0, 0, 0) as u32
}

/// `ioctl` request results for standard output.
#[derive(Clone, Copy)]
struct Screen {
	is_tty: bool,
	columns: u16,
	rows: u16,
}

fn screen() -> Screen {
	// The kernel's `struct termios` is under 64 bytes on every architecture.
	let mut settings = [0u8; 64];
	let is_tty = call(Sys::Ioctl, 1, arch::TCGETS, settings.as_mut_ptr() as usize) == 0;
	let mut size = [0u16; 4];
	let known = is_tty && call(Sys::Ioctl, 1, arch::TIOCGWINSZ, size.as_mut_ptr() as usize) == 0;
	Screen {
		is_tty,
		rows: if known { size[0] } else { 0 },
		columns: if known { size[1] } else { 0 },
	}
}

/// Linux as the core sees it.
pub struct Linux {
	environment: *const *const u8,
	screen: Screen,
}

impl Linux {
	fn new(environment: *const *const u8) -> Self {
		Linux {
			environment,
			screen: screen(),
		}
	}

	fn var(&self, name: &[u8]) -> Option<&[u8]> {
		let mut entry = self.environment;
		// SAFETY: `environment` is the kernel's null-terminated array of NUL-terminated `NAME=value` strings.
		unsafe {
			while !(*entry).is_null() {
				let text = c_bytes(*entry);
				if let Some(value) = text.strip_prefix(name).and_then(|rest| rest.strip_prefix(b"=")) {
					return Some(value);
				}
				entry = entry.add(1);
			}
		}
		None
	}

	fn fill_user(&self, facts: &mut Facts) {
		let named = [&b"USER"[..], b"LOGNAME"]
			.into_iter()
			.find_map(|name| self.var(name).filter(|value| !value.is_empty()));
		if let Some(name) = named {
			facts.user.push(name);
			return;
		}
		let uid = getuid();
		read_lines(b"/etc/passwd\0", &mut |line| {
			if facts.user.is_empty()
				&& let Some(name) = passwd::name_for_uid(line, uid)
			{
				facts.user.push(name);
			}
		});
	}

	fn fill_os(&self, facts: &mut Facts, machine: &[u8]) {
		let mut release = OsRelease::new();
		let mut feed = |line: &[u8]| release.feed_line(line);
		let _ = read_lines(b"/etc/os-release\0", &mut feed) || read_lines(b"/usr/lib/os-release\0", &mut feed);
		let row = facts.row_mut(FactKey::Os);
		release.describe(row);
		if row.is_empty() {
			row.push(b"Linux");
		}
		if !machine.is_empty() {
			row.push(b" (").push(machine).push(b")");
		}
	}

	fn fill_model(&self, facts: &mut Facts) {
		let mut contents = [0u8; 80];
		let count = read_small(b"/sys/devices/virtual/dmi/id/product_name\0", &mut contents);
		model::clean(&contents[..count], facts.row_mut(FactKey::Host));
		if !facts.has(FactKey::Host) {
			let count = read_small(b"/proc/device-tree/model\0", &mut contents);
			model::clean(&contents[..count], facts.row_mut(FactKey::Host));
		}
	}

	fn fill_uptime(&self, facts: &mut Facts) {
		let mut contents = [0u8; 64];
		let count = read_small(b"/proc/uptime\0", &mut contents);
		let seconds = uptime::seconds(&contents[..count]).or_else(|| sysinfo().map(|info| info.uptime.max(0) as u64));
		if let Some(seconds) = seconds {
			facts.set_uptime(seconds);
		}
	}

	fn fill_cpu(&self, facts: &mut Facts) {
		let mut info = CpuInfo::new();
		read_lines(b"/proc/cpuinfo\0", &mut |line| info.feed_line(line));
		info.describe(facts.row_mut(FactKey::Cpu));
	}

	fn fill_memory(&self, facts: &mut Facts) {
		let mut info = MemInfo::new();
		read_lines(b"/proc/meminfo\0", &mut |line| info.feed_line(line));
		let (mut total, mut available) = (info.total_bytes(), info.available_bytes());
		if total == 0
			&& let Some(system) = sysinfo()
		{
			let unit = u64::from(system.memory_unit.max(1));
			total = system.total_ram as u64 * unit;
			available = (system.free_ram as u64 + system.buffer_ram as u64) * unit;
		}
		if total > 0 {
			facts.set_memory(available, total);
		}
	}

	fn fill_terminal(&self, facts: &mut Facts) {
		let name = [&b"TERM_PROGRAM"[..], b"TERM"]
			.into_iter()
			.find_map(|name| self.var(name).filter(|value| !value.is_empty()));
		format_terminal(
			facts.row_mut(FactKey::Terminal),
			name.unwrap_or(b""),
			u32::from(self.screen.columns),
			u32::from(self.screen.rows),
		);
	}
}

impl Platform for Linux {
	fn facts(&mut self, facts: &mut Facts) {
		self.fill_user(facts);
		let names = uname();
		let machine = names.as_ref().map_or(&b""[..], |names| c_str(&names.machine));
		if let Some(names) = &names {
			facts.host.push(c_str(&names.nodename));
			facts
				.row_mut(FactKey::Kernel)
				.push(c_str(&names.sysname))
				.push(b" ")
				.push(c_str(&names.release));
		}
		// The kernel's own name for the machine, as it gives it (AF-D44): `x86_64`, `i686`, `aarch64`, `armv7l`.
		facts.arch.push(machine);
		self.fill_os(facts, machine);
		self.fill_model(facts);
		self.fill_uptime(facts);
		self.fill_cpu(facts);
		self.fill_memory(facts);
		// No display row: a console program cannot honestly say what the screen is (AF-D20).
		if let Some(desktop) = self.var(b"XDG_CURRENT_DESKTOP").filter(|value| !value.is_empty()) {
			facts.row_mut(FactKey::Desktop).push(desktop);
		}
		self.fill_terminal(facts);
		if let Some(shell) = self.var(b"SHELL") {
			facts.row_mut(FactKey::Shell).push(basename(shell));
		}
	}

	fn terminal(&mut self) -> TerminalInfo {
		TerminalInfo {
			is_tty: self.screen.is_tty,
			columns: usize::from(self.screen.columns),
			depth_hint: depth::unix_hint(self.var(b"COLORTERM"), self.var(b"TERM")),
			utf8: locale_is_utf8(|name| self.var(name)),
		}
	}

	fn env(&self, name: &[u8]) -> Option<&[u8]> {
		self.var(name)
	}

	fn default_logo(&self) -> &'static str {
		default_logo("linux")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"linux-").push(TARGET_ARCH.as_bytes());
	}
}

/// The architecture part of the target name (SPEC section 2) for the processor this build runs on. The Rust names do not say which
/// Arm, Mips or PowerPC target a build is, so the endianness and the Arm version decide.
const TARGET_ARCH: &str = if cfg!(target_arch = "x86_64") {
	"x86_64"
} else if cfg!(target_arch = "x86") {
	"i586"
} else if cfg!(target_arch = "aarch64") {
	"aarch64"
} else if cfg!(target_arch = "arm") {
	if cfg!(arm_v7) { "armv7" } else { "armv6" }
} else if cfg!(target_arch = "mips") {
	if cfg!(target_endian = "little") { "mipsel" } else { "mips" }
} else if cfg!(target_arch = "mips64") {
	"mips64el"
} else if cfg!(target_arch = "riscv64") {
	"riscv64"
} else if cfg!(target_arch = "powerpc64") {
	"ppc64le"
} else {
	"unknown"
};

/// The whole program. The core's exit code already says when the output could not be written (74): `run` asks the terminal at the end.
pub fn main(args: Args, environment: *const *const u8) -> u8 {
	let mut platform = Linux::new(environment);
	let mut sink = FdSink::new(1);
	let mut out = Out::new(&mut sink);
	run(&mut platform, &mut out, args)
}
