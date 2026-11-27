// A style as a text-mode attribute: the low nibble is the foreground colour, the high nibble the background, in the console's own order
// (blue 1, green 2, red 4, intensity 8). The Dusk table is laid out in that order, so a slot number is the nibble.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{Style, Weight};

const FOREGROUND: u16 = 0x000F;
const BACKGROUND: u16 = 0x00F0;
const INTENSITY: u16 = 0x0008;

/// The attribute for `style` over the console's own attribute `original`. A colour the style leaves at the terminal's own comes from
/// `original`, and the bits above the two nibbles (reverse video and the like) are kept. Bold brightens the foreground, which is all
/// the mono depth has; dim has nothing to map to.
pub fn from_style(original: u16, style: Style) -> u16 {
	let mut foreground = style.fg.slot().map_or(original & FOREGROUND, u16::from);
	let background = style.bg.slot().map_or(original & BACKGROUND, |slot| u16::from(slot) << 4);
	if style.weight == Weight::Bold {
		foreground |= INTENSITY;
	}
	(original & !(FOREGROUND | BACKGROUND)) | background | (foreground & FOREGROUND)
}

#[cfg(test)]
mod tests {
	use super::*;
	use afterfetch_core::{Paint, Tone};

	#[test]
	fn the_plain_style_is_the_original_attribute() {
		assert_eq!(from_style(0x07, Style::PLAIN), 0x07);
		assert_eq!(from_style(0x1F, Style::PLAIN), 0x1F);
	}

	#[test]
	fn text_sets_the_foreground_and_keeps_the_background() {
		// Gold is console slot 14.
		assert_eq!(from_style(0x07, Style::text(Tone::Gold)), 0x0E);
		assert_eq!(from_style(0x17, Style::text(Tone::Rose)), 0x14);
	}

	#[test]
	fn a_fill_sets_the_background_and_keeps_the_foreground() {
		assert_eq!(from_style(0x07, Style::fill(Paint::Tone(Tone::Coral))), 0xC7);
		assert_eq!(from_style(0x07, Style::fill(Paint::Slot(8))), 0x87);
	}

	#[test]
	fn bits_above_the_colours_survive() {
		assert_eq!(from_style(0x4007, Style::text(Tone::Gold)), 0x400E);
	}

	#[test]
	fn bold_brightens_the_original_foreground() {
		assert_eq!(from_style(0x07, Style::mono(Tone::Gold)), 0x0F);
		assert_eq!(from_style(0x07, Style::mono(Tone::Slate)), 0x07);
	}
}
