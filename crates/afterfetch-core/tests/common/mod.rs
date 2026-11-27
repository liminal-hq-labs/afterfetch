// Test helpers shared by the integration tests: a platform with fixed facts and a terminal that records what it is asked to draw.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(dead_code)]

use afterfetch_core::{Depth, FactKey, Facts, LogoName, Platform, Style, Terminal, TerminalInfo};

/// A platform whose answers are set by the test.
pub struct Fake {
	pub info: TerminalInfo,
	pub environment: Vec<(&'static str, &'static str)>,
	pub logo: LogoName,
	pub full: bool,
}

impl Fake {
	/// The reactos-liminal sample: a Win32 console of 80 columns and every fact the original printed.
	pub fn reactos() -> Fake {
		Fake {
			info: TerminalInfo {
				is_tty: false,
				columns: 80,
				depth_hint: Depth::Console16,
				utf8: false,
			},
			environment: Vec::new(),
			logo: LogoName::Win32,
			full: true,
		}
	}
}

impl Platform for Fake {
	fn facts(&mut self, facts: &mut Facts) {
		if !self.full {
			return;
		}
		facts.user.push(b"administrator");
		facts.host.push(b"reactos");
		facts.row_mut(FactKey::Os).push(b"ReactOS 0.4.16-x86");
		facts.row_mut(FactKey::Kernel).push(b"NT 5.2.3790 SP3");
		facts.row_mut(FactKey::Uptime).push(b"0d 0h 1m");
		facts.row_mut(FactKey::Memory).push(b"387 MB free of 511 MB");
		facts.row_mut(FactKey::Display).push(b"1024 x 768, 32-bit");
		facts.row_mut(FactKey::Shell).push(b"cmd.exe");
	}

	fn terminal(&mut self) -> TerminalInfo {
		self.info
	}

	fn env(&self, name: &[u8]) -> Option<&[u8]> {
		self.environment
			.iter()
			.find(|(key, _)| key.as_bytes() == name)
			.map(|(_, value)| value.as_bytes())
	}

	fn default_logo(&self) -> LogoName {
		self.logo
	}
}

/// Records the text, with a style marker where the style changes. `{a:..}` is the console attribute over a default of 0x07.
#[derive(Default)]
pub struct Recorder {
	pub text: String,
	pub styles: usize,
	pub errors: String,
}

impl Terminal for Recorder {
	fn write(&mut self, bytes: &[u8]) {
		self.text.push_str(&String::from_utf8_lossy(bytes));
	}
	fn set_style(&mut self, _style: Style) {
		self.styles += 1;
	}
	fn end_line(&mut self) {
		self.text.push('\n');
	}
	fn write_error_line(&mut self, bytes: &[u8]) {
		self.errors.push_str(&String::from_utf8_lossy(bytes));
		self.errors.push('\n');
	}
}

pub fn run_with(platform: &mut Fake, arguments: &[&str]) -> (u8, Recorder) {
	let mut recorder = Recorder::default();
	let code = afterfetch_core::run(platform, &mut recorder, arguments.iter().map(|argument| argument.as_bytes()));
	(code, recorder)
}
