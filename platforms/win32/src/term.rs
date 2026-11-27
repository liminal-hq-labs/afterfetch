// Output: a buffered handle sink, and the terminal the core draws through. On a console a style becomes a console attribute and every
// line ends by putting the console's own attribute back. On a pipe or a file there are no attributes and no escape sequences, whatever
// depth was asked for. The 64-bit builds first ask a console for virtual terminal processing (Windows 10 and later, AF-D52): when it
// accepts, the core's `AnsiTerminal` writes escape sequences, in UTF-8 when the console takes that code page too. The Pentium build has
// no room for the escape-sequence writer (1.3 KB) and keeps to attributes.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ptr::{null, null_mut};

use afterfetch_core::{AnsiTerminal, ByteSink, Depth, Style, Terminal};
use afterfetch_win32::attribute;

use crate::sys::{
	CODE_PAGE_UTF8, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetConsoleOutputCP, GetStdHandle, Handle,
	STD_ERROR_HANDLE, SetConsoleMode, SetConsoleOutputCP, SetConsoleTextAttribute, WriteConsoleA, WriteFile,
};

/// True for the builds that may write escape sequences to a console: everything but the Pentium build.
const VT_BUILD: bool = cfg!(not(target_arch = "x86"));

/// What a console was before afterfetch asked it for escape sequences, so it can be put back at exit.
#[derive(Clone, Copy)]
pub struct ConsoleMode {
	mode: u32,
	code_page: u32,
	/// True when the console also took UTF-8, so the half blocks can be drawn.
	pub utf8: bool,
}

impl ConsoleMode {
	/// Asks the console for virtual terminal processing, and for UTF-8 output. None when this is not a 64-bit build or the console
	/// refuses (Windows before 10 1511, ReactOS, a pipe), in which case nothing has changed.
	pub fn enable(handle: Handle) -> Option<ConsoleMode> {
		let mut mode = 0u32;
		// SAFETY: the handle is a console's output; the call fills `mode`.
		unsafe {
			if !VT_BUILD
				|| GetConsoleMode(handle, &mut mode) == 0
				|| SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0
			{
				return None;
			}
			let code_page = GetConsoleOutputCP();
			Some(ConsoleMode {
				mode,
				code_page,
				utf8: SetConsoleOutputCP(CODE_PAGE_UTF8) != 0,
			})
		}
	}

	fn restore(&self, handle: Handle) {
		// SAFETY: the handle is the console's output the mode was read from.
		unsafe {
			SetConsoleMode(handle, self.mode);
			SetConsoleOutputCP(self.code_page);
		}
	}
}

/// Standard output, buffered so a full picture is a few writes and not one per colour change. `console` says whether it is written with
/// `WriteConsoleA` (a console) or `WriteFile` (a pipe, a file, a device).
pub struct HandleSink {
	handle: Handle,
	console: bool,
	buffer: [u8; 2048],
	length: usize,
	failed: bool,
}

impl HandleSink {
	pub const fn new(handle: Handle, console: bool) -> Self {
		HandleSink {
			handle,
			console,
			buffer: [0; 2048],
			length: 0,
			failed: false,
		}
	}

	/// Writes out what is buffered. A failure is remembered, and what follows is dropped.
	pub fn flush(&mut self) {
		let mut sent = 0;
		while !self.failed && sent < self.length {
			let mut written = 0u32;
			let rest = &self.buffer[sent..self.length];
			// SAFETY: `rest` is valid for its length and `written` is a live local.
			let done = unsafe {
				if self.console {
					WriteConsoleA(self.handle, rest.as_ptr(), rest.len() as u32, &mut written, null())
				} else {
					WriteFile(self.handle, rest.as_ptr(), rest.len() as u32, &mut written, null_mut())
				}
			};
			if done == 0 || written == 0 {
				self.failed = true;
			}
			sent += written as usize;
		}
		self.length = 0;
	}

	/// True when a write failed, such as to a closed pipe.
	pub fn failed(&self) -> bool {
		self.failed
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
}

/// The terminal the core draws through: `\r\n` line ends, and errors to standard error.
pub struct Out {
	sink: HandleSink,
	/// The console's attribute at the start, or None when standard output is not a console.
	original: Option<u16>,
	/// The attribute the console has now.
	current: u16,
	/// The console's mode before escape sequences were asked for, when they were accepted.
	vt: Option<ConsoleMode>,
	/// The run draws with escape sequences (a console that accepted them, at a depth that has them).
	escapes: bool,
	/// The depth the run settled on.
	depth: Depth,
	/// Something is drawn on this line that needs a reset (escape mode).
	styled: bool,
}

const LINE_END: &[u8] = b"\r\n";

impl Out {
	pub const fn new(handle: Handle, console_attribute: Option<u16>, vt: Option<ConsoleMode>) -> Self {
		Out {
			vt,
			escapes: false,
			depth: Depth::None,
			styled: false,
			sink: HandleSink::new(handle, console_attribute.is_some()),
			original: console_attribute,
			current: match console_attribute {
				Some(attribute) => attribute,
				None => 0,
			},
		}
	}

	fn set_attribute(&mut self, wanted: u16) {
		if self.original.is_some() && wanted != self.current {
			self.sink.flush();
			// SAFETY: the handle is the console's output.
			unsafe { SetConsoleTextAttribute(self.sink.handle, wanted) };
			self.current = wanted;
		}
	}
}

impl Terminal for Out {
	fn write(&mut self, bytes: &[u8]) {
		self.sink.write_bytes(bytes);
	}

	fn set_depth(&mut self, depth: Depth) {
		self.depth = depth;
		self.escapes =
			VT_BUILD && self.vt.is_some() && matches!(depth, Depth::Mono | Depth::Ansi16 | Depth::Ansi256 | Depth::TrueColour);
	}

	fn set_style(&mut self, style: Style) {
		if VT_BUILD && self.escapes {
			AnsiTerminal::new(&mut self.sink, self.depth, LINE_END).set_style(style);
			self.styled = style != Style::PLAIN;
		} else if let Some(original) = self.original {
			self.set_attribute(attribute::from_style(original, style));
		}
	}

	fn end_line(&mut self) {
		if VT_BUILD && self.escapes {
			if self.styled {
				self.sink.write_bytes(b"\x1b[0m");
				self.styled = false;
			}
			self.sink.write_bytes(LINE_END);
			return;
		}
		if let Some(original) = self.original {
			self.set_attribute(original);
		}
		self.sink.write_bytes(LINE_END);
	}

	/// Puts the console's own attribute back and writes what is left, then says whether every write worked.
	fn flush(&mut self) -> bool {
		if let Some(original) = self.original {
			self.set_attribute(original);
		}
		self.sink.flush();
		if let Some(vt) = self.vt.filter(|_| VT_BUILD) {
			vt.restore(self.sink.handle);
		}
		!self.sink.failed()
	}

	fn write_error_line(&mut self, bytes: &[u8]) {
		// SAFETY: asking for a standard handle has no preconditions; the buffers are valid for their lengths. Failing to report an error has
		// nowhere further to go.
		unsafe {
			let handle = GetStdHandle(STD_ERROR_HANDLE);
			let mut written = 0u32;
			WriteFile(handle, bytes.as_ptr(), bytes.len() as u32, &mut written, null_mut());
			WriteFile(handle, LINE_END.as_ptr(), LINE_END.len() as u32, &mut written, null_mut());
		}
	}
}
