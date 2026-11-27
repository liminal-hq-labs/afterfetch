// The command line of `afterfetch-preview`: parse the arguments and produce the bytes to write. Nothing here touches the file system or
// the terminal, so the whole tool is tested through `execute`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::html::{self, Window};
use crate::render::{DepthChoice, Request, render_ansi, render_grid, render_json, render_text};
use crate::sheet::{build_note, contact_sheet, tile_title};
use crate::systems::{SHEET_SYSTEMS, SYSTEMS, System};

pub const USAGE: &str = "\
afterfetch-preview: what afterfetch looks like on any supported system, from sample facts

Usage: afterfetch-preview [options]

  --system <name>     linux (default), freebsd, macos, win32, win95, reactos, dos, win16, desk, desk-hosted, other
  --arch <name>       the system's architecture (see --list); `all` with --all renders every one
  --colour <mode>     auto (the system's own default), never, mono, 16, ansi16, console16, 256, truecolour
  --width <n>         terminal width in columns (default 80; the logo shows from 72, or 76 for win32)
  --logo <name>       a logo from `afterfetch --logo list` instead of the system's own
  --no-logo           the facts alone
  --cells             draw the logo as cells, as a terminal without UTF-8 does
  --format <kind>     text (default), ansi, png, html, json
  --out <file>        write there instead of standard output (needed for png when standard output is a terminal)
  --all               every system (with its default architecture, or every architecture with `--arch all`)
  --sheet <file.png>  a contact sheet of the eight logos, each on its own system, at the chosen depth and width
  --list              the systems and their architectures
  -h, --help          this text
";

/// What to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
	Text,
	Ansi,
	Png,
	Html,
	Json,
}

/// The parsed command line.
#[derive(Debug)]
pub struct Cli {
	pub system: &'static System,
	pub arch: Option<String>,
	pub depth: DepthChoice,
	pub columns: usize,
	pub logo: Option<String>,
	pub no_logo: bool,
	pub cells: bool,
	pub format: Format,
	pub out: Option<String>,
	pub all: bool,
	pub sheet: Option<String>,
	pub list: bool,
	pub help: bool,
}

fn value<'a>(arguments: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<&'a String, String> {
	arguments.next().ok_or_else(|| format!("missing value for {flag}"))
}

/// Parses the arguments after the program's name.
pub fn parse(arguments: &[String]) -> Result<Cli, String> {
	let mut cli = Cli {
		system: &SYSTEMS[0],
		arch: None,
		depth: DepthChoice::Auto,
		columns: 80,
		logo: None,
		no_logo: false,
		cells: false,
		format: Format::Text,
		out: None,
		all: false,
		sheet: None,
		list: false,
		help: false,
	};
	let mut arguments = arguments.iter();
	while let Some(argument) = arguments.next() {
		let (flag, inline) = match argument.split_once('=') {
			Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
			_ => (argument.as_str(), None),
		};
		let take = |arguments: &mut std::slice::Iter<'_, String>| match &inline {
			Some(value) => Ok(value.clone()),
			None => value(arguments, flag).cloned(),
		};
		match flag {
			"--system" => {
				let name = take(&mut arguments)?;
				cli.system = System::find(&name).ok_or_else(|| format!("unknown system `{name}` (try --list)"))?;
			}
			"--arch" => cli.arch = Some(take(&mut arguments)?),
			"--colour" | "--color" => {
				let name = take(&mut arguments)?;
				cli.depth =
					DepthChoice::parse(&name).ok_or_else(|| format!("unknown colour `{name}` ({})", DepthChoice::NAMES))?;
			}
			"--width" => {
				let text = take(&mut arguments)?;
				cli.columns = text
					.parse::<usize>()
					.ok()
					.filter(|columns| (20..=400).contains(columns))
					.ok_or_else(|| format!("bad width `{text}` (20 to 400)"))?;
			}
			"--logo" => cli.logo = Some(take(&mut arguments)?),
			"--no-logo" => cli.no_logo = true,
			"--cells" => cli.cells = true,
			"--format" => {
				let name = take(&mut arguments)?;
				cli.format = match name.as_str() {
					"text" => Format::Text,
					"ansi" => Format::Ansi,
					"png" => Format::Png,
					"html" => Format::Html,
					"json" => Format::Json,
					other => return Err(format!("unknown format `{other}` (text, ansi, png, html, json)")),
				};
			}
			"--out" => cli.out = Some(take(&mut arguments)?),
			"--all" => cli.all = true,
			"--sheet" => cli.sheet = Some(take(&mut arguments)?),
			"--list" => cli.list = true,
			"-h" | "--help" => cli.help = true,
			other => return Err(format!("unknown option `{other}` (try --help)")),
		}
	}
	if let Some(name) = &cli.logo {
		let known =
			afterfetch_art::LOGOS.iter().any(|logo| logo.name.eq_ignore_ascii_case(name)) || name.eq_ignore_ascii_case("auto");
		if !known {
			return Err(format!("unknown logo `{name}`"));
		}
	}
	Ok(cli)
}

/// What a run produces: the bytes, and the file they go to when one was named.
#[derive(Debug)]
pub struct Output {
	pub bytes: Vec<u8>,
	pub file: Option<String>,
}

fn requests(cli: &Cli) -> Result<Vec<Request>, String> {
	let request_for = |system: &'static System, arch: &'static str| Request {
		system,
		arch,
		depth: cli.depth,
		columns: cli.columns,
		logo: cli.logo.clone(),
		utf8: cli.cells.then_some(false),
		no_logo: cli.no_logo,
	};
	if cli.all {
		let every_arch = cli.arch.as_deref().is_some_and(|arch| arch.eq_ignore_ascii_case("all"));
		let mut list = Vec::new();
		for system in &SYSTEMS {
			if every_arch {
				list.extend(system.arches.iter().map(|arch| request_for(system, arch)));
			} else {
				let arch = match &cli.arch {
					Some(name) => system.find_arch(name).unwrap_or(system.arches[0]),
					None => system.arches[0],
				};
				list.push(request_for(system, arch));
			}
		}
		return Ok(list);
	}
	let arch = match &cli.arch {
		None => cli.system.arches[0],
		Some(name) => cli.system.find_arch(name).ok_or_else(|| {
			format!(
				"{} has no architecture `{name}` (it has {})",
				cli.system.name,
				cli.system.arches.join(", ")
			)
		})?,
	};
	Ok(vec![request_for(cli.system, arch)])
}

fn heading(requests: &[Request]) -> String {
	let first = &requests[0];
	let depth = match first.depth {
		DepthChoice::Auto => "each system's own depth",
		DepthChoice::Never => "no colour",
		DepthChoice::Mono => "mono",
		DepthChoice::Sixteen | DepthChoice::Ansi16 => "16 colours",
		DepthChoice::Console16 => "console 16",
		DepthChoice::Ansi256 => "256 colours",
		DepthChoice::TrueColour => "truecolour",
	};
	format!(
		"afterfetch {} - {depth} - {} columns",
		afterfetch_core::VERSION,
		first.columns
	)
}

/// Runs a parsed command line.
pub fn execute(cli: &Cli) -> Result<Output, String> {
	if cli.help {
		return Ok(Output {
			bytes: USAGE.as_bytes().to_vec(),
			file: None,
		});
	}
	if cli.list {
		let mut text = String::new();
		for system in &SYSTEMS {
			text.push_str(&format!(
				"{:<12} {}\n{:<12} architectures: {}\n",
				system.name,
				system.summary,
				"",
				system.arches.join(", ")
			));
		}
		text.push_str("\nContact sheet systems: ");
		text.push_str(&SHEET_SYSTEMS.join(", "));
		text.push('\n');
		return Ok(Output {
			bytes: text.into_bytes(),
			file: None,
		});
	}
	if let Some(file) = &cli.sheet {
		let sheet: Vec<Request> = SHEET_SYSTEMS
			.iter()
			.map(|name| Request {
				system: System::find(name).expect("the sheet names real systems"),
				arch: System::find(name).expect("the sheet names real systems").arches[0],
				depth: cli.depth,
				columns: cli.columns,
				logo: None,
				utf8: cli.cells.then_some(false),
				no_logo: false,
			})
			.collect();
		let canvas = contact_sheet(&heading(&sheet), &sheet, 2);
		return Ok(Output {
			bytes: canvas.to_png(),
			file: Some(file.clone()),
		});
	}
	let list = requests(cli)?;
	let bytes = match cli.format {
		Format::Text => join(&list, |request| render_text(request).into_bytes(), true),
		Format::Ansi => join(&list, render_ansi, true),
		Format::Json => join(&list, |request| render_json(request).into_bytes(), true),
		Format::Png => contact_sheet(&heading(&list), &list, if list.len() > 1 { 2 } else { 1 }).to_png(),
		Format::Html => {
			let windows: Vec<Window> = list
				.iter()
				.map(|request| Window {
					title: tile_title(request),
					note: build_note(request),
					grid: render_grid(request),
				})
				.collect();
			html::page(&heading(&list), &windows).into_bytes()
		}
	};
	Ok(Output {
		bytes,
		file: cli.out.clone(),
	})
}

/// The outputs one after another, with a header line before each when there is more than one.
fn join(list: &[Request], render: impl Fn(&Request) -> Vec<u8>, header: bool) -> Vec<u8> {
	let mut out = Vec::new();
	for (position, request) in list.iter().enumerate() {
		if list.len() > 1 && header {
			if position > 0 {
				out.push(b'\n');
			}
			out.extend_from_slice(format!("== {} ({}) ==\n", tile_title(request), build_note(request)).as_bytes());
		}
		out.extend(render(request));
	}
	out
}
