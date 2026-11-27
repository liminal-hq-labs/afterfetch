// Linux on 32-bit MIPS (o32, big and little endian): `syscall`, its numbers (offset by 4000) and MIPS's own `ioctl` requests.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on, aligns the stack to 8 bytes and leaves
// the 16-byte argument area the o32 convention reserves for the callee. A call goes through `$t9`, which the convention wants holding
// the callee's address. The assembler is told not to reorder, so the delay slot after `jalr` is the `nop` written here.
global_asm!(
	".set push",
	".set noreorder",
	".set noat",
	".globl _start",
	".type _start,@function",
	"_start:",
	"move $4, $29",
	"li $8, -8",
	"and $29, $4, $8",
	"addiu $29, $29, -16",
	"move $30, $0",
	"move $31, $0",
	"lui $25, %hi({entry})",
	"addiu $25, $25, %lo({entry})",
	"jalr $25",
	"nop",
	"break 0",
	"nop",
	".set pop",
	entry = sym crate::rt::start,
);

/// System call numbers (the o32 table, which starts at 4000).
pub mod nr {
	pub const READ: usize = 4000 + 3;
	pub const WRITE: usize = 4000 + 4;
	pub const CLOSE: usize = 4000 + 6;
	pub const GETUID: usize = 4000 + 24;
	pub const IOCTL: usize = 4000 + 54;
	pub const SYSINFO: usize = 4000 + 116;
	pub const UNAME: usize = 4000 + 122;
	pub const EXIT_GROUP: usize = 4000 + 246;
	pub const OPENAT: usize = 4000 + 288;
}

/// `ioctl` request: read the terminal's settings (succeeds only on a terminal). MIPS numbers it 0x540D, where x86 uses 0x5401.
pub const TCGETS: usize = 0x540D;
/// `ioctl` request: read the terminal's size. `_IOR('t', 104, struct winsize)` in MIPS's layout, where the read direction is bit 30 (0x4000_0000) and the size field is 13 bits; x86 uses 0x5413.
pub const TIOCGWINSZ: usize = 0x4008_7468;

/// Makes a system call with up to three arguments. Returns the result, which is `-errno` on failure.
///
/// The kernel returns the call's result in `$v0` and sets `$a3` to 1 on failure, with the positive error number in `$v0`.
///
/// # Safety
/// The arguments must be valid for the call `number` names.
pub unsafe fn syscall3(number: usize, first: usize, second: usize, third: usize) -> isize {
	let result: isize;
	let failed: usize;
	// SAFETY: the caller upholds the call's contract. The kernel preserves every register except `$v0` and `$a3`, and is free to change
	// `$v1`, the temporaries and the multiply registers, so they are declared as clobbered.
	unsafe {
		asm!(
			"syscall",
			inlateout("$2") number as isize => result,
			in("$4") first,
			in("$5") second,
			inlateout("$6") third => _,
			lateout("$7") failed,
			lateout("$3") _,
			lateout("$8") _,
			lateout("$9") _,
			lateout("$10") _,
			lateout("$11") _,
			lateout("$12") _,
			lateout("$13") _,
			lateout("$14") _,
			lateout("$15") _,
			lateout("$24") _,
			lateout("$25") _,
			options(nostack),
		);
	}
	if failed != 0 { -result } else { result }
}
