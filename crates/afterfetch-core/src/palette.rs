// The colours afterfetch uses, as names for the roles they play in the picture, each with a colour at every depth from the generated
// palette in `afterfetch-art`, and the console's 16 slots in the Dusk table for the swatches.
//
// The Win32 image sets the console's colour table from the Afterglow Dusk palette (HKCU\Console, ColorTable00 to 15), so a slot here is
// an Afterglow colour on that image and whatever the user's own table holds elsewhere. The slots follow the order of the table, which
// reads: 0 navy, 1 periwinkle, 2 green, 3 teal, 4 rose, 5 violet, 6 orange, 7 text, 8 slate, then the bright set 9 to 15.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_art::Mono;

use crate::depth::Depth;

/// The generated tones, kept as one table for every lookup.
static TONES: &[afterfetch_art::Tone; 15] = &afterfetch_art::TONES;

/// The Afterglow Dusk console table as 0xRRGGBB, indexed by console slot (blue is bit 0, red is bit 2, as on Windows and DOS).
pub const DUSK: [u32; 16] = [
	0x0a1230, 0x7f95f2, 0x86cf9a, 0x62c6d6, 0xf43f5e, 0xc98be6, 0xffaa40, 0xe9e2d8, 0x7c8796, 0xa9b8ff, 0xb0efc0, 0x98e6f0,
	0xf995a6, 0xe4b8f8, 0xffd08e, 0xfff7ea,
];

/// A tone of the palette: a role in the picture, with the id the tone has in `afterfetch-art` (the discriminant) and a colour at every
/// depth from that crate's generated table. `Plain` is the console's own foreground and background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Tone {
	Plain,
	/// The sun's crown: pale gold.
	Gold,
	/// Crown to body.
	Amber,
	/// The sun's body.
	Orange,
	/// Body to foot.
	Peach,
	/// The sun at the horizon.
	Coral,
	/// The horizon line.
	Rose,
	/// The sky's light end and the water just under the horizon.
	Lilac,
	/// The middle of the water.
	Violet,
	/// The deeper water.
	Periwinkle,
	/// The darkest water, and quiet text.
	Slate,
	/// Highlights.
	Cream,
	/// Silhouettes.
	Night,
	/// The RISC-V accent.
	Teal,
	/// Desk's halo, middle ring.
	Umber,
	/// Desk's halo, outer ring.
	Soot,
}

/// How strongly a tone is drawn when colour is not available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
	Normal,
	Bold,
	Dim,
}

impl Tone {
	const ALL: [Tone; 16] = [
		Tone::Plain,
		Tone::Gold,
		Tone::Amber,
		Tone::Orange,
		Tone::Peach,
		Tone::Coral,
		Tone::Rose,
		Tone::Lilac,
		Tone::Violet,
		Tone::Periwinkle,
		Tone::Slate,
		Tone::Cream,
		Tone::Night,
		Tone::Teal,
		Tone::Umber,
		Tone::Soot,
	];

	/// The tone with this art id (1 to 15), or `Plain` for 0 and anything else.
	pub fn from_id(id: u8) -> Tone {
		Tone::ALL.get(usize::from(id)).copied().unwrap_or(Tone::Plain)
	}

	/// The generated table's entry for this tone, or None for `Plain`.
	fn entry(self) -> Option<&'static afterfetch_art::Tone> {
		// Ids are positions plus one (held by a test in `afterfetch-art`), so no search is needed.
		TONES.get(usize::from(self as u8).wrapping_sub(1))
	}

	/// The console colour slot (0 to 15) this tone is drawn with at 16 colours, or None for `Plain` and for a tone with no 16-colour
	/// match (soot), which is then drawn as transparent.
	pub fn index(self) -> Option<u8> {
		self.entry()?.console16
	}

	/// The tone's design colour as `[red, green, blue]`, or None for `Plain`.
	pub fn rgb(self) -> Option<[u8; 3]> {
		Some(self.entry()?.rgb)
	}

	/// The xterm 256-colour number, or None for `Plain`.
	pub fn xterm256(self) -> Option<u8> {
		Some(self.entry()?.xterm256)
	}

	/// How the tone is drawn on a mono display: the sun and the horizon stand out, the water recedes.
	pub fn mono_weight(self) -> Weight {
		match self.entry().and_then(|entry| entry.mono) {
			Some(Mono::Bright) => Weight::Bold,
			Some(Mono::Dim) => Weight::Dim,
			_ => Weight::Normal,
		}
	}

	/// True when the tone is drawn at `depth`. A tone with no 16-colour match is transparent at 16 colours, and one with no mono weight
	/// is transparent in mono (`docs/logo-format.md`, depth forms). Without colour (`None`) every character of the ASCII form is drawn.
	pub fn shows(self, depth: Depth) -> bool {
		match depth {
			Depth::None | Depth::Ansi256 | Depth::TrueColour => true,
			Depth::Ansi16 | Depth::Console16 => self.index().is_some(),
			Depth::Mono => self.entry().is_some_and(|entry| entry.mono.is_some()),
		}
	}

	/// A number that is the same for two tones exactly when they look the same at `depth`, so a cell of two such tones is one solid block.
	pub fn shade(self, depth: Depth) -> u32 {
		match depth {
			Depth::None => 0,
			Depth::Mono => self.mono_weight() as u32,
			Depth::Ansi16 | Depth::Console16 => u32::from(self.index().unwrap_or(0)),
			Depth::Ansi256 => u32::from(self.xterm256().unwrap_or(0)),
			Depth::TrueColour => self
				.rgb()
				.map_or(0, |[red, green, blue]| u32::from_be_bytes([0, red, green, blue])),
		}
	}
}

/// The Dusk colour of console slot `slot` (only the low four bits count) as `[red, green, blue]`.
pub const fn slot_rgb(slot: u8) -> [u8; 3] {
	let colour = DUSK[(slot & 0x0F) as usize];
	[(colour >> 16) as u8, (colour >> 8) as u8, colour as u8]
}

/// The ANSI colour number (0 to 15) of console slot `slot`: the console puts blue in bit 0 and red in bit 2, ANSI the other way round.
pub const fn ansi16_of_slot(slot: u8) -> u8 {
	let slot = slot & 0x0F;
	(slot & 0x0A) | ((slot & 1) << 2) | ((slot >> 2) & 1)
}

/// The nearest xterm 256-colour number to `rgb`, from the 6 x 6 x 6 cube and the 24-step grey ramp (the 16 system colours are not used,
/// because terminals draw them from their own theme).
pub const fn xterm256_nearest(rgb: [u8; 3]) -> u8 {
	const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
	let mut cube = [0usize; 3];
	let mut channel = 0;
	while channel < 3 {
		let mut best = 0;
		let mut best_gap = i32::MAX;
		let mut level = 0;
		while level < 6 {
			let gap = (rgb[channel] as i32 - LEVELS[level]).abs();
			if gap < best_gap {
				best_gap = gap;
				best = level;
			}
			level += 1;
		}
		cube[channel] = best;
		channel += 1;
	}
	let cube_distance = distance(rgb, [LEVELS[cube[0]], LEVELS[cube[1]], LEVELS[cube[2]]]);

	let average = (rgb[0] as i32 + rgb[1] as i32 + rgb[2] as i32) / 3;
	let mut step = (average - 8 + 5) / 10;
	if step < 0 {
		step = 0;
	}
	if step > 23 {
		step = 23;
	}
	let grey = 8 + 10 * step;
	if distance(rgb, [grey, grey, grey]) < cube_distance {
		(232 + step) as u8
	} else {
		(16 + 36 * cube[0] + 6 * cube[1] + cube[2]) as u8
	}
}

const fn distance(rgb: [u8; 3], other: [i32; 3]) -> i32 {
	let red = rgb[0] as i32 - other[0];
	let green = rgb[1] as i32 - other[1];
	let blue = rgb[2] as i32 - other[2];
	red * red + green * green + blue * blue
}

/// The console attribute for text in `tone` over the console's own background: `default` is the attribute the console had at the start.
/// Bits above the colour nibbles (such as reverse video) are kept.
pub fn text_attribute(default: u16, tone: Tone) -> u16 {
	match tone.index() {
		None => default,
		Some(index) => (default & !0x000F) | index as u16,
	}
}

/// The console attribute for a filled cell in `tone`: a space on a background of that colour.
pub fn fill_attribute(default: u16, tone: Tone) -> u16 {
	match tone.index() {
		None => default,
		Some(index) => (default & !0x00F0) | ((index as u16) << 4),
	}
}

/// The attribute of a swatch cell: console colour `index` as the background.
pub const fn swatch_attribute(default: u16, index: u8) -> u16 {
	(default & !0x00F0) | (((index & 0x0F) as u16) << 4)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tones_map_to_console_colours_the_image_sets() {
		// The Afterglow Dusk console table, as the image writes it (desktop/install/afterglow-original.reg in Afterglow).
		let wanted = [
			(Tone::Gold, 0xffd08e),
			(Tone::Orange, 0xffaa40),
			(Tone::Coral, 0xf995a6),
			(Tone::Rose, 0xf43f5e),
			(Tone::Lilac, 0xe4b8f8),
			(Tone::Violet, 0xc98be6),
			(Tone::Periwinkle, 0x7f95f2),
			(Tone::Slate, 0x7c8796),
		];
		for (tone, colour) in wanted {
			assert_eq!(DUSK[tone.index().unwrap() as usize], colour, "{tone:?}");
		}
		assert_eq!(Tone::Plain.index(), None);
		assert_eq!(Tone::Plain.rgb(), None);
	}

	#[test]
	fn attributes_keep_the_console_background() {
		// A console of colour 7 on colour 0 (0x07), and of colour 0 on colour 15 (0xF0, the light scheme).
		assert_eq!(text_attribute(0x07, Tone::Gold), 0x0e);
		assert_eq!(text_attribute(0x07, Tone::Plain), 0x07);
		assert_eq!(text_attribute(0xF0, Tone::Rose), 0xF4);
		assert_eq!(fill_attribute(0x07, Tone::Gold), 0xe7);
		assert_eq!(fill_attribute(0x07, Tone::Plain), 0x07);
		assert_eq!(swatch_attribute(0x07, 9), 0x97);
		// Bits above the colours, such as reverse video, are not disturbed.
		assert_eq!(text_attribute(0x4007, Tone::Violet), 0x4005);
	}

	#[test]
	fn console_slots_swap_to_ansi_numbers() {
		// Console: 1 blue, 2 green, 4 red; ANSI: 1 red, 2 green, 4 blue. The bright bit stays at 8.
		assert_eq!(ansi16_of_slot(0), 0);
		assert_eq!(ansi16_of_slot(1), 4);
		assert_eq!(ansi16_of_slot(4), 1);
		assert_eq!(ansi16_of_slot(6), 3);
		assert_eq!(ansi16_of_slot(7), 7);
		assert_eq!(ansi16_of_slot(12), 9);
		assert_eq!(ansi16_of_slot(14), 11);
		assert_eq!(ansi16_of_slot(15), 15);
	}

	#[test]
	fn xterm_256_picks_cube_and_grey_entries() {
		assert_eq!(xterm256_nearest([0, 0, 0]), 16);
		assert_eq!(xterm256_nearest([255, 255, 255]), 231);
		assert_eq!(xterm256_nearest([255, 0, 0]), 196);
		assert_eq!(xterm256_nearest([128, 128, 128]), 244);
		assert_eq!(xterm256_nearest(Tone::Gold.rgb().unwrap()), 222);
	}

	#[test]
	fn every_tone_has_a_weight_and_the_sun_is_bold() {
		assert_eq!(Tone::Gold.mono_weight(), Weight::Bold);
		assert_eq!(Tone::Slate.mono_weight(), Weight::Dim);
		assert_eq!(Tone::Plain.mono_weight(), Weight::Normal);
	}
}
