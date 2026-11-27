// FreeBSD on ARM64: the entry stub and the `svc` instruction. FreeBSD reports a failed call with the carry flag and puts the error
// number in `x0`; the stub turns that into the negative result the shared code expects.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::{asm, global_asm};

// The kernel starts a program with `x0` pointing at `argc`. The stub leaves `x0` as it is for the call and aligns the stack.
global_asm!(
	".globl _start",
	".type _start,%function",
	"_start:",
	"mov x29, xzr",
	"mov x30, xzr",
	"mov x1, sp",
	"and sp, x1, #-16",
	"bl {entry}",
	"brk #0",
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
	// SAFETY: the caller upholds the call's contract; the kernel may change every argument register (x1 is a second result), so all of them are
	// declared as changed. Carry is set on failure.
	unsafe {
		asm!(
			"svc #0",
			"b.cc 2f",
			"neg x0, x0",
			"2:",
			in("x8") number,
			inlateout("x0") first as isize => result,
			inlateout("x1") second => _,
			inlateout("x2") third => _,
			inlateout("x3") fourth => _,
			inlateout("x4") fifth => _,
			inlateout("x5") sixth => _,
			options(nostack),
		);
	}
	result
}
