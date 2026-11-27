// Test helpers shared by the integration tests: a platform with fixed facts and a terminal that records what it is asked to draw.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(dead_code)]

use afterfetch_core::{ByteSink, Depth, FactKey, Facts, Platform, Style, Terminal, TerminalInfo};

/// A platform whose answers are set by the test.
pub struct Fake {
	pub info: TerminalInfo,
	pub environment: Vec<(&'static str, &'static str)>,
	pub logo: &'static str,
	pub full: bool,
	/// The target name the platform gives; empty keeps the trait's default.
	pub target: &'static str,
	/// The raw numbers and architecture the facts carry (reactos: 387 MB free of 511 MB, up 60 s, `x86`).
	pub raw: bool,
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
			logo: "win32",
			full: true,
			target: "windows-i586",
			raw: true,
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
		if self.raw {
			facts.arch.push(b"x86");
			facts.set_uptime(60);
			facts.set_memory(387 << 20, 511 << 20);
		} else {
			facts.row_mut(FactKey::Uptime).push(b"0d 0h 1m");
			facts.row_mut(FactKey::Memory).push(b"387 MB free of 511 MB");
		}
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

	fn default_logo(&self) -> &'static str {
		self.logo
	}

	fn target_name(&self, out: &mut afterfetch_core::Buf<32>) {
		out.push(self.target.as_bytes());
	}
}

/// Records the text, with a style marker where the style changes. `{a:..}` is the console attribute over a default of 0x07.
#[derive(Default)]
pub struct Recorder {
	pub text: String,
	pub styles: usize,
	pub errors: String,
	/// When set, `flush` reports a failed write, as a closed pipe would.
	pub fail: bool,
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
	fn flush(&mut self) -> bool {
		!self.fail
	}
	fn write_error_line(&mut self, bytes: &[u8]) {
		self.errors.push_str(&String::from_utf8_lossy(bytes));
		self.errors.push('\n');
	}
}

pub fn run_with(platform: &mut Fake, arguments: &[&str]) -> (u8, Recorder) {
	run_failing(platform, arguments, false)
}

/// Like `run_with`, with a terminal whose writes fail when `fail` is true.
pub fn run_failing(platform: &mut Fake, arguments: &[&str], fail: bool) -> (u8, Recorder) {
	let mut recorder = Recorder {
		fail,
		..Recorder::default()
	};
	let code = afterfetch_core::run(platform, &mut recorder, arguments.iter().map(|argument| argument.as_bytes()));
	(code, recorder)
}

/// A byte sink that collects into a `Vec`.
#[derive(Default)]
pub struct Collect(pub Vec<u8>);

impl ByteSink for Collect {
	fn write_bytes(&mut self, bytes: &[u8]) {
		self.0.extend_from_slice(bytes);
	}
}
