// A screen of character cells that records everything the core draws: each cell's character and the style it was written in.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{Depth, Style, Terminal};

/// One character cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
	pub ch: char,
	pub style: Style,
}

/// What the core drew, row by row, and the depth it settled on.
#[derive(Clone, Debug)]
pub struct Grid {
	pub rows: Vec<Vec<Cell>>,
	pub depth: Depth,
	pub errors: Vec<String>,
	style: Style,
}

impl Default for Grid {
	fn default() -> Self {
		Grid {
			rows: vec![Vec::new()],
			depth: Depth::None,
			errors: Vec::new(),
			style: Style::PLAIN,
		}
	}
}

impl Grid {
	/// The widest row, in cells.
	pub fn width(&self) -> usize {
		self.rows.iter().map(Vec::len).max().unwrap_or(0)
	}

	/// The number of rows that have anything in them (the core ends every line, so the last, open row is empty).
	pub fn height(&self) -> usize {
		match self.rows.last() {
			Some(last) if last.is_empty() => self.rows.len() - 1,
			_ => self.rows.len(),
		}
	}

	/// The text of one row with trailing spaces removed.
	pub fn line(&self, row: usize) -> String {
		let text: String = self.rows[row].iter().map(|cell| cell.ch).collect();
		text.trim_end().to_string()
	}

	/// All the text, one line per row, with trailing spaces removed.
	pub fn text(&self) -> String {
		(0..self.height()).map(|row| self.line(row) + "\n").collect()
	}
}

impl Terminal for Grid {
	fn write(&mut self, bytes: &[u8]) {
		let style = self.style;
		let row = self.rows.last_mut().expect("a grid always has an open row");
		row.extend(String::from_utf8_lossy(bytes).chars().map(|ch| Cell { ch, style }));
	}

	fn set_style(&mut self, style: Style) {
		self.style = style;
	}

	fn end_line(&mut self) {
		self.style = Style::PLAIN;
		self.rows.push(Vec::new());
	}

	fn set_depth(&mut self, depth: Depth) {
		self.depth = depth;
	}

	fn write_error_line(&mut self, bytes: &[u8]) {
		self.errors.push(String::from_utf8_lossy(bytes).into_owned());
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lines_collect_and_trailing_spaces_go() {
		let mut grid = Grid::default();
		grid.write(b"ab  ");
		grid.end_line();
		grid.write("\u{2580}x".as_bytes());
		grid.end_line();
		assert_eq!(grid.height(), 2);
		assert_eq!(grid.width(), 4);
		assert_eq!(grid.text(), "ab\n\u{2580}x\n");
	}
}
