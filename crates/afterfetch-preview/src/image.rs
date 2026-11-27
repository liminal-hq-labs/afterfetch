// A canvas of pixels drawn from a character grid, and a small PNG encoder for it: an indexed or RGB image, fixed-Huffman deflate
// with a greedy match search, no dependencies. The same canvas always encodes to the same bytes.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::HashMap;

use crate::colour::{Rgb, Theme, style_colours};
use crate::font::{GLYPH_HEIGHT, GLYPH_WIDTH, glyph};
use crate::grid::Grid;

/// A character cell is this many pixels wide and tall: a 1:2 cell, so a logo's half-block pixels are square.
pub const CELL_WIDTH: usize = 12;
pub const CELL_HEIGHT: usize = 24;
const GLYPH_SCALE_X: usize = 2;
const GLYPH_SCALE_Y: usize = 3;

/// A rectangle of pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
	pub width: usize,
	pub height: usize,
	pixels: Vec<Rgb>,
}

impl Canvas {
	pub fn new(width: usize, height: usize, fill: Rgb) -> Canvas {
		Canvas {
			width,
			height,
			pixels: vec![fill; width * height],
		}
	}

	pub fn pixel(&self, x: usize, y: usize) -> Rgb {
		self.pixels[y * self.width + x]
	}

	pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, colour: Rgb) {
		for row in y..(y + height).min(self.height) {
			for column in x..(x + width).min(self.width) {
				self.pixels[row * self.width + column] = colour;
			}
		}
	}

	/// Draws one character cell at pixel (`x`, `y`): a half block fills half of the cell, anything else is a glyph.
	pub fn draw_cell(&mut self, x: usize, y: usize, ch: char, foreground: Rgb, background: Rgb) {
		let half = CELL_HEIGHT / 2;
		self.fill_rect(x, y, CELL_WIDTH, CELL_HEIGHT, background);
		match ch {
			'\u{2580}' => self.fill_rect(x, y, CELL_WIDTH, half, foreground),
			'\u{2584}' => self.fill_rect(x, y + half, CELL_WIDTH, half, foreground),
			'\u{2588}' => self.fill_rect(x, y, CELL_WIDTH, CELL_HEIGHT, foreground),
			_ => {
				let offset_x = (CELL_WIDTH - GLYPH_WIDTH * GLYPH_SCALE_X) / 2;
				let offset_y = (CELL_HEIGHT - GLYPH_HEIGHT * GLYPH_SCALE_Y) / 2;
				for (column, bits) in glyph(ch).into_iter().enumerate() {
					for row in 0..GLYPH_HEIGHT {
						if bits >> row & 1 == 1 {
							self.fill_rect(
								x + offset_x + column * GLYPH_SCALE_X,
								y + offset_y + row * GLYPH_SCALE_Y,
								GLYPH_SCALE_X,
								GLYPH_SCALE_Y,
								foreground,
							);
						}
					}
				}
			}
		}
	}

	/// Draws a whole grid with its top left corner at pixel (`x`, `y`), over `theme`'s background, `columns` cells wide at least.
	pub fn draw_grid(&mut self, x: usize, y: usize, grid: &Grid, columns: usize, theme: Theme) {
		for row in 0..grid.height() {
			for column in 0..columns.max(grid.width()) {
				let (ch, foreground, background) = match grid.rows[row].get(column) {
					Some(cell) => {
						let (foreground, background) = style_colours(cell.style, grid.depth, theme);
						(cell.ch, foreground, background)
					}
					None => (' ', theme.foreground, theme.background),
				};
				self.draw_cell(x + column * CELL_WIDTH, y + row * CELL_HEIGHT, ch, foreground, background);
			}
		}
	}

	/// Draws `text` as a line of cells.
	pub fn draw_text(&mut self, x: usize, y: usize, text: &str, foreground: Rgb, background: Rgb) {
		for (column, ch) in text.chars().enumerate() {
			self.draw_cell(x + column * CELL_WIDTH, y, ch, foreground, background);
		}
	}

	/// The image as a PNG file.
	pub fn to_png(&self) -> Vec<u8> {
		let mut palette: Vec<Rgb> = Vec::new();
		let mut index_of: HashMap<Rgb, u8> = HashMap::new();
		let mut indexed = true;
		for &pixel in &self.pixels {
			if let std::collections::hash_map::Entry::Vacant(slot) = index_of.entry(pixel) {
				if palette.len() == 256 {
					indexed = false;
					break;
				}
				slot.insert(palette.len() as u8);
				palette.push(pixel);
			}
		}
		let mut raw = Vec::with_capacity(self.pixels.len() * if indexed { 1 } else { 3 } + self.height);
		for row in self.pixels.chunks(self.width) {
			raw.push(0);
			for pixel in row {
				if indexed {
					raw.push(index_of[pixel]);
				} else {
					raw.extend_from_slice(pixel);
				}
			}
		}
		let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
		let mut header = Vec::new();
		header.extend_from_slice(&(self.width as u32).to_be_bytes());
		header.extend_from_slice(&(self.height as u32).to_be_bytes());
		header.extend_from_slice(&[8, if indexed { 3 } else { 2 }, 0, 0, 0]);
		chunk(&mut png, b"IHDR", &header);
		if indexed {
			chunk(&mut png, b"PLTE", &palette.concat());
		}
		chunk(&mut png, b"IDAT", &zlib(&raw));
		chunk(&mut png, b"IEND", &[]);
		png
	}
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
	out.extend_from_slice(&(data.len() as u32).to_be_bytes());
	let start = out.len();
	out.extend_from_slice(kind);
	out.extend_from_slice(data);
	let checksum = crc32(&out[start..]);
	out.extend_from_slice(&checksum.to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
	let mut crc = !0u32;
	for &byte in bytes {
		crc ^= u32::from(byte);
		for _ in 0..8 {
			crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
		}
	}
	!crc
}

fn adler32(bytes: &[u8]) -> u32 {
	let (mut low, mut high) = (1u32, 0u32);
	for &byte in bytes {
		low = (low + u32::from(byte)) % 65521;
		high = (high + low) % 65521;
	}
	(high << 16) | low
}

/// A zlib stream holding `data` as one fixed-Huffman deflate block.
pub fn zlib(data: &[u8]) -> Vec<u8> {
	let mut out = vec![0x78, 0x01];
	out.extend(deflate(data));
	out.extend_from_slice(&adler32(data).to_be_bytes());
	out
}

const LENGTH_BASE: [u16; 29] = [
	3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
	0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
	1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193,
	12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

struct Bits {
	bytes: Vec<u8>,
	accumulator: u64,
	count: u32,
}

impl Bits {
	/// Writes the low `count` bits of `value`, least significant first.
	fn put(&mut self, value: u32, count: u32) {
		self.accumulator |= u64::from(value) << self.count;
		self.count += count;
		while self.count >= 8 {
			self.bytes.push(self.accumulator as u8);
			self.accumulator >>= 8;
			self.count -= 8;
		}
	}

	/// Writes a Huffman code, which deflate packs most significant bit first.
	fn put_code(&mut self, code: u32, length: u32) {
		let reversed = code.reverse_bits() >> (32 - length);
		self.put(reversed, length);
	}

	fn literal_or_length(&mut self, symbol: u32) {
		match symbol {
			0..=143 => self.put_code(0x30 + symbol, 8),
			144..=255 => self.put_code(0x190 + symbol - 144, 9),
			256..=279 => self.put_code(symbol - 256, 7),
			_ => self.put_code(0xc0 + symbol - 280, 8),
		}
	}
}

/// One fixed-Huffman block with greedy LZ77 matches, found through chains of earlier positions with the same three bytes.
fn deflate(data: &[u8]) -> Vec<u8> {
	const WINDOW: usize = 32768;
	const TRIES: usize = 12;
	const HASH_SIZE: usize = 1 << 15;
	let mut bits = Bits {
		bytes: Vec::new(),
		accumulator: 0,
		count: 0,
	};
	bits.put(1, 1);
	bits.put(1, 2);
	let hash =
		|at: usize| (usize::from(data[at]) << 10 ^ usize::from(data[at + 1]) << 5 ^ usize::from(data[at + 2])) & (HASH_SIZE - 1);
	let mut head = vec![usize::MAX; HASH_SIZE];
	let mut previous = vec![usize::MAX; data.len()];
	let insert = |head: &mut Vec<usize>, previous: &mut Vec<usize>, at: usize| {
		if at + 3 <= data.len() {
			let slot = hash(at);
			previous[at] = head[slot];
			head[slot] = at;
		}
	};
	let mut at = 0;
	while at < data.len() {
		let (mut best_length, mut best_distance) = (0, 0);
		if at + 3 <= data.len() {
			let mut candidate = head[hash(at)];
			let mut tries = 0;
			while candidate != usize::MAX && at - candidate <= WINDOW && tries < TRIES {
				let limit = 258.min(data.len() - at);
				let mut length = 0;
				while length < limit && data[candidate + length] == data[at + length] {
					length += 1;
				}
				if length > best_length {
					best_length = length;
					best_distance = at - candidate;
				}
				candidate = previous[candidate];
				tries += 1;
			}
		}
		if best_length >= 3 {
			let length_code = LENGTH_BASE
				.iter()
				.rposition(|&base| usize::from(base) <= best_length)
				.unwrap();
			bits.literal_or_length(257 + length_code as u32);
			bits.put(
				(best_length - usize::from(LENGTH_BASE[length_code])) as u32,
				u32::from(LENGTH_EXTRA[length_code]),
			);
			let distance_code = DISTANCE_BASE
				.iter()
				.rposition(|&base| usize::from(base) <= best_distance)
				.unwrap();
			bits.put_code(distance_code as u32, 5);
			bits.put(
				(best_distance - usize::from(DISTANCE_BASE[distance_code])) as u32,
				u32::from(DISTANCE_EXTRA[distance_code]),
			);
			for step in 0..best_length {
				insert(&mut head, &mut previous, at + step);
			}
			at += best_length;
		} else {
			bits.literal_or_length(u32::from(data[at]));
			insert(&mut head, &mut previous, at);
			at += 1;
		}
	}
	bits.literal_or_length(256);
	bits.put(0, 7);
	bits.bytes
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A decoder for the fixed-Huffman blocks this encoder writes, to check the round trip.
	pub fn inflate_fixed(stream: &[u8]) -> Vec<u8> {
		let mut position = 0usize;
		let mut read = |count: u32| -> u32 {
			let mut value = 0;
			for shift in 0..count {
				let bit = stream[position / 8] >> (position % 8) & 1;
				value |= u32::from(bit) << shift;
				position += 1;
			}
			value
		};
		assert_eq!(read(1), 1, "final block");
		assert_eq!(read(2), 1, "fixed Huffman");
		let mut out = Vec::new();
		loop {
			// Read a code a bit at a time, most significant first, until it is one of the fixed codes.
			let mut code = 0u32;
			let mut length = 0;
			let symbol = loop {
				code = code << 1 | read(1);
				length += 1;
				match (length, code) {
					(7, 0..=23) => break 256 + code,
					(8, 0x30..=0xbf) => break code - 0x30,
					(8, 0xc0..=0xc7) => break 280 + code - 0xc0,
					(9, 0x190..=0x1ff) => break 144 + code - 0x190,
					(9..=u32::MAX, _) => panic!("bad code"),
					_ => {}
				}
			};
			match symbol {
				256 => break,
				0..=255 => out.push(symbol as u8),
				_ => {
					let index = (symbol - 257) as usize;
					let length = usize::from(LENGTH_BASE[index]) + read(u32::from(LENGTH_EXTRA[index])) as usize;
					let mut distance_code = 0usize;
					for _ in 0..5 {
						distance_code = distance_code << 1 | read(1) as usize;
					}
					let distance =
						usize::from(DISTANCE_BASE[distance_code]) + read(u32::from(DISTANCE_EXTRA[distance_code])) as usize;
					for _ in 0..length {
						out.push(out[out.len() - distance]);
					}
				}
			}
		}
		out
	}

	#[test]
	fn deflate_round_trips_runs_text_and_noise() {
		let mut noise = Vec::new();
		let mut state = 12345u32;
		for _ in 0..5000 {
			state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
			noise.push((state >> 24) as u8);
		}
		let inputs: [Vec<u8>; 5] = [
			Vec::new(),
			vec![7; 100_000],
			b"abcabcabcabc the quick brown fox jumps over the lazy dog the quick brown fox".repeat(40),
			noise,
			(0..=255).collect(),
		];
		for input in inputs {
			let stream = deflate(&input);
			assert_eq!(inflate_fixed(&stream), input);
		}
	}

	#[test]
	fn checksums_match_known_values() {
		assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
		assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
	}

	#[test]
	fn a_flat_image_is_tiny_and_the_same_every_time() {
		let canvas = Canvas::new(200, 100, [1, 2, 3]);
		let png = canvas.to_png();
		assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
		assert!(png.len() < 300, "{}", png.len());
		assert_eq!(png, canvas.to_png());
	}

	#[test]
	fn more_than_256_colours_fall_back_to_rgb() {
		let mut canvas = Canvas::new(300, 1, [0, 0, 0]);
		for x in 0..300 {
			canvas.fill_rect(x, 0, 1, 1, [(x % 256) as u8, (x / 256) as u8, 9]);
		}
		let png = canvas.to_png();
		assert_eq!(png[25], 2, "colour type is RGB");
	}

	#[test]
	fn a_half_block_fills_half_the_cell() {
		let mut canvas = Canvas::new(CELL_WIDTH, CELL_HEIGHT, [0, 0, 0]);
		canvas.draw_cell(0, 0, '\u{2580}', [9, 9, 9], [1, 1, 1]);
		assert_eq!(canvas.pixel(3, 0), [9, 9, 9]);
		assert_eq!(canvas.pixel(3, CELL_HEIGHT / 2 - 1), [9, 9, 9]);
		assert_eq!(canvas.pixel(3, CELL_HEIGHT / 2), [1, 1, 1]);
	}
}
