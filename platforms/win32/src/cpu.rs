// Turning CPUID's registers into text. The registers come from the program (`os.rs`); this part needs no processor, so the host tests it.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;

/// Appends the bytes of `words`, each least significant byte first, as CPUID lays text out in its registers. A brand string ends at the
/// first NUL, and what follows it is padding.
pub fn push_words<const N: usize>(out: &mut Buf<N>, words: &[u32]) {
	for byte in words.iter().flat_map(|word| word.to_le_bytes()) {
		if byte == 0 {
			return;
		}
		out.push(&[byte]);
	}
}

/// `GenuineIntel family 5` for a processor with no brand string: the vendor (EBX, EDX, ECX of leaf 0) and the family from leaf 1's EAX.
pub fn push_vendor_family<const N: usize>(out: &mut Buf<N>, vendor: [u32; 3], signature: u32) {
	push_words(out, &vendor);
	out.push(b" family ").push_u32((signature >> 8) & 0xF);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn text(out: &Buf<64>) -> &str {
		core::str::from_utf8(out.as_bytes()).unwrap()
	}

	#[test]
	fn registers_are_text_in_little_endian_order() {
		let mut out = Buf::<64>::new();
		push_words(&mut out, &[0x756E_6547, 0x4965_6E69, 0x6C65_746E]);
		assert_eq!(text(&out), "GenuineIntel");
	}

	#[test]
	fn a_brand_string_ends_at_its_nul() {
		let mut out = Buf::<64>::new();
		let words = [b"Pro ", b"cess", b"or\0j", b"jjjj"].map(|text| u32::from_le_bytes(*text));
		push_words(&mut out, &words);
		assert_eq!(text(&out), "Pro cessor");
	}

	#[test]
	fn a_pentium_with_no_brand_gets_its_vendor_and_family() {
		let mut out = Buf::<64>::new();
		// Leaf 0 gives the vendor in EBX, EDX, ECX; a Pentium's leaf 1 EAX is 0x0000_0542 (family 5, model 4, stepping 2).
		push_vendor_family(&mut out, [0x756E_6547, 0x4965_6E69, 0x6C65_746E], 0x542);
		assert_eq!(text(&out), "GenuineIntel family 5");
	}
}
