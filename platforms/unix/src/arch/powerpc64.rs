// Linux on 64-bit little-endian PowerPC (ELFv2): `sc`, its numbers and the PowerPC `ioctl` requests.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub passes that pointer on, aligns the stack to 16 bytes and
// opens a frame with a zero back chain (ELFv2 reserves 32 bytes for the callee's linkage area). The callee's global entry point
// computes its own TOC pointer from r12, which must hold its address (ELFv2 has no function descriptors), so the address is built in
// r12 and the call goes through the count register. It is an absolute address, which is fine at the fixed base `0x400000`.
global_asm!(
	".globl _start",
	".type _start,@function",
	"_start:",
	"mr 3, 1",
	"clrrdi 1, 1, 4",
	"li 0, 0",
	"stdu 0, -32(1)",
	"lis 12, {entry}@ha",
	"addi 12, 12, {entry}@l",
	"mtctr 12",
	"bctrl",
	"trap",
	entry = sym crate::rt::start,
);

/// System call numbers (the PowerPC table, shared by the 32-bit and 64-bit kernels).
pub mod nr {
	pub const READ: usize = 3;
	pub const WRITE: usize = 4;
	pub const CLOSE: usize = 6;
	pub const GETUID: usize = 24;
	pub const IOCTL: usize = 54;
	pub const SYSINFO: usize = 116;
	pub const UNAME: usize = 122;
	pub const EXIT_GROUP: usize = 234;
	pub const OPENAT: usize = 286;
}

/// `ioctl` request: read the terminal's settings (succeeds only on a terminal). PowerPC's kernel `termios` is 44 bytes, so the request is
/// `_IOR('t', 19, 44 bytes)`, 0x402C7413, where x86 uses 0x5401.
pub const TCGETS: usize = 0x402C_7413;
/// `ioctl` request: read the terminal's size. `_IOR('t', 104, struct winsize)`; x86 uses 0x5413.
pub const TIOCGWINSZ: usize = 0x4008_7468;

/// Makes a system call with up to three arguments. Returns the result, which is `-errno` on failure.
///
/// The kernel returns the call's result in r3 and sets the summary-overflow bit of condition register 0 on failure, with the positive
/// error number in r3.
///
/// # Safety
/// The arguments must be valid for the call `number` names.
pub unsafe fn syscall3(number: usize, first: usize, second: usize, third: usize) -> isize {
	let result: isize;
	// SAFETY: the caller upholds the call's contract. The kernel preserves the non-volatile registers and may change r0, r4 to r12, the
	// count register and condition register 0, so they are declared as clobbered.
	unsafe {
		asm!(
			"sc",
			"bns+ 2f",
			"neg 3, 3",
			"2:",
			inlateout("r0") number => _,
			inlateout("r3") first as isize => result,
			inlateout("r4") second => _,
			inlateout("r5") third => _,
			lateout("r6") _,
			lateout("r7") _,
			lateout("r8") _,
			lateout("r9") _,
			lateout("r10") _,
			lateout("r11") _,
			lateout("r12") _,
			lateout("ctr") _,
			lateout("cr0") _,
			options(nostack),
		);
	}
	result
}
