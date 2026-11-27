// The whole program: parse the arguments, ask the platform for its facts and its terminal, decide the depth, then print. A shell is an
// entry point, its system calls and implementations of `Platform` and `Terminal`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::{self, Action, LogoChoice, UsageKind};
use crate::depth::{self, Depth, DepthInputs};
use crate::facts::Facts;
use crate::format::Buf;
use crate::json;
use crate::layout::{self, Layout};
use crate::logo::{Blocks, Logo, find};
use crate::sink::{Style, Terminal};

/// The program's name, as `--version` prints it.
pub const NAME: &str = "afterfetch";

/// The program's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Exit code for a normal run.
pub const EXIT_OK: u8 = 0;
/// Exit code for a usage error.
pub const EXIT_USAGE: u8 = 2;
/// Exit code a shell uses when it crashes.
pub const EXIT_CRASH: u8 = 70;
/// Exit code when the output could not be written: a closed pipe, or a `/DUMP` file that cannot be created (the BSD `EX_IOERR`, AF-D33).
pub const EXIT_OUTPUT: u8 = 74;

/// The exit code for a run that ended with `code` once it is known whether every write worked. A run that would have succeeded exits
/// with `EXIT_OUTPUT` instead; a failure keeps its own code, since that is the more useful thing to report.
///
/// `run` applies this itself to what the terminal says through `Terminal::flush`. A shell calls it for a failure `run` cannot see, such
/// as a `/DUMP` file it could not create (`output_exit(code, false)`).
pub const fn output_exit(code: u8, written: bool) -> u8 {
	if code == EXIT_OK && !written { EXIT_OUTPUT } else { code }
}

/// What the layout needs to know about the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalInfo {
	/// Standard output is a terminal and not a pipe or a file.
	pub is_tty: bool,
	/// The width in columns, or 0 when it is not known (taken as 80).
	pub columns: usize,
	/// The depth the terminal is detected to support.
	pub depth_hint: Depth,
	/// The terminal draws UTF-8.
	pub utf8: bool,
}

/// Everything the core needs from the system underneath.
pub trait Platform {
	/// Fills in the facts this platform can honestly report and leaves the rest empty, with the raw numbers (`arch`, `uptime_seconds`,
	/// `memory_available_bytes`, `memory_total_bytes`) that `--json` carries beside the text.
	fn facts(&mut self, facts: &mut Facts);
	fn terminal(&mut self) -> TerminalInfo;
	/// The value of an environment variable.
	fn env(&self, name: &[u8]) -> Option<&[u8]>;
	/// The name of the logo drawn for `--logo auto`: `default_logo` of the art crate for this platform's name (`linux`, `win32`, `dos`).
	fn default_logo(&self) -> &'static str;
	/// The target name of SPEC section 2 (`linux-x86_64`, `linux-i586`, `windows-i586`): what `--version` prints in brackets and `--json`
	/// calls `target`. Every shell names its own; the default is only the build's own operating system and Rust architecture, which is
	/// wrong for a build that has a name of its own (a Pentium build is `i586`, not `x86`).
	fn target_name(&self, out: &mut Buf<32>) {
		out.push(build_os().as_bytes()).push(b"-").push(build_arch().as_bytes());
	}
}

/// The operating system this build was compiled for, as a short name.
pub const fn build_os() -> &'static str {
	if cfg!(target_os = "linux") {
		"linux"
	} else if cfg!(target_os = "windows") {
		"windows"
	} else if cfg!(target_os = "freebsd") {
		"freebsd"
	} else if cfg!(target_os = "macos") {
		"macos"
	} else {
		"none"
	}
}

/// The processor architecture this build was compiled for, as a short name.
pub const fn build_arch() -> &'static str {
	if cfg!(target_arch = "x86_64") {
		"x86_64"
	} else if cfg!(target_arch = "x86") {
		"x86"
	} else if cfg!(target_arch = "aarch64") {
		"aarch64"
	} else if cfg!(target_arch = "arm") {
		"arm"
	} else if cfg!(target_arch = "mips") {
		"mips"
	} else if cfg!(target_arch = "mips64") {
		"mips64"
	} else if cfg!(target_arch = "riscv64") {
		"riscv64"
	} else if cfg!(target_arch = "powerpc64") {
		"powerpc64"
	} else {
		"unknown"
	}
}

const HELP: &[&str] = &[
	"afterfetch: system information with an Afterglow sun",
	"",
	"Usage: afterfetch [options]",
	"",
	"  --plain            no colour and the ASCII logo (what a pipe gets)",
	"  --no-logo          the facts alone",
	"  --logo <name>      auto (default), or a name from --logo list",
	"  --colour <mode>    auto (default), never, always, mono, 16, 256, truecolour",
	"  --json             the facts as one JSON object",
	"  -h, --help         this text",
	"  -V, --version      the version",
	"",
	"/PLAIN, /NOLOGO, /LOGO:name, /COLOUR:mode, /MONO, /JSON and /? work as well.",
];

fn lines(out: &mut impl Terminal, text: &[&str]) {
	for line in text {
		out.write(line.as_bytes());
		out.end_line();
	}
}

/// Runs the program. `argv` is the arguments after the program's name. Returns the exit code: 0, 2 for a usage error, or 74 when the
/// terminal reports at the end that a write failed.
pub fn run<'a>(platform: &mut impl Platform, out: &mut impl Terminal, argv: impl Iterator<Item = &'a [u8]>) -> u8 {
	let code = run_once(platform, out, argv);
	output_exit(code, out.flush())
}

fn run_once<'a>(platform: &mut impl Platform, out: &mut impl Terminal, argv: impl Iterator<Item = &'a [u8]>) -> u8 {
	let options = match args::parse(argv) {
		Ok(options) => options,
		Err(error) => {
			let mut message = Buf::<80>::new();
			message.push(NAME.as_bytes()).push(match error.kind {
				UsageKind::UnknownOption => b": unknown option in argument ".as_slice(),
				UsageKind::MissingValue => b": missing value for argument ".as_slice(),
				UsageKind::BadValue => b": bad value in argument ".as_slice(),
			});
			message.push_u32(u32::try_from(error.index + 1).unwrap_or(u32::MAX));
			out.write_error_line(message.as_bytes());
			out.write_error_line(b"Try --help.");
			return EXIT_USAGE;
		}
	};
	match options.action {
		Action::Help => {
			lines(out, HELP);
			return EXIT_OK;
		}
		Action::Version => {
			let mut target = Buf::<32>::new();
			platform.target_name(&mut target);
			out.write(NAME.as_bytes());
			out.write(b" ");
			out.write(VERSION.as_bytes());
			out.write(b" (");
			out.write(target.as_bytes());
			out.write(b")");
			out.end_line();
			return EXIT_OK;
		}
		Action::ListLogos => {
			for logo in afterfetch_art::LOGOS {
				out.write(logo.name.as_bytes());
				out.end_line();
			}
			return EXIT_OK;
		}
		Action::Run => {}
	}

	let mut facts = Facts::new();
	platform.facts(&mut facts);
	if options.json {
		let mut target = Buf::<32>::new();
		platform.target_name(&mut target);
		json::write(out, &facts, target.as_bytes());
		return EXIT_OK;
	}

	let info = platform.terminal();
	let set = |name: &[u8]| platform.env(name).is_some_and(|value| !value.is_empty());
	let depth = depth::resolve(DepthInputs {
		mode: options.colour,
		plain: options.plain,
		no_color: set(b"NO_COLOR"),
		force: set(b"CLICOLOR_FORCE") || set(b"FORCE_COLOR"),
		is_tty: info.is_tty,
		hint: info.depth_hint,
	});
	let logo_name = match options.logo {
		LogoChoice::Auto => platform.default_logo(),
		LogoChoice::Named(name) => name,
	};
	// A blob that fails to validate would be a build defect; the facts are still worth printing.
	let entry = find(logo_name.as_bytes()).or_else(|| find(afterfetch_art::FALLBACK_LOGO.as_bytes()));
	let logo = entry
		.filter(|_| !options.no_logo)
		.and_then(|entry| Logo::parse(entry.blob).ok());
	let blocks = if info.utf8 { Blocks::Utf8 } else { Blocks::None };
	let columns = if info.columns == 0 { 80 } else { info.columns };
	out.set_depth(depth);
	out.set_style(Style::PLAIN);
	layout::render(
		out,
		&facts,
		Layout {
			logo: logo.as_ref(),
			depth,
			blocks,
			columns,
		},
	);
	EXIT_OK
}
