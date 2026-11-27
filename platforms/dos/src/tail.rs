// The command tail as arguments. DOS hands a program the text after its name, in the 126 bytes at PSP:81h. This splits it on blanks and
// tabs in a fixed buffer, with no allocation. Quotes are not special: DOS programs of this kind take slash options and file names.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// The longest tail DOS allows.
pub const TAIL_MAX: usize = 127;
/// The most arguments kept; the program takes four at most.
pub const ARGS_MAX: usize = 16;

/// The arguments of one command tail.
pub struct Tail {
	buffer: [u8; TAIL_MAX],
	spans: [(u8, u8); ARGS_MAX],
	count: usize,
}

impl Tail {
	/// Splits `tail`, which is cut off at 127 bytes.
	pub fn parse(tail: &[u8]) -> Tail {
		let mut parsed = Tail {
			buffer: [0; TAIL_MAX],
			spans: [(0, 0); ARGS_MAX],
			count: 0,
		};
		let length = tail.len().min(TAIL_MAX);
		parsed.buffer[..length].copy_from_slice(&tail[..length]);
		let is_blank = |byte: u8| byte == b' ' || byte == b'\t';
		let mut at = 0;
		while parsed.count < ARGS_MAX {
			while at < length && is_blank(parsed.buffer[at]) {
				at += 1;
			}
			if at >= length {
				break;
			}
			let start = at;
			while at < length && !is_blank(parsed.buffer[at]) {
				at += 1;
			}
			parsed.spans[parsed.count] = (start as u8, at as u8);
			parsed.count += 1;
		}
		parsed
	}

	/// The arguments, in order.
	pub fn args(&self) -> impl Iterator<Item = &[u8]> {
		self.spans[..self.count]
			.iter()
			.map(|&(start, end)| &self.buffer[usize::from(start)..usize::from(end)])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn split(tail: &[u8]) -> std::vec::Vec<&[u8]> {
		// The tail outlives the call in these tests, so leaking the parse is fine.
		let parsed = std::boxed::Box::leak(std::boxed::Box::new(Tail::parse(tail)));
		parsed.args().collect()
	}

	#[test]
	fn arguments_split_on_blanks_and_tabs() {
		assert_eq!(split(b" /PLAIN  /NOLOGO\t/JSON"), [&b"/PLAIN"[..], b"/NOLOGO", b"/JSON"]);
	}

	#[test]
	fn an_empty_or_blank_tail_has_no_arguments() {
		assert!(split(b"").is_empty());
		assert!(split(b"   \t ").is_empty());
	}

	#[test]
	fn a_long_tail_is_cut_off_and_the_count_is_capped() {
		let long = [b'x'; 300];
		assert_eq!(split(&long).len(), 1);
		assert_eq!(split(&long)[0].len(), TAIL_MAX);
		let many = b"a ".repeat(40);
		assert_eq!(split(&many).len(), ARGS_MAX);
	}
}
