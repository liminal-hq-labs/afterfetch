// afterfetch.exe: an Afterglow sunset and a few facts about this system, in the console's own colours.
//
// A Win32 console program for ReactOS and Windows 95 and later, `no_std` and `no_main` so that neither Rust's standard library nor the C
// runtime is needed. The colours are console attributes (SetConsoleTextAttribute), the facts come from native calls, and the console's
// own attributes are put back at the end of every line and at exit. Plain text when the output is a pipe or a file.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

mod os;
mod processor;
mod rt;
#[cfg(not(target_arch = "x86"))]
mod rt_probe;
#[cfg(target_arch = "x86")]
mod rt_x86;
mod sys;
mod term;

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
	let code = os::main();
	// SAFETY: ends the process.
	unsafe { sys::ExitProcess(u32::from(code)) }
}
