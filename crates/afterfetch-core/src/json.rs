// `--json`: the facts as one JSON object on one line. The keys are stable, every value is a string or `null`, and nothing here allocates.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::facts::{FactKey, Facts};
use crate::sink::Terminal;

/// Writes `{"user":…,"hostname":…,"os":…,…}` and ends the line. Facts a platform could not report are `null`.
pub fn write(out: &mut impl Terminal, facts: &Facts) {
	out.write(b"{");
	member(out, "user", facts.user.as_bytes());
	out.write(b",");
	member(out, "hostname", facts.host.as_bytes());
	for key in FactKey::ALL {
		out.write(b",");
		member(out, key.json_key(), facts.row(key));
	}
	out.write(b"}");
	out.end_line();
}

fn member(out: &mut impl Terminal, key: &str, value: &[u8]) {
	string(out, key.as_bytes());
	out.write(b":");
	if value.is_empty() {
		out.write(b"null");
	} else {
		string(out, value);
	}
}

/// A JSON string. Valid UTF-8 passes through; for anything else (CP437 on DOS, say) each byte from 0x80 up is written as the escape of the
/// Latin-1 character with that number, so the output is always valid JSON.
fn string(out: &mut impl Terminal, bytes: &[u8]) {
	let utf8 = core::str::from_utf8(bytes).is_ok();
	out.write(b"\"");
	let mut start = 0;
	for (at, &byte) in bytes.iter().enumerate() {
		let escape = byte < 0x20 || byte == b'"' || byte == b'\\' || byte == 0x7f || (byte >= 0x80 && !utf8);
		if !escape {
			continue;
		}
		out.write(&bytes[start..at]);
		start = at + 1;
		match byte {
			b'"' => out.write(b"\\\""),
			b'\\' => out.write(b"\\\\"),
			b'\n' => out.write(b"\\n"),
			b'\r' => out.write(b"\\r"),
			b'\t' => out.write(b"\\t"),
			_ => {
				const HEX: &[u8; 16] = b"0123456789abcdef";
				out.write(&[
					b'\\',
					b'u',
					b'0',
					b'0',
					HEX[usize::from(byte >> 4)],
					HEX[usize::from(byte & 15)],
				]);
			}
		}
	}
	out.write(&bytes[start..]);
	out.write(b"\"");
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sink::Style;

	#[derive(Default)]
	struct Lines(Vec<u8>);

	impl Terminal for Lines {
		fn write(&mut self, bytes: &[u8]) {
			self.0.extend_from_slice(bytes);
		}
		fn set_style(&mut self, _style: Style) {
			panic!("JSON has no style");
		}
		fn end_line(&mut self) {
			self.0.push(b'\n');
		}
	}

	fn json_of(facts: &Facts) -> String {
		let mut lines = Lines::default();
		write(&mut lines, facts);
		String::from_utf8(lines.0).unwrap()
	}

	#[test]
	fn empty_facts_are_all_null_on_one_line() {
		let text = json_of(&Facts::new());
		assert_eq!(
			text,
			"{\"user\":null,\"hostname\":null,\"os\":null,\"kernel\":null,\"model\":null,\"uptime\":null,\"cpu\":null,\
			 \"memory\":null,\"display\":null,\"terminal\":null,\"shell\":null,\"resources\":null}\n"
		);
	}

	#[test]
	fn facts_appear_under_their_keys_in_a_fixed_order() {
		let mut facts = Facts::new();
		facts.user.push(b"ann");
		facts.host.push(b"box");
		facts.row_mut(FactKey::Os).push(b"Debian GNU/Linux 13 (x86_64)");
		facts.row_mut(FactKey::Shell).push(b"zsh");
		let text = json_of(&facts);
		assert!(
			text.starts_with("{\"user\":\"ann\",\"hostname\":\"box\",\"os\":\"Debian GNU/Linux 13 (x86_64)\",\"kernel\":null,")
		);
		assert!(text.ends_with("\"shell\":\"zsh\",\"resources\":null}\n"));
		assert_eq!(text.matches('\n').count(), 1);
	}

	#[test]
	fn quotes_backslashes_and_controls_are_escaped() {
		let mut facts = Facts::new();
		facts.row_mut(FactKey::Os).push(b"a\"b\\c\nd\te\x01f\x7f");
		let text = json_of(&facts);
		assert!(text.contains(r#""os":"a\"b\\c\nd\te\u0001f\u007f""#), "{text}");
	}

	#[test]
	fn valid_utf8_passes_through() {
		let mut facts = Facts::new();
		facts.host.push("café".as_bytes());
		assert!(json_of(&facts).contains("\"hostname\":\"café\""));
	}

	#[test]
	fn other_bytes_become_latin_1_escapes() {
		let mut facts = Facts::new();
		facts.row_mut(FactKey::Os).push(b"caf\x82");
		let text = json_of(&facts);
		assert!(text.contains(r#""os":"caf\u0082""#), "{text}");
		assert!(text.is_ascii());
	}
}
