// Output. On the console the picture is drawn a cell at a time through the BIOS (INT 10h), which respects the active page and works on MDA,
// CGA, EGA and VGA, and in a Windows 3.1 DOS box; a style becomes a text-mode attribute and every line ends by putting the console's own
// attribute back. Anywhere else (a file, a pipe, NUL) the text goes to handle 1 with INT 21h and nothing is coloured.
//
// A test hook: when `AF_DUMP` names a file, the text screen is read back from video memory after a console run and written there, so a
// headless test can see what was drawn.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{ByteSink, Style, Terminal};
use afterfetch_dos::attribute;

use crate::rt::{bios, dos, dpmi};

const LINE_END: &[u8] = b"\r\n";
/// The BIOS data area: the video mode, the columns, the rows less one, the active page and its start offset.
const BDA_MODE: u32 = 0x449;
const BDA_COLUMNS: u32 = 0x44A;
const BDA_PAGE_START: u32 = 0x44E;
const BDA_PAGE: u32 = 0x462;
const BDA_ROWS: u32 = 0x484;

fn bda_byte(at: u32) -> u8 {
	// SAFETY: the BIOS data area is in conventional memory, mapped at linear 0.
	unsafe { dpmi::linear_to_ptr::<u8>(at).read_volatile() }
}

fn bda_word(at: u32) -> u16 {
	// SAFETY: as `bda_byte`.
	unsafe { dpmi::linear_to_ptr::<u16>(at).read_volatile() }
}

/// The text mode the screen is in: the columns, the rows, and whether it is a monochrome adapter's mode 7.
#[derive(Clone, Copy)]
pub struct Mode {
	pub columns: u16,
	pub rows: u16,
	pub mono: bool,
}

impl Mode {
	pub fn read() -> Mode {
		let rows = bda_byte(BDA_ROWS);
		Mode {
			columns: bda_word(BDA_COLUMNS).max(1),
			// The rows byte exists on EGA and later; 0 means a CGA or MDA with 25.
			rows: if rows == 0 { 25 } else { u16::from(rows) + 1 },
			mono: bda_byte(BDA_MODE) == 7,
		}
	}
}

/// The screen being drawn on: the cursor is tracked here, and set once per cell, since the BIOS writes a character at the cursor and does
/// not move it.
struct Screen {
	mode: Mode,
	page: u8,
	row: u16,
	column: u16,
	/// The attribute at the start, which every line goes back to.
	original: u8,
	current: u8,
}

impl Screen {
	fn new(mode: Mode) -> Screen {
		let page = bda_byte(BDA_PAGE);
		let cursor = bios::int10(0x0300, u32::from(page) << 8, 0, 0);
		let attribute = bios::int10(0x0800, u32::from(page) << 8, 0, 0);
		let original = (attribute.ax() >> 8) as u8;
		if !mode.mono {
			// Bit 7 of an attribute means a bright background instead of blinking, which the Dusk fills need. EGA and VGA honour it; the
			// mode is left that way, as the screen would blink otherwise, and a mode change gives blinking back.
			bios::int10(0x1003, 0, 0, 0);
		}
		Screen {
			mode,
			page,
			row: (cursor.dx() >> 8).min(mode.rows - 1),
			column: (cursor.dx() & 0xFF).min(mode.columns - 1),
			original,
			current: original,
		}
	}

	fn place_cursor(&self) {
		bios::int10(
			0x0200,
			u32::from(self.page) << 8,
			0,
			(u32::from(self.row) << 8) | u32::from(self.column),
		);
	}

	fn put(&mut self, byte: u8) {
		bios::int10(
			0x0900 | u32::from(byte),
			(u32::from(self.page) << 8) | u32::from(self.current),
			1,
			0,
		);
		self.column += 1;
		if self.column >= self.mode.columns {
			self.new_line();
		} else {
			self.place_cursor();
		}
	}

	/// Moves to the start of the next line, scrolling the screen up when this is the last one. The new line is cleared in the console's own
	/// attribute, so a coloured fill never spills onto it.
	fn new_line(&mut self) {
		self.column = 0;
		if self.row + 1 >= self.mode.rows {
			let bottom_right = ((self.mode.rows - 1) << 8) | (self.mode.columns - 1);
			bios::int10(0x0601, u32::from(self.original) << 8, 0, u32::from(bottom_right));
		} else {
			self.row += 1;
		}
		self.place_cursor();
	}

	fn write(&mut self, bytes: &[u8]) {
		for &byte in bytes {
			match byte {
				b'\r' => {
					self.column = 0;
					self.place_cursor();
				}
				b'\n' => self.new_line(),
				_ => self.put(byte),
			}
		}
	}
}

/// Standard output as a file, buffered so a full picture is a few writes.
struct HandleSink {
	buffer: [u8; 1024],
	length: usize,
	failed: bool,
}

impl HandleSink {
	fn flush(&mut self) {
		if self.length > 0 && !self.failed && !dos::write(1, &self.buffer[..self.length]) {
			self.failed = true;
		}
		self.length = 0;
	}
}

impl ByteSink for HandleSink {
	fn write_bytes(&mut self, bytes: &[u8]) {
		for chunk in bytes.chunks(self.buffer.len()) {
			if self.length + chunk.len() > self.buffer.len() {
				self.flush();
			}
			self.buffer[self.length..self.length + chunk.len()].copy_from_slice(chunk);
			self.length += chunk.len();
		}
	}

	fn finish(&mut self) -> bool {
		self.flush();
		!self.failed
	}
}

// The file buffer is the larger variant, and there is only ever one `Out`, so the difference costs nothing.
#[allow(clippy::large_enum_variant)]
enum Target {
	Console(Screen),
	Handle(HandleSink),
}

/// The terminal the core draws through.
pub struct Out {
	target: Target,
	/// The file the finished screen is written to, for tests (`AF_DUMP`).
	dump: Option<&'static [u8]>,
}

impl Out {
	/// A console when `console` is true, else standard output as a file.
	pub fn new(console: bool, mode: Mode, dump: Option<&'static [u8]>) -> Out {
		Out {
			target: if console {
				Target::Console(Screen::new(mode))
			} else {
				Target::Handle(HandleSink {
					buffer: [0; 1024],
					length: 0,
					failed: false,
				})
			},
			dump,
		}
	}

	/// Writes the text screen to the `AF_DUMP` file: the size, each row's characters, a blank line, then each row's attributes as hex.
	fn dump_screen(screen: &Screen, path: &[u8]) -> bool {
		let Some(handle) = dos::create(path) else { return false };
		let segment: u32 = if screen.mode.mono { 0xB0000 } else { 0xB8000 };
		let start = segment + u32::from(bda_word(BDA_PAGE_START));
		let cell = |row: u16, column: u16| {
			let at = start + (u32::from(row) * u32::from(screen.mode.columns) + u32::from(column)) * 2;
			// SAFETY: the text page is video memory in the first megabyte, mapped at linear 0.
			unsafe { dpmi::linear_to_ptr::<u16>(at).read_volatile() }
		};
		let mut written = true;
		let mut line = [0u8; 256];
		let columns = usize::from(screen.mode.columns).min(120);
		for pass in 0..2 {
			for row in 0..screen.mode.rows {
				let mut length = 0;
				for column in 0..columns as u16 {
					let value = cell(row, column);
					if pass == 0 {
						let character = value as u8;
						line[length] = if character < 0x20 { b' ' } else { character };
						length += 1;
					} else {
						line[length] = b"0123456789ABCDEF"[usize::from((value >> 12) & 0xF)];
						line[length + 1] = b"0123456789ABCDEF"[usize::from((value >> 8) & 0xF)];
						length += 2;
					}
				}
				line[length..length + 2].copy_from_slice(LINE_END);
				written &= dos::write(handle, &line[..length + 2]);
			}
			if pass == 0 {
				written &= dos::write(handle, LINE_END);
			}
		}
		dos::close(handle) && written
	}
}

impl Terminal for Out {
	fn write(&mut self, bytes: &[u8]) {
		match &mut self.target {
			Target::Console(screen) => screen.write(bytes),
			Target::Handle(sink) => sink.write_bytes(bytes),
		}
	}

	fn set_style(&mut self, style: Style) {
		if let Target::Console(screen) = &mut self.target {
			screen.current = attribute::from_style(u16::from(screen.original), style) as u8;
		}
	}

	fn end_line(&mut self) {
		match &mut self.target {
			Target::Console(screen) => {
				screen.current = screen.original;
				screen.new_line();
			}
			Target::Handle(sink) => sink.write_bytes(LINE_END),
		}
	}

	fn flush(&mut self) -> bool {
		match &mut self.target {
			Target::Console(screen) => {
				screen.current = screen.original;
				match self.dump {
					Some(path) => Out::dump_screen(screen, path),
					None => true,
				}
			}
			Target::Handle(sink) => sink.finish(),
		}
	}

	fn write_error_line(&mut self, bytes: &[u8]) {
		// Standard error is handle 2. Failing to report an error has nowhere further to go.
		let _ = dos::write(2, bytes) && dos::write(2, LINE_END);
	}
}
