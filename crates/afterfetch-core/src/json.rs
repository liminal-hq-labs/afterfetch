// `--json`: the facts as one JSON object on one line (schema 1, AF-D34, AF-D43). The keys and their order are stable, every value is a
// string, a number or `null`, and nothing here allocates.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::facts::{FactKey, Facts};
use crate::format::Buf;
use crate::sink::Terminal;

/// The schema number `--json` starts with. It changes only when a key is renamed, removed or changes meaning.
pub const SCHEMA: u32 = 1;

/// Writes `{"schema":1,"user":…,"hostname":…,"os":…,…,"resources":…,"target":…,"arch":…,"uptime_seconds":…,…}` and ends the line. `target` is
/// the build's target name. Facts a platform could not report are `null`.
pub fn write(out: &mut impl Terminal, facts: &Facts, target: &[u8]) {
	out.write(b"{\"schema\":");
	let mut number = Buf::<4>::new();
	number.push_u32(SCHEMA);
	out.write(number.as_bytes());
	out.write(b",");
	member(out, "user", facts.user.as_bytes());
	out.write(b",");
	member(out, "hostname", facts.host.as_bytes());
	for key in FactKey::ALL {
		out.write(b",");
		member(out, key.json_key(), facts.row(key));
	}
	out.write(b",");
	member(out, "target", target);
	out.write(b",");
	member(out, "arch", facts.arch.as_bytes());
	out.write(b",");
	count(out, "uptime_seconds", facts.uptime_seconds);
	out.write(b",");
	count(out, "memory_available_bytes", facts.memory_available_bytes);
	out.write(b",");
	count(out, "memory_total_bytes", facts.memory_total_bytes);
	out.write(b"}");
	out.end_line();
}

/// A member whose value is a whole number, or `null`.
fn count(out: &mut impl Terminal, key: &str, value: Option<u64>) {
	string(out, key.as_bytes());
	out.write(b":");
	match value {
		Some(value) => {
			let mut digits = Buf::<20>::new();
			digits.push_u64(value);
			out.write(digits.as_bytes());
		}
		None => out.write(b"null"),
	}
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
		write(&mut lines, facts, b"linux-x86_64");
		String::from_utf8(lines.0).unwrap()
	}

	#[test]
	fn empty_facts_are_all_null_on_one_line() {
		let text = json_of(&Facts::new());
		assert_eq!(
			text,
			"{\"schema\":1,\"user\":null,\"hostname\":null,\"os\":null,\"kernel\":null,\"model\":null,\"uptime\":null,\"cpu\":null,\
			 \"memory\":null,\"display\":null,\"desktop\":null,\"terminal\":null,\"shell\":null,\"resources\":null,\
			 \"target\":\"linux-x86_64\",\"arch\":null,\"uptime_seconds\":null,\"memory_available_bytes\":null,\
			 \"memory_total_bytes\":null}\n"
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
		assert!(text.starts_with(
			"{\"schema\":1,\"user\":\"ann\",\"hostname\":\"box\",\"os\":\"Debian GNU/Linux 13 (x86_64)\",\"kernel\":null,"
		));
		assert!(text.contains("\"shell\":\"zsh\",\"resources\":null,\"target\":"));
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

	#[test]
	fn schema_comes_first_and_the_new_keys_last() {
		let text = json_of(&Facts::new());
		assert!(text.starts_with("{\"schema\":1,\"user\":"));
		let positions: Vec<usize> = [
			"\"resources\"",
			"\"target\"",
			"\"arch\"",
			"\"uptime_seconds\"",
			"\"memory_available_bytes\"",
			"\"memory_total_bytes\"",
		]
		.iter()
		.map(|key| text.find(key).unwrap())
		.collect();
		assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
		assert!(text.ends_with("\"memory_total_bytes\":null}\n"));
	}

	#[test]
	fn the_raw_numbers_are_json_numbers_not_strings() {
		let mut facts = Facts::new();
		facts.arch.push(b"i686");
		facts.set_uptime(93_784);
		facts.set_memory(2_000_000_000, 17_179_869_184);
		let text = json_of(&facts);
		assert!(
			text.ends_with(
				"\"target\":\"linux-x86_64\",\"arch\":\"i686\",\"uptime_seconds\":93784,\
			 \"memory_available_bytes\":2000000000,\"memory_total_bytes\":17179869184}\n"
			),
			"{text}"
		);
	}

	#[test]
	fn a_number_past_32_bits_and_the_largest_number_print_in_full() {
		let mut facts = Facts::new();
		facts.memory_total_bytes = Some(u64::MAX);
		facts.memory_available_bytes = Some(1 << 32);
		let text = json_of(&facts);
		assert!(
			text.contains("\"memory_available_bytes\":4294967296,\"memory_total_bytes\":18446744073709551615}"),
			"{text}"
		);
	}

	#[test]
	fn the_target_is_escaped_like_any_string_and_an_empty_one_is_null() {
		let mut lines = Lines::default();
		write(&mut lines, &Facts::new(), b"");
		assert!(String::from_utf8(lines.0).unwrap().contains("\"target\":null,"));
	}

	#[test]
	fn the_whole_line_parses_as_one_object_with_nineteen_keys() {
		let mut facts = Facts::new();
		facts.user.push(b"ann");
		facts.set_uptime(5);
		let text = json_of(&facts);
		assert_eq!(text.matches("\":").count(), 19);
		assert!(text.starts_with('{') && text.ends_with("}\n"));
	}
}
