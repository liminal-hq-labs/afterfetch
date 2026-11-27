// The CPUID instruction: the brand string where the processor has one, else the vendor and family. CPUID exists on late 486s and every
// Pentium, so the program first asks DPMI for the processor class (a 386 never has it) and then tries to flip the ID bit of EFLAGS, which
// only a processor with CPUID lets stay flipped.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;
use afterfetch_dos::cpu::{push_vendor_family, push_words};
use core::arch::x86::__cpuid;

/// True when the processor has CPUID: a 486 or later whose ID bit (bit 21) of EFLAGS can be flipped.
fn has_cpuid(processor: u8) -> bool {
	if processor < 4 {
		return false;
	}
	let changed: u32;
	// SAFETY: reads EFLAGS, flips the ID bit, reads it again, and puts the original value back; touches no memory but the stack.
	unsafe {
		core::arch::asm!(
			"pushfd",
			"pop {flags}",
			"mov {original}, {flags}",
			"xor {flags}, 0x200000",
			"push {flags}",
			"popfd",
			"pushfd",
			"pop {flags}",
			"xor {flags}, {original}",
			"push {original}",
			"popfd",
			flags = out(reg) changed,
			original = out(reg) _,
		);
	}
	changed & 0x0020_0000 != 0
}

/// The processor's brand string into `out`, or nothing when it has none.
pub fn brand(out: &mut Buf<64>, processor: u8) {
	if !has_cpuid(processor) {
		return;
	}
	// CPUID is present, and an unsupported leaf only returns the highest one.
	if __cpuid(0x8000_0000).eax >= 0x8000_0004 {
		for leaf in 0x8000_0002..=0x8000_0004 {
			let registers = __cpuid(leaf);
			push_words(out, &[registers.eax, registers.ebx, registers.ecx, registers.edx]);
		}
	}
}

/// The vendor and family into `out`, for a processor with CPUID but no brand string (a Pentium).
pub fn vendor_family(out: &mut Buf<64>, processor: u8) {
	if !has_cpuid(processor) {
		return;
	}
	let vendor = __cpuid(0);
	if vendor.eax >= 1 {
		push_vendor_family(out, [vendor.ebx, vendor.edx, vendor.ecx], __cpuid(1).eax);
	}
}
