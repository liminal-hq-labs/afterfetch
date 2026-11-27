// macOS: the `libSystem` functions afterfetch calls (and only those), and the `Platform` that reads the facts through `sysctlbyname`.
// macOS has no stable system call interface, so the program imports a handful of C functions from `libSystem`, through the hand-written
// stub in `platforms/unix/macos/libSystem.tbd`; the Apple SDK is not used. The parsing and the layout live in the core.
//
// This file has never run: there is no macOS machine here. A GitHub macOS runner runs it (AF-D5, AF-D56).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ffi::{c_char, c_int, c_ulong, c_void};
use core::ptr;

use afterfetch_core::format::{c_str, format_cpu, format_terminal};
use afterfetch_core::parse::lines::LineFeed;
use afterfetch_core::parse::{basename, model, passwd};
use afterfetch_core::{Buf, FactKey, Facts, Platform, TerminalInfo, default_logo, depth, run};

use crate::rt::{Args, c_bytes};
use crate::term::{FdSink, Out, locale_is_utf8};

// Every symbol imported from `libSystem`. `libSystem.tbd` lists the same names, and `check.sh` keeps the two equal. The three variadic
// functions are declared as such: on ARM64 macOS passes variadic arguments on the stack, not in registers.
unsafe extern "C" {
	fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
	fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
	fn open(path: *const c_char, flags: c_int, ...) -> c_int;
	fn close(fd: c_int) -> c_int;
	fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
	fn sysctlbyname(name: *const c_char, old: *mut c_void, old_length: *mut usize, new: *mut c_void, new_length: usize) -> c_int;
	fn gettimeofday(time: *mut Timeval, zone: *mut c_void) -> c_int;
	fn getuid() -> u32;
	fn _exit(code: c_int) -> !;
	fn __error() -> *mut c_int;
}

/// `struct timeval` on 64-bit macOS: seconds, then microseconds and padding.
#[repr(C)]
#[derive(Clone, Copy)]
struct Timeval {
	seconds: i64,
	microseconds: i32,
	padding: i32,
}

const EINTR: i32 = 4;
const O_RDONLY: c_int = 0;
/// `ioctl` request: read the terminal's settings (succeeds only on a terminal). `_IOR('t', 19, struct termios)`, 72 bytes on macOS.
const TIOCGETA: c_ulong = 0x4048_7413;
/// `ioctl` request: read the terminal's size. `_IOR('t', 104, struct winsize)`.
const TIOCGWINSZ: c_ulong = 0x4008_7468;

/// Ends the program.
pub fn exit(code: u8) -> ! {
	// SAFETY: `_exit` takes any code and does not return.
	unsafe { _exit(c_int::from(code)) }
}

/// True when the call that just failed was interrupted by a signal, from the thread's `errno`.
fn interrupted(result: isize) -> bool {
	// SAFETY: `__error` returns the address of this thread's `errno`.
	result < 0 && unsafe { *__error() } == EINTR
}

/// Writes all of `bytes` to `fd`, retrying after a short write. False when the write fails.
pub fn write_all(fd: i32, mut bytes: &[u8]) -> bool {
	while !bytes.is_empty() {
		// SAFETY: the pointer and length describe `bytes`.
		let written = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
		if interrupted(written) {
			continue;
		}
		if written <= 0 {
			return false;
		}
		bytes = &bytes[written as usize..];
	}
	true
}

/// Reads the file at `path` (NUL-terminated) in chunks, passing each line to `handle`. False when it cannot be opened.
fn read_lines(path: &[u8], mut handle: &mut dyn FnMut(&[u8])) -> bool {
	debug_assert!(path.last() == Some(&0));
	// SAFETY: `path` is NUL-terminated.
	let fd = unsafe { open(path.as_ptr().cast(), O_RDONLY) };
	if fd < 0 {
		return false;
	}
	let mut chunk = [0u8; 1024];
	let mut feed = LineFeed::<192>::new();
	loop {
		// SAFETY: the pointer and length describe `chunk`.
		let count = unsafe { read(fd, chunk.as_mut_ptr().cast(), chunk.len()) };
		if count <= 0 {
			break;
		}
		feed.feed(&chunk[..count as usize], &mut handle);
	}
	feed.finish(&mut handle);
	// SAFETY: `fd` is open.
	unsafe { close(fd) };
	true
}

/// Reads the sysctl `name` (NUL-terminated) into `buffer`, returning the bytes written.
fn sysctl(name: &[u8], buffer: &mut [u8]) -> Option<usize> {
	debug_assert!(name.last() == Some(&0));
	let mut length = buffer.len();
	// SAFETY: `name` is NUL-terminated and `buffer` is `length` bytes long.
	let status = unsafe {
		sysctlbyname(
			name.as_ptr().cast(),
			buffer.as_mut_ptr().cast(),
			&raw mut length,
			ptr::null_mut(),
			0,
		)
	};
	(status == 0).then_some(length.min(buffer.len()))
}

/// A numeric sysctl, 4 or 8 bytes, little-endian on both processors macOS runs on.
fn sysctl_number(name: &[u8]) -> Option<u64> {
	let mut buffer = [0u8; 8];
	let length = sysctl(name, &mut buffer)?;
	(length == 4 || length == 8).then(|| u64::from_le_bytes(buffer) & if length == 4 { 0xffff_ffff } else { u64::MAX })
}

/// A string sysctl, without its NUL, into `out`.
fn sysctl_text<const N: usize>(name: &[u8], out: &mut Buf<N>) -> bool {
	let mut buffer = [0u8; 128];
	let Some(length) = sysctl(name, &mut buffer) else {
		return false;
	};
	out.push(c_str(&buffer[..length]));
	!out.is_empty()
}

/// `ioctl` results for standard output.
#[derive(Clone, Copy)]
struct Screen {
	is_tty: bool,
	columns: u16,
	rows: u16,
}

fn screen() -> Screen {
	let mut settings = [0u8; 128];
	// SAFETY: the buffer is larger than macOS's `struct termios`.
	let is_tty = unsafe { ioctl(1, TIOCGETA, settings.as_mut_ptr()) } == 0;
	let mut size = [0u16; 4];
	// SAFETY: the buffer is a `struct winsize`.
	let known = is_tty && unsafe { ioctl(1, TIOCGWINSZ, size.as_mut_ptr()) } == 0;
	Screen {
		is_tty,
		rows: if known { size[0] } else { 0 },
		columns: if known { size[1] } else { 0 },
	}
}

/// macOS as the core sees it.
pub struct MacOs {
	environment: *const *const u8,
	screen: Screen,
}

impl MacOs {
	fn new(environment: *const *const u8) -> Self {
		MacOs {
			environment,
			screen: screen(),
		}
	}

	fn var(&self, name: &[u8]) -> Option<&[u8]> {
		let mut entry = self.environment;
		// SAFETY: `environment` is the C runtime's null-terminated array of NUL-terminated `NAME=value` strings.
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
		// SAFETY: `getuid` takes no arguments.
		let uid = unsafe { getuid() };
		read_lines(b"/etc/passwd\0", &mut |line| {
			if facts.user.is_empty()
				&& let Some(name) = passwd::name_for_uid(line, uid)
			{
				facts.user.push(name);
			}
		});
	}

	/// `macOS 15.7.1 (arm64)` and `Darwin 24.6.0`.
	fn fill_system(&self, facts: &mut Facts) {
		let mut machine = Buf::<24>::new();
		sysctl_text(b"hw.machine\0", &mut machine);
		facts.arch.push(machine.as_bytes());
		let row = facts.row_mut(FactKey::Os);
		row.push(b"macOS");
		let mut version = Buf::<32>::new();
		if sysctl_text(b"kern.osproductversion\0", &mut version) {
			row.push(b" ").push(version.as_bytes());
		}
		if !machine.is_empty() {
			row.push(b" (").push(machine.as_bytes()).push(b")");
		}
		let mut release = Buf::<32>::new();
		if sysctl_text(b"kern.osrelease\0", &mut release) {
			facts.row_mut(FactKey::Kernel).push(b"Darwin ").push(release.as_bytes());
		}
	}

	fn fill_uptime(&self, facts: &mut Facts) {
		let mut boot = [0u8; 16];
		let mut now = Timeval {
			seconds: 0,
			microseconds: 0,
			padding: 0,
		};
		// SAFETY: `now` is a `struct timeval`; the zone is not wanted.
		if sysctl(b"kern.boottime\0", &mut boot) == Some(16) && unsafe { gettimeofday(&raw mut now, ptr::null_mut()) } == 0 {
			let started = i64::from_le_bytes(boot[..8].try_into().unwrap_or([0; 8]));
			facts.set_uptime((now.seconds - started).max(0) as u64);
		}
	}

	fn fill_cpu(&self, facts: &mut Facts) {
		let mut brand = [0u8; 96];
		let length = sysctl(b"machdep.cpu.brand_string\0", &mut brand).unwrap_or(0);
		let cores = sysctl_number(b"hw.ncpu\0").unwrap_or(0) as u32;
		format_cpu(facts.row_mut(FactKey::Cpu), c_str(&brand[..length]), cores);
	}

	/// Free and speculative pages of the physical memory; speculative pages are read-ahead the system gives up first.
	fn fill_memory(&self, facts: &mut Facts) {
		let total = sysctl_number(b"hw.memsize\0").unwrap_or(0);
		let page = sysctl_number(b"hw.pagesize\0").unwrap_or(0);
		let pages =
			sysctl_number(b"vm.page_free_count\0").unwrap_or(0) + sysctl_number(b"vm.page_speculative_count\0").unwrap_or(0);
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

impl Platform for MacOs {
	fn facts(&mut self, facts: &mut Facts) {
		self.fill_user(facts);
		sysctl_text(b"kern.hostname\0", &mut facts.host);
		self.fill_system(facts);
		let mut contents = [0u8; 80];
		if let Some(count) = sysctl(b"hw.model\0", &mut contents) {
			model::clean(c_str(&contents[..count]), facts.row_mut(FactKey::Host));
		}
		self.fill_uptime(facts);
		self.fill_cpu(facts);
		self.fill_memory(facts);
		// No Display row: there is no CoreGraphics here (AF-D20).
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
		default_logo("macos")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"macos-").push(TARGET_ARCH.as_bytes());
	}
}

/// The architecture part of the target name (SPEC section 2).
const TARGET_ARCH: &str = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };

/// The C entry point: `dyld` calls `main(argc, argv, envp, apple)` through `LC_MAIN`. The core's exit code already says when the output
/// could not be written (74).
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, argv: *const *const u8, environment: *const *const u8) -> c_int {
	// SAFETY: `argv` is the C runtime's null-terminated array, which starts with the program's name.
	let args = unsafe { Args::after_name(argv) };
	let mut platform = MacOs::new(environment);
	let mut sink = FdSink::new(1);
	let mut out = Out::new(&mut sink);
	c_int::from(run(&mut platform, &mut out, args))
}
