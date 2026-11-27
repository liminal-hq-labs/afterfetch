// Golden output: what reactos-liminal's afterfetch prints for the same facts, byte for byte, and the shapes of the other outputs.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod common;

use std::fmt::Write;
use std::path::PathBuf;

use afterfetch_core::{AnsiTerminal, Depth, Style, Terminal, Weight};
use common::{Collect, Fake, run_with};

const WIN32_PLAIN_80: &str = include_str!("golden/win32-plain-80.txt");

#[test]
fn plain_win32_logo_matches_the_reactos_liminal_output() {
	let (code, recorder) = run_with(&mut Fake::reactos(), &["--logo", "win32", "--plain"]);
	assert_eq!(code, 0);
	assert_eq!(recorder.text, WIN32_PLAIN_80);
}

#[test]
fn a_pipe_gets_the_same_plain_output_without_asking_for_it() {
	let (_, recorder) = run_with(&mut Fake::reactos(), &["--logo", "win32"]);
	assert_eq!(recorder.text, WIN32_PLAIN_80);
}

#[test]
fn the_slash_spelling_gives_the_same_picture() {
	let (_, recorder) = run_with(&mut Fake::reactos(), &["/LOGO:win32", "/PLAIN"]);
	assert_eq!(recorder.text, WIN32_PLAIN_80);
}

#[test]
fn a_console_draws_the_picture_with_styles() {
	let mut platform = Fake::reactos();
	platform.info.is_tty = true;
	let (_, recorder) = run_with(&mut platform, &[]);
	assert!(recorder.styles > 50);
	// The plain text under the colours: the sun is spaces, so the first fact line still reads the same.
	assert!(recorder.text.contains("administrator@reactos"));
	assert!(!recorder.text.contains('#'));
}

/// How one golden is drawn: a depth, a width and whether the terminal draws UTF-8 half blocks.
struct Case {
	depth: &'static str,
	columns: usize,
	utf8: bool,
}

/// A console shell's terminal: it shows every attribute change as `{a:XX}`, over a console whose own attribute is 0x07.
#[derive(Default)]
struct Attributes {
	text: String,
	shown: u16,
	styled: bool,
}

impl Attributes {
	const DEFAULT: u16 = 0x07;

	fn attribute(style: Style) -> u16 {
		let foreground = style.fg.slot().map_or(Self::DEFAULT & 0x0F, u16::from);
		let background = style.bg.slot().map_or(Self::DEFAULT & 0xF0, |slot| u16::from(slot) << 4);
		let bold = if style.weight == Weight::Bold { 0x08 } else { 0 };
		foreground | bold | background
	}
}

impl Terminal for Attributes {
	fn write(&mut self, bytes: &[u8]) {
		self.text.push_str(&String::from_utf8_lossy(bytes));
	}
	fn set_style(&mut self, style: Style) {
		let attribute = if style == Style::PLAIN {
			Self::DEFAULT
		} else {
			Self::attribute(style)
		};
		if attribute != self.shown {
			write!(self.text, "{{a:{attribute:02X}}}").unwrap();
			self.shown = attribute;
		}
		self.styled = attribute != Self::DEFAULT;
	}
	fn end_line(&mut self) {
		if self.styled {
			self.text.push_str("{a:07}");
		}
		self.text.push('\n');
		self.shown = Self::DEFAULT;
		self.styled = false;
	}
}

/// What `afterfetch --logo <logo>` prints for the reactos-liminal sample at this depth and width, with ESC shown as `\u{241b}`.
fn render(logo: &str, case: &Case) -> String {
	let mut platform = Fake::reactos();
	platform.info.columns = case.columns;
	platform.info.utf8 = case.utf8;
	platform.info.is_tty = true;
	platform.info.depth_hint = match case.depth {
		"console16" => Depth::Console16,
		"ansi16" => Depth::Ansi16,
		_ => Depth::TrueColour,
	};
	let mode = match case.depth {
		"none" => "never",
		"console16" | "ansi16" => "16",
		"ansi256" => "256",
		"truecolour" => "truecolour",
		other => other,
	};
	let arguments = ["--logo", logo, "--colour", mode];
	let arguments = arguments.iter().map(|argument| argument.as_bytes());
	if case.depth == "console16" {
		let mut terminal = Attributes::default();
		assert_eq!(afterfetch_core::run(&mut platform, &mut terminal, arguments), 0);
		terminal.text
	} else {
		let mut sink = Collect::default();
		{
			let mut terminal = AnsiTerminal::new(&mut sink, Depth::None, b"\n");
			assert_eq!(afterfetch_core::run(&mut platform, &mut terminal, arguments), 0);
		}
		String::from_utf8(sink.0).unwrap().replace('\x1b', "\u{241b}")
	}
}

/// Compares `actual` with the golden file, or rewrites the file when `AF_UPDATE_GOLDEN` is set.
fn check(name: &str, actual: &str) {
	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
	if std::env::var_os("AF_UPDATE_GOLDEN").is_some() {
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(&path, actual).unwrap();
		return;
	}
	let wanted = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{name} is missing; run with AF_UPDATE_GOLDEN=1"));
	assert!(
		wanted == actual,
		"{name} differs; run with AF_UPDATE_GOLDEN=1 to rewrite it and read the diff\n--- wanted\n{wanted}--- got\n{actual}"
	);
}

const DEPTHS: [&str; 6] = ["none", "mono", "ansi16", "ansi256", "truecolour", "console16"];

#[test]
fn every_logo_at_every_depth_and_width() {
	for logo in afterfetch_art::LOGOS {
		for depth in DEPTHS {
			for columns in [80, 120] {
				// The Win32 console draws cells; every other depth is a UTF-8 terminal here.
				let case = Case {
					depth,
					columns,
					utf8: depth != "console16",
				};
				check(&format!("{}/{depth}-{columns}.txt", logo.name), &render(logo.name, &case));
			}
		}
	}
}

#[test]
fn every_logo_as_cells_on_a_terminal_without_utf8() {
	for logo in afterfetch_art::LOGOS {
		for depth in ["mono", "ansi16", "ansi256", "truecolour"] {
			let case = Case {
				depth,
				columns: 80,
				utf8: false,
			};
			check(&format!("{}/{depth}-80-cells.txt", logo.name), &render(logo.name, &case));
		}
	}
}

#[test]
fn a_narrow_console_drops_the_logo_at_every_depth() {
	for depth in DEPTHS {
		let narrow = |columns| {
			let case = Case {
				depth,
				columns,
				utf8: true,
			};
			render("afterglow", &case)
		};
		// Afterglow's information column is 38, so the logo needs 72 columns.
		assert!(narrow(71).lines().count() < 12, "{depth}");
		assert!(narrow(72).lines().count() >= 12, "{depth}");
		let win32 = |columns| {
			let case = Case {
				depth,
				columns,
				utf8: true,
			};
			render("win32", &case)
		};
		assert!(win32(75).lines().count() < 12, "{depth}");
		assert!(win32(76).lines().count() >= 12, "{depth}");
	}
}

#[test]
fn the_plain_golden_is_the_reactos_one() {
	let case = Case {
		depth: "none",
		columns: 80,
		utf8: true,
	};
	assert_eq!(render("win32", &case), WIN32_PLAIN_80);
	assert_eq!(
		std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/win32/none-80.txt")).unwrap(),
		WIN32_PLAIN_80
	);
}

#[test]
fn colour_goldens_carry_no_stray_bytes() {
	// Every escape is shown, and no raw ESC or carriage return reaches a golden.
	let case = Case {
		depth: "truecolour",
		columns: 80,
		utf8: true,
	};
	let text = render("afterglow", &case);
	assert!(text.contains('\u{241b}') && !text.contains('\x1b') && !text.contains('\r'));
}
