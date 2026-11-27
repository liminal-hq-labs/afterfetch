// The command tail and the environment, both read from the PSP.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The stub's information block holds a selector for the PSP, whose base is the PSP's linear address. The command tail is the length byte
// at PSP:80h and up to 126 characters after it. The word at PSP:2Ch is the environment, but as a protected-mode selector, not a real-mode
// segment: the DPMI host rewrites it when the program enters protected mode. Its base is the environment's linear address and its limit
// the size of the block. The runtime reaches all of it through the flat data segment (conventional memory is mapped at linear 0), so it
// only asks the host for the bases.

use crate::rt::{dpmi, start};

fn psp() -> *const u8 {
	dpmi::linear_to_ptr::<u8>(dpmi::segment_base(start::psp_selector()).unwrap_or(0))
}

/// The raw command tail: what follows the program's name, without the carriage return that ends it.
pub fn command_tail() -> &'static [u8] {
	let psp = psp();
	// SAFETY: the PSP is 256 bytes at `psp`; the tail is `length` <= 127 bytes at offset 0x81.
	unsafe {
		let length = usize::from(*psp.add(0x80)).min(127);
		let tail = core::slice::from_raw_parts(psp.add(0x81), length);
		&tail[..tail.iter().position(|&byte| byte == b'\r').unwrap_or(length)]
	}
}

/// The environment block: `NAME=value` strings each ending in NUL, then one more NUL.
pub fn block() -> &'static [u8] {
	let psp = psp();
	// SAFETY: PSP:2Ch is a word in the PSP. The selector's limit bounds the environment, which sits in conventional memory.
	unsafe {
		let selector = u16::from_le_bytes([*psp.add(0x2C), *psp.add(0x2D)]);
		let base = dpmi::segment_base(selector).unwrap_or(0);
		core::slice::from_raw_parts(dpmi::linear_to_ptr::<u8>(base), selector_limit(selector).min(32768) as usize)
	}
}

/// The limit of a selector in bytes (LSL), 0 if the selector is not valid.
fn selector_limit(selector: u16) -> u32 {
	let (limit, valid): (u32, u32);
	// SAFETY: LSL reads a descriptor field, and ZF says whether the selector was valid.
	unsafe {
		core::arch::asm!(
			"lsl {limit}, {selector:e}",
			"setz {valid:l}",
			"movzx {valid:e}, {valid:l}",
			selector = in(reg) u32::from(selector),
			limit = out(reg) limit,
			valid = out(reg_abcd) valid,
			options(nostack),
		);
	}
	if valid != 0 { limit + 1 } else { 0 }
}
