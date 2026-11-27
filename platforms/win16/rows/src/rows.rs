// The text of the Windows 3.1 facts, from the numbers Windows returns. Each function is a pure formatting step, so the host tests it; the
// program reads the numbers (`facts.rs`).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;

/// `GetWinFlags` bits.
pub const WF_CPU286: u32 = 0x0002;
pub const WF_CPU386: u32 = 0x0004;
pub const WF_CPU486: u32 = 0x0008;
pub const WF_STANDARD: u32 = 0x0010;
pub const WF_ENHANCED: u32 = 0x0020;
pub const WF_80X87: u32 = 0x0400;

/// `Windows 3.1 on DOS 6.22` from `GetVersion`'s two halves: Windows' version has the major in the low byte and the minor in the high one
/// (10 for 3.1), DOS's has the major in the high byte and the minor in the low one.
pub fn format_os<const N: usize>(out: &mut Buf<N>, windows: u16, dos: u16) {
	let (major, minor) = (u32::from(windows & 0xFF), u32::from(windows >> 8));
	out.push(b"Windows ").push_u32(major).push(b".");
	// 10 is 3.1 and 11 is 3.11; a minor that is a plain tenth drops its zero.
	if minor.is_multiple_of(10) {
		out.push_u32(minor / 10);
	} else {
		out.push_u32(minor);
	}
	out.push(b" on DOS ").push_u32(u32::from(dos >> 8)).push(b".");
	if dos & 0xFF < 10 {
		out.push(b"0");
	}
	out.push_u32(u32::from(dos & 0xFF));
}

/// `386 enhanced mode`, `standard mode`: the mode Windows is running in.
pub fn format_mode<const N: usize>(out: &mut Buf<N>, flags: u32) {
	if flags & WF_ENHANCED != 0 {
		out.push(b"386 enhanced mode");
	} else if flags & WF_STANDARD != 0 {
		out.push(b"Standard mode");
	} else {
		out.push(b"Real mode");
	}
}

/// The processor's Intel name for the JSON `arch` key: `i386`, `i486`, or `i286`.
pub fn arch_name(flags: u32) -> &'static [u8] {
	if flags & WF_CPU486 != 0 {
		b"i486"
	} else if flags & WF_CPU386 != 0 {
		b"i386"
	} else if flags & WF_CPU286 != 0 {
		b"i286"
	} else {
		b"i086"
	}
}

/// `486, FPU` or `386, no FPU`.
pub fn format_cpu<const N: usize>(out: &mut Buf<N>, flags: u32) {
	let class: &[u8] = if flags & WF_CPU486 != 0 {
		b"486"
	} else if flags & WF_CPU386 != 0 {
		b"386"
	} else if flags & WF_CPU286 != 0 {
		b"286"
	} else {
		b"8086"
	};
	out.push(class)
		.push(if flags & WF_80X87 != 0 { b", FPU" } else { b", no FPU" });
}

/// `7 MB free`, or `640 KB free` below a megabyte: the free global heap, which is all Windows 3.1 reports (there is no total).
pub fn format_free<const N: usize>(out: &mut Buf<N>, bytes: u32) {
	if bytes >= 1 << 20 {
		out.push_u32(bytes >> 20).push(b" MB free");
	} else {
		out.push_u32(bytes >> 10).push(b" KB free");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn text<const N: usize>(out: &Buf<N>) -> &str {
		core::str::from_utf8(out.as_bytes()).unwrap()
	}

	#[test]
	fn the_os_row_names_the_windows_and_dos_versions() {
		let mut out = Buf::<64>::new();
		format_os(&mut out, 0x0A03, 0x0616);
		assert_eq!(text(&out), "Windows 3.1 on DOS 6.22");
		let mut out = Buf::<64>::new();
		format_os(&mut out, 0x0B03, 0x0500);
		assert_eq!(text(&out), "Windows 3.11 on DOS 5.00");
	}

	#[test]
	fn the_mode_comes_from_the_win_flags() {
		let mut out = Buf::<64>::new();
		format_mode(&mut out, WF_ENHANCED | WF_CPU486);
		assert_eq!(text(&out), "386 enhanced mode");
		let mut out = Buf::<64>::new();
		format_mode(&mut out, WF_STANDARD | WF_CPU286);
		assert_eq!(text(&out), "Standard mode");
	}

	#[test]
	fn the_cpu_row_gives_the_class_and_the_coprocessor() {
		let mut out = Buf::<64>::new();
		format_cpu(&mut out, WF_CPU486 | WF_80X87);
		assert_eq!(text(&out), "486, FPU");
		let mut out = Buf::<64>::new();
		format_cpu(&mut out, WF_CPU386);
		assert_eq!(text(&out), "386, no FPU");
		assert_eq!(arch_name(WF_CPU486), b"i486");
		assert_eq!(arch_name(WF_CPU286), b"i286");
	}

	#[test]
	fn free_memory_is_megabytes_or_kilobytes() {
		let mut out = Buf::<64>::new();
		format_free(&mut out, 7 << 20);
		assert_eq!(text(&out), "7 MB free");
		let mut out = Buf::<64>::new();
		format_free(&mut out, 640 << 10);
		assert_eq!(text(&out), "640 KB free");
	}
}
