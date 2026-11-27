// The layout: the logo on the left, the facts on the right. It writes through a `Terminal`, which a shell backs with the console and the
// tests back with a string, so the layout and the colour choices are tested without a console.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::depth::Depth;
use crate::facts::{FactKey, Facts};
use crate::logo::{Blocks, Logo};
use crate::palette::{Tone, arch_accent};
use crate::sink::{Paint, Style, Terminal};

/// The room between the logo and the facts, and the least room the facts need beside it.
const FACT_ROOM: usize = 34;

/// What to draw and how.
#[derive(Clone, Copy)]
pub struct Layout<'a> {
	/// The logo, or None for the facts alone.
	pub logo: Option<&'a Logo<'a>>,
	pub depth: Depth,
	/// The half blocks the terminal can draw, if any.
	pub blocks: Blocks,
	/// The terminal's width in columns.
	pub columns: usize,
}

/// One line of the information column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Line {
	Title,
	Rule,
	Fact(FactKey),
	Gap,
	Swatches(u8),
}

const MAX_LINES: usize = 2 + crate::facts::ROW_COUNT + 3;

/// The information lines for these facts: the title and its rule when there is a user or host, the facts that have a value, and, with
/// colour, a gap and two rows of swatches. A fact with no value has no line.
fn lines(facts: &Facts, colour: bool) -> ([Line; MAX_LINES], usize) {
	let mut list = [Line::Gap; MAX_LINES];
	let mut count = 0;
	let mut add = |line| {
		list[count] = line;
		count += 1;
	};
	if facts.has_title() {
		add(Line::Title);
		add(Line::Rule);
	}
	for key in FactKey::ALL {
		if facts.has(key) {
			add(Line::Fact(key));
		}
	}
	if colour {
		if facts.has_title() || FactKey::ALL.into_iter().any(|key| facts.has(key)) {
			add(Line::Gap);
		}
		add(Line::Swatches(0));
		add(Line::Swatches(8));
	}
	(list, count)
}

/// Writes the whole picture to `out`.
pub fn render(out: &mut impl Terminal, facts: &Facts, layout: Layout<'_>) {
	let colour = layout.depth.has_colour();
	let (list, count) = lines(facts, colour);
	let list = &list[..count];

	let label_width = list
		.iter()
		.filter_map(|line| match line {
			Line::Fact(key) => Some(key.label().len() + 1),
			_ => None,
		})
		.fold(10, usize::max);
	let side_by_side = layout.logo.filter(|logo| layout.columns >= logo.info_column() + FACT_ROOM);
	let left = side_by_side.map_or(0, Logo::info_column);
	let room = layout.columns.saturating_sub(left + label_width + 1);

	// With the logo, the facts start on its second row; without it, at the top.
	let first_line = usize::from(side_by_side.is_some());
	let mut drawing = side_by_side.map(|logo| logo.drawing(layout.depth, layout.blocks));
	let logo_rows = drawing.as_ref().map_or(0, |drawing| drawing.rows());
	let total = if drawing.is_some() {
		logo_rows.max(first_line + list.len())
	} else {
		list.len()
	};
	for row in 0..total {
		let mut written = 0;
		if let Some(drawing) = drawing.as_mut().filter(|_| row < logo_rows) {
			written = drawing.draw_row(out);
		}
		if let Some(line) = row.checked_sub(first_line).and_then(|at| list.get(at)) {
			// A gap row is empty, so its padding would be trailing spaces.
			if *line != Line::Gap {
				pad(out, left.saturating_sub(written));
				info_line(out, facts, *line, layout.depth, label_width, room);
			}
		}
		out.end_line();
	}
}

fn spaces(out: &mut impl Terminal, count: usize) {
	const SPACES: &[u8] = b"                                                                ";
	let mut left = count;
	while left > 0 {
		let take = left.min(SPACES.len());
		out.write(&SPACES[..take]);
		left -= take;
	}
}

fn pad(out: &mut impl Terminal, count: usize) {
	if count > 0 {
		out.set_style(Style::PLAIN);
		spaces(out, count);
	}
}

/// The title's rule: `count` dashes in the architecture's accent tone.
fn dashes(out: &mut impl Terminal, depth: Depth, count: usize, accent: Tone) {
	const DASHES: &[u8] = b"--------------------------------------------------------------------------------";
	out.set_style(Style::at(depth, accent));
	out.write(&DASHES[..count.min(DASHES.len())]);
}

/// `bytes` cut to `room` bytes without leaving half of a UTF-8 character at the end.
fn cut(bytes: &[u8], room: usize) -> &[u8] {
	if bytes.len() <= room {
		return bytes;
	}
	let mut end = room;
	while end > 0 && bytes[end] & 0xC0 == 0x80 {
		end -= 1;
	}
	&bytes[..end]
}

fn info_line(out: &mut impl Terminal, facts: &Facts, line: Line, depth: Depth, label_width: usize, room: usize) {
	match line {
		Line::Title => {
			let (user, host) = (facts.user.as_bytes(), facts.host.as_bytes());
			if !user.is_empty() {
				out.set_style(Style::at(depth, Tone::Gold));
				out.write(user);
			}
			if !user.is_empty() && !host.is_empty() {
				out.set_style(Style::at(depth, Tone::Slate));
				out.write(b"@");
			}
			if !host.is_empty() {
				out.set_style(Style::at(depth, if user.is_empty() { Tone::Gold } else { Tone::Lilac }));
				out.write(host);
			}
		}
		Line::Rule => {
			let (user, host) = (facts.user.len(), facts.host.len());
			let at = usize::from(user > 0 && host > 0);
			dashes(out, depth, user + at + host, arch_accent(facts.arch.as_bytes()));
		}
		Line::Fact(key) => {
			out.set_style(Style::at(depth, key.tone()));
			out.write(key.label().as_bytes());
			pad(out, label_width.saturating_sub(key.label().len()));
			out.set_style(Style::PLAIN);
			out.write(cut(facts.row(key), room));
		}
		Line::Gap => {}
		Line::Swatches(first) => {
			for slot in first..first + 8 {
				out.set_style(Style::fill(Paint::Slot(slot)));
				out.write(b"  ");
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::palette::{Weight, fill_attribute, text_attribute};

	/// The `win32` logo's ASCII form, as drawn by hand: an art line and a tone line for each row.
	const WIN32_ASCII: &str = include_str!("../../../art/ascii/win32.txt");
	const INFO_COLUMN: usize = 42;

	fn win32() -> Logo<'static> {
		Logo::parse(afterfetch_art::find("win32").unwrap().blob).unwrap()
	}

	/// The art lines of the `win32` ASCII form.
	fn art_rows() -> Vec<&'static str> {
		WIN32_ASCII.lines().step_by(2).collect()
	}

	/// Collects the output as plain text, and the tone of every character as a parallel string of letters.
	struct Capture {
		plain: String,
		tones: String,
		style: Style,
	}

	fn letter(style: Style) -> char {
		let paint = if style.bg == Paint::Default { style.fg } else { style.bg };
		match paint {
			Paint::Default => ' ',
			Paint::Slot(slot) => char::from_digit(u32::from(slot) % 10, 10).unwrap(),
			Paint::Tone(tone) => afterfetch_art::tone(tone as u8).map_or(' ', |tone| char::from(tone.letter)),
		}
	}

	impl Terminal for Capture {
		fn write(&mut self, bytes: &[u8]) {
			for &byte in bytes {
				self.plain.push(byte as char);
				self.tones.push(letter(self.style));
			}
		}
		fn set_style(&mut self, style: Style) {
			self.style = style;
		}
		fn end_line(&mut self) {
			self.plain.push('\n');
			self.tones.push('\n');
			self.style = Style::PLAIN;
		}
	}

	fn sample() -> Facts {
		let mut facts = Facts::new();
		facts.user.push(b"administrator");
		facts.host.push(b"reactos");
		facts.row_mut(FactKey::Os).push(b"ReactOS 0.4.16-x86");
		facts.row_mut(FactKey::Kernel).push(b"NT 5.2.3790 SP3");
		facts.row_mut(FactKey::Uptime).push(b"0d 0h 1m");
		facts.row_mut(FactKey::Memory).push(b"387 MB free of 511 MB");
		facts.row_mut(FactKey::Display).push(b"1024 x 768, 32-bit");
		facts.row_mut(FactKey::Shell).push(b"cmd.exe");
		facts
	}

	fn run_with(depth: Depth, columns: usize, logo: bool, facts: &Facts) -> (Vec<String>, Vec<String>) {
		let mut capture = Capture {
			plain: String::new(),
			tones: String::new(),
			style: Style::PLAIN,
		};
		let win32 = win32();
		let layout = Layout {
			logo: logo.then_some(&win32),
			depth,
			blocks: Blocks::None,
			columns,
		};
		render(&mut capture, facts, layout);
		(
			capture.plain.lines().map(String::from).collect(),
			capture.tones.lines().map(String::from).collect(),
		)
	}

	fn run(colour: bool, columns: usize, facts: &Facts) -> (Vec<String>, Vec<String>) {
		run_with(if colour { Depth::Console16 } else { Depth::None }, columns, true, facts)
	}

	#[test]
	fn plain_output_is_the_ascii_picture_beside_the_facts() {
		let (lines, _) = run(false, 80, &sample());
		assert_eq!(lines.len(), 12);
		let rows = art_rows();
		let beside = |row: usize, fact: &str| format!("{:<width$}{fact}", rows[row], width = INFO_COLUMN);
		assert_eq!(lines[0], rows[0]);
		assert_eq!(lines[1], beside(1, "administrator@reactos"));
		assert_eq!(lines[2], beside(2, "---------------------"));
		assert_eq!(lines[3], beside(3, "OS:       ReactOS 0.4.16-x86"));
		assert_eq!(lines[4], beside(4, "Kernel:   NT 5.2.3790 SP3"));
		assert_eq!(lines[5], beside(5, "Uptime:   0d 0h 1m"));
		assert_eq!(lines[6], beside(6, "Memory:   387 MB free of 511 MB"));
		assert_eq!(lines[7], beside(7, "Display:  1024 x 768, 32-bit"));
		assert_eq!(lines[8], beside(8, "Shell:    cmd.exe"));
		// No swatches on a pipe: the last rows hold the water alone.
		assert_eq!(lines[9], rows[9]);
		assert_eq!(lines[10], rows[10]);
		assert_eq!(lines[11], rows[11]);
	}

	#[test]
	fn facts_start_in_one_column() {
		let (lines, _) = run(false, 80, &sample());
		for (row, wanted) in [
			(1, "administrator@reactos"),
			(2, "---------------------"),
			(3, "OS:"),
			(6, "Memory:"),
			(8, "Shell:"),
		] {
			assert_eq!(lines[row].find(wanted), Some(INFO_COLUMN), "row {row}");
		}
		for line in &lines {
			assert!(line.len() < 80, "a line of {} characters would wrap: {line}", line.len());
		}
	}

	#[test]
	fn the_sunset_runs_from_gold_through_dusk_to_slate() {
		let (_, tones) = run(true, 80, &sample());
		// The logo's half of each row (before the information column), top to bottom.
		let first_tone = |row: usize, wanted: char| tones[row].chars().take(INFO_COLUMN).any(|tone| tone == wanted);
		assert!(first_tone(3, 'g') && first_tone(4, 'g'), "the sun's top is gold");
		assert!(first_tone(5, 'o'), "then orange");
		assert!(first_tone(6, 'c'), "coral at the horizon");
		assert!(first_tone(7, 'r'), "a rose horizon line");
		assert!(
			first_tone(8, 'l') && first_tone(9, 'v') && first_tone(10, 'p') && first_tone(11, 's'),
			"water darkens downwards"
		);
	}

	#[test]
	fn colour_output_draws_the_sun_solid_and_adds_the_swatches() {
		let (lines, tones) = run(true, 80, &sample());
		// The sun is spaces on a coloured background, not `#`.
		assert!(!lines[3].contains('#'));
		assert!(tones[3].contains("gggggggg"));
		assert_eq!(lines[3].trim_start().get(..2), Some(".:"));
		let rows = art_rows();
		// A gap row, then two rows of eight two-cell swatches.
		assert_eq!(lines[9], rows[9], "the gap row has no swatches");
		assert!(lines[10].ends_with(&" ".repeat(16)));
		assert!(tones[10].ends_with("0011223344556677"));
		assert!(tones[11].ends_with("8899001122334455"));
	}

	#[test]
	fn a_missing_fact_is_left_out() {
		let mut facts = sample();
		facts.row_mut(FactKey::Uptime).clear();
		let (lines, _) = run(false, 80, &facts);
		assert!(lines.iter().all(|line| !line.contains("Uptime")));
		assert!(lines[5].contains("Memory:"), "the rows below close up: {}", lines[5]);
		assert!(lines.iter().all(|line| !line.contains("-@-")));
	}

	#[test]
	fn no_user_and_no_host_leave_out_the_title_and_its_rule() {
		let mut facts = Facts::new();
		facts.row_mut(FactKey::Os).push(b"DOS 6.22");
		let (lines, _) = run_with(Depth::None, 80, false, &facts);
		assert_eq!(lines, ["OS:       DOS 6.22"]);
	}

	#[test]
	fn a_title_with_only_a_host_or_only_a_user() {
		let mut facts = Facts::new();
		facts.host.push(b"box");
		let (lines, _) = run_with(Depth::None, 80, false, &facts);
		assert_eq!(lines, ["box", "---"]);
		let mut facts = Facts::new();
		facts.user.push(b"ann");
		let (lines, _) = run_with(Depth::None, 80, false, &facts);
		assert_eq!(lines, ["ann", "---"]);
	}

	#[test]
	fn a_long_value_is_cut_to_the_console_width() {
		let mut facts = sample();
		facts.row_mut(FactKey::Os).clear();
		facts
			.row_mut(FactKey::Os)
			.push(b"a very long operating system name that cannot fit");
		let (lines, _) = run(false, 80, &facts);
		assert!(lines[3].len() <= 79);
	}

	#[test]
	fn a_cut_never_splits_a_utf8_character() {
		assert_eq!(cut("héllo".as_bytes(), 2), b"h");
		assert_eq!(cut("héllo".as_bytes(), 3), "hé".as_bytes());
		assert_eq!(cut(b"abc", 10), b"abc");
		assert_eq!(cut(b"abc", 0), b"");
	}

	#[test]
	fn a_narrow_console_gets_the_facts_without_the_logo() {
		let (lines, _) = run(false, 40, &sample());
		assert_eq!(lines[0], "administrator@reactos");
		assert_eq!(lines[2], "OS:       ReactOS 0.4.16-x86");
		assert_eq!(lines.len(), 8);
		assert!(lines.iter().all(|line| !line.contains('~')));
	}

	#[test]
	fn the_logo_needs_room_for_the_facts_beside_it() {
		let (wide, _) = run(false, 76, &sample());
		assert_eq!(wide.len(), 12);
		let (narrow, _) = run(false, 75, &sample());
		assert_eq!(narrow.len(), 8);
	}

	#[test]
	fn more_facts_than_logo_rows_run_on_below_the_logo() {
		let mut facts = sample();
		for key in FactKey::ALL {
			if !facts.has(key) {
				facts.row_mut(key).push(b"x");
			}
		}
		let (lines, _) = run_with(Depth::None, 80, true, &facts);
		// Title, rule and eleven facts start on row 1, so the picture is 14 lines.
		assert_eq!(lines.len(), 14);
		assert_eq!(lines[13].find("Resources:"), Some(INFO_COLUMN));
		assert_eq!(lines[13].find("Resources: x"), Some(INFO_COLUMN));
	}

	#[test]
	fn mono_draws_the_sun_as_hashes_and_weights_the_tones() {
		struct Weights(Vec<Weight>);
		impl Terminal for Weights {
			fn write(&mut self, _bytes: &[u8]) {}
			fn set_style(&mut self, style: Style) {
				assert_eq!(style.fg, Paint::Default);
				assert_eq!(style.bg, Paint::Default);
				self.0.push(style.weight);
			}
			fn end_line(&mut self) {}
		}
		let mut weights = Weights(Vec::new());
		let win32 = win32();
		let layout = Layout {
			logo: Some(&win32),
			depth: Depth::Mono,
			blocks: Blocks::None,
			columns: 80,
		};
		render(&mut weights, &sample(), layout);
		assert!(weights.0.contains(&Weight::Bold));
		assert!(weights.0.contains(&Weight::Dim));
		let (lines, _) = run_with(Depth::Mono, 80, true, &sample());
		assert!(lines[3].contains('#'));
		assert_eq!(lines.len(), 12, "mono draws no swatches");
	}

	#[test]
	fn console_attributes_follow_the_styles_the_layout_asks_for() {
		// A console shell turns `Style::fill(Tone)` into this attribute over its default of 0x07.
		assert_eq!(fill_attribute(0x07, Tone::Gold), 0xe7);
		assert_eq!(text_attribute(0x07, Tone::Gold), 0x0e);
	}

	/// The tone the title's rule is drawn in: the style set just before the run of dashes.
	fn rule_tone(depth: Depth, arch: &[u8]) -> Style {
		#[derive(Default)]
		struct Rule {
			style: Option<Style>,
			found: Option<Style>,
		}
		impl Terminal for Rule {
			fn write(&mut self, bytes: &[u8]) {
				if !bytes.is_empty() && bytes.iter().all(|&byte| byte == b'-') {
					self.found = self.style;
				}
			}
			fn set_style(&mut self, style: Style) {
				self.style = Some(style);
			}
			fn end_line(&mut self) {}
		}
		let mut facts = sample();
		facts.arch.push(arch);
		let mut rule = Rule::default();
		render(
			&mut rule,
			&facts,
			Layout {
				logo: None,
				depth,
				blocks: Blocks::None,
				columns: 80,
			},
		);
		rule.found.expect("the rule is drawn")
	}

	#[test]
	fn the_rule_takes_the_accent_of_the_architecture() {
		for (arch, tone) in [
			(&b"x86_64"[..], Tone::Gold),
			(b"i686", Tone::Gold),
			(b"aarch64", Tone::Periwinkle),
			(b"armv7l", Tone::Periwinkle),
			(b"mips", Tone::Violet),
			(b"riscv64", Tone::Teal),
			(b"ppc64le", Tone::Coral),
			(b"sparc64", Tone::Slate),
			(b"", Tone::Slate),
		] {
			assert_eq!(rule_tone(Depth::TrueColour, arch), Style::text(tone), "{arch:?}");
		}
	}

	#[test]
	fn the_accent_is_a_weight_in_mono_and_nothing_without_colour() {
		assert_eq!(rule_tone(Depth::Mono, b"riscv64"), Style::mono(Tone::Teal));
		assert_eq!(rule_tone(Depth::None, b"riscv64"), Style::PLAIN);
	}

	#[test]
	fn the_accent_leaves_the_plain_text_alone() {
		let mut facts = sample();
		let (plain, _) = run(false, 80, &facts);
		facts.arch.push(b"riscv64");
		let (with_arch, _) = run(false, 80, &facts);
		assert_eq!(plain, with_arch);
	}
}
