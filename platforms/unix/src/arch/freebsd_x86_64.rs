// FreeBSD on x86-64: the entry stub and the `syscall` instruction. FreeBSD reports a failed call with the carry flag and puts the
// error number in `rax`; the stub turns that into the negative result the shared code expects.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with `rdi` pointing at `argc` (the stack pointer is not that address on FreeBSD). The stub leaves `rdi` as it
// is for the call and aligns the stack.
global_asm!(
	".globl _start",
	".type _start,@function",
	"_start:",
	"xor ebp, ebp",
	"and rsp, -16",
	"call {entry}",
	"ud2",
	entry = sym crate::rt::start,
);

/// Makes a system call with up to six arguments. Returns the result, which is `-errno` on failure.
///
/// # Safety
/// The arguments must be valid for the call `number` names.
pub unsafe fn syscall6(
	number: usize,
	first: usize,
	second: usize,
	third: usize,
	fourth: usize,
	fifth: usize,
	sixth: usize,
) -> isize {
	let result: isize;
	// SAFETY: the caller upholds the call's contract; `syscall` clobbers rcx and r11, and the kernel may change every argument register (rdx is a
	// second result), so all of them are declared as changed. Carry is set on failure.
	unsafe {
		asm!(
			"syscall",
			"jnc 2f",
			"neg rax",
			"2:",
			inlateout("rax") number as isize => result,
			inlateout("rdi") first => _,
			inlateout("rsi") second => _,
			inlateout("rdx") third => _,
			inlateout("r10") fourth => _,
			inlateout("r8") fifth => _,
			inlateout("r9") sixth => _,
			lateout("rcx") _,
			lateout("r11") _,
			options(nostack),
		);
	}
	result
}
