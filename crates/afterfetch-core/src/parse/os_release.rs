// `/etc/os-release`: shell-style `KEY=value` lines. Only the keys that name the system are kept.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{split_pair, trim};
use crate::format::Buf;

/// The naming keys of an os-release file.
#[derive(Clone, Copy, Default)]
pub struct OsRelease {
	pretty_name: Buf<64>,
	name: Buf<32>,
	version: Buf<24>,
	id: Buf<24>,
}

impl OsRelease {
	pub const fn new() -> Self {
		OsRelease {
			pretty_name: Buf::new(),
			name: Buf::new(),
			version: Buf::new(),
			id: Buf::new(),
		}
	}

	/// Reads one line. Comments, blank lines and keys that do not name the system are ignored, and the last assignment of a key wins.
	pub fn feed_line(&mut self, line: &[u8]) {
		let Some((key, value)) = split_pair(line, b'=') else { return };
		match key {
			b"PRETTY_NAME" => store(&mut self.pretty_name, value),
			b"NAME" => store(&mut self.name, value),
			b"VERSION_ID" => store(&mut self.version, value),
			b"ID" => store(&mut self.id, value),
			_ => {}
		}
	}

	/// The name to show: `PRETTY_NAME`, else `NAME` and `VERSION_ID`, else `ID`. Empty when the file named nothing.
	pub fn describe<const N: usize>(&self, out: &mut Buf<N>) {
		if !self.pretty_name.is_empty() {
			out.push(self.pretty_name.as_bytes());
		} else if !self.name.is_empty() {
			out.push(self.name.as_bytes());
			if !self.version.is_empty() {
				out.push(b" ").push(self.version.as_bytes());
			}
		} else {
			out.push(self.id.as_bytes());
		}
	}
}

fn store<const N: usize>(target: &mut Buf<N>, value: &[u8]) {
	target.clear();
	unquote(value, target);
}

/// Writes the value of a shell word to `out`: the inside of a double or single quote, with the escapes a double quote allows, or an
/// unquoted word up to the first blank or `#`.
fn unquote<const N: usize>(value: &[u8], out: &mut Buf<N>) {
	let value = trim(value);
	match value.first() {
		Some(&quote @ (b'"' | b'\'')) => {
			let mut escaped = false;
			for &byte in &value[1..] {
				if escaped {
					if !matches!(byte, b'"' | b'\\' | b'$' | b'`') {
						out.push(b"\\");
					}
					out.push(&[byte]);
					escaped = false;
				} else if byte == b'\\' && quote == b'"' {
					escaped = true;
				} else if byte == quote {
					break;
				} else {
					out.push(&[byte]);
				}
			}
		}
		_ => {
			let end = value
				.iter()
				.position(|&byte| matches!(byte, b' ' | b'\t' | b'#'))
				.unwrap_or(value.len());
			out.push(&value[..end]);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parse::lines::for_each_line;

	fn described(text: &[u8]) -> std::string::String {
		let mut release = OsRelease::new();
		for_each_line::<160>(text, 7, &mut |line| release.feed_line(line));
		let mut out = Buf::<64>::new();
		release.describe(&mut out);
		std::string::String::from_utf8_lossy(out.as_bytes()).into_owned()
	}

	#[test]
	fn pretty_name_is_preferred() {
		assert_eq!(
			described(b"NAME=\"Debian GNU/Linux\"\nPRETTY_NAME=\"Debian GNU/Linux 13 (trixie)\"\n"),
			"Debian GNU/Linux 13 (trixie)"
		);
	}

	#[test]
	fn name_and_version_id_stand_in_for_a_missing_pretty_name() {
		assert_eq!(described(b"NAME='Odd OS'\nVERSION_ID=3.2\n"), "Odd OS 3.2");
		assert_eq!(described(b"NAME=Plain\n"), "Plain");
		assert_eq!(described(b"ID=bare\n"), "bare");
		assert_eq!(described(b""), "");
	}

	#[test]
	fn quotes_escapes_comments_and_crlf_are_handled() {
		assert_eq!(
			described(b"# a comment\r\nPRETTY_NAME=\"Say \\\"hi\\\" \\$HOME\"\r\n"),
			"Say \"hi\" $HOME"
		);
		assert_eq!(described(b"PRETTY_NAME=unquoted word # tail\n"), "unquoted");
		assert_eq!(described(b"PRETTY_NAME='single $quoted'\n"), "single $quoted");
		assert_eq!(described(b"  PRETTY_NAME =  \"spaced\"  \n"), "spaced");
	}

	#[test]
	fn the_last_assignment_wins_and_an_unterminated_quote_keeps_what_it_has() {
		assert_eq!(described(b"PRETTY_NAME=first\nPRETTY_NAME=second\n"), "second");
		assert_eq!(described(b"PRETTY_NAME=\"open\n"), "open");
	}
}
