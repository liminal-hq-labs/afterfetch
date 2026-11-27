// Linux on 64-bit RISC-V: the `ecall` instruction and its numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on and aligns the stack to 16 bytes. The
// call is a `jal` through a register-free pair, which is fine at the fixed base address.
global_asm!(
	".globl _start",
	".type _start,@function",
	"_start:",
	"mv fp, zero",
	"mv ra, zero",
	"mv a0, sp",
	"andi sp, a0, -16",
	"call {entry}",
	"ebreak",
	entry = sym crate::rt::start,
);

/// System call numbers. RISC-V has the generic table: no `open`, only `openat`.
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
	// SAFETY: the caller upholds the call's contract; `ecall` keeps every register except a0, which holds the result.
	unsafe {
		asm!(
			"ecall",
			in("a7") number,
			inlateout("a0") first as isize => result,
			in("a1") second,
			in("a2") third,
			options(nostack),
		);
	}
	result
}
