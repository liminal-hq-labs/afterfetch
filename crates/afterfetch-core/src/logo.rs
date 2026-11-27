// The Afterglow sunset as ASCII. Each row is the picture beside a row of the same width that names the tone of every character, so the
// gradient can be read straight off the page: gold sun, orange, coral at the horizon, a rose line, then water from lilac through
// violet and periwinkle to slate.
//
// A `#` is the sun's body. With colour it is drawn as a solid cell (a space on a coloured background); without, it stays a `#`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::palette::Tone;

/// The logos `--logo` can name. Until the art generator lands every name draws the `win32` rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoName {
	Afterglow,
	Dos,
	Win16,
	Win32,
	Linux,
	FreeBsd,
	MacOs,
	Desk,
}

impl LogoName {
	/// Every logo, in the order `--logo list` prints them.
	pub const ALL: [LogoName; 8] = [
		LogoName::Afterglow,
		LogoName::Dos,
		LogoName::Win16,
		LogoName::Win32,
		LogoName::Linux,
		LogoName::FreeBsd,
		LogoName::MacOs,
		LogoName::Desk,
	];

	/// The name as typed on the command line.
	pub const fn name(self) -> &'static str {
		match self {
			LogoName::Afterglow => "afterglow",
			LogoName::Dos => "dos",
			LogoName::Win16 => "win16",
			LogoName::Win32 => "win32",
			LogoName::Linux => "linux",
			LogoName::FreeBsd => "freebsd",
			LogoName::MacOs => "macos",
			LogoName::Desk => "desk",
		}
	}

	/// The logo called `name`, in any case.
	pub fn parse(name: &[u8]) -> Option<LogoName> {
		LogoName::ALL
			.into_iter()
			.find(|logo| name.eq_ignore_ascii_case(logo.name().as_bytes()))
	}

	/// The ASCII form of this logo.
	pub const fn ascii(self) -> &'static Ascii {
		&WIN32
	}
}

/// One row of the picture.
pub struct Row {
	pub art: &'static str,
	/// One letter per character of `art`: g gold, o orange, c coral, r rose, l lilac, v violet, p periwinkle, s slate, space plain.
	pub tones: &'static str,
}

/// A logo's ASCII form and the column where the information starts, which is also the width the logo is padded to.
pub struct Ascii {
	pub rows: &'static [Row],
	pub info_column: usize,
}

/// The column where the information starts beside the `win32` logo.
pub const INFO_COLUMN: usize = 42;

pub const ROWS: [Row; 12] = [
	Row {
		art: "                 .   .   .",
		tones: "                 l   l   l",
	},
	Row {
		art: "           .      \\  :  /      .",
		tones: "           l      g  g  g      l",
	},
	Row {
		art: "               .   \\ : /   .",
		tones: "               l   g g g   l",
	},
	Row {
		art: "                .:########:.",
		tones: "                gggggggggggg",
	},
	Row {
		art: "            .:##############:.",
		tones: "            gggggggggggggggggg",
	},
	Row {
		art: "           :##################:",
		tones: "           oooooooooooooooooooo",
	},
	Row {
		art: "       ====:##################:====",
		tones: "       rrrrccccccccccccccccccccrrrr",
	},
	Row {
		art: "      ----------------------------------",
		tones: "      rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr",
	},
	Row {
		art: "       ~~~~  ~~~~~~~~~  ~~~~~~~  ~~~~",
		tones: "       llll  vvvvvvvvv  lllllll  vvvv",
	},
	Row {
		art: "         ~~~~~~~  ~~~~~~~~~~  ~~~~~~",
		tones: "         vvvvvvv  pppppppppp  vvvvvv",
	},
	Row {
		art: "            ~~~~~~~~~~  ~~~~~~~~~",
		tones: "            pppppppppp  sssssssss",
	},
	Row {
		art: "               ~~~~~~~~~~~~~~",
		tones: "               ssssssssssssss",
	},
];

static WIN32: Ascii = Ascii {
	rows: &ROWS,
	info_column: INFO_COLUMN,
};

/// The tone a letter of a `tones` row names.
pub const fn tone_of(letter: u8) -> Tone {
	match letter {
		b'g' => Tone::Gold,
		b'o' => Tone::Orange,
		b'c' => Tone::Coral,
		b'r' => Tone::Rose,
		b'l' => Tone::Lilac,
		b'v' => Tone::Violet,
		b'p' => Tone::Periwinkle,
		b's' => Tone::Slate,
		_ => Tone::Plain,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_row_names_a_tone_for_each_character() {
		for (number, row) in ROWS.iter().enumerate() {
			assert_eq!(row.art.len(), row.tones.len(), "row {number}");
			assert!(row.art.is_ascii(), "row {number}");
			assert!(row.art.len() < INFO_COLUMN, "row {number} reaches the information column");
		}
	}

	#[test]
	fn only_known_letters_and_spaces_are_used() {
		for row in &ROWS {
			for letter in row.tones.bytes() {
				assert!(b" gocrlvps".contains(&letter), "unknown tone letter {}", letter as char);
			}
		}
	}

	#[test]
	fn spaces_are_plain_and_drawn_characters_are_toned() {
		for row in &ROWS {
			for (character, letter) in row.art.bytes().zip(row.tones.bytes()) {
				assert_eq!(character == b' ', letter == b' ');
			}
		}
	}

	#[test]
	fn the_sun_is_solid_and_toned_by_row() {
		let rows_with_sun: Vec<usize> = ROWS
			.iter()
			.enumerate()
			.filter(|(_, row)| row.art.contains('#'))
			.map(|(number, _)| number)
			.collect();
		assert_eq!(rows_with_sun, [3, 4, 5, 6]);
		for number in rows_with_sun {
			let row = &ROWS[number];
			for (character, letter) in row.art.bytes().zip(row.tones.bytes()) {
				if character == b'#' {
					assert_ne!(tone_of(letter), Tone::Plain);
				}
			}
		}
	}

	#[test]
	fn names_round_trip_and_ignore_case() {
		for logo in LogoName::ALL {
			assert_eq!(LogoName::parse(logo.name().as_bytes()), Some(logo));
		}
		assert_eq!(LogoName::parse(b"FreeBSD"), Some(LogoName::FreeBsd));
		assert_eq!(LogoName::parse(b"tux"), None);
		assert_eq!(LogoName::parse(b""), None);
	}
}
