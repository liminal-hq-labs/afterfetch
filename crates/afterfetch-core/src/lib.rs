// The portable core of afterfetch: no platform knowledge, no allocation, no panics on any input.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

/// The program's name, as `--version` prints it.
pub const NAME: &str = "afterfetch";

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn name_is_afterfetch() {
		assert_eq!(NAME, "afterfetch");
	}
}
