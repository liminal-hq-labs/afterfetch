// Where output goes: the `Terminal` trait the layout writes through, the `Style` it asks for, and `AnsiTerminal`, which turns styles into
// escape sequences for the five ANSI-style depths over any byte sink.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::depth::Depth;
use crate::palette::{Tone, Weight, ansi16_of_slot, slot_rgb, xterm256_nearest};

/// A colour request: nothing, a role in the picture, or one of the console's 16 slots (for the colour swatches).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
	/// The terminal's own colour.
	Default,
	Tone(Tone),
	/// Console slot 0 to 15 in the Dusk table.
	Slot(u8),
}

impl Paint {
	/// The console slot the paint stands for, or None for the terminal's own colour (and for a tone with no 16-colour match).
	pub fn slot(self) -> Option<u8> {
		match self {
			Paint::Default => None,
			Paint::Tone(tone) => tone.index(),
			Paint::Slot(slot) => Some(slot & 0x0F),
		}
	}

	/// The 24-bit colour: a tone's design colour, or the swatch slot's Dusk colour.
	fn rgb(self) -> Option<[u8; 3]> {
		match self {
			Paint::Default => None,
			Paint::Tone(tone) => tone.rgb(),
			Paint::Slot(slot) => Some(slot_rgb(slot)),
		}
	}

	/// The xterm 256-colour number: a tone's table entry (which is not always the nearest, see `docs/art.md`), or the nearest to the
	/// swatch slot's Dusk colour.
	fn xterm256(self) -> Option<u8> {
		match self {
			Paint::Tone(tone) => tone.xterm256(),
			_ => self.rgb().map(xterm256_nearest),
		}
	}
}

/// The look of the text that follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
	pub fg: Paint,
	pub bg: Paint,
	pub weight: Weight,
}

impl Style {
	/// The terminal's own look.
	pub const PLAIN: Style = Style {
		fg: Paint::Default,
		bg: Paint::Default,
		weight: Weight::Normal,
	};

	/// Text in `tone` over the terminal's own background.
	pub const fn text(tone: Tone) -> Style {
		Style {
			fg: Paint::Tone(tone),
			bg: Paint::Default,
			weight: Weight::Normal,
		}
	}

	/// A solid cell: a space on a background of `paint`.
	pub const fn fill(paint: Paint) -> Style {
		Style {
			fg: Paint::Default,
			bg: paint,
			weight: Weight::Normal,
		}
	}

	/// Text in `tone` as `depth` draws it: its colour with colour, its weight in mono, nothing otherwise.
	pub fn at(depth: Depth, tone: Tone) -> Style {
		match depth {
			Depth::None => Style::PLAIN,
			Depth::Mono => Style::mono(tone),
			_ => Style::text(tone),
		}
	}

	/// Text in `tone` drawn by weight alone, for the mono depth.
	pub fn mono(tone: Tone) -> Style {
		Style {
			fg: Paint::Default,
			bg: Paint::Default,
			weight: tone.mono_weight(),
		}
	}
}

/// What the layout writes through. A console shell implements `set_style` with an attribute call and `end_line` by restoring the
/// console's own attribute and moving to the next line; a pipe shell ignores styles. Bytes are written as given.
pub trait Terminal {
	fn write(&mut self, bytes: &[u8]);
	/// Sets the style of everything written from now on.
	fn set_style(&mut self, style: Style);
	/// Restores the terminal's own style, then ends the line.
	fn end_line(&mut self);
	/// Tells the terminal the depth the run settled on, before anything is drawn. A terminal that encodes styles itself needs it, because
	/// the depth is decided inside `run` from the options and the environment; a console shell ignores it.
	fn set_depth(&mut self, _depth: Depth) {}
	/// Writes one line to the error stream, line end included. The default writes it to the output; a shell with a separate error
	/// stream overrides this.
	fn write_error_line(&mut self, bytes: &[u8]) {
		self.write(bytes);
		self.end_line();
	}
}

/// A place bytes can be written: a file descriptor, a handle, a buffer.
pub trait ByteSink {
	fn write_bytes(&mut self, bytes: &[u8]);
}

/// A `Terminal` that writes ANSI escape sequences for `depth` (`Ansi16`, `Ansi256`, `TrueColour` and `Mono`; `None` and `Console16` write
/// no escapes) to a `ByteSink`. Every line ends with `ESC [ 0 m` when a style was set on it, so a scroll never paints the next line.
pub struct AnsiTerminal<'a, S: ByteSink> {
	sink: &'a mut S,
	depth: Depth,
	line_end: &'static [u8],
	styled: bool,
}

impl<'a, S: ByteSink> AnsiTerminal<'a, S> {
	/// `line_end` is `\n` on Unix and `\r\n` on DOS and Windows.
	pub fn new(sink: &'a mut S, depth: Depth, line_end: &'static [u8]) -> Self {
		AnsiTerminal {
			sink,
			depth,
			line_end,
			styled: false,
		}
	}

	fn escapes(&self) -> bool {
		matches!(self.depth, Depth::Mono | Depth::Ansi16 | Depth::Ansi256 | Depth::TrueColour)
	}

	fn colour(&mut self, paint: Paint, background: bool) {
		let mut digits = crate::format::Buf::<24>::new();
		match self.depth {
			Depth::Ansi16 => {
				let Some(slot) = paint.slot() else { return };
				let number = ansi16_of_slot(slot);
				let base = match (background, number >= 8) {
					(false, false) => 30,
					(false, true) => 90,
					(true, false) => 40,
					(true, true) => 100,
				};
				digits.push_u32(u32::from(base + (number & 7)));
			}
			Depth::Ansi256 => {
				let Some(number) = paint.xterm256() else { return };
				digits
					.push(if background { b"48;5;" } else { b"38;5;" })
					.push_u32(u32::from(number));
			}
			Depth::TrueColour => {
				let Some([red, green, blue]) = paint.rgb() else { return };
				digits
					.push(if background { b"48;2;" } else { b"38;2;" })
					.push_u32(u32::from(red))
					.push(b";")
					.push_u32(u32::from(green))
					.push(b";")
					.push_u32(u32::from(blue));
			}
			_ => return,
		}
		self.sequence(digits.as_bytes());
	}

	fn sequence(&mut self, parameters: &[u8]) {
		self.sink.write_bytes(b"\x1b[");
		self.sink.write_bytes(parameters);
		self.sink.write_bytes(b"m");
	}
}

impl<S: ByteSink> Terminal for AnsiTerminal<'_, S> {
	fn write(&mut self, bytes: &[u8]) {
		self.sink.write_bytes(bytes);
	}

	fn set_depth(&mut self, depth: Depth) {
		self.depth = depth;
	}

	fn set_style(&mut self, style: Style) {
		if !self.escapes() {
			return;
		}
		// One reset, then the whole style: no escape depends on the one before it.
		self.sequence(b"0");
		match style.weight {
			Weight::Normal => {}
			Weight::Bold => self.sequence(b"1"),
			Weight::Dim => self.sequence(b"2"),
		}
		if self.depth != Depth::Mono {
			self.colour(style.fg, false);
			self.colour(style.bg, true);
		}
		self.styled = style != Style::PLAIN;
	}

	fn end_line(&mut self) {
		if self.styled {
			self.sequence(b"0");
			self.styled = false;
		}
		self.sink.write_bytes(self.line_end);
	}
}

#[cfg(test)]
pub(crate) mod testing {
	use super::*;

	/// A byte sink that collects into a `Vec`.
	#[derive(Default)]
	pub struct Collect(pub std::vec::Vec<u8>);

	impl ByteSink for Collect {
		fn write_bytes(&mut self, bytes: &[u8]) {
			self.0.extend_from_slice(bytes);
		}
	}

	/// The collected output with ESC shown as a visible mark.
	pub fn visible(bytes: &[u8]) -> std::string::String {
		std::string::String::from_utf8_lossy(bytes).replace('\x1b', "<ESC>")
	}
}

#[cfg(test)]
mod tests {
	use super::testing::{Collect, visible};
	use super::*;

	fn paint_one(depth: Depth, style: Style) -> std::string::String {
		let mut collect = Collect::default();
		{
			let mut terminal = AnsiTerminal::new(&mut collect, depth, b"\n");
			terminal.set_style(style);
			terminal.write(b"x");
			terminal.end_line();
		}
		visible(&collect.0)
	}

	#[test]
	fn ansi16_uses_the_swapped_numbers_and_bright_codes() {
		// Gold is slot 14 (bright yellow, ANSI 11), rose is slot 4 (red, ANSI 1).
		assert_eq!(
			paint_one(Depth::Ansi16, Style::text(Tone::Gold)),
			"<ESC>[0m<ESC>[93mx<ESC>[0m\n"
		);
		assert_eq!(
			paint_one(Depth::Ansi16, Style::text(Tone::Rose)),
			"<ESC>[0m<ESC>[31mx<ESC>[0m\n"
		);
		assert_eq!(
			paint_one(Depth::Ansi16, Style::fill(Paint::Tone(Tone::Coral))),
			"<ESC>[0m<ESC>[101mx<ESC>[0m\n"
		);
	}

	#[test]
	fn ansi256_and_truecolour_name_the_colour() {
		assert_eq!(
			paint_one(Depth::Ansi256, Style::text(Tone::Gold)),
			"<ESC>[0m<ESC>[38;5;222mx<ESC>[0m\n"
		);
		assert_eq!(
			paint_one(Depth::TrueColour, Style::text(Tone::Gold)),
			"<ESC>[0m<ESC>[38;2;255;208;142mx<ESC>[0m\n"
		);
		assert_eq!(
			paint_one(Depth::TrueColour, Style::fill(Paint::Slot(8))),
			"<ESC>[0m<ESC>[48;2;124;135;150mx<ESC>[0m\n"
		);
	}

	#[test]
	fn mono_uses_weight_and_no_colour() {
		assert_eq!(paint_one(Depth::Mono, Style::mono(Tone::Gold)), "<ESC>[0m<ESC>[1mx<ESC>[0m\n");
		assert_eq!(
			paint_one(Depth::Mono, Style::mono(Tone::Slate)),
			"<ESC>[0m<ESC>[2mx<ESC>[0m\n"
		);
		assert_eq!(paint_one(Depth::Mono, Style::text(Tone::Gold)), "<ESC>[0mx<ESC>[0m\n");
	}

	#[test]
	fn the_depth_can_be_set_after_construction() {
		let mut collect = Collect::default();
		{
			let mut terminal = AnsiTerminal::new(&mut collect, Depth::None, b"\n");
			terminal.set_depth(Depth::Ansi256);
			terminal.set_style(Style::text(Tone::Gold));
			terminal.write(b"x");
			terminal.end_line();
		}
		assert_eq!(visible(&collect.0), "<ESC>[0m<ESC>[38;5;222mx<ESC>[0m\n");
	}

	#[test]
	fn none_and_console16_write_no_escapes() {
		assert_eq!(paint_one(Depth::None, Style::text(Tone::Gold)), "x\n");
		assert_eq!(paint_one(Depth::Console16, Style::text(Tone::Gold)), "x\n");
	}

	#[test]
	fn an_unstyled_line_ends_without_a_reset_and_with_the_chosen_line_end() {
		let mut collect = Collect::default();
		{
			let mut terminal = AnsiTerminal::new(&mut collect, Depth::TrueColour, b"\r\n");
			terminal.set_style(Style::PLAIN);
			terminal.write(b"a");
			terminal.end_line();
		}
		assert_eq!(visible(&collect.0), "<ESC>[0ma\r\n");
	}
}
