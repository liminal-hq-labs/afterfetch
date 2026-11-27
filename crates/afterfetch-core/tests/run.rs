// The whole program through a fake platform: options, exit codes, JSON, the version line and the omitted rows.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod common;

use afterfetch_core::Depth;
use common::{Fake, run_with};

#[test]
fn json_has_the_stable_keys_and_no_logo() {
	let (code, recorder) = run_with(&mut Fake::reactos(), &["--json"]);
	assert_eq!(code, 0);
	assert_eq!(
		recorder.text,
		"{\"user\":\"administrator\",\"hostname\":\"reactos\",\"os\":\"ReactOS 0.4.16-x86\",\"kernel\":\"NT 5.2.3790 SP3\",\
		 \"model\":null,\"uptime\":\"0d 0h 1m\",\"cpu\":null,\"memory\":\"387 MB free of 511 MB\",\
		 \"display\":\"1024 x 768, 32-bit\",\"desktop\":null,\"terminal\":null,\"shell\":\"cmd.exe\",\"resources\":null}\n"
	);
	assert_eq!(recorder.styles, 0);
}

#[test]
fn json_of_a_platform_with_no_facts_is_all_null() {
	let mut platform = Fake::reactos();
	platform.full = false;
	let (_, recorder) = run_with(&mut platform, &["--json"]);
	assert!(recorder.text.starts_with("{\"user\":null,\"hostname\":null,"));
	assert_eq!(recorder.text.matches("null").count(), 13);
}

#[test]
fn no_logo_prints_the_facts_alone() {
	let (_, recorder) = run_with(&mut Fake::reactos(), &["--no-logo", "--plain"]);
	let lines: Vec<&str> = recorder.text.lines().collect();
	assert_eq!(lines.len(), 8);
	assert_eq!(lines[0], "administrator@reactos");
	assert_eq!(lines[7], "Shell:    cmd.exe");
}

#[test]
fn help_and_version_exit_cleanly() {
	let (code, recorder) = run_with(&mut Fake::reactos(), &["--help"]);
	assert_eq!(code, 0);
	assert!(recorder.text.contains("Usage: afterfetch"));
	assert!(recorder.text.lines().all(|line| line.len() <= 80));
	let (code, recorder) = run_with(&mut Fake::reactos(), &["-V"]);
	assert_eq!(code, 0);
	assert!(
		recorder
			.text
			.starts_with(&format!("afterfetch {} (", env!("CARGO_PKG_VERSION"))),
		"{}",
		recorder.text
	);
	assert!(recorder.text.ends_with(")\n"));
}

#[test]
fn logo_list_prints_every_name() {
	let (code, recorder) = run_with(&mut Fake::reactos(), &["--logo", "list"]);
	assert_eq!(code, 0);
	assert_eq!(recorder.text, "afterglow\ndos\nwin16\nwin32\nlinux\nfreebsd\nmacos\ndesk\n");
}

#[test]
fn a_usage_error_exits_with_2_and_writes_to_the_error_stream() {
	let (code, recorder) = run_with(&mut Fake::reactos(), &["--plain", "--wat"]);
	assert_eq!(code, 2);
	assert!(recorder.text.is_empty());
	assert_eq!(recorder.errors, "afterfetch: unknown option in argument 2\nTry --help.\n");
}

#[test]
fn colour_never_and_no_color_leave_no_styles_that_matter() {
	let mut platform = Fake::reactos();
	platform.info.is_tty = true;
	platform.environment.push(("NO_COLOR", "1"));
	let (_, quiet) = run_with(&mut platform, &[]);
	assert!(!quiet.text.contains('\u{1b}'));
	assert!(quiet.text.contains('#'), "no colour keeps the sun as hashes");
	let (_, forced) = run_with(&mut platform, &["--colour", "always"]);
	assert!(!forced.text.contains('#'), "always draws the sun as cells");
}

#[test]
fn force_variables_colour_a_pipe() {
	let mut platform = Fake::reactos();
	platform.info.depth_hint = Depth::TrueColour;
	let (_, pipe) = run_with(&mut platform, &[]);
	assert!(pipe.text.contains('#'));
	platform.environment.push(("CLICOLOR_FORCE", "1"));
	let (_, forced) = run_with(&mut platform, &[]);
	assert!(!forced.text.contains('#'));
}

#[test]
fn the_width_decides_whether_the_logo_is_drawn() {
	let mut platform = Fake::reactos();
	platform.info.columns = 60;
	let (_, narrow) = run_with(&mut platform, &["--plain"]);
	assert_eq!(narrow.text.lines().count(), 8);
	platform.info.columns = 0;
	let (_, unknown) = run_with(&mut platform, &["--plain"]);
	assert_eq!(unknown.text.lines().count(), 12, "an unknown width is taken as 80");
}

#[test]
fn arbitrary_arguments_never_panic() {
	let samples: [&[&str]; 6] = [
		&["--logo"],
		&["--colour=", "--logo="],
		&["", "-", "--", "/", "/?:x"],
		&["--logo", "--colour", "--json"],
		&[
			"a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
		],
		&["\u{1f600}", "--logo=\u{e9}"],
	];
	for sample in samples {
		let (code, _) = run_with(&mut Fake::reactos(), sample);
		assert!(code == 0 || code == 2, "{sample:?}");
	}
}
