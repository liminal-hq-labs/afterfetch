// Afterfetch art: the registry of generated logos and palette tables.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

/// The logo used when nothing more specific applies.
pub const FALLBACK_LOGO: &str = "afterglow";

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fallback_logo_is_afterglow() {
		assert_eq!(FALLBACK_LOGO, "afterglow");
	}
}
