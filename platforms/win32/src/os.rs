// Windows as the core sees it: the `Platform` that reads the facts from the Win32 calls on `imports.allow`, and the program's whole run.
// Every call is an ANSI entry point that Windows 95 has.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ffi::CStr;
use core::mem::MaybeUninit;
use core::ptr::null_mut;

use afterfetch_core::format::{c_str, format_display, format_kernel, format_memory, format_os, format_shell, format_uptime_ms};
use afterfetch_core::{Buf, Depth, FactKey, Facts, LogoName, Platform, TerminalInfo, run};
use afterfetch_win32::cmdline::CommandLine;

use crate::sys::*;
use crate::term::Out;

/// The environment variables the core and the facts read. Windows has no environment block to walk without a call that Windows 95 lacks,
/// so these are asked for once at the start.
const VARIABLES: [&CStr; 4] = [c"NO_COLOR", c"CLICOLOR_FORCE", c"FORCE_COLOR", c"COMSPEC"];
const VALUE_MAX: usize = 260;

/// A system as the core asks about it.
pub struct Windows {
	/// The console's attributes and width, or None when standard output is not a console.
	console: Option<(u16, usize)>,
	values: [([u8; VALUE_MAX], usize); VARIABLES.len()],
}

/// Standard output, and the console's attributes and width when it is a console.
fn standard_output() -> (Handle, Option<(u16, usize)>) {
	// SAFETY: asking for a standard handle has no preconditions; the console call only fills the structure it is given.
	unsafe {
		let handle = GetStdHandle(STD_OUTPUT_HANDLE);
		if handle.is_null() || handle == INVALID_HANDLE_VALUE || GetFileType(handle) != FILE_TYPE_CHAR {
			return (handle, None);
		}
		let mut screen = MaybeUninit::<ConsoleScreenBufferInfo>::zeroed();
		if GetConsoleScreenBufferInfo(handle, screen.as_mut_ptr()) == 0 {
			return (handle, None);
		}
		let screen = screen.assume_init();
		let columns = (i32::from(screen.window.right) - i32::from(screen.window.left) + 1).max(1) as usize;
		(handle, Some((screen.attributes, columns)))
	}
}

impl Windows {
	fn new(console: Option<(u16, usize)>) -> Self {
		let mut values = [([0u8; VALUE_MAX], 0usize); VARIABLES.len()];
		for (variable, (value, length)) in VARIABLES.iter().zip(values.iter_mut()) {
			// SAFETY: the name ends in a NUL and the length passed is the buffer's.
			let found =
				unsafe { GetEnvironmentVariableA(variable.as_ptr().cast(), value.as_mut_ptr(), VALUE_MAX as u32) } as usize;
			// Zero is unset or empty; a length past the buffer means the value did not fit.
			*length = if found < VALUE_MAX { found } else { 0 };
		}
		Windows { console, values }
	}

	fn var(&self, name: &[u8]) -> Option<&[u8]> {
		let at = VARIABLES
			.iter()
			.position(|variable| variable.to_bytes().eq_ignore_ascii_case(name))?;
		let (value, length) = &self.values[at];
		(*length > 0).then(|| &value[..*length])
	}

	fn fill_names(&self, facts: &mut Facts) {
		facts.user.push(name_from(GetUserNameA, &mut [0u8; 64]));
		facts.user.make_ascii_lowercase();
		facts.host.push(name_from(GetComputerNameA, &mut [0u8; 64]));
		facts.host.make_ascii_lowercase();
	}

	fn fill_system(&self, facts: &mut Facts) {
		let mut version = MaybeUninit::<OsVersionInfoA>::zeroed();
		// SAFETY: zeroed is a valid OsVersionInfoA; the size field is set before the call.
		let version = unsafe {
			(*version.as_mut_ptr()).size = size_of::<OsVersionInfoA>() as u32;
			(GetVersionExA(version.as_mut_ptr()) != 0).then(|| version.assume_init())
		};
		if let Some(version) = &version {
			// Windows 9x keeps flags in the high word of the build number.
			let build = if version.platform == PLATFORM_NT {
				version.build
			} else {
				version.build & 0xFFFF
			};
			format_kernel(
				facts.row_mut(FactKey::Kernel),
				version.major,
				version.minor,
				build,
				c_str(&version.service_pack),
			);
		}

		// The product name and the build lab, which for ReactOS carries the release.
		let key = c"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
		let (mut product, mut lab) = ([0u8; 96], [0u8; 96]);
		let product = registry_string(key, c"ProductName", &mut product);
		let build_lab = registry_string(key, c"BuildLab", &mut lab);
		let mut system = MaybeUninit::<SystemInfo>::zeroed();
		// SAFETY: the call fills the structure it is given.
		let architecture = unsafe {
			GetSystemInfo(system.as_mut_ptr());
			system.assume_init().processor_architecture
		};
		let row = facts.row_mut(FactKey::Os);
		format_os(row, product, build_lab, architecture == ARCHITECTURE_X86);
		// A known ReactOS release carries its architecture already (`0.4.16-x86`); any other system gets it in brackets.
		if !row.as_bytes().ends_with(b"-x86") {
			let name: &[u8] = match architecture {
				ARCHITECTURE_X86 => b"x86",
				ARCHITECTURE_X86_64 => b"x86_64",
				ARCHITECTURE_ARM => b"arm",
				ARCHITECTURE_ARM64 => b"aarch64",
				_ => b"",
			};
			if !name.is_empty() {
				row.push(b" (").push(name).push(b")");
			}
		}
	}

	fn fill_resources(&self, facts: &mut Facts) {
		// SAFETY: a call with no arguments.
		format_uptime_ms(facts.row_mut(FactKey::Uptime), unsafe { GetTickCount() });

		let mut memory = MaybeUninit::<MemoryStatus>::zeroed();
		// SAFETY: the length field is set before the call; the call fills the rest.
		let memory = unsafe {
			(*memory.as_mut_ptr()).length = size_of::<MemoryStatus>() as u32;
			GlobalMemoryStatus(memory.as_mut_ptr());
			memory.assume_init()
		};
		format_memory(
			facts.row_mut(FactKey::Memory),
			u64::from(memory.avail_phys),
			u64::from(memory.total_phys),
		);

		// SAFETY: GetSystemMetrics takes no pointers; the screen DC is released after the colour depth is read.
		let (width, height, depth) = unsafe {
			let dc = GetDC(null_mut());
			let depth = if dc.is_null() {
				0
			} else {
				let depth = (GetDeviceCaps(dc, BITSPIXEL) * GetDeviceCaps(dc, PLANES)) as u32;
				ReleaseDC(null_mut(), dc);
				depth
			};
			(GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN), depth)
		};
		if width > 0 && height > 0 {
			format_display(facts.row_mut(FactKey::Display), width as u32, height as u32, depth);
		}
	}
}

/// Fills `buffer` from a Win32 call that takes a size in and gives the length out, and returns what was written.
fn name_from(call: unsafe extern "system" fn(*mut u8, *mut u32) -> i32, buffer: &mut [u8]) -> &[u8] {
	let mut size = buffer.len() as u32;
	// SAFETY: `size` is the length of `buffer`.
	if unsafe { call(buffer.as_mut_ptr(), &mut size) } == 0 {
		return &[];
	}
	c_str(buffer)
}

/// A string value of a key under HKEY_LOCAL_MACHINE, or an empty slice.
fn registry_string<'a>(key: &CStr, name: &CStr, buffer: &'a mut [u8]) -> &'a [u8] {
	let mut handle = 0usize;
	// SAFETY: `key` and `name` end in a NUL; the buffer length is passed in `size`.
	unsafe {
		if RegOpenKeyExA(HKEY_LOCAL_MACHINE, key.as_ptr().cast(), 0, KEY_QUERY_VALUE, &mut handle) != ERROR_SUCCESS {
			return &[];
		}
		let mut size = buffer.len() as u32 - 1;
		let mut kind = 0u32;
		let status = RegQueryValueExA(
			handle,
			name.as_ptr().cast(),
			null_mut(),
			&mut kind,
			buffer.as_mut_ptr(),
			&mut size,
		);
		RegCloseKey(handle);
		if status != ERROR_SUCCESS || kind != REG_SZ {
			return &[];
		}
		buffer[size as usize] = 0;
	}
	c_str(buffer)
}

impl Platform for Windows {
	fn facts(&mut self, facts: &mut Facts) {
		self.fill_names(facts);
		self.fill_system(facts);
		self.fill_resources(facts);
		format_shell(facts.row_mut(FactKey::Shell), self.var(b"COMSPEC").unwrap_or(b""));
		// No desktop row: a console program cannot honestly name one (AF-D20).
	}

	fn terminal(&mut self) -> TerminalInfo {
		TerminalInfo {
			is_tty: self.console.is_some(),
			columns: self.console.map_or(0, |(_, columns)| columns),
			depth_hint: Depth::Console16,
			utf8: false,
		}
	}

	fn env(&self, name: &[u8]) -> Option<&[u8]> {
		self.var(name)
	}

	fn default_logo(&self) -> LogoName {
		LogoName::Win32
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"windows-").push(
			if cfg!(target_arch = "x86") {
				"i586"
			} else {
				afterfetch_core::run::build_arch()
			}
			.as_bytes(),
		);
	}
}

/// The whole program: the exit code is the core's, or 74 when the output could not be written.
pub fn main() -> u8 {
	let (handle, console) = standard_output();
	let mut platform = Windows::new(console);
	let mut out = Out::new(handle, console.map(|(attributes, _)| attributes));
	// SAFETY: the command line is a NUL-terminated string that lives as long as the process.
	let line = unsafe { CStr::from_ptr(GetCommandLineA().cast()) };
	let command_line = CommandLine::parse(line.to_bytes());
	let code = run(&mut platform, &mut out, command_line.args());
	// Puts the console's attributes back too, which matters when a run was cut short.
	let written = out.finish();
	if code == 0 && !written { 74 } else { code }
}
