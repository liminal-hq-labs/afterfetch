// Splits a file read in chunks into lines. A line longer than the buffer is cut, which is fine for the files afterfetch reads: only the
// start of a line matters, and the longest line that does (`model name`) is far shorter.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Collects bytes into lines of at most `N` bytes. Lines that end in a carriage return and a newline lose both.
pub struct LineFeed<const N: usize> {
	line: [u8; N],
	len: usize,
}

impl<const N: usize> Default for LineFeed<N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<const N: usize> LineFeed<N> {
	pub const fn new() -> Self {
		LineFeed { line: [0; N], len: 0 }
	}

	/// Passes each complete line in `chunk` (and any line it completes from earlier chunks) to `handle`.
	pub fn feed(&mut self, chunk: &[u8], handle: &mut impl FnMut(&[u8])) {
		let mut rest = chunk;
		while let Some(at) = rest.iter().position(|&byte| byte == b'\n') {
			self.append(&rest[..at]);
			self.emit(handle);
			rest = &rest[at + 1..];
		}
		self.append(rest);
	}

	/// Passes the last line when the file does not end in a newline.
	pub fn finish(&mut self, handle: &mut impl FnMut(&[u8])) {
		if self.len > 0 {
			self.emit(handle);
		}
	}

	fn append(&mut self, text: &[u8]) {
		let take = text.len().min(N - self.len);
		self.line[self.len..self.len + take].copy_from_slice(&text[..take]);
		self.len += take;
	}

	fn emit(&mut self, handle: &mut impl FnMut(&[u8])) {
		let mut line = &self.line[..self.len];
		if let [rest @ .., b'\r'] = line {
			line = rest;
		}
		handle(line);
		self.len = 0;
	}
}

/// Feeds the whole of `text` through a `LineFeed` in chunks of `chunk_size` bytes (at least 1). The shell reads a file this way, and the
/// tests use it to prove a split makes no difference.
pub fn for_each_line<const N: usize>(text: &[u8], chunk_size: usize, handle: &mut impl FnMut(&[u8])) {
	let mut feed = LineFeed::<N>::new();
	for chunk in text.chunks(chunk_size.max(1)) {
		feed.feed(chunk, handle);
	}
	feed.finish(handle);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn collect(text: &[u8], chunk_size: usize) -> std::vec::Vec<std::vec::Vec<u8>> {
		let mut lines = std::vec::Vec::new();
		for_each_line::<8>(text, chunk_size, &mut |line| lines.push(line.to_vec()));
		lines
	}

	#[test]
	fn lines_come_out_the_same_whatever_the_chunking() {
		let text = b"one\ntwo\r\n\nlast";
		let expected = collect(text, 1000);
		assert_eq!(expected, [b"one".to_vec(), b"two".to_vec(), b"".to_vec(), b"last".to_vec()]);
		for size in 1..=text.len() {
			assert_eq!(collect(text, size), expected, "chunk size {size}");
		}
	}

	#[test]
	fn a_long_line_is_cut_and_the_next_one_is_clean() {
		assert_eq!(collect(b"0123456789abc\nxy\n", 3), [b"01234567".to_vec(), b"xy".to_vec()]);
	}

	#[test]
	fn a_trailing_newline_makes_no_extra_line() {
		assert_eq!(collect(b"a\n", 4), [b"a".to_vec()]);
		assert!(collect(b"", 4).is_empty());
	}
}
