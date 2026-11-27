// The CPUID instruction: the brand string where the processor has one, else the vendor and family. CPUID exists on late 486s and every
// Pentium, so a 32-bit build first tries to flip the ID bit of EFLAGS, which only a processor with CPUID lets stay flipped. Windows on
// ARM has no CPUID, and its name comes from the registry.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86 {
	use afterfetch_core::Buf;
	use afterfetch_win32::cpu::{push_vendor_family, push_words};

	#[cfg(target_arch = "x86")]
	use core::arch::x86::__cpuid;
	#[cfg(target_arch = "x86_64")]
	use core::arch::x86_64::__cpuid;

	/// True when the processor has CPUID: every x86-64 processor does, and a 32-bit one if the ID bit (bit 21) of EFLAGS can be flipped.
	fn has_cpuid() -> bool {
		#[cfg(target_arch = "x86_64")]
		{
			true
		}
		#[cfg(target_arch = "x86")]
		{
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
	}

	pub fn brand(out: &mut Buf<64>) {
		if !has_cpuid() {
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

	pub fn vendor_family(out: &mut Buf<64>) {
		if !has_cpuid() {
			return;
		}
		// CPUID is present; leaf 1 is read only when leaf 0 says it exists.
		let vendor = __cpuid(0);
		if vendor.eax >= 1 {
			push_vendor_family(out, [vendor.ebx, vendor.edx, vendor.ecx], __cpuid(1).eax);
		}
	}
}

/// The processor's brand string into `out`, or nothing when it has none.
pub fn brand(out: &mut Buf<64>) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::brand(out);
	#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
	let _ = out;
}

/// The vendor and family into `out`, for a processor with CPUID but no brand string (a Pentium).
pub fn vendor_family(out: &mut Buf<64>) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::vendor_family(out);
	#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
	let _ = out;
}
