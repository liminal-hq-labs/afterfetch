// FreeBSD: the system calls afterfetch makes (and only those), and the `Platform` that reads the facts through `__sysctl`, `kenv` and
// `clock_gettime`. The parsing and the layout live in the core; this file asks the kernel and hands the answers over.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::format::{c_str, format_cpu, format_terminal};
use afterfetch_core::parse::lines::LineFeed;
use afterfetch_core::parse::{basename, model, passwd};
use afterfetch_core::{Buf, FactKey, Facts, Platform, TerminalInfo, default_logo, depth, run};

use crate::arch;
use crate::rt::{Args, c_bytes};
use crate::term::{FdSink, Out, locale_is_utf8};

/// Every system call the program makes. `syscalls-freebsd.allow` lists the same names, and `check.sh` keeps the two equal. The numbers
/// are the same on every processor.
#[derive(Clone, Copy)]
enum Sys {
	Exit,
	Read,
	Write,
	Close,
	Getuid,
	Ioctl,
	Sysctl,
	ClockGettime,
	Kenv,
	OpenAt,
}

impl Sys {
	const fn number(self) -> usize {
		match self {
			Sys::Exit => 1,
			Sys::Read => 3,
			Sys::Write => 4,
			Sys::Close => 6,
			Sys::Getuid => 24,
			Sys::Ioctl => 54,
			Sys::Sysctl => 202,
			Sys::ClockGettime => 232,
			Sys::Kenv => 390,
			Sys::OpenAt => 499,
		}
	}
}

fn call(sys: Sys, first: usize, second: usize, third: usize, fourth: usize, fifth: usize, sixth: usize) -> isize {
	// SAFETY: every caller in this file passes pointers to live buffers of the length it passes beside them.
	unsafe { arch::syscall6(sys.number(), first, second, third, fourth, fifth, sixth) }
}

const EINTR: isize = -4;
const AT_FDCWD: usize = -100isize as usize;
const O_RDONLY: usize = 0;
const O_CLOEXEC: usize = 0x0010_0000;
/// `ioctl` request: read the terminal's settings (succeeds only on a terminal). `_IOR('t', 19, struct termios)`.
const TIOCGETA: usize = 0x402c_7413;
/// `ioctl` request: read the terminal's size. `_IOR('t', 104, struct winsize)`.
const TIOCGWINSZ: usize = 0x4008_7468;
/// `CLOCK_UPTIME`, also called `CLOCK_BOOTTIME`: seconds since boot, not counting time asleep.
const CLOCK_UPTIME: usize = 5;
/// `kenv` action: read one variable.
const KENV_GET: usize = 0;
/// `sysctl.name2oid`: the pseudo-node that turns a name into its numeric identifier.
const NAME_TO_OID: [i32; 2] = [0, 3];

/// Ends the program.
pub fn exit(code: u8) -> ! {
	call(Sys::Exit, usize::from(code), 0, 0, 0, 0, 0);
	loop {
		core::hint::spin_loop();
	}
}

/// Writes all of `bytes` to `fd`, retrying after a signal and a short write. False when the write fails.
pub fn write_all(fd: i32, mut bytes: &[u8]) -> bool {
	while !bytes.is_empty() {
		let written = call(Sys::Write, fd as usize, bytes.as_ptr() as usize, bytes.len(), 0, 0, 0);
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
		let count = call(Sys::Read, fd as usize, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0);
		if count != EINTR {
			return count.max(0) as usize;
		}
	}
}

/// Reads the file at `path` (NUL-terminated) in chunks, passing each line to `handle`. False when it cannot be opened.
fn read_lines(path: &[u8], mut handle: &mut dyn FnMut(&[u8])) -> bool {
	debug_assert!(path.last() == Some(&0));
	let fd = call(Sys::OpenAt, AT_FDCWD, path.as_ptr() as usize, O_RDONLY | O_CLOEXEC, 0, 0, 0);
	if fd < 0 {
		return false;
	}
	let fd = fd as i32;
	let mut chunk = [0u8; 1024];
	let mut feed = LineFeed::<192>::new();
	loop {
		let count = read(fd, &mut chunk);
		if count == 0 {
			break;
		}
		feed.feed(&chunk[..count], &mut handle);
	}
	feed.finish(&mut handle);
	call(Sys::Close, fd as usize, 0, 0, 0, 0, 0);
	true
}

/// One `__sysctl` call: the identifier `mib`, the answer into `buffer` and its length (the capacity in, the size out), and optionally a
/// new value. False when the kernel refuses.
fn sysctl_raw(mib: &[i32], buffer: *mut u8, length: &mut usize, new: &[u8]) -> bool {
	let new_pointer = if new.is_empty() { 0 } else { new.as_ptr() as usize };
	call(
		Sys::Sysctl,
		mib.as_ptr() as usize,
		mib.len(),
		buffer as usize,
		length as *mut usize as usize,
		new_pointer,
		new.len(),
	) == 0
}

/// Reads the sysctl `name` (no NUL) into `buffer`, returning the bytes written. The name becomes its numeric identifier first, as
/// `sysctlbyname` does.
fn sysctl(name: &[u8], buffer: &mut [u8]) -> Option<usize> {
	let mut oid = [0i32; 24];
	let mut oid_bytes = size_of_val(&oid);
	if !sysctl_raw(&NAME_TO_OID, oid.as_mut_ptr().cast(), &mut oid_bytes, name) {
		return None;
	}
	let mut length = buffer.len();
	sysctl_raw(&oid[..oid_bytes / size_of::<i32>()], buffer.as_mut_ptr(), &mut length, b"").then_some(length.min(buffer.len()))
}

/// A numeric sysctl, 4 or 8 bytes, little-endian on every processor FreeBSD builds here.
fn sysctl_number(name: &[u8]) -> Option<u64> {
	let mut buffer = [0u8; 8];
	let length = sysctl(name, &mut buffer)?;
	(length == 4 || length == 8).then(|| u64::from_le_bytes(buffer) & if length == 4 { 0xffff_ffff } else { u64::MAX })
}

/// A string sysctl, without its NUL, into `out`.
fn sysctl_text<const N: usize>(name: &[u8], out: &mut Buf<N>) -> bool {
	let mut buffer = [0u8; 96];
	let Some(length) = sysctl(name, &mut buffer) else {
		return false;
	};
	out.push(c_str(&buffer[..length]));
	!out.is_empty()
}

/// The kernel environment variable `name` (NUL-terminated) into `buffer`; 0 when it is absent or hidden.
fn kenv(name: &[u8], buffer: &mut [u8]) -> usize {
	let length = call(
		Sys::Kenv,
		KENV_GET,
		name.as_ptr() as usize,
		buffer.as_mut_ptr() as usize,
		buffer.len(),
		0,
		0,
	);
	length.clamp(0, buffer.len() as isize) as usize
}

/// Seconds since boot.
fn uptime_seconds() -> Option<u64> {
	let mut time = [0i64; 2];
	(call(Sys::ClockGettime, CLOCK_UPTIME, time.as_mut_ptr() as usize, 0, 0, 0, 0) == 0).then_some(time[0].max(0) as u64)
}

fn getuid() -> u32 {
	call(Sys::Getuid, 0, 0, 0, 0, 0, 0) as u32
}

/// `ioctl` results for standard output.
#[derive(Clone, Copy)]
struct Screen {
	is_tty: bool,
	columns: u16,
	rows: u16,
}

fn screen() -> Screen {
	// FreeBSD's `struct termios` is 44 bytes.
	let mut settings = [0u8; 64];
	let is_tty = call(Sys::Ioctl, 1, TIOCGETA, settings.as_mut_ptr() as usize, 0, 0, 0) == 0;
	let mut size = [0u16; 4];
	let known = is_tty && call(Sys::Ioctl, 1, TIOCGWINSZ, size.as_mut_ptr() as usize, 0, 0, 0) == 0;
	Screen {
		is_tty,
		rows: if known { size[0] } else { 0 },
		columns: if known { size[1] } else { 0 },
	}
}

/// FreeBSD as the core sees it.
pub struct FreeBsd {
	environment: *const *const u8,
	screen: Screen,
}

impl FreeBsd {
	fn new(environment: *const *const u8) -> Self {
		FreeBsd {
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

	/// The OS row is `FreeBSD 15.1-RELEASE (amd64)`; the kernel row is the name, release and configuration from the first line of `kern.version`
	/// (`FreeBSD 15.1-RELEASE GENERIC`).
	fn fill_system(&self, facts: &mut Facts) {
		let mut machine = Buf::<24>::new();
		sysctl_text(b"hw.machine", &mut machine);
		facts.arch.push(machine.as_bytes());
		let row = facts.row_mut(FactKey::Os);
		if !sysctl_text(b"kern.ostype", row) {
			row.push(b"FreeBSD");
		}
		let mut release = Buf::<32>::new();
		if sysctl_text(b"kern.osrelease", &mut release) {
			row.push(b" ").push(release.as_bytes());
		}
		if !machine.is_empty() {
			row.push(b" (").push(machine.as_bytes()).push(b")");
		}
		let mut buffer = [0u8; 160];
		if let Some(length) = sysctl(b"kern.version", &mut buffer) {
			let first_line = buffer[..length]
				.split(|&byte| byte == b'\n' || byte == 0)
				.next()
				.unwrap_or(b"");
			// `FreeBSD 15.1-RELEASE releng/15.1-n283562-96841ea08dcf GENERIC` is shown as `FreeBSD 15.1-RELEASE GENERIC`: the build's
			// branch and hash say nothing a reader wants.
			let mut words = first_line.split(|&byte| byte == b' ').filter(|word| !word.is_empty());
			let row = facts.row_mut(FactKey::Kernel);
			let (name, release) = (words.next(), words.next());
			if let (Some(name), Some(release)) = (name, release) {
				row.push(name).push(b" ").push(release);
				if let Some(config) = words.next_back() {
					row.push(b" ").push(config);
				}
			} else {
				row.push(first_line);
			}
		}
	}

	fn fill_model(&self, facts: &mut Facts) {
		let mut contents = [0u8; 80];
		let count = kenv(b"smbios.system.product\0", &mut contents);
		model::clean(&contents[..count], facts.row_mut(FactKey::Host));
	}

	fn fill_cpu(&self, facts: &mut Facts) {
		let mut brand = [0u8; 96];
		let length = sysctl(b"hw.model", &mut brand).unwrap_or(0);
		let cores = sysctl_number(b"hw.ncpu").unwrap_or(0) as u32;
		format_cpu(facts.row_mut(FactKey::Cpu), c_str(&brand[..length]), cores);
	}

	/// Free and inactive pages (the inactive ones are reclaimed before anything is refused) of the physical memory.
	fn fill_memory(&self, facts: &mut Facts) {
		let total = sysctl_number(b"hw.physmem").unwrap_or(0);
		let page = sysctl_number(b"hw.pagesize").unwrap_or(0);
		let pages =
			sysctl_number(b"vm.stats.vm.v_free_count").unwrap_or(0) + sysctl_number(b"vm.stats.vm.v_inactive_count").unwrap_or(0);
		if total > 0 {
			facts.set_memory((pages * page).min(total), total);
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

impl Platform for FreeBsd {
	fn facts(&mut self, facts: &mut Facts) {
		self.fill_user(facts);
		sysctl_text(b"kern.hostname", &mut facts.host);
		self.fill_system(facts);
		self.fill_model(facts);
		if let Some(seconds) = uptime_seconds() {
			facts.set_uptime(seconds);
		}
		self.fill_cpu(facts);
		self.fill_memory(facts);
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
		default_logo("freebsd")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"freebsd-").push(TARGET_ARCH.as_bytes());
	}
}

/// The architecture part of the target name (SPEC section 2).
const TARGET_ARCH: &str = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };

/// The whole program. The core's exit code already says when the output could not be written (74): `run` asks the terminal at the end.
pub fn main(args: Args, environment: *const *const u8) -> u8 {
	let mut platform = FreeBsd::new(environment);
	let mut sink = FdSink::new(1);
	let mut out = Out::new(&mut sink);
	run(&mut platform, &mut out, args)
}
