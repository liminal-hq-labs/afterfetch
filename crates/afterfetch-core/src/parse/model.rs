// The machine's model: `/sys/devices/virtual/dmi/id/product_name` on a PC, `/proc/device-tree/model` on an ARM or MIPS board.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::trim;
use crate::format::{Buf, c_str};

/// Placeholders firmware makers leave in the product name. Showing one would be noise, not a fact.
const PLACEHOLDERS: [&[u8]; 6] = [
	b"to be filled by o.e.m.",
	b"system product name",
	b"default string",
	b"not specified",
	b"none",
	b"n/a",
];

/// Writes the model in `contents` (a file's bytes, possibly NUL-terminated or ending in a newline) to `out`, or nothing when it is empty or
/// a placeholder.
pub fn clean<const N: usize>(contents: &[u8], out: &mut Buf<N>) {
	let text = trim(c_str(contents));
	if text.is_empty() || PLACEHOLDERS.iter().any(|placeholder| text.eq_ignore_ascii_case(placeholder)) {
		return;
	}
	out.push(text);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cleaned(contents: &[u8]) -> std::string::String {
		let mut out = Buf::<64>::new();
		clean(contents, &mut out);
		std::string::String::from_utf8_lossy(out.as_bytes()).into_owned()
	}

	#[test]
	fn a_dmi_name_loses_its_newline() {
		assert_eq!(cleaned(b"ThinkPad X1 Carbon Gen 11\n"), "ThinkPad X1 Carbon Gen 11");
	}

	#[test]
	fn a_device_tree_name_stops_at_the_nul() {
		assert_eq!(cleaned(b"Raspberry Pi 4 Model B Rev 1.1\0"), "Raspberry Pi 4 Model B Rev 1.1");
	}

	#[test]
	fn placeholders_and_blanks_give_nothing() {
		assert_eq!(cleaned(b"To Be Filled By O.E.M.\n"), "");
		assert_eq!(cleaned(b"System Product Name\n"), "");
		assert_eq!(cleaned(b"\n"), "");
		assert_eq!(cleaned(b""), "");
	}
}
