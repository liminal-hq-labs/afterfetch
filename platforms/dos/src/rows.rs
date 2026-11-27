// The text of the DOS facts, from the numbers the BIOS and DOS return. Each function is a pure formatting step, so the host tests it; the
// program reads the numbers (`os.rs`).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;

/// The product from the OEM number AH=30h returns in BH. FreeDOS says 0xFD, PC DOS 0, MS-DOS 0xFF, and DR-DOS 0xEE or 0xEF (DR-DOS also
/// answers AX=4452h, which the caller passes as `dr_dos`).
pub fn product(oem: u8, dr_dos: bool) -> &'static [u8] {
	match oem {
		_ if dr_dos => b"DR-DOS",
		0xFD => b"FreeDOS",
		0x00 => b"PC DOS",
		0xEE | 0xEF => b"DR-DOS",
		0xFF => b"MS-DOS",
		_ => b"DOS",
	}
}

/// `6.22`: the major number, then the minor as two digits, as DOS prints its version.
pub fn push_dos_version<const N: usize>(out: &mut Buf<N>, major: u8, minor: u8) {
	out.push_u32(u32::from(major)).push(b".");
	if minor < 10 {
		out.push(b"0");
	}
	out.push_u32(u32::from(minor));
}

/// `3.1` or `3.11`: Windows reports the minor number as 10 for 3.1, so a trailing zero is dropped. A minor of 0 is `4.0`.
fn push_windows_version<const N: usize>(out: &mut Buf<N>, major: u8, minor: u8) {
	out.push_u32(u32::from(major)).push(b".");
	if minor.is_multiple_of(10) {
		out.push_u32(u32::from(minor / 10));
	} else {
		out.push_u32(u32::from(minor));
	}
}

/// `MS-DOS 6.22`, and `MS-DOS 6.22 in Windows 3.11` when the program runs in a DOS box (`windows` is the version INT 2Fh AX=1600h gave).
pub fn format_os<const N: usize>(out: &mut Buf<N>, product: &[u8], major: u8, minor: u8, windows: Option<(u8, u8)>) {
	out.push(product).push(b" ");
	push_dos_version(out, major, minor);
	if let Some((windows_major, windows_minor)) = windows {
		out.push(b" in Windows ");
		push_windows_version(out, windows_major, windows_minor);
	}
}

/// `DOS 6.22`, with `, high` when the kernel is loaded in the high memory area (AX=3306h, DH bit 4).
pub fn format_kernel<const N: usize>(out: &mut Buf<N>, major: u8, minor: u8, high: bool) {
	out.push(b"DOS ");
	push_dos_version(out, major, minor);
	if high {
		out.push(b", high");
	}
}

/// What the system model byte at F000:FFFE says.
fn model_name(byte: u8) -> Option<&'static [u8]> {
	match byte {
		0xFF => Some(b"PC"),
		0xFE | 0xFB => Some(b"XT"),
		0xFD => Some(b"PCjr"),
		0xFC => Some(b"AT"),
		0xFA => Some(b"PS/2 Model 30"),
		0xF8 => Some(b"PS/2 Model 80"),
		_ => None,
	}
}

/// True for the eight characters `mm/dd/yy` the BIOS date at F000:FFF5 holds.
fn is_bios_date(date: &[u8]) -> bool {
	date.len() == 8
		&& date.iter().enumerate().all(|(at, byte)| {
			if at == 2 || at == 5 {
				*byte == b'/'
			} else {
				byte.is_ascii_digit()
			}
		})
}

/// `AT, BIOS 01/10/92`: the model and the BIOS date, whichever is known. Empty when neither is.
pub fn format_host<const N: usize>(out: &mut Buf<N>, date: &[u8], model: u8) {
	if let Some(name) = model_name(model) {
		out.push(name);
	}
	if is_bios_date(date) {
		if !out.is_empty() {
			out.push(b", ");
		}
		out.push(b"BIOS ").push(date);
	}
}

/// The Intel name of a processor class, as the architecture name of the JSON `arch` key: DPMI reports 3 for a 386, 4 for a 486, 5 for a
/// Pentium and more for later ones.
pub fn arch_name(processor: u8) -> &'static [u8] {
	match processor {
		0..=3 => b"i386",
		4 => b"i486",
		5 => b"i586",
		_ => b"i686",
	}
}

/// `486, FPU` or `386, no FPU`: the class of a processor that has no brand string, and whether there is a coprocessor.
pub fn format_class<const N: usize>(out: &mut Buf<N>, processor: u8, fpu: bool) {
	out.push(match processor {
		0..=3 => b"386",
		4 => b"486",
		_ => b"Pentium",
	});
	push_fpu(out, fpu);
}

/// `, FPU` or `, no FPU`.
pub fn push_fpu<const N: usize>(out: &mut Buf<N>, fpu: bool) {
	out.push(if fpu { b", FPU" } else { b", no FPU" });
}

/// The adapter name from the display combination code INT 10h AX=1A00h returns in BL.
pub fn adapter_name(code: u8) -> Option<&'static [u8]> {
	match code {
		1 => Some(b"MDA"),
		2 => Some(b"CGA"),
		4 | 5 => Some(b"EGA"),
		6 => Some(b"PGA"),
		7 | 8 => Some(b"VGA"),
		0x0A..=0x0C => Some(b"MCGA"),
		_ => None,
	}
}

/// `VGA, 80 x 25`, and `VGA, VESA 2.0 (4 MB), 80 x 25` when the VESA BIOS answers. `vesa` is the version word (major in the high
/// byte) and the memory in 64 KiB blocks.
pub fn format_display<const N: usize>(out: &mut Buf<N>, adapter: &[u8], vesa: Option<(u16, u16)>, columns: u32, rows: u32) {
	out.push(adapter);
	if let Some((version, blocks)) = vesa {
		out.push(b", VESA ")
			.push_u32(u32::from(version >> 8))
			.push(b".")
			.push_u32(u32::from(version & 0xFF));
		let kilobytes = u32::from(blocks) * 64;
		if kilobytes >= 1024 {
			out.push(b" (").push_u32(kilobytes / 1024).push(b" MB)");
		} else if kilobytes > 0 {
			out.push(b" (").push_u32(kilobytes).push(b" KB)");
		}
	}
	out.push(b", ").push_u32(columns).push(b" x ").push_u32(rows);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn text<const N: usize>(out: &Buf<N>) -> &str {
		core::str::from_utf8(out.as_bytes()).unwrap()
	}

	#[test]
	fn the_product_comes_from_the_oem_number() {
		assert_eq!(product(0xFF, false), b"MS-DOS");
		assert_eq!(product(0x00, false), b"PC DOS");
		assert_eq!(product(0xFD, false), b"FreeDOS");
		assert_eq!(product(0xEE, false), b"DR-DOS");
		assert_eq!(product(0x12, true), b"DR-DOS");
		assert_eq!(product(0x12, false), b"DOS");
	}

	#[test]
	fn versions_print_two_digit_minors() {
		let mut out = Buf::<64>::new();
		format_os(&mut out, b"MS-DOS", 6, 22, None);
		assert_eq!(text(&out), "MS-DOS 6.22");
		let mut out = Buf::<64>::new();
		format_os(&mut out, b"MS-DOS", 5, 0, None);
		assert_eq!(text(&out), "MS-DOS 5.00");
	}

	#[test]
	fn a_dos_box_says_which_windows() {
		let mut out = Buf::<64>::new();
		format_os(&mut out, b"MS-DOS", 6, 22, Some((3, 10)));
		assert_eq!(text(&out), "MS-DOS 6.22 in Windows 3.1");
		let mut out = Buf::<64>::new();
		format_os(&mut out, b"MS-DOS", 7, 10, Some((3, 11)));
		assert_eq!(text(&out), "MS-DOS 7.10 in Windows 3.11");
		let mut out = Buf::<64>::new();
		format_os(&mut out, b"MS-DOS", 7, 10, Some((4, 0)));
		assert_eq!(text(&out), "MS-DOS 7.10 in Windows 4.0");
	}

	#[test]
	fn the_kernel_notes_the_high_memory_area() {
		let mut out = Buf::<64>::new();
		format_kernel(&mut out, 6, 22, true);
		assert_eq!(text(&out), "DOS 6.22, high");
		let mut out = Buf::<64>::new();
		format_kernel(&mut out, 5, 0, false);
		assert_eq!(text(&out), "DOS 5.00");
	}

	#[test]
	fn the_host_is_the_model_and_the_bios_date_when_known() {
		let mut out = Buf::<64>::new();
		format_host(&mut out, b"01/10/92", 0xFC);
		assert_eq!(text(&out), "AT, BIOS 01/10/92");
		let mut out = Buf::<64>::new();
		format_host(&mut out, b"garbage!", 0xFC);
		assert_eq!(text(&out), "AT");
		let mut out = Buf::<64>::new();
		format_host(&mut out, b"13/31/99", 0x12);
		assert_eq!(text(&out), "BIOS 13/31/99");
		let mut out = Buf::<64>::new();
		format_host(&mut out, b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF", 0xFF);
		assert_eq!(text(&out), "PC");
		let mut out = Buf::<64>::new();
		format_host(&mut out, b"", 0x12);
		assert!(out.is_empty());
	}

	#[test]
	fn the_processor_class_gives_the_architecture_name_and_the_row() {
		assert_eq!(arch_name(3), b"i386");
		assert_eq!(arch_name(4), b"i486");
		assert_eq!(arch_name(5), b"i586");
		assert_eq!(arch_name(6), b"i686");
		let mut out = Buf::<64>::new();
		format_class(&mut out, 4, true);
		assert_eq!(text(&out), "486, FPU");
		let mut out = Buf::<64>::new();
		format_class(&mut out, 3, false);
		assert_eq!(text(&out), "386, no FPU");
	}

	#[test]
	fn the_display_row_names_the_adapter_vesa_and_the_text_mode() {
		let mut out = Buf::<64>::new();
		format_display(&mut out, b"VGA", None, 80, 25);
		assert_eq!(text(&out), "VGA, 80 x 25");
		let mut out = Buf::<64>::new();
		format_display(&mut out, b"VGA", Some((0x0200, 64)), 80, 50);
		assert_eq!(text(&out), "VGA, VESA 2.0 (4 MB), 80 x 50");
		let mut out = Buf::<64>::new();
		format_display(&mut out, b"VGA", Some((0x0102, 4)), 80, 25);
		assert_eq!(text(&out), "VGA, VESA 1.2 (256 KB), 80 x 25");
		assert_eq!(adapter_name(8), Some(&b"VGA"[..]));
		assert_eq!(adapter_name(1), Some(&b"MDA"[..]));
		assert_eq!(adapter_name(0), None);
	}
}
