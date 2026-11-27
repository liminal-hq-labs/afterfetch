// How a style looks on a screen: the colour each depth gives a paint, on the terminal's own theme or on a classic console's.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::palette::{ansi16_of_slot, slot_rgb, xterm256_nearest};
use afterfetch_core::{Depth, Paint, Style, Weight};

/// A colour as red, green and blue.
pub type Rgb = [u8; 3];

/// The sixteen colours of a typical terminal emulator (xterm's defaults), in ANSI order.
const TERMINAL_16: [Rgb; 16] = [
	[0x00, 0x00, 0x00],
	[0xcd, 0x00, 0x00],
	[0x00, 0xcd, 0x00],
	[0xcd, 0xcd, 0x00],
	[0x00, 0x00, 0xee],
	[0xcd, 0x00, 0xcd],
	[0x00, 0xcd, 0xcd],
	[0xe5, 0xe5, 0xe5],
	[0x7f, 0x7f, 0x7f],
	[0xff, 0x00, 0x00],
	[0x00, 0xff, 0x00],
	[0xff, 0xff, 0x00],
	[0x5c, 0x5c, 0xff],
	[0xff, 0x00, 0xff],
	[0x00, 0xff, 0xff],
	[0xff, 0xff, 0xff],
];

/// The sixteen colours of a VGA text screen and the Windows console, in ANSI order.
const CONSOLE_16: [Rgb; 16] = [
	[0x00, 0x00, 0x00],
	[0xaa, 0x00, 0x00],
	[0x00, 0xaa, 0x00],
	[0xaa, 0x55, 0x00],
	[0x00, 0x00, 0xaa],
	[0xaa, 0x00, 0xaa],
	[0x00, 0xaa, 0xaa],
	[0xaa, 0xaa, 0xaa],
	[0x55, 0x55, 0x55],
	[0xff, 0x55, 0x55],
	[0x55, 0xff, 0x55],
	[0xff, 0xff, 0x55],
	[0x55, 0x55, 0xff],
	[0xff, 0x55, 0xff],
	[0x55, 0xff, 0xff],
	[0xff, 0xff, 0xff],
];

/// The terminal's own foreground and background.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
	pub foreground: Rgb,
	pub background: Rgb,
	/// What bold text without a colour of its own shows as.
	pub bright: Rgb,
}

impl Theme {
	/// A dark terminal emulator.
	pub const TERMINAL: Theme = Theme {
		foreground: [0xd0, 0xd4, 0xdc],
		background: [0x12, 0x15, 0x1d],
		bright: [0xff, 0xff, 0xff],
	};

	/// A DOS or Windows console: light grey on black.
	pub const CONSOLE: Theme = Theme {
		foreground: [0xaa, 0xaa, 0xaa],
		background: [0x00, 0x00, 0x00],
		bright: [0xff, 0xff, 0xff],
	};

	/// The theme a depth is shown on: the console's for console attributes, a terminal's otherwise.
	pub fn for_depth(depth: Depth) -> Theme {
		if depth == Depth::Console16 {
			Theme::CONSOLE
		} else {
			Theme::TERMINAL
		}
	}
}

/// The xterm 256-colour number as a colour.
pub fn xterm_rgb(number: u8) -> Rgb {
	const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
	match number {
		0..=15 => TERMINAL_16[usize::from(number)],
		16..=231 => {
			let index = usize::from(number - 16);
			[LEVELS[index / 36], LEVELS[(index / 6) % 6], LEVELS[index % 6]]
		}
		_ => {
			let grey = 8 + 10 * (number - 232);
			[grey, grey, grey]
		}
	}
}

/// The colour a paint shows at `depth`, or None for the terminal's own colour.
pub fn paint_rgb(paint: Paint, depth: Depth) -> Option<Rgb> {
	match depth {
		Depth::None | Depth::Mono => None,
		Depth::Console16 => paint.slot().map(|slot| CONSOLE_16[usize::from(ansi16_of_slot(slot))]),
		Depth::Ansi16 => paint.slot().map(|slot| TERMINAL_16[usize::from(ansi16_of_slot(slot))]),
		Depth::Ansi256 => match paint {
			Paint::Default => None,
			Paint::Tone(tone) => tone.xterm256().map(xterm_rgb),
			Paint::Slot(slot) => Some(xterm_rgb(xterm256_nearest(slot_rgb(slot)))),
		},
		Depth::TrueColour => match paint {
			Paint::Default => None,
			Paint::Tone(tone) => tone.rgb(),
			Paint::Slot(slot) => Some(slot_rgb(slot)),
		},
	}
}

fn mix(from: Rgb, to: Rgb) -> Rgb {
	[0, 1, 2].map(|at| ((u16::from(from[at]) + u16::from(to[at])) / 2) as u8)
}

/// The foreground and background a style shows as at `depth`, on `theme`.
pub fn style_colours(style: Style, depth: Depth, theme: Theme) -> (Rgb, Rgb) {
	let background = paint_rgb(style.bg, depth).unwrap_or(theme.background);
	let foreground = match (paint_rgb(style.fg, depth), style.weight) {
		(Some(colour), _) => colour,
		(None, Weight::Bold) => theme.bright,
		(None, Weight::Dim) => mix(theme.foreground, background),
		(None, Weight::Normal) => theme.foreground,
	};
	(foreground, background)
}

#[cfg(test)]
mod tests {
	use super::*;
	use afterfetch_core::Tone;

	#[test]
	fn truecolour_is_the_design_colour() {
		assert_eq!(paint_rgb(Paint::Tone(Tone::Gold), Depth::TrueColour), Some([255, 208, 142]));
	}

	#[test]
	fn the_terminals_own_colour_has_no_paint() {
		assert_eq!(paint_rgb(Paint::Default, Depth::TrueColour), None);
		assert_eq!(paint_rgb(Paint::Tone(Tone::Gold), Depth::None), None);
	}

	#[test]
	fn the_xterm_cube_and_greys() {
		assert_eq!(xterm_rgb(16), [0, 0, 0]);
		assert_eq!(xterm_rgb(231), [255, 255, 255]);
		assert_eq!(xterm_rgb(232), [8, 8, 8]);
		assert_eq!(xterm_rgb(255), [238, 238, 238]);
	}

	#[test]
	fn console_slot_one_is_blue_and_ansi_one_is_red() {
		assert_eq!(paint_rgb(Paint::Slot(1), Depth::Console16), Some([0x00, 0x00, 0xaa]));
		assert_eq!(paint_rgb(Paint::Slot(1), Depth::Ansi16), Some([0x00, 0x00, 0xee]));
		assert_eq!(paint_rgb(Paint::Slot(4), Depth::Console16), Some([0xaa, 0x00, 0x00]));
	}
}
