// Linux on 32-bit ARM (EABI, ARMv6 and ARMv7, hard float): `svc` and its numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on and aligns the stack for the call.
global_asm!(
	".arm",
	".globl _start",
	".type _start,%function",
	"_start:",
	"mov fp, #0",
	"mov lr, #0",
	"mov r0, sp",
	"bic sp, r0, #15",
	"bl {entry}",
	"udf #0",
	entry = sym crate::rt::start,
);

/// System call numbers (the EABI table).
pub mod nr {
	pub const READ: usize = 3;
	pub const WRITE: usize = 4;
	pub const CLOSE: usize = 6;
	pub const IOCTL: usize = 54;
	pub const UNAME: usize = 122;
	pub const SYSINFO: usize = 116;
	/// `getuid32`, which takes 32-bit ids.
	pub const GETUID: usize = 199;
	pub const EXIT_GROUP: usize = 248;
	pub const OPENAT: usize = 322;
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
	// SAFETY: the caller upholds the call's contract. Thumb code reserves r7 (the frame pointer), so the call number goes in through a
	// scratch register and r7 is saved in r12 around the call; `svc 0` keeps every register except r0, which holds the result.
	unsafe {
		asm!(
			"mov r12, r7",
			"mov r7, {number}",
			"svc 0",
			"mov r7, r12",
			number = in(reg) number,
			out("r12") _,
			inlateout("r0") first as isize => result,
			in("r1") second,
			in("r2") third,
			options(nostack),
		);
	}
	result
}
