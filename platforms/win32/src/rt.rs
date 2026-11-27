// What a program with no C library has to supply: the panic handler, and on the Pentium build the helpers the 32-bit MSVC ABI expects
// (`rt_x86.rs`). Nothing here allocates, so there is no allocator. The memory routines (`memcpy` and the rest) come from `compiler_builtins`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::sys::ExitProcess;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
	// SAFETY: ends the process. 70 is the crash code on every platform (SPEC.md).
	unsafe { ExitProcess(u32::from(afterfetch_core::run::EXIT_CRASH)) }
}

/// Tells the MSVC linker that floating point is in use somewhere (compiler-builtins mentions it); nothing of ours uses any.
#[unsafe(no_mangle)]
pub static _fltused: i32 = 0;
