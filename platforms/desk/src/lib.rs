// The logic of AFETCH.DMD, over a line sink, so the host can run it against a golden and the module runs it over `desk_log`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

use afterfetch_core::{Buf, Depth, FactKey, Facts, Platform, Style, Terminal, TerminalInfo, default_logo, run as run_core};

/// The longest line sent in one piece, which is what `desk_guest::LINE_BYTES` allows the core's log.
pub const LINE_BYTES: usize = 160;

/// The track the module is packed for, as the OS row says it.
const TRACK: &[u8] = if cfg!(feature = "hosted") {
	b"hosted on DOS"
} else {
	b"native"
};

/// Desk as the core asks about it. A module never touches hardware, so there is one fact: the OS row, from what the build knows (AF-D26).
pub struct Desk;

impl Platform for Desk {
	fn facts(&mut self, facts: &mut Facts) {
		facts.arch.push(b"i486");
		facts.row_mut(FactKey::Os).push(b"Desk SDK 1 (").push(TRACK).push(b", i486)");
	}

	fn terminal(&mut self) -> TerminalInfo {
		TerminalInfo {
			is_tty: false,
			columns: 80,
			depth_hint: Depth::None,
			utf8: false,
		}
	}

	fn env(&self, _name: &[u8]) -> Option<&[u8]> {
		None
	}

	fn default_logo(&self) -> &'static str {
		default_logo("desk")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"desk-i486");
	}
}

/// The terminal the core draws through: text collects in a line, and each line end hands the line to the sink, which adds its own newline.
/// A line longer than `LINE_BYTES` is cut.
pub struct Lines<S: FnMut(&[u8])> {
	buffer: [u8; LINE_BYTES],
	length: usize,
	sink: S,
}

impl<S: FnMut(&[u8])> Lines<S> {
	pub fn new(sink: S) -> Self {
		Lines {
			buffer: [0; LINE_BYTES],
			length: 0,
			sink,
		}
	}

	fn send(&mut self) {
		(self.sink)(&self.buffer[..self.length]);
		self.length = 0;
	}
}

impl<S: FnMut(&[u8])> Terminal for Lines<S> {
	fn write(&mut self, bytes: &[u8]) {
		let room = LINE_BYTES - self.length;
		let kept = bytes.len().min(room);
		self.buffer[self.length..self.length + kept].copy_from_slice(&bytes[..kept]);
		self.length += kept;
	}

	fn set_style(&mut self, _style: Style) {}

	fn end_line(&mut self) {
		self.send();
	}

	fn flush(&mut self) -> bool {
		// A line written without an end is still a line.
		if self.length > 0 {
			self.send();
		}
		true
	}
}

/// The whole module: the picture and the OS row, as plain text, one `sink` call per line. Returns the exit status.
pub fn run(sink: impl FnMut(&[u8])) -> u8 {
	run_core(&mut Desk, &mut Lines::new(sink), [&b"--plain"[..]].into_iter())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn lines() -> std::vec::Vec<std::string::String> {
		let mut collected = std::vec::Vec::new();
		let code = run(|line| collected.push(std::string::String::from_utf8(line.to_vec()).unwrap()));
		assert_eq!(code, 0);
		collected
	}

	#[test]
	fn the_module_prints_the_desk_logo_and_the_os_row() {
		let text = lines().join("\n") + "\n";
		assert_eq!(text, include_str!("../tests/golden/plain-80.txt"));
	}

	#[test]
	fn the_os_row_names_the_track_and_no_other_row_appears() {
		let text = lines().join("\n");
		assert!(text.contains("OS:       Desk SDK 1 (native, i486)"), "{text}");
		for label in ["Kernel:", "Host:", "Uptime:", "CPU:", "Memory:", "Display:", "Shell:"] {
			assert!(!text.contains(label), "{label} in {text}");
		}
	}

	#[test]
	fn no_line_passes_the_log_limit_and_nothing_is_coloured() {
		for line in lines() {
			assert!(line.len() <= LINE_BYTES);
			assert!(!line.contains('\x1b'));
		}
	}

	#[test]
	fn a_long_line_is_cut_and_an_unfinished_one_is_still_sent() {
		let mut collected = std::vec::Vec::new();
		let mut terminal = Lines::new(|line: &[u8]| collected.push(line.len()));
		terminal.write(&[b'x'; 300]);
		terminal.end_line();
		terminal.write(b"tail");
		assert!(terminal.flush());
		assert_eq!(collected, [LINE_BYTES, 4]);
	}
}
