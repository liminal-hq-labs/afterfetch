// The `.afl` logo decoder and the three ways a logo is drawn: half-block cells, plain cells and the ASCII form, at every depth.
//
// A blob is validated once, whole, when it is opened (`Logo::parse`), and drawn row by row afterwards into a 48-cell buffer on the stack:
// nothing is allocated, nothing is built up front and no input can make the reader panic. The format is `docs/logo-format.md`; the colours
// each tone shows at each depth are the generated tables in `afterfetch-art`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::depth::Depth;
use crate::format::Buf;
use crate::palette::{Tone, Weight};
use crate::sink::{Paint, Style, Terminal};

/// The widest picture or ASCII form a blob may hold, in pixels or columns.
pub const MAX_COLUMNS: usize = 48;

/// The logo called `name`, in any case. The same as `afterfetch_art::find`, on bytes, so the arguments need not be UTF-8 (and the
/// program carries no UTF-8 validator for it).
pub fn find(name: &[u8]) -> Option<&'static afterfetch_art::LogoEntry> {
	afterfetch_art::LOGOS
		.iter()
		.find(|logo| name.eq_ignore_ascii_case(logo.name.as_bytes()))
}

/// The gap between the logo and the information beside it.
const GAP: usize = 2;

/// Why a blob was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoError {
	/// The blob ends inside its header or a section.
	Truncated,
	/// The first four bytes are not `AFL1`.
	Magic,
	/// A size in the header is out of range, or the palette is.
	Header,
	/// A section is missing, repeated, out of order, of a kind a reader must understand and does not, or present when the header says
	/// it should not be.
	Section,
	/// The pixel rows do not add up.
	Pixels,
	/// The ASCII rows do not add up.
	Ascii,
}

/// A validated logo: the sizes, the palette and where each section's bytes are.
#[derive(Clone, Copy, Debug)]
pub struct Logo<'a> {
	width: u8,
	height: u8,
	ascii_width: u8,
	ascii_rows: u8,
	palette: &'a [u8],
	pixels: &'a [u8],
	low: Option<&'a [u8]>,
	ascii: Option<&'a [u8]>,
}

impl<'a> Logo<'a> {
	/// Reads and validates a whole blob. Every length is checked before it is used, so a truncated or hostile blob is an error.
	pub fn parse(blob: &'a [u8]) -> Result<Logo<'a>, LogoError> {
		let header = blob.first_chunk::<9>().ok_or(LogoError::Truncated)?;
		let [a, f, l, one, width, height, ascii_width, ascii_rows, palette_len] = *header;
		if [a, f, l, one] != *b"AFL1" {
			return Err(LogoError::Magic);
		}
		let in_range = (1..=48).contains(&width)
			&& (2..=48).contains(&height)
			&& height % 2 == 0
			&& ascii_width <= 48
			&& ascii_rows <= 24
			&& (ascii_width == 0) == (ascii_rows == 0)
			&& (1..=15).contains(&palette_len);
		if !in_range {
			return Err(LogoError::Header);
		}
		let palette = blob.get(9..9 + usize::from(palette_len)).ok_or(LogoError::Truncated)?;
		let mut seen = 0u16;
		for &id in palette {
			if !(1..=15).contains(&id) || seen >> id & 1 != 0 {
				return Err(LogoError::Header);
			}
			seen |= 1 << id;
		}

		let mut logo = Logo {
			width,
			height,
			ascii_width,
			ascii_rows,
			palette,
			pixels: &[],
			low: None,
			ascii: None,
		};
		let mut at = 9 + palette.len();
		let mut last = 0;
		let mut have_pixels = false;
		while at < blob.len() {
			let &[kind, low, high] = blob
				.get(at..)
				.and_then(<[u8]>::first_chunk::<3>)
				.ok_or(LogoError::Truncated)?;
			let length = usize::from(u16::from_le_bytes([low, high]));
			let payload = blob.get(at + 3..at + 3 + length).ok_or(LogoError::Truncated)?;
			// Kinds ascend and never repeat; 0 is not a kind, 0x04 to 0x7F are skipped, 0x80 and above must be understood.
			if kind == 0 || kind >= 0x80 || kind <= last {
				return Err(LogoError::Section);
			}
			last = kind;
			match kind {
				1 => {
					logo.pixels = payload;
					have_pixels = true;
				}
				2 => logo.low = Some(payload),
				3 => logo.ascii = Some(payload),
				_ => {}
			}
			at += 3 + length;
		}
		if !have_pixels || logo.ascii.is_some() != (ascii_width > 0) {
			return Err(LogoError::Section);
		}

		if !logo.pixels_valid(logo.pixels) || logo.low.is_some_and(|low| !logo.pixels_valid(low)) {
			return Err(LogoError::Pixels);
		}
		if let Some(mut rows) = logo.ascii_rows() {
			let (mut art, mut tones) = ([0; MAX_COLUMNS], [0; MAX_COLUMNS]);
			for _ in 0..ascii_rows {
				rows.next_row(&mut art, &mut tones).ok_or(LogoError::Ascii)?;
			}
			if !rows.finished() {
				return Err(LogoError::Ascii);
			}
		}
		Ok(logo)
	}

	fn pixels_valid(&self, payload: &'a [u8]) -> bool {
		let mut rows = self.rows_of(payload);
		let mut row = [0; MAX_COLUMNS];
		(0..self.height).all(|_| rows.next_row(&mut row).is_some()) && rows.finished()
	}

	fn rows_of(&self, payload: &'a [u8]) -> PixelRows<'a> {
		PixelRows {
			bytes: payload,
			at: 0,
			width: usize::from(self.width),
			palette_len: self.palette.len() as u8,
		}
	}

	/// Logo pixels across.
	pub fn width(&self) -> usize {
		usize::from(self.width)
	}

	/// Logo pixels down; two to a row of cells.
	pub fn height(&self) -> usize {
		usize::from(self.height)
	}

	/// The ASCII form's columns and rows; both 0 when the logo has none.
	pub fn ascii_size(&self) -> (usize, usize) {
		(usize::from(self.ascii_width), usize::from(self.ascii_rows))
	}

	/// The column where the information starts beside the logo, which is also the width the logo is padded to: the widest of its forms
	/// and the gap. It is the same for every form so the facts do not move when the depth does.
	pub fn info_column(&self) -> usize {
		self.width().max(usize::from(self.ascii_width)) + GAP
	}

	/// The tone a palette index stands for; `Plain` for 0 and for an index outside the palette.
	pub fn tone(&self, index: u8) -> Tone {
		index
			.checked_sub(1)
			.and_then(|at| self.palette.get(usize::from(at)))
			.map_or(Tone::Plain, |&id| Tone::from_id(id))
	}

	/// The rows of the picture, top to bottom, one palette index per pixel.
	pub fn pixel_rows(&self) -> PixelRows<'a> {
		self.rows_of(self.pixels)
	}

	/// The rows of the picture for 16 colours, mono and the cells form, when the logo has its own.
	pub fn low_rows(&self) -> Option<PixelRows<'a>> {
		self.low.map(|payload| self.rows_of(payload))
	}

	/// The rows of the ASCII form, when the logo has one.
	pub fn ascii_rows(&self) -> Option<AsciiRows<'a>> {
		self.ascii.map(|payload| AsciiRows {
			bytes: payload,
			at: 0,
			width: usize::from(self.ascii_width),
			palette_len: self.palette.len() as u8,
		})
	}

	/// Starts drawing the logo at `depth`, with half blocks where `blocks` allows them.
	pub fn drawing(&self, depth: Depth, blocks: Blocks) -> Drawing<'a> {
		let pixels = if matches!(depth, Depth::Mono | Depth::Ansi16 | Depth::Console16) {
			self.low_rows().unwrap_or_else(|| self.pixel_rows())
		} else {
			self.pixel_rows()
		};
		let half_rows = self.height() / 2;
		let half = blocks != Blocks::None && depth != Depth::None;
		let (source, rows) = match (half, self.ascii_rows()) {
			(true, _) => (Source::Half(pixels), half_rows),
			(false, Some(ascii)) => (Source::Cells(ascii), usize::from(self.ascii_rows)),
			(false, None) => (Source::PixelCells(pixels), half_rows),
		};
		Drawing {
			logo: *self,
			depth,
			blocks,
			source,
			rows,
		}
	}
}

/// The pixel rows of a picture, decoded one at a time.
#[derive(Clone, Copy, Debug)]
pub struct PixelRows<'a> {
	bytes: &'a [u8],
	at: usize,
	width: usize,
	palette_len: u8,
}

impl PixelRows<'_> {
	/// Decodes the next row into `out`: a palette index per pixel, 0 for transparent. None when the bytes run out, a run crosses the
	/// row's end or an index is outside the palette.
	pub fn next_row(&mut self, out: &mut [u8; MAX_COLUMNS]) -> Option<()> {
		let mut x = 0;
		while x < self.width {
			let byte = *self.bytes.get(self.at)?;
			self.at += 1;
			let run = usize::from(byte >> 4) + 1;
			let index = byte & 0x0F;
			if index > self.palette_len || x + run > self.width {
				return None;
			}
			out.get_mut(x..x + run)?.fill(index);
			x += run;
		}
		Some(())
	}

	/// True when every byte has been used.
	pub fn finished(&self) -> bool {
		self.at == self.bytes.len()
	}
}

/// The rows of an ASCII form, decoded one at a time.
#[derive(Clone, Copy, Debug)]
pub struct AsciiRows<'a> {
	bytes: &'a [u8],
	at: usize,
	width: usize,
	palette_len: u8,
}

impl AsciiRows<'_> {
	/// Decodes the next row: its characters into `art` and a palette index for each into `tones`, both padded with spaces and 0 to the
	/// full 48, and the number of characters before the padding. None when the row is malformed: a byte that is neither a character nor
	/// a repeat, a repeat with nothing before it, more characters than the form is wide, tone runs that do not add up, an index outside
	/// the palette, or a space and its index that disagree.
	pub fn next_row(&mut self, art: &mut [u8; MAX_COLUMNS], tones: &mut [u8; MAX_COLUMNS]) -> Option<usize> {
		art.fill(b' ');
		tones.fill(0);
		let length = usize::from(*self.bytes.get(self.at)?);
		let encoded = self.bytes.get(self.at + 1..self.at + 1 + length)?;
		self.at += 1 + length;

		let mut count = 0usize;
		for &byte in encoded {
			let (character, times) = match byte {
				0x20..=0x7E => (byte, 1),
				0x80..=0xFF => (*art.get(count.checked_sub(1)?)?, usize::from(byte & 0x7F) + 1),
				_ => return None,
			};
			if count + times > self.width {
				return None;
			}
			art.get_mut(count..count + times)?.fill(character);
			count += times;
		}

		let mut x = 0;
		while x < count {
			let byte = *self.bytes.get(self.at)?;
			self.at += 1;
			let run = usize::from(byte >> 4) + 1;
			let index = byte & 0x0F;
			if index > self.palette_len || x + run > count {
				return None;
			}
			tones.get_mut(x..x + run)?.fill(index);
			x += run;
		}
		// A space is plain and every other character has a tone.
		art.iter()
			.zip(tones.iter())
			.all(|(&character, &index)| (character == b' ') == (index == 0))
			.then_some(count)
	}

	/// True when every byte has been used.
	pub fn finished(&self) -> bool {
		self.at == self.bytes.len()
	}
}

/// Whether the terminal can draw half blocks, and in which encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocks {
	/// No: the logo is drawn as cells.
	None,
	/// `▀ ▄ █` in UTF-8.
	Utf8,
	/// The CP437 half blocks `0xDF 0xDC 0xDB`.
	Cp437,
}

/// The half-block glyphs, kept apart from ASCII in a cell's `glyph` byte by the high bit.
const UPPER: u8 = 0x80;
const LOWER: u8 = 0x81;
const FULL: u8 = 0x82;

impl Blocks {
	fn bytes(self, glyph: u8) -> &'static [u8] {
		match (self, glyph) {
			(Blocks::Cp437, UPPER) => b"\xDF",
			(Blocks::Cp437, LOWER) => b"\xDC",
			(Blocks::Cp437, _) => b"\xDB",
			(_, UPPER) => "\u{2580}".as_bytes(),
			(_, LOWER) => "\u{2584}".as_bytes(),
			_ => "\u{2588}".as_bytes(),
		}
	}
}

/// One cell of the picture: a character (or one of the half blocks) and the style it is drawn in.
#[derive(Clone, Copy)]
struct Cell {
	glyph: u8,
	style: Style,
}

const BLANK: Cell = Cell {
	glyph: b' ',
	style: Style::PLAIN,
};

/// Where a drawing's rows come from.
enum Source<'a> {
	/// Pairs of pixel rows, each pair a row of half-block cells.
	Half(PixelRows<'a>),
	/// The ASCII form: characters coloured by tone, with `#` as a filled cell.
	Cells(AsciiRows<'a>),
	/// Pairs of pixel rows, each pixel pair a filled cell, for a logo with no ASCII form.
	PixelCells(PixelRows<'a>),
}

/// A logo being drawn at one depth. Rows are drawn top to bottom.
pub struct Drawing<'a> {
	logo: Logo<'a>,
	depth: Depth,
	blocks: Blocks,
	source: Source<'a>,
	rows: usize,
}

impl Drawing<'_> {
	/// How many rows of cells the logo has in this form.
	pub fn rows(&self) -> usize {
		self.rows
	}

	/// Draws the next row of the logo, with no trailing blank cells and no line end, and returns how many columns it wrote.
	pub fn draw_row(&mut self, out: &mut impl Terminal) -> usize {
		let (logo, depth) = (self.logo, self.depth);
		let mut cells = [BLANK; MAX_COLUMNS];
		let (mut upper, mut lower) = ([0; MAX_COLUMNS], [0; MAX_COLUMNS]);
		let half = matches!(self.source, Source::Half(_));
		match &mut self.source {
			Source::Cells(rows) => {
				if rows.next_row(&mut upper, &mut lower).is_some() {
					for (cell, (&character, &index)) in cells.iter_mut().zip(upper.iter().zip(lower.iter())) {
						*cell = text_cell(&logo, depth, character, index);
					}
				}
			}
			Source::Half(rows) | Source::PixelCells(rows) => {
				if rows.next_row(&mut upper).is_some() && rows.next_row(&mut lower).is_some() {
					for (cell, (&above, &below)) in cells.iter_mut().zip(upper.iter().zip(lower.iter())).take(logo.width()) {
						*cell = if half {
							half_cell(&logo, depth, above, below)
						} else {
							let index = if above != 0 { above } else { below };
							text_cell(&logo, depth, if index == 0 { b' ' } else { b'#' }, index)
						};
					}
				}
			}
		}

		let end = cells
			.iter()
			.rposition(|cell| cell.glyph != b' ' || cell.style != Style::PLAIN)
			.map_or(0, |last| last + 1);
		let mut run = Buf::<{ MAX_COLUMNS * 3 }>::new();
		let mut style = Style::PLAIN;
		for cell in &cells[..end] {
			if cell.style != style {
				out.write(run.as_bytes());
				run.clear();
				out.set_style(cell.style);
				style = cell.style;
			}
			if cell.glyph < 0x80 {
				run.push(&[cell.glyph]);
			} else {
				run.push(self.blocks.bytes(cell.glyph));
			}
		}
		out.write(run.as_bytes());
		end
	}
}

/// A cell of the half-block form: the pixel above and the pixel below.
fn half_cell(logo: &Logo, depth: Depth, upper: u8, lower: u8) -> Cell {
	let shown = |index: u8| Some(logo.tone(index)).filter(|tone| index != 0 && tone.shows(depth));
	match (shown(upper), shown(lower)) {
		(None, None) => BLANK,
		(Some(above), None) => Cell {
			glyph: UPPER,
			style: Style::at(depth, above),
		},
		(None, Some(below)) => Cell {
			glyph: LOWER,
			style: Style::at(depth, below),
		},
		// Two pixels that look alike are one solid block. In mono there is one colour, so two drawn pixels are always one block.
		(Some(above), Some(below)) if depth == Depth::Mono || above.shade(depth) == below.shade(depth) => Cell {
			glyph: FULL,
			style: Style::at(depth, above),
		},
		(Some(above), Some(below)) => Cell {
			glyph: UPPER,
			style: Style {
				fg: Paint::Tone(above),
				bg: Paint::Tone(below),
				weight: Weight::Normal,
			},
		},
	}
}

/// A cell of the cells and ASCII forms: a character and its palette index. With colour a `#` is a filled cell; without, it stays a `#`.
/// A tone that is transparent at the depth draws nothing.
fn text_cell(logo: &Logo, depth: Depth, character: u8, index: u8) -> Cell {
	let tone = logo.tone(index);
	if character == b' ' || !tone.shows(depth) {
		BLANK
	} else if character == b'#' && depth.has_colour() {
		Cell {
			glyph: b' ',
			style: Style::fill(Paint::Tone(tone)),
		}
	} else {
		Cell {
			glyph: character,
			style: Style::at(depth, tone),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The 26-byte example of `docs/logo-format.md`: a 4 x 2 logo with one gold bar and the ASCII form `.##.`.
	const EXAMPLE: [u8; 26] = [
		0x41, 0x46, 0x4C, 0x31, 0x04, 0x02, 0x04, 0x01, 0x01, 0x01, 0x01, 0x04, 0x00, 0x00, 0x11, 0x00, 0x30, 0x03, 0x06, 0x00,
		0x04, 0x2E, 0x23, 0x80, 0x2E, 0x31,
	];

	#[derive(Default)]
	struct Capture {
		text: std::vec::Vec<u8>,
		styles: std::vec::Vec<Style>,
	}

	impl Terminal for Capture {
		fn write(&mut self, bytes: &[u8]) {
			self.text.extend_from_slice(bytes);
		}
		fn set_style(&mut self, style: Style) {
			self.styles.push(style);
		}
		fn end_line(&mut self) {}
	}

	fn draw(blob: &[u8], depth: Depth, blocks: Blocks) -> (std::vec::Vec<std::string::String>, Capture) {
		let (lines, capture) = draw_raw(blob, depth, blocks);
		let lines = lines
			.iter()
			.map(|line| std::string::String::from_utf8_lossy(line).into_owned());
		(lines.collect(), capture)
	}

	fn draw_raw(blob: &[u8], depth: Depth, blocks: Blocks) -> (std::vec::Vec<std::vec::Vec<u8>>, Capture) {
		let logo = Logo::parse(blob).unwrap();
		let mut drawing = logo.drawing(depth, blocks);
		let mut capture = Capture::default();
		let mut lines = std::vec::Vec::new();
		for _ in 0..drawing.rows() {
			capture.text.clear();
			drawing.draw_row(&mut capture);
			lines.push(capture.text.clone());
		}
		(lines, capture)
	}

	#[test]
	fn the_documented_example_decodes() {
		let logo = Logo::parse(&EXAMPLE).unwrap();
		assert_eq!((logo.width(), logo.height(), logo.ascii_size()), (4, 2, (4, 1)));
		assert_eq!(logo.info_column(), 6);
		let mut pixels = logo.pixel_rows();
		let mut row = [9; MAX_COLUMNS];
		pixels.next_row(&mut row).unwrap();
		assert_eq!(&row[..4], [0, 1, 1, 0]);
		pixels.next_row(&mut row).unwrap();
		assert_eq!(&row[..4], [0, 0, 0, 0]);
		assert!(pixels.finished());
		let (mut art, mut tones) = ([0; MAX_COLUMNS], [0; MAX_COLUMNS]);
		let mut ascii = logo.ascii_rows().unwrap();
		assert_eq!(ascii.next_row(&mut art, &mut tones), Some(4));
		assert_eq!(&art[..5], b".##. ");
		assert_eq!(&tones[..5], [1, 1, 1, 1, 0]);
		assert!(ascii.finished());
		assert_eq!(logo.tone(1), Tone::Gold);
		assert_eq!(logo.tone(0), Tone::Plain);
		assert_eq!(logo.tone(2), Tone::Plain);
	}

	#[test]
	fn the_example_draws_in_every_form() {
		let (lines, _) = draw(&EXAMPLE, Depth::None, Blocks::Utf8);
		assert_eq!(lines, [".##."]);
		let (lines, capture) = draw(&EXAMPLE, Depth::TrueColour, Blocks::None);
		assert_eq!(lines, [".  ."]);
		assert!(capture.styles.contains(&Style::fill(Paint::Tone(Tone::Gold))));
		let (lines, capture) = draw(&EXAMPLE, Depth::TrueColour, Blocks::Utf8);
		assert_eq!(lines, [" \u{2580}\u{2580}"]);
		assert_eq!(capture.styles, [Style::text(Tone::Gold)]);
		let (lines, _) = draw_raw(&EXAMPLE, Depth::Console16, Blocks::Cp437);
		assert_eq!(lines[0], b" \xDF\xDF");
		let (lines, capture) = draw(&EXAMPLE, Depth::Mono, Blocks::None);
		assert_eq!(lines, [".##."]);
		assert_eq!(capture.styles, [Style::mono(Tone::Gold)]);
	}

	fn with(edit: impl FnOnce(&mut std::vec::Vec<u8>)) -> Result<(), LogoError> {
		let mut blob = EXAMPLE.to_vec();
		edit(&mut blob);
		Logo::parse(&blob).map(|_| ())
	}

	#[test]
	fn a_bad_header_is_refused() {
		assert_eq!(Logo::parse(&[]).unwrap_err(), LogoError::Truncated);
		assert_eq!(Logo::parse(&EXAMPLE[..8]).unwrap_err(), LogoError::Truncated);
		assert_eq!(with(|blob| blob[3] = b'2'), Err(LogoError::Magic));
		assert_eq!(with(|blob| blob[4] = 0), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[4] = 49), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[5] = 3), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[5] = 0), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[6] = 0), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[7] = 25), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[8] = 0), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[8] = 16), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[9] = 0), Err(LogoError::Header));
		assert_eq!(with(|blob| blob[9] = 16), Err(LogoError::Header));
	}

	#[test]
	fn a_repeated_tone_in_the_palette_is_refused() {
		let mut blob = EXAMPLE.to_vec();
		blob.splice(9..10, [1, 1]);
		blob[8] = 2;
		assert_eq!(Logo::parse(&blob).unwrap_err(), LogoError::Header);
	}

	#[test]
	fn sections_must_be_whole_ordered_and_known() {
		// A section that runs past the end.
		assert_eq!(with(|blob| blob[11] = 200), Err(LogoError::Truncated));
		assert_eq!(with(|blob| blob.truncate(12)), Err(LogoError::Truncated));
		// Kind 0, and kinds a reader must understand.
		assert_eq!(with(|blob| blob[10] = 0), Err(LogoError::Section));
		assert_eq!(with(|blob| blob[17] = 0x80), Err(LogoError::Section));
		assert_eq!(with(|blob| blob[17] = 0xFF), Err(LogoError::Section));
		// A kind that repeats, or comes out of order.
		assert_eq!(with(|blob| blob[17] = 1), Err(LogoError::Section));
		// No pixels.
		assert_eq!(with(|blob| blob[10] = 2), Err(LogoError::Section));
		// ASCII missing while the header promises it, and present while it does not.
		assert_eq!(with(|blob| blob.truncate(17)), Err(LogoError::Section));
		assert_eq!(
			with(|blob| {
				blob[6] = 0;
				blob[7] = 0;
			}),
			Err(LogoError::Section)
		);
	}

	#[test]
	fn reserved_kinds_are_skipped() {
		let mut blob = EXAMPLE.to_vec();
		blob.extend_from_slice(&[0x04, 0x02, 0x00, 0xAA, 0xBB, 0x7F, 0x00, 0x00]);
		assert!(Logo::parse(&blob).is_ok());
		// A low-colour picture is a section of its own, between the pixels and the ASCII.
		let mut blob = EXAMPLE.to_vec();
		blob.splice(17..17, [0x02, 0x04, 0x00, 0x00, 0x11, 0x00, 0x30]);
		let logo = Logo::parse(&blob).unwrap();
		assert!(logo.low_rows().is_some());
	}

	#[test]
	fn pixel_rows_must_add_up() {
		// A run that crosses the row's end, a row that falls short, a stray byte after the last row, an index outside the palette.
		assert_eq!(with(|blob| blob[13] = 0x20), Err(LogoError::Pixels));
		assert_eq!(with(|blob| blob[14] = 0x01), Err(LogoError::Pixels));
		assert_eq!(
			with(|blob| {
				blob[11] = 5;
				blob.insert(17, 0x30);
			}),
			Err(LogoError::Pixels)
		);
		assert_eq!(with(|blob| blob[14] = 0x12), Err(LogoError::Pixels));
	}

	#[test]
	fn ascii_rows_must_add_up() {
		// A repeat first in the row, a byte outside the character range, art wider than the form, tones too short, an unplain space.
		assert_eq!(with(|blob| blob[21] = 0x80), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[21] = 0x1F), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[21] = 0x7F), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[20] = 0xFF), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[23] = 0x81), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[25] = 0x11), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[25] = 0x41), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[25] = 0x30), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[21] = 0x20), Err(LogoError::Ascii));
		assert_eq!(with(|blob| blob[25] = 0x32), Err(LogoError::Ascii));
	}

	#[test]
	fn a_logo_with_no_ascii_form_draws_filled_cells_and_hashes() {
		let mut blob = EXAMPLE[..17].to_vec();
		blob[6] = 0;
		blob[7] = 0;
		let (lines, capture) = draw(&blob, Depth::Ansi16, Blocks::None);
		assert_eq!(lines, ["   "]);
		assert_eq!(capture.styles, [Style::fill(Paint::Tone(Tone::Gold))]);
		let (lines, _) = draw(&blob, Depth::None, Blocks::Utf8);
		assert_eq!(lines, [" ##"]);
	}

	#[test]
	fn a_tone_with_no_match_at_a_depth_is_transparent_there() {
		// Soot (15) has no 16-colour entry and no mono weight; it is drawn at 256 colours and truecolour.
		let mut blob = EXAMPLE.to_vec();
		blob[9] = 15;
		for (depth, drawn) in [
			(Depth::Ansi16, false),
			(Depth::Console16, false),
			(Depth::Mono, false),
			(Depth::Ansi256, true),
			(Depth::TrueColour, true),
		] {
			let (lines, _) = draw(&blob, depth, Blocks::Utf8);
			assert_eq!(lines[0].trim().is_empty(), !drawn, "{depth:?}");
		}
	}
}
