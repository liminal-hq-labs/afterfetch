// Output: a buffered handle sink, and the terminal the core draws through. On a console a style becomes a console attribute and every
// line ends by putting the console's own attribute back. On a pipe or a file there are no attributes and no escape sequences, whatever
// depth was asked for: the escape-sequence writer costs 1.3 KB, which the 16 KiB budget does not have (the VT path, AF-WP13, decides).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ptr::{null, null_mut};

use afterfetch_core::{ByteSink, Style, Terminal};
use afterfetch_win32::attribute;

use crate::sys::{GetStdHandle, Handle, STD_ERROR_HANDLE, SetConsoleTextAttribute, WriteConsoleA, WriteFile};

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
}

const LINE_END: &[u8] = b"\r\n";

impl Out {
	pub const fn new(handle: Handle, console_attribute: Option<u16>) -> Self {
		Out {
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

	/// Puts the console's own attribute back and writes what is left, then says whether every write worked.
	pub fn finish(&mut self) -> bool {
		if let Some(original) = self.original {
			self.set_attribute(original);
		}
		self.sink.flush();
		!self.sink.failed()
	}
}

impl Terminal for Out {
	fn write(&mut self, bytes: &[u8]) {
		self.sink.write_bytes(bytes);
	}

	fn set_style(&mut self, style: Style) {
		if let Some(original) = self.original {
			self.set_attribute(attribute::from_style(original, style));
		}
	}

	fn end_line(&mut self) {
		if let Some(original) = self.original {
			self.set_attribute(original);
		}
		self.sink.write_bytes(LINE_END);
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
