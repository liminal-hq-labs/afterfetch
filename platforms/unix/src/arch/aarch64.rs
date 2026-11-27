// Linux on ARM64: the `svc` instruction and its numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on and aligns the stack for the call.
global_asm!(
	".globl _start",
	".type _start,%function",
	"_start:",
	"mov x29, xzr",
	"mov x30, xzr",
	"mov x0, sp",
	"and sp, x0, #-16",
	"bl {entry}",
	"brk #0",
	entry = sym crate::rt::start,
);

/// System call numbers. ARM64 has the generic table: no `open`, only `openat`.
pub mod nr {
	pub const READ: usize = 63;
	pub const WRITE: usize = 64;
	pub const CLOSE: usize = 57;
	pub const IOCTL: usize = 29;
	pub const UNAME: usize = 160;
	pub const GETUID: usize = 174;
	pub const SYSINFO: usize = 179;
	pub const EXIT_GROUP: usize = 94;
	pub const OPENAT: usize = 56;
}

/// `ioctl` request: read the terminal's settings (succeeds only on a terminal).
pub const TCGETS: usize = 0x5401;
/// `ioctl` request: read the terminal's size.
pub const TIOCGWINSZ: usize = 0x5413;

/// Makes a system call with up to three arguments. Returns the result, which is `-errno` on failure.
///
/// # Safety
/// The arguments must be valid for the call `number` names.
pub unsafe fn syscall3(number: usize, first: usize, second: usize, third: usize) -> isize {
	let result: isize;
	// SAFETY: the caller upholds the call's contract; `svc` keeps every register except x0, which holds the result.
	unsafe {
		asm!(
			"svc #0",
			in("x8") number,
			inlateout("x0") first as isize => result,
			in("x1") second,
			in("x2") third,
			options(nostack),
		);
	}
	result
}
