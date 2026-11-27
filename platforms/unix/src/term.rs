// Output: a buffered file-descriptor sink, the terminal the core draws through, and the checks that depend on the environment (is the
// locale UTF-8).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{AnsiTerminal, ByteSink, Depth, Style, Terminal};

use crate::os::system;

/// Standard output, buffered so a full picture is a few `write` calls and not one per escape sequence.
pub struct FdSink {
	fd: i32,
	buffer: [u8; 2048],
	length: usize,
	failed: bool,
}

impl FdSink {
	pub const fn new(fd: i32) -> Self {
		FdSink {
			fd,
			buffer: [0; 2048],
			length: 0,
			failed: false,
		}
	}

	/// Writes out what is buffered. A failure is remembered and reported by `finish`.
	fn flush(&mut self) {
		if self.length > 0 && !system::write_all(self.fd, &self.buffer[..self.length]) {
			self.failed = true;
		}
		self.length = 0;
	}
}

impl ByteSink for FdSink {
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

/// The terminal the core draws through: ANSI escapes to standard output, `\n` line ends, and errors to standard error.
pub struct Out<'a> {
	ansi: AnsiTerminal<'a, FdSink>,
}

impl<'a> Out<'a> {
	/// The depth starts at none; `run` sets it once it is decided.
	pub fn new(sink: &'a mut FdSink) -> Self {
		Out {
			ansi: AnsiTerminal::new(sink, Depth::None, b"\n"),
		}
	}
}

impl Terminal for Out<'_> {
	fn write(&mut self, bytes: &[u8]) {
		self.ansi.write(bytes);
	}

	fn set_style(&mut self, style: Style) {
		self.ansi.set_style(style);
	}

	fn end_line(&mut self) {
		self.ansi.end_line();
	}

	fn set_depth(&mut self, depth: Depth) {
		self.ansi.set_depth(depth);
	}

	fn flush(&mut self) -> bool {
		self.ansi.flush()
	}

	fn write_error_line(&mut self, bytes: &[u8]) {
		// Failing to report an error has nowhere further to go.
		let _ = system::write_all(2, bytes) && system::write_all(2, b"\n");
	}
}

/// True when the locale names UTF-8. `LC_ALL`, then `LC_CTYPE`, then `LANG`: the first that is set and not empty decides.
pub fn locale_is_utf8<'a>(lookup: impl Fn(&[u8]) -> Option<&'a [u8]>) -> bool {
	let setting = [&b"LC_ALL"[..], b"LC_CTYPE", b"LANG"]
		.into_iter()
		.find_map(|name| lookup(name).filter(|value| !value.is_empty()));
	setting.is_some_and(|value| {
		value.windows(4).any(|window| window.eq_ignore_ascii_case(b"utf8"))
			|| value.windows(5).any(|window| window.eq_ignore_ascii_case(b"utf-8"))
	})
}
