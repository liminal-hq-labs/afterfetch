// Parsers for the text files a Linux system keeps in `/etc`, `/proc` and `/sys`. Each is a pure function over bytes, fed line by line, so a
// shell can read a file in small chunks into a stack buffer and never hold the whole of it.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod cpuinfo;
pub mod lines;
pub mod meminfo;
pub mod model;
pub mod os_release;
pub mod passwd;
pub mod uptime;

/// The bytes of `text` without leading and trailing spaces, tabs and carriage returns.
pub fn trim(text: &[u8]) -> &[u8] {
	let is_blank = |byte: &u8| matches!(byte, b' ' | b'\t' | b'\r' | b'\n');
	let start = text.iter().position(|byte| !is_blank(byte)).unwrap_or(text.len());
	let end = text.iter().rposition(|byte| !is_blank(byte)).map_or(start, |at| at + 1);
	&text[start..end]
}

/// Splits `line` at the first `separator` into a trimmed key and a trimmed value.
pub fn split_pair(line: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
	let at = line.iter().position(|&byte| byte == separator)?;
	Some((trim(&line[..at]), trim(&line[at + 1..])))
}

/// The leading decimal digits of `text` as a number (saturating), and how many digits there were.
pub fn leading_number(text: &[u8]) -> Option<u64> {
	let digits = text.iter().take_while(|byte| byte.is_ascii_digit()).count();
	if digits == 0 {
		return None;
	}
	Some(text[..digits].iter().fold(0u64, |total, &digit| {
		total.saturating_mul(10).saturating_add(u64::from(digit - b'0'))
	}))
}

/// The last path component of `path` (`/bin/zsh` is `zsh`). A trailing slash is ignored.
pub fn basename(path: &[u8]) -> &[u8] {
	let path = match path {
		[rest @ .., b'/'] => rest,
		_ => path,
	};
	let start = path.iter().rposition(|&byte| byte == b'/').map_or(0, |at| at + 1);
	&path[start..]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn trim_strips_blanks_from_both_ends() {
		assert_eq!(trim(b"  a b\t\r\n"), b"a b");
		assert_eq!(trim(b"   "), b"");
		assert_eq!(trim(b""), b"");
	}

	#[test]
	fn split_pair_cuts_at_the_first_separator() {
		assert_eq!(
			split_pair(b"model name\t: Intel: x", b':'),
			Some((&b"model name"[..], &b"Intel: x"[..]))
		);
		assert_eq!(split_pair(b"no separator", b':'), None);
		assert_eq!(split_pair(b"key=", b'='), Some((&b"key"[..], &b""[..])));
	}

	#[test]
	fn leading_number_reads_digits_and_saturates() {
		assert_eq!(leading_number(b"16384 kB"), Some(16384));
		assert_eq!(leading_number(b"x1"), None);
		assert_eq!(leading_number(b"99999999999999999999999"), Some(u64::MAX));
	}

	#[test]
	fn basename_takes_the_last_component() {
		assert_eq!(basename(b"/usr/bin/zsh"), b"zsh");
		assert_eq!(basename(b"fish"), b"fish");
		assert_eq!(basename(b"/bin/"), b"bin");
		assert_eq!(basename(b""), b"");
	}
}
