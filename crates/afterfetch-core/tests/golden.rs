// Golden output: what reactos-liminal's afterfetch prints for the same facts, byte for byte, and the shapes of the other outputs.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod common;

use common::{Fake, run_with};

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
