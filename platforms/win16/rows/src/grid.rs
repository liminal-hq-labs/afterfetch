// A grid of character cells the core draws into, and the window paints from. The core writes text and styles as it would to a console; each
// cell keeps its character and a text-mode attribute (the low nibble the foreground slot, the high nibble the background slot, in the Dusk
// table), so the window draws a run of one attribute with one `TextOut`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{Style, Terminal};

use crate::attribute;

/// The attribute of a cell nothing has been drawn on: the Dusk text colour on the Dusk navy, slot 7 on slot 0.
pub const BLANK: u8 = 0x07;

/// `COLUMNS` by `ROWS` cells. Text past the last column is dropped, as is a line past the last row.
pub struct Grid<const COLUMNS: usize, const ROWS: usize> {
	characters: [[u8; COLUMNS]; ROWS],
	attributes: [[u8; COLUMNS]; ROWS],
	row: usize,
	column: usize,
	current: u8,
}

impl<const COLUMNS: usize, const ROWS: usize> Default for Grid<COLUMNS, ROWS> {
	fn default() -> Self {
		Self::new()
	}
}

impl<const COLUMNS: usize, const ROWS: usize> Grid<COLUMNS, ROWS> {
	pub const fn new() -> Self {
		Grid {
			characters: [[b' '; COLUMNS]; ROWS],
			attributes: [[BLANK; COLUMNS]; ROWS],
			row: 0,
			column: 0,
			current: BLANK,
		}
	}

	/// The rows with something written on them, counting a started row.
	pub fn rows_used(&self) -> usize {
		(self.row + usize::from(self.column > 0)).min(ROWS)
	}

	pub fn characters(&self, row: usize) -> &[u8; COLUMNS] {
		&self.characters[row]
	}

	pub fn attributes(&self, row: usize) -> &[u8; COLUMNS] {
		&self.attributes[row]
	}

	/// The text of a row without its trailing blanks, for a plain file.
	pub fn text(&self, row: usize) -> &[u8] {
		let line = &self.characters[row];
		let end = line.iter().rposition(|&byte| byte != b' ').map_or(0, |at| at + 1);
		&line[..end]
	}
}

impl<const COLUMNS: usize, const ROWS: usize> Terminal for Grid<COLUMNS, ROWS> {
	fn write(&mut self, bytes: &[u8]) {
		for &byte in bytes {
			if self.row < ROWS && self.column < COLUMNS {
				self.characters[self.row][self.column] = if byte < 0x20 { b' ' } else { byte };
				self.attributes[self.row][self.column] = self.current;
			}
			self.column += 1;
		}
	}

	fn set_style(&mut self, style: Style) {
		self.current = attribute::from_style(u16::from(BLANK), style) as u8;
	}

	fn end_line(&mut self) {
		self.current = BLANK;
		self.row += 1;
		self.column = 0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use afterfetch_core::{Paint, Tone};

	#[test]
	fn text_lands_in_cells_with_the_style_in_force() {
		let mut grid = Grid::<8, 3>::new();
		grid.set_style(Style::text(Tone::Gold));
		grid.write(b"ab");
		grid.set_style(Style::PLAIN);
		grid.write(b"c");
		grid.end_line();
		assert_eq!(&grid.characters(0)[..4], b"abc ");
		assert_eq!(&grid.attributes(0)[..4], [0x0E, 0x0E, 0x07, 0x07]);
		assert_eq!(grid.rows_used(), 1);
	}

	#[test]
	fn a_fill_sets_the_background_and_a_line_end_restores_the_blank_attribute() {
		let mut grid = Grid::<8, 3>::new();
		grid.set_style(Style::fill(Paint::Tone(Tone::Coral)));
		grid.write(b"  ");
		grid.end_line();
		grid.write(b"x");
		assert_eq!(grid.attributes(0)[0], 0xC7);
		assert_eq!(grid.attributes(1)[0], BLANK);
		assert_eq!(grid.rows_used(), 2);
	}

	#[test]
	fn text_past_the_edges_is_dropped() {
		let mut grid = Grid::<4, 2>::new();
		grid.write(b"abcdefgh");
		grid.end_line();
		grid.end_line();
		grid.end_line();
		grid.write(b"lost");
		assert_eq!(grid.characters(0), b"abcd");
		assert_eq!(grid.characters(1), b"    ");
		assert_eq!(grid.rows_used(), 2);
	}

	#[test]
	fn plain_text_drops_trailing_blanks_and_control_bytes_become_blanks() {
		let mut grid = Grid::<8, 2>::new();
		grid.write(b"hi\x01x  ");
		assert_eq!(grid.text(0), b"hi x");
		assert_eq!(grid.text(1), b"");
	}
}
