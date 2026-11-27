// `/proc/uptime`: seconds since boot as a decimal, then the idle seconds.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{leading_number, trim};

/// The whole seconds of uptime in the file's contents, or None when it does not start with a number.
pub fn seconds(contents: &[u8]) -> Option<u64> {
	leading_number(trim(contents))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_fraction_and_the_idle_time_are_dropped() {
		assert_eq!(seconds(b"350735.47 234388.90\n"), Some(350_735));
		assert_eq!(seconds(b"  42\n"), Some(42));
	}

	#[test]
	fn junk_is_none() {
		assert_eq!(seconds(b""), None);
		assert_eq!(seconds(b"up 3 days"), None);
	}
}
