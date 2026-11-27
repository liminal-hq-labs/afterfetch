// Linux on 32-bit x86 (i586): `int 0x80` and its numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with the stack pointing at `argc`. The stub aligns the stack, then pushes that pointer as the one argument.
global_asm!(
	".globl _start",
	".type _start,@function",
	"_start:",
	"xor ebp, ebp",
	"mov eax, esp",
	"and esp, -16",
	"sub esp, 12",
	"push eax",
	"call {entry}",
	"ud2",
	entry = sym crate::rt::start,
);

/// System call numbers.
pub mod nr {
	pub const EXIT_GROUP: usize = 252;
	pub const READ: usize = 3;
	pub const WRITE: usize = 4;
	pub const CLOSE: usize = 6;
	pub const IOCTL: usize = 54;
	pub const UNAME: usize = 122;
	pub const SYSINFO: usize = 116;
	/// `getuid32`, which takes 32-bit ids.
	pub const GETUID: usize = 199;
	pub const OPENAT: usize = 295;
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
	// SAFETY: the caller upholds the call's contract. LLVM reserves ebx, so the first argument is swapped into it around the call.
	unsafe {
		asm!(
			"xchg {first:e}, ebx",
			"int 0x80",
			"xchg {first:e}, ebx",
			first = inout(reg) first => _,
			inlateout("eax") number as isize => result,
			in("ecx") second,
			in("edx") third,
			options(nostack),
		);
	}
	result
}
