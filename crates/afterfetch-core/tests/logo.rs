// The logo decoder against the real blobs and against hostile ones: the ASCII forms read back as authored, and no truncation, corruption
// or random blob makes the reader panic or draw outside its buffer.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{Blocks, Depth, Logo, LogoError, Style, Terminal};

const DEPTHS: [Depth; 6] = [
	Depth::None,
	Depth::Mono,
	Depth::Ansi16,
	Depth::Ansi256,
	Depth::TrueColour,
	Depth::Console16,
];
const BLOCKS: [Blocks; 3] = [Blocks::None, Blocks::Utf8, Blocks::Cp437];

/// Counts what a drawing writes and checks every row stays within the logo's columns.
#[derive(Default)]
struct Counter {
	bytes: usize,
}

impl Terminal for Counter {
	fn write(&mut self, bytes: &[u8]) {
		self.bytes += bytes.len();
	}
	fn set_style(&mut self, _style: Style) {}
	fn end_line(&mut self) {}
}

/// Draws a parsed logo in every form at every depth.
fn draw_everywhere(logo: &Logo) {
	draw_in(logo, &DEPTHS, &BLOCKS);
}

/// Draws a parsed logo at the given depths and block forms.
fn draw_in(logo: &Logo, depths: &[Depth], blocks: &[Blocks]) {
	for &depth in depths {
		for &blocks in blocks {
			let mut drawing = logo.drawing(depth, blocks);
			let mut counter = Counter::default();
			for _ in 0..drawing.rows() {
				let columns = drawing.draw_row(&mut counter);
				assert!(columns <= 48);
			}
		}
	}
}

/// The authored text of a logo's ASCII form: art lines and tone lines alternate.
fn authored(name: &str) -> Vec<(String, String)> {
	let path = format!("{}/../../art/ascii/{name}.txt", env!("CARGO_MANIFEST_DIR"));
	let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{path} is missing"));
	let lines: Vec<&str> = text.lines().collect();
	lines
		.chunks(2)
		.map(|pair| {
			(
				pair[0].trim_end().to_string(),
				pair.get(1).map_or(String::new(), |tones| tones.trim_end().to_string()),
			)
		})
		.collect()
}

#[test]
fn every_real_blob_is_valid_and_agrees_with_its_index_entry() {
	for entry in afterfetch_art::LOGOS {
		let logo = Logo::parse(entry.blob).unwrap_or_else(|error| panic!("{}: {error:?}", entry.name));
		assert_eq!(logo.width(), usize::from(entry.width), "{}", entry.name);
		assert_eq!(logo.height(), usize::from(entry.height), "{}", entry.name);
		assert_eq!(
			logo.ascii_size(),
			(usize::from(entry.ascii_width), usize::from(entry.ascii_rows)),
			"{}",
			entry.name
		);
		assert_eq!(
			logo.info_column(),
			usize::from(entry.width.max(entry.ascii_width)) + 2,
			"{}",
			entry.name
		);
		assert_eq!(logo.low_rows().is_some(), entry.low.is_some(), "{}", entry.name);
		draw_everywhere(&logo);
	}
}

#[test]
fn the_ascii_forms_decode_to_the_authored_text() {
	for entry in afterfetch_art::LOGOS {
		let logo = Logo::parse(entry.blob).unwrap();
		let wanted = authored(entry.name);
		let mut rows = logo.ascii_rows().unwrap();
		let (mut art, mut tones) = ([0; 48], [0; 48]);
		assert_eq!(wanted.len(), logo.ascii_size().1, "{}", entry.name);
		for (number, (wanted_art, wanted_tones)) in wanted.iter().enumerate() {
			let length = rows.next_row(&mut art, &mut tones).unwrap();
			let decoded: String = art[..length].iter().map(|&byte| char::from(byte)).collect();
			assert_eq!(decoded.trim_end(), wanted_art, "{} row {number}", entry.name);
			let letters: String = tones[..length]
				.iter()
				.map(|&index| match logo.tone(index) {
					afterfetch_core::Tone::Plain => ' ',
					tone => char::from(afterfetch_art::tone(tone as u8).unwrap().letter),
				})
				.collect();
			assert_eq!(letters.trim_end(), wanted_tones, "{} row {number} tones", entry.name);
		}
		assert!(rows.finished(), "{}", entry.name);
	}
}

#[test]
fn every_truncation_of_every_blob_is_refused() {
	for entry in afterfetch_art::LOGOS {
		for length in 0..entry.blob.len() {
			let result = Logo::parse(&entry.blob[..length]);
			// A cut that lands on a section boundary leaves a shorter blob that is missing a section, never a valid one.
			assert!(result.is_err(), "{} cut to {length} bytes parsed", entry.name);
		}
	}
}

#[test]
fn every_single_byte_corruption_is_refused_or_drawn_safely() {
	for entry in afterfetch_art::LOGOS {
		let mut blob = entry.blob.to_vec();
		for at in 0..blob.len() {
			let original = blob[at];
			for value in 0..=255u8 {
				blob[at] = value;
				if let Ok(logo) = Logo::parse(&blob) {
					// The half-block form, the cells form and the ASCII form, once each, to keep the sweep quick.
					draw_in(&logo, &[Depth::TrueColour, Depth::Mono], &[Blocks::Utf8]);
					draw_in(&logo, &[Depth::Console16, Depth::None], &[Blocks::None]);
				}
			}
			blob[at] = original;
		}
	}
}

#[test]
fn random_blobs_behind_a_valid_header_never_panic() {
	let mut state = 0x2545_F491_4F6C_DD1Du64;
	let mut next = move || {
		state ^= state << 13;
		state ^= state >> 7;
		state ^= state << 17;
		state
	};
	let mut parsed = 0;
	for _ in 0..40_000 {
		let length = (next() % 120) as usize;
		let mut blob = b"AFL1".to_vec();
		blob.extend_from_slice(&[
			(next() % 6 + 1) as u8,
			((next() % 3 + 1) * 2) as u8,
			(next() % 8) as u8,
			(next() % 3) as u8,
			(next() % 3 + 1) as u8,
			1,
			2,
			3,
		]);
		blob.extend((0..length).map(|_| next() as u8));
		if let Ok(logo) = Logo::parse(&blob) {
			parsed += 1;
			draw_everywhere(&logo);
		}
	}
	// Almost all random blobs are refused; the point is that none of them panic.
	assert!(parsed < 40_000);
}

#[test]
fn crafted_bad_blobs_are_refused() {
	let cases: [(&[u8], LogoError); 6] = [
		(b"", LogoError::Truncated),
		(b"AFL", LogoError::Truncated),
		(b"AFL2\x04\x02\x00\x00\x01\x01\x01\x04\x00\x00\x11\x00\x30", LogoError::Magic),
		(
			b"AFL1\x04\x02\x00\x00\x01\x01\x01\x04\x00\x00\x11\x00\x30\x80\x00\x00",
			LogoError::Section,
		),
		(
			b"AFL1\x04\x02\x00\x00\x01\x01\x01\x04\x00\x00\x11\x00\x30\x01\x00\x00",
			LogoError::Section,
		),
		(
			b"AFL1\x04\x02\x00\x00\x01\x01\x01\x04\x00\x00\x11\x00\x30\x03\xFF\xFF",
			LogoError::Truncated,
		),
	];
	for (blob, wanted) in cases {
		assert_eq!(Logo::parse(blob).unwrap_err(), wanted, "{blob:?}");
	}
	// The same blob, ending after its pixels, is whole.
	assert!(Logo::parse(b"AFL1\x04\x02\x00\x00\x01\x01\x01\x04\x00\x00\x11\x00\x30").is_ok());
}

#[test]
fn the_largest_logo_the_format_allows_draws_within_its_buffers() {
	let mut blob = b"AFL1".to_vec();
	blob.extend_from_slice(&[48, 48, 48, 24, 15]);
	blob.extend(1..=15);
	// 48 pixels of index 15 in three runs of 16, for 48 rows.
	let pixels: Vec<u8> = (0..48).flat_map(|_| [0xFF, 0xFF, 0xFF]).collect();
	blob.extend_from_slice(&[1, pixels.len() as u8, (pixels.len() >> 8) as u8]);
	blob.extend_from_slice(&pixels);
	// 24 rows of 48 hashes: `#`, then a repeat of 47 more; its tones are three runs of 16 of index 1.
	let ascii: Vec<u8> = (0..24).flat_map(|_| [2, b'#', 0xAE, 0xF1, 0xF1, 0xF1]).collect();
	blob.extend_from_slice(&[3, ascii.len() as u8, (ascii.len() >> 8) as u8]);
	blob.extend_from_slice(&ascii);
	let logo = Logo::parse(&blob).unwrap();
	assert_eq!(logo.info_column(), 50);
	draw_everywhere(&logo);
}
