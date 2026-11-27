// Runs the real core over sample facts: the same `run` a platform shell calls, with a fake `Platform` and a terminal that records the
// cells (for text, pictures and pages) or writes the escape sequences (for the exact bytes a terminal would receive).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::{AnsiTerminal, Buf, ByteSink, Depth, Facts, Platform, Style, Terminal, TerminalInfo};

use crate::grid::Grid;
use crate::systems::{Sample, System, sample};

/// The colour depth asked for, by the spelling of `--colour`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthChoice {
	/// What the system's terminal detects (its own default).
	Auto,
	Never,
	Mono,
	/// `16`: console attributes where the system's colour is console attributes, ANSI otherwise.
	Sixteen,
	/// ANSI 16 colours, even on a system that would use console attributes.
	Ansi16,
	/// Console attributes, even on a system that would use ANSI.
	Console16,
	Ansi256,
	TrueColour,
}

impl DepthChoice {
	pub fn parse(name: &str) -> Option<DepthChoice> {
		Some(match name.to_ascii_lowercase().as_str() {
			"auto" => DepthChoice::Auto,
			"never" | "none" | "plain" => DepthChoice::Never,
			"mono" => DepthChoice::Mono,
			"16" => DepthChoice::Sixteen,
			"ansi16" => DepthChoice::Ansi16,
			"console16" => DepthChoice::Console16,
			"256" => DepthChoice::Ansi256,
			"truecolour" | "truecolor" | "24bit" => DepthChoice::TrueColour,
			_ => return None,
		})
	}

	/// The names `--colour` accepts, for the usage text.
	pub const NAMES: &'static str = "auto|never|mono|16|ansi16|console16|256|truecolour";
}

/// One preview: which system and architecture, how wide the terminal is, which depth and logo.
#[derive(Clone, Debug)]
pub struct Request {
	pub system: &'static System,
	pub arch: &'static str,
	pub depth: DepthChoice,
	pub columns: usize,
	/// A logo name, or None for the system's own.
	pub logo: Option<String>,
	/// Whether the terminal draws UTF-8 half blocks; None takes the system's own.
	pub utf8: Option<bool>,
	/// Run with `--no-logo`.
	pub no_logo: bool,
}

impl Request {
	/// The default look of `system`: its first architecture, its own depth, 80 columns.
	pub fn new(system: &'static System) -> Request {
		Request {
			system,
			arch: system.arches[0],
			depth: DepthChoice::Auto,
			columns: 80,
			logo: None,
			utf8: None,
			no_logo: false,
		}
	}

	/// The arguments the real program would be given.
	pub fn arguments(&self) -> Vec<String> {
		let mut arguments: Vec<String> = self.system.forced.iter().map(|argument| argument.to_string()).collect();
		let mode = match self.depth {
			DepthChoice::Auto => None,
			DepthChoice::Never => Some("never"),
			DepthChoice::Mono => Some("mono"),
			DepthChoice::Sixteen | DepthChoice::Ansi16 | DepthChoice::Console16 => Some("16"),
			DepthChoice::Ansi256 => Some("256"),
			DepthChoice::TrueColour => Some("truecolour"),
		};
		if let Some(mode) = mode {
			arguments.push("--colour".into());
			arguments.push(mode.into());
		}
		if let Some(logo) = &self.logo {
			arguments.push("--logo".into());
			arguments.push(logo.clone());
		}
		if self.no_logo {
			arguments.push("--no-logo".into());
		}
		arguments
	}

	/// What the terminal detects for this request.
	fn hint(&self) -> Depth {
		match self.depth {
			DepthChoice::Ansi16 => Depth::Ansi16,
			DepthChoice::Console16 => Depth::Console16,
			DepthChoice::Mono => Depth::Mono,
			_ => self.system.hint,
		}
	}

	fn utf8(&self) -> bool {
		self.utf8.unwrap_or(self.system.utf8)
	}
}

/// A platform that answers from a sample.
struct Preview {
	sample: Sample,
	info: TerminalInfo,
	platform: &'static str,
	target: String,
}

impl Platform for Preview {
	fn facts(&mut self, facts: &mut Facts) {
		facts.user.push(self.sample.user.as_bytes());
		facts.host.push(self.sample.host.as_bytes());
		facts.arch.push(self.sample.arch.as_bytes());
		if let Some(seconds) = self.sample.uptime_seconds {
			facts.set_uptime(seconds);
		}
		if let Some((available, total)) = self.sample.memory {
			facts.set_memory(available, total);
		}
		for (key, value) in &self.sample.rows {
			facts.row_mut(*key).push(value.as_bytes());
		}
	}

	fn terminal(&mut self) -> TerminalInfo {
		self.info
	}

	fn env(&self, _name: &[u8]) -> Option<&[u8]> {
		None
	}

	fn default_logo(&self) -> &'static str {
		afterfetch_core::default_logo(self.platform)
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(self.target.as_bytes());
	}
}

fn preview_for(request: &Request) -> Preview {
	Preview {
		sample: sample(request.system, request.arch, request.columns),
		info: TerminalInfo {
			is_tty: true,
			columns: request.columns,
			depth_hint: request.hint(),
			utf8: request.utf8(),
		},
		platform: request.system.platform,
		target: request.system.target(request.arch),
	}
}

fn run_with(request: &Request, terminal: &mut impl Terminal) -> u8 {
	let arguments = request.arguments();
	afterfetch_core::run(
		&mut preview_for(request),
		terminal,
		arguments.iter().map(|argument| argument.as_bytes()),
	)
}

/// Runs the program and records every cell it draws.
pub fn render_grid(request: &Request) -> Grid {
	let mut grid = Grid::default();
	run_with(request, &mut grid);
	grid
}

struct Collect(Vec<u8>);

impl ByteSink for Collect {
	fn write_bytes(&mut self, bytes: &[u8]) {
		self.0.extend_from_slice(bytes);
	}
}

/// Passes everything to an ANSI terminal, with the console attributes of Win32 and DOS written as 16-colour escapes, which is the
/// nearest a terminal gets.
struct Console16AsAnsi<'a, S: ByteSink>(AnsiTerminal<'a, S>);

impl<S: ByteSink> Terminal for Console16AsAnsi<'_, S> {
	fn write(&mut self, bytes: &[u8]) {
		self.0.write(bytes);
	}
	fn set_style(&mut self, style: Style) {
		self.0.set_style(style);
	}
	fn end_line(&mut self) {
		self.0.end_line();
	}
	fn flush(&mut self) -> bool {
		self.0.flush()
	}
	fn set_depth(&mut self, depth: Depth) {
		self.0
			.set_depth(if depth == Depth::Console16 { Depth::Ansi16 } else { depth });
	}
}

/// The bytes a terminal would receive: escape sequences from the core's own `AnsiTerminal`. Console attributes (Win32, DOS) have no
/// escape form, so they are written as 16-colour ANSI.
pub fn render_ansi(request: &Request) -> Vec<u8> {
	let mut collect = Collect(Vec::new());
	{
		let mut terminal = Console16AsAnsi(AnsiTerminal::new(&mut collect, Depth::None, b"\n"));
		run_with(request, &mut terminal);
	}
	collect.0
}

/// The plain text of the picture: no escapes, trailing spaces removed.
pub fn render_text(request: &Request) -> String {
	render_grid(request).text()
}

/// The exact `--json` the real program would print.
pub fn render_json(request: &Request) -> String {
	let mut json = request.clone();
	json.depth = DepthChoice::Auto;
	let arguments: Vec<String> = ["--json".to_string()].into();
	let mut grid = Grid::default();
	afterfetch_core::run(
		&mut preview_for(&json),
		&mut grid,
		arguments.iter().map(|argument| argument.as_bytes()),
	);
	grid.text()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn request(system: &str) -> Request {
		Request::new(System::find(system).unwrap())
	}

	#[test]
	fn arguments_follow_the_request() {
		let mut linux = request("linux");
		linux.depth = DepthChoice::Ansi256;
		linux.logo = Some("macos".into());
		assert_eq!(linux.arguments(), ["--colour", "256", "--logo", "macos"]);
		assert_eq!(request("desk").arguments(), ["--plain"]);
	}

	#[test]
	fn desk_prints_one_line_and_no_title() {
		assert_eq!(
			render_text(&request("desk"))
				.lines()
				.filter(|line| line.contains("Desk 0.1.0"))
				.count(),
			1
		);
		let facts_only = render_text(&Request {
			no_logo: true,
			..request("desk")
		});
		assert_eq!(facts_only, "OS:       Desk 0.1.0 (native)\n");
	}

	#[test]
	fn the_json_has_every_key_with_null_for_a_missing_fact() {
		let json = render_json(&request("dos"));
		assert!(
			json.contains("\"desktop\":null") || json.contains("\"desktop\": null"),
			"{json}"
		);
	}

	#[test]
	fn ansi_output_has_escapes_and_text_has_none() {
		let linux = request("linux");
		assert!(render_ansi(&linux).contains(&0x1b));
		assert!(!render_text(&linux).contains('\x1b'));
	}

	#[test]
	fn console_attributes_are_written_as_sixteen_colour_escapes() {
		let dos = Request {
			depth: DepthChoice::Sixteen,
			..request("dos")
		};
		let bytes = String::from_utf8(render_ansi(&dos)).unwrap();
		assert!(bytes.contains("\x1b[9") || bytes.contains("\x1b[3"), "{bytes:?}");
	}
}
