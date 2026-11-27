// The Kernel row from the version Windows reports: `NT 5.2.3790 SP3` on the NT family and `Windows 95 4.00.950` on Windows 9x (SPEC
// section 4). Windows 9x calls itself 4.00, 4.10 and 4.90, which are the three names; a version it does not know is `Windows`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;
use afterfetch_core::format::format_kernel as format_nt_kernel;

/// `VER_PLATFORM_WIN32_NT` from `GetVersionExA`.
pub const PLATFORM_NT: u32 = 2;

/// The row for a system whose `GetVersionExA` gave `platform`, `major`, `minor`, the build (the low word on 9x, which keeps flags in the
/// high one) and the service pack text.
pub fn format_kernel<const N: usize>(out: &mut Buf<N>, platform: u32, major: u32, minor: u32, build: u32, service_pack: &[u8]) {
	if platform == PLATFORM_NT {
		return format_nt_kernel(out, major, minor, build, service_pack);
	}
	out.push(match (major, minor) {
		(4, 0) => b"Windows 95 ",
		(4, 10) => b"Windows 98 ",
		(4, 90) => b"Windows Me ",
		_ => b"Windows ",
	});
	out.push_u32(major).push(b".");
	if minor < 10 {
		out.push(b"0");
	}
	out.push_u32(minor).push(b".").push_u32(build & 0xFFFF);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn row(platform: u32, major: u32, minor: u32, build: u32, service_pack: &[u8]) -> std::string::String {
		let mut out = Buf::<64>::new();
		format_kernel(&mut out, platform, major, minor, build, service_pack);
		std::string::String::from_utf8(out.as_bytes().to_vec()).unwrap()
	}

	#[test]
	fn the_nt_family_reads_nt_and_its_service_pack() {
		assert_eq!(row(2, 5, 2, 3790, b"Service Pack 3"), "NT 5.2.3790 SP3");
		assert_eq!(row(2, 10, 0, 19045, b""), "NT 10.0.19045");
	}

	#[test]
	fn windows_9x_names_its_release_and_pads_the_minor_version() {
		assert_eq!(row(1, 4, 0, 950, b""), "Windows 95 4.00.950");
		assert_eq!(row(1, 4, 0, 0xC000_0000 | 1111, b" B"), "Windows 95 4.00.1111");
		assert_eq!(row(1, 4, 10, 2222, b" A "), "Windows 98 4.10.2222");
		assert_eq!(row(1, 4, 90, 3000, b""), "Windows Me 4.90.3000");
		assert_eq!(row(1, 4, 5, 1, b""), "Windows 4.05.1");
	}
}
