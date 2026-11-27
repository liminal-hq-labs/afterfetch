// The preview must print what the real program prints: the core's own goldens, reproduced through the preview's fake platform and
// terminal, and the shapes the review pack depends on.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::PathBuf;

use afterfetch_preview::cli::{self, Format};
use afterfetch_preview::render::{DepthChoice, Request, render_ansi, render_text};
use afterfetch_preview::systems::{SYSTEMS, System};

fn golden(name: &str) -> String {
	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../afterfetch-core/tests/golden")
		.join(name);
	std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{}", path.display()))
}

/// The ReactOS sample is the one fact set the core's goldens use.
fn reactos(logo: &str, depth: DepthChoice, columns: usize, utf8: bool) -> Request {
	Request {
		depth,
		columns,
		logo: Some(logo.into()),
		utf8: Some(utf8),
		..Request::new(System::find("reactos").unwrap())
	}
}

fn strip_escapes(text: &str) -> String {
	let mut out = String::new();
	let mut chars = text.chars();
	while let Some(ch) = chars.next() {
		if ch == '\u{241b}' {
			for next in chars.by_ref() {
				if next == 'm' {
					break;
				}
			}
		} else {
			out.push(ch);
		}
	}
	out
}

#[test]
fn the_ansi_bytes_equal_the_core_goldens_for_every_logo_depth_and_width() {
	let depths = [
		("none", DepthChoice::Never),
		("mono", DepthChoice::Mono),
		("ansi16", DepthChoice::Ansi16),
		("ansi256", DepthChoice::Ansi256),
		("truecolour", DepthChoice::TrueColour),
	];
	for logo in afterfetch_art::LOGOS {
		for (name, depth) in depths {
			for columns in [80, 120] {
				let request = reactos(logo.name, depth, columns, true);
				let bytes = String::from_utf8(render_ansi(&request)).unwrap().replace('\x1b', "\u{241b}");
				assert_eq!(
					bytes,
					golden(&format!("{}/{name}-{columns}.txt", logo.name)),
					"{} {name} {columns}",
					logo.name
				);
			}
		}
	}
}

#[test]
fn plain_text_equals_the_plain_golden_and_the_reactos_original() {
	assert_eq!(
		render_text(&reactos("win32", DepthChoice::Never, 80, true)),
		golden("win32-plain-80.txt")
	);
	for logo in afterfetch_art::LOGOS {
		let text = render_text(&reactos(logo.name, DepthChoice::Never, 80, true));
		assert_eq!(
			text,
			golden(&format!("{}/none-80.txt", logo.name)).replace(" \n", "\n"),
			"{}",
			logo.name
		);
	}
}

#[test]
fn mono_text_is_the_mono_golden_without_its_escapes() {
	let request = reactos("win32", DepthChoice::Mono, 80, true);
	let wanted = strip_escapes(&golden("win32/mono-80.txt"));
	let wanted: String = wanted.lines().map(|line| line.trim_end().to_string() + "\n").collect();
	assert_eq!(render_text(&request), wanted);
}

#[test]
fn a_cells_terminal_gets_the_cells_golden() {
	for logo in afterfetch_art::LOGOS {
		let request = reactos(logo.name, DepthChoice::TrueColour, 80, false);
		let bytes = String::from_utf8(render_ansi(&request)).unwrap().replace('\x1b', "\u{241b}");
		assert_eq!(
			bytes,
			golden(&format!("{}/truecolour-80-cells.txt", logo.name)),
			"{}",
			logo.name
		);
	}
}

#[test]
fn the_narrow_console_drops_the_logo_at_the_documented_widths() {
	for (name, drops_at, shows_at) in [("linux", 71, 72), ("win32", 75, 76), ("dos", 71, 72)] {
		let system = System::find(name).unwrap();
		let rows = |columns| {
			render_text(&Request {
				columns,
				..Request::new(system)
			})
			.lines()
			.count()
		};
		let facts_only = render_text(&Request {
			no_logo: true,
			..Request::new(system)
		})
		.lines()
		.count();
		assert_eq!(rows(drops_at), facts_only, "{name} at {drops_at}");
		assert!(rows(shows_at) > facts_only || rows(shows_at) >= 12, "{name} at {shows_at}");
		for columns in [60, 72, 76] {
			let text = render_text(&Request {
				columns,
				..Request::new(system)
			});
			assert!(
				text.lines().all(|line| line.chars().count() <= columns),
				"{name} at {columns}"
			);
		}
	}
}

#[test]
fn every_system_renders_at_every_depth_and_width() {
	for system in &SYSTEMS {
		for arch in system.arches {
			for depth in [
				DepthChoice::Auto,
				DepthChoice::Never,
				DepthChoice::Mono,
				DepthChoice::Sixteen,
				DepthChoice::Ansi256,
				DepthChoice::TrueColour,
			] {
				for columns in [60, 72, 76, 80, 120] {
					let request = Request {
						arch,
						depth,
						columns,
						..Request::new(system)
					};
					let text = render_text(&request);
					assert!(!text.is_empty(), "{} {arch} {depth:?} {columns}", system.name);
					assert!(text.contains("OS:"), "{} {arch}", system.name);
				}
			}
		}
	}
}

fn args(list: &[&str]) -> Vec<String> {
	list.iter().map(|argument| argument.to_string()).collect()
}

#[test]
fn the_command_line_renders_text_html_png_and_the_sheet() {
	let run = |list: &[&str]| cli::execute(&cli::parse(&args(list)).unwrap()).unwrap();
	let text = run(&["--system", "linux", "--arch", "aarch64", "--colour", "mono"]);
	assert!(String::from_utf8(text.bytes).unwrap().contains("Cortex-A72"));
	let html = String::from_utf8(run(&["--system=macos", "--format=html", "--colour=256"]).bytes).unwrap();
	assert!(html.contains("<!doctype html>") && html.contains("Mac15,6"));
	let png = run(&["--system", "dos", "--format", "png", "--out", "x.png"]);
	assert_eq!(&png.bytes[1..4], b"PNG");
	assert_eq!(png.file.as_deref(), Some("x.png"));
	let sheet = run(&["--sheet", "s.png", "--colour", "16"]);
	assert_eq!(&sheet.bytes[1..4], b"PNG");
	let all = String::from_utf8(run(&["--all", "--arch", "all"]).bytes).unwrap();
	for system in &SYSTEMS {
		for arch in system.arches {
			assert!(
				all.contains(&format!("== {} {arch} (", system.name)),
				"{} {arch}",
				system.name
			);
		}
	}
	assert_eq!(cli::parse(&args(&["--format", "png"])).unwrap().format, Format::Png);
}

#[test]
fn bad_arguments_are_errors_not_panics() {
	for bad in [
		&["--system", "plan9"][..],
		&["--width", "5"],
		&["--colour", "pink"],
		&["--logo", "nope"],
		&["--arch"],
		&["--bogus"],
	] {
		assert!(cli::parse(&args(bad)).is_err(), "{bad:?}");
	}
	let wrong_arch = cli::parse(&args(&["--system", "dos", "--arch", "arm64"])).unwrap();
	assert!(cli::execute(&wrong_arch).is_err());
}

#[test]
fn json_carries_the_target_the_architecture_and_the_raw_numbers() {
	let linux = System::find("linux").unwrap();
	for (arch, target) in [
		("x86_64", "linux-x86_64"),
		("i586", "linux-i586"),
		("armv7l", "linux-armv7"),
		("riscv64", "linux-riscv64"),
	] {
		let json = afterfetch_preview::render::render_json(&Request {
			arch,
			..Request::new(linux)
		});
		assert!(json.contains(&format!("\"target\":\"{target}\"")), "{json}");
		assert!(
			json.contains("\"schema\":1") && json.contains("\"uptime_seconds\":") && json.contains("\"memory_total_bytes\":"),
			"{json}"
		);
	}
	let windows = afterfetch_preview::render::render_json(&Request::new(System::find("win32").unwrap()));
	assert!(
		windows.contains("\"target\":\"windows-x86_64\"") && windows.contains("\"arch\":\"x86_64\""),
		"{windows}"
	);
}

#[test]
fn the_title_rule_takes_the_architecture_accent() {
	let linux = System::find("linux").unwrap();
	let rule = |arch| {
		let bytes = render_ansi(&Request {
			arch,
			depth: DepthChoice::TrueColour,
			..Request::new(linux)
		});
		let text = String::from_utf8(bytes).unwrap();
		let dashes = text.find("-----").unwrap();
		let start = text[..dashes].rfind("\u{1b}[38;2;").unwrap();
		text[start..dashes].split('m').next().unwrap().to_string()
	};
	assert_ne!(rule("x86_64"), rule("aarch64"));
	assert_ne!(rule("aarch64"), rule("riscv64"));
	assert_ne!(rule("riscv64"), rule("ppc64le"));
}
