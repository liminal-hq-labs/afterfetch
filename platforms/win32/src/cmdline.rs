// The command line as arguments. Windows hands a program one string; this splits it the way the Microsoft C runtime does (quotes group,
// `\"` is a quote, backslashes count only before a quote), in a fixed buffer, with no allocation. The program's own name is dropped.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// The longest command line kept; anything past it is cut off. Windows 95 allows 259 characters, NT 8191.
pub const LINE_MAX: usize = 512;
/// The most arguments kept; the program takes four at most.
pub const ARGS_MAX: usize = 16;

/// The arguments of one command line, split in place.
pub struct CommandLine {
	buffer: [u8; LINE_MAX],
	spans: [(usize, usize); ARGS_MAX],
	count: usize,
}

impl CommandLine {
	/// Splits `line` (without its NUL).
	pub fn parse(line: &[u8]) -> CommandLine {
		let mut parsed = CommandLine {
			buffer: [0; LINE_MAX],
			spans: [(0, 0); ARGS_MAX],
			count: 0,
		};
		let length = line.len().min(LINE_MAX);
		parsed.buffer[..length].copy_from_slice(&line[..length]);
		let buffer = &mut parsed.buffer;

		// The program's name: up to the closing quote when it starts with one, otherwise up to the first blank.
		let mut read = 0;
		if length > 0 && buffer[0] == b'"' {
			read = 1;
			while read < length && buffer[read] != b'"' {
				read += 1;
			}
			read += 1;
		} else {
			while read < length && !is_blank(buffer[read]) {
				read += 1;
			}
		}

		let mut write = 0;
		while parsed.count < ARGS_MAX {
			while read < length && is_blank(buffer[read]) {
				read += 1;
			}
			if read >= length {
				break;
			}
			let start = write;
			let mut quoted = false;
			// Decoding goes back into the same buffer, which is safe because it never writes more than it has read.
			loop {
				let mut backslashes = 0;
				while read < length && buffer[read] == b'\\' {
					backslashes += 1;
					read += 1;
				}
				if read < length && buffer[read] == b'"' {
					for _ in 0..backslashes / 2 {
						buffer[write] = b'\\';
						write += 1;
					}
					if backslashes % 2 == 0 {
						quoted = !quoted;
					} else {
						buffer[write] = b'"';
						write += 1;
					}
					read += 1;
					continue;
				}
				for _ in 0..backslashes {
					buffer[write] = b'\\';
					write += 1;
				}
				if read >= length || (!quoted && is_blank(buffer[read])) {
					break;
				}
				buffer[write] = buffer[read];
				write += 1;
				read += 1;
			}
			parsed.spans[parsed.count] = (start, write - start);
			parsed.count += 1;
		}
		parsed
	}

	/// The arguments, in order.
	pub fn args(&self) -> impl Iterator<Item = &[u8]> {
		self.spans[..self.count]
			.iter()
			.map(|&(start, length)| &self.buffer[start..start + length])
	}
}

const fn is_blank(byte: u8) -> bool {
	byte == b' ' || byte == b'\t'
}

#[cfg(test)]
mod tests {
	use super::*;

	fn split(line: &str) -> Vec<String> {
		CommandLine::parse(line.as_bytes())
			.args()
			.map(|arg| String::from_utf8(arg.to_vec()).unwrap())
			.collect()
	}

	#[test]
	fn the_program_name_is_dropped() {
		assert!(split("afterfetch.exe").is_empty());
		assert_eq!(split("afterfetch.exe --plain"), ["--plain"]);
		assert_eq!(split(r#""C:\Program Files\afterfetch.exe" --plain"#), ["--plain"]);
		assert_eq!(split(""), Vec::<String>::new());
	}

	#[test]
	fn blanks_separate_and_runs_of_them_collapse() {
		assert_eq!(split("a.exe   --logo \t win32  "), ["--logo", "win32"]);
	}

	#[test]
	fn quotes_group_and_leave_no_trace() {
		assert_eq!(split(r#"a.exe /LOGO:"win 32" x"#), ["/LOGO:win 32", "x"]);
		assert_eq!(split(r#"a.exe "" x"#), ["", "x"]);
	}

	#[test]
	fn backslashes_count_only_before_a_quote() {
		assert_eq!(split(r#"a.exe C:\dir\file"#), [r"C:\dir\file"]);
		assert_eq!(split(r#"a.exe \\"x y\\" z"#), [r"\x y\", "z"]);
		assert_eq!(split(r#"a.exe \"quoted\""#), [r#""quoted""#]);
	}

	#[test]
	fn a_long_line_and_too_many_arguments_are_cut_not_fatal() {
		let long = format!("a.exe {}", "x".repeat(2000));
		assert_eq!(split(&long)[0].len(), LINE_MAX - "a.exe ".len());
		let many = format!("a.exe {}", "y ".repeat(40));
		assert_eq!(split(&many).len(), ARGS_MAX);
	}

	#[test]
	fn an_unterminated_quote_runs_to_the_end() {
		assert_eq!(split(r#"a.exe "open ended"#), ["open ended"]);
	}
}
