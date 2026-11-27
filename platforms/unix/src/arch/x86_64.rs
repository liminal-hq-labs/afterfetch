// Linux on x86_64: the `syscall` instruction and its numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on and aligns the stack for the call.
global_asm!(
	".globl _start",
	".type _start,@function",
	"_start:",
	"xor ebp, ebp",
	"mov rdi, rsp",
	"and rsp, -16",
	"call {entry}",
	"ud2",
	entry = sym crate::rt::start,
);

/// System call numbers.
pub mod nr {
	pub const READ: usize = 0;
	pub const WRITE: usize = 1;
	pub const CLOSE: usize = 3;
	pub const IOCTL: usize = 16;
	pub const UNAME: usize = 63;
	pub const GETUID: usize = 102;
	pub const SYSINFO: usize = 99;
	pub const EXIT_GROUP: usize = 231;
	pub const OPENAT: usize = 257;
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
	// SAFETY: the caller upholds the call's contract; `syscall` clobbers rcx and r11 only.
	unsafe {
		asm!(
			"syscall",
			inlateout("rax") number as isize => result,
			in("rdi") first,
			in("rsi") second,
			in("rdx") third,
			lateout("rcx") _,
			lateout("r11") _,
			options(nostack),
		);
	}
	result
}
