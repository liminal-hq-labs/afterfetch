// The command line: GNU-style long options (`--logo win32`, `--logo=win32`) and the slash forms DOS and Windows users type (`/NOLOGO`,
// `/LOGO:win32`, `/?`). Parsed from raw argument bytes with no allocation.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// What the program was asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
	/// Print the facts.
	Run,
	Help,
	Version,
	/// `--logo list`: print the logo names.
	ListLogos,
}

/// How colour is chosen (`--colour`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourMode {
	/// From the terminal and the environment.
	Auto,
	Never,
	/// The detected depth even on a pipe.
	Always,
	Mono,
	Ansi16,
	Ansi256,
	TrueColour,
}

impl ColourMode {
	/// The mode named by `name`, case-insensitively: `auto`, `never`, `always`, `mono`, `16`, `256`, `truecolour` (also `truecolor`,
	/// `24bit`).
	pub fn parse(name: &[u8]) -> Option<ColourMode> {
		const NAMES: [(&[u8], ColourMode); 8] = [
			(b"auto", ColourMode::Auto),
			(b"never", ColourMode::Never),
			(b"always", ColourMode::Always),
			(b"mono", ColourMode::Mono),
			(b"16", ColourMode::Ansi16),
			(b"256", ColourMode::Ansi256),
			(b"truecolour", ColourMode::TrueColour),
			(b"truecolor", ColourMode::TrueColour),
		];
		if name.eq_ignore_ascii_case(b"24bit") {
			return Some(ColourMode::TrueColour);
		}
		NAMES
			.iter()
			.find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
			.map(|&(_, mode)| mode)
	}
}

/// Which logo to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoChoice {
	/// The platform's own.
	Auto,
	/// A logo of `afterfetch-art`, by its own name.
	Named(&'static str),
}

/// The parsed command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
	pub action: Action,
	/// No colour and the ASCII logo.
	pub plain: bool,
	pub no_logo: bool,
	pub json: bool,
	pub logo: LogoChoice,
	pub colour: ColourMode,
}

impl Default for Options {
	fn default() -> Self {
		Options {
			action: Action::Run,
			plain: false,
			no_logo: false,
			json: false,
			logo: LogoChoice::Auto,
			colour: ColourMode::Auto,
		}
	}
}

/// What was wrong with an argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageKind {
	UnknownOption,
	/// An option that needs a value was last, or its value was empty.
	MissingValue,
	/// A value that is not a logo name or colour mode, or a value given to an option that takes none.
	BadValue,
}

/// A usage error, naming the argument (counting from the first after the program name, which is 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageError {
	pub kind: UsageKind,
	pub index: usize,
}

/// Which option an argument names, and the value written with it (`--logo=win32`, `/LOGO:win32`).
struct Named<'a> {
	flag: Flag,
	value: Option<&'a [u8]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flag {
	Plain,
	NoLogo,
	Json,
	Mono,
	Help,
	Version,
	Logo,
	Colour,
}

fn name_of(argument: &[u8]) -> Option<Named<'_>> {
	if let Some(rest) = argument.strip_prefix(b"--") {
		let (name, value) = match rest.iter().position(|&byte| byte == b'=') {
			Some(at) => (&rest[..at], Some(&rest[at + 1..])),
			None => (rest, None),
		};
		let flag = match name {
			b"plain" => Flag::Plain,
			b"no-logo" => Flag::NoLogo,
			b"json" => Flag::Json,
			b"help" => Flag::Help,
			b"version" => Flag::Version,
			b"logo" => Flag::Logo,
			b"colour" | b"color" => Flag::Colour,
			_ => return None,
		};
		return Some(Named { flag, value });
	}
	match argument {
		b"-h" => {
			return Some(Named {
				flag: Flag::Help,
				value: None,
			});
		}
		b"-V" => {
			return Some(Named {
				flag: Flag::Version,
				value: None,
			});
		}
		_ => {}
	}
	let rest = argument.strip_prefix(b"/")?;
	let (name, value) = match rest.iter().position(|&byte| byte == b':') {
		Some(at) => (&rest[..at], Some(&rest[at + 1..])),
		None => (rest, None),
	};
	let is = |wanted: &[u8]| name.eq_ignore_ascii_case(wanted);
	let flag = if is(b"?") || is(b"help") {
		Flag::Help
	} else if is(b"plain") {
		Flag::Plain
	} else if is(b"nologo") {
		Flag::NoLogo
	} else if is(b"json") {
		Flag::Json
	} else if is(b"mono") {
		Flag::Mono
	} else if is(b"version") {
		Flag::Version
	} else if is(b"logo") {
		Flag::Logo
	} else if is(b"colour") || is(b"color") {
		Flag::Colour
	} else {
		return None;
	};
	Some(Named { flag, value })
}

/// Parses the arguments after the program name.
pub fn parse<'a>(argv: impl Iterator<Item = &'a [u8]>) -> Result<Options, UsageError> {
	let mut options = Options::default();
	let mut argv = argv.enumerate();
	while let Some((index, argument)) = argv.next() {
		let error = |kind| UsageError { kind, index };
		let Some(Named { flag, value }) = name_of(argument) else {
			return Err(error(UsageKind::UnknownOption));
		};
		match flag {
			Flag::Plain | Flag::NoLogo | Flag::Json | Flag::Mono | Flag::Help | Flag::Version if value.is_some() => {
				return Err(error(UsageKind::BadValue));
			}
			Flag::Plain => options.plain = true,
			Flag::NoLogo => options.no_logo = true,
			Flag::Json => options.json = true,
			Flag::Mono => options.colour = ColourMode::Mono,
			Flag::Help => options.action = Action::Help,
			Flag::Version => options.action = Action::Version,
			Flag::Logo | Flag::Colour => {
				let value = match value {
					Some(value) => value,
					None => argv.next().map(|(_, next)| next).ok_or(error(UsageKind::MissingValue))?,
				};
				if value.is_empty() {
					return Err(error(UsageKind::MissingValue));
				}
				if flag == Flag::Colour {
					options.colour = ColourMode::parse(value).ok_or(error(UsageKind::BadValue))?;
				} else if value.eq_ignore_ascii_case(b"list") {
					options.action = Action::ListLogos;
				} else if value.eq_ignore_ascii_case(b"auto") {
					options.logo = LogoChoice::Auto;
				} else {
					options.logo = LogoChoice::Named(crate::logo::find(value).ok_or(error(UsageKind::BadValue))?.name);
				}
			}
		}
	}
	Ok(options)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_all(arguments: &[&str]) -> Result<Options, UsageError> {
		parse(arguments.iter().map(|argument| argument.as_bytes()))
	}

	#[test]
	fn no_arguments_run_with_the_defaults() {
		assert_eq!(parse_all(&[]), Ok(Options::default()));
	}

	#[test]
	fn the_simple_flags() {
		let options = parse_all(&["--plain", "--no-logo", "--json"]).unwrap();
		assert!(options.plain && options.no_logo && options.json);
		assert_eq!(options.action, Action::Run);
	}

	#[test]
	fn a_logo_by_name_in_both_spellings() {
		let wanted = LogoChoice::Named("win32");
		assert_eq!(parse_all(&["--logo", "win32"]).unwrap().logo, wanted);
		assert_eq!(parse_all(&["--logo=win32"]).unwrap().logo, wanted);
		assert_eq!(parse_all(&["--logo", "WIN32"]).unwrap().logo, wanted);
		assert_eq!(parse_all(&["--logo", "auto"]).unwrap().logo, LogoChoice::Auto);
	}

	#[test]
	fn logo_list_is_an_action() {
		assert_eq!(parse_all(&["--logo", "list"]).unwrap().action, Action::ListLogos);
	}

	#[test]
	fn every_colour_mode_parses() {
		for (name, mode) in [
			("auto", ColourMode::Auto),
			("never", ColourMode::Never),
			("always", ColourMode::Always),
			("mono", ColourMode::Mono),
			("16", ColourMode::Ansi16),
			("256", ColourMode::Ansi256),
			("truecolour", ColourMode::TrueColour),
			("truecolor", ColourMode::TrueColour),
			("24bit", ColourMode::TrueColour),
		] {
			assert_eq!(parse_all(&["--colour", name]).unwrap().colour, mode, "{name}");
			assert_eq!(parse_all(&["--color", name]).unwrap().colour, mode, "{name}");
			assert_eq!(parse_all(&[&format!("--colour={name}")]).unwrap().colour, mode, "{name}");
		}
	}

	#[test]
	fn help_and_version_in_every_spelling() {
		for argument in ["--help", "-h", "/?", "/HELP"] {
			assert_eq!(parse_all(&[argument]).unwrap().action, Action::Help, "{argument}");
		}
		for argument in ["--version", "-V", "/VERSION"] {
			assert_eq!(parse_all(&[argument]).unwrap().action, Action::Version, "{argument}");
		}
	}

	#[test]
	fn the_slash_forms_match_the_long_ones_in_any_case() {
		let options = parse_all(&["/plain", "/NOLOGO", "/Logo:dos", "/MONO", "/json"]).unwrap();
		assert!(options.plain && options.no_logo && options.json);
		assert_eq!(options.logo, LogoChoice::Named("dos"));
		assert_eq!(options.colour, ColourMode::Mono);
		assert_eq!(parse_all(&["/COLOUR:256"]).unwrap().colour, ColourMode::Ansi256);
	}

	#[test]
	fn later_arguments_win() {
		let options = parse_all(&["--colour", "256", "--colour", "never", "--logo", "dos", "--logo", "linux"]).unwrap();
		assert_eq!(options.colour, ColourMode::Never);
		assert_eq!(options.logo, LogoChoice::Named("linux"));
	}

	#[test]
	fn an_unknown_option_names_its_position() {
		assert_eq!(
			parse_all(&["--plain", "--bogus"]),
			Err(UsageError {
				kind: UsageKind::UnknownOption,
				index: 1
			})
		);
		assert_eq!(parse_all(&["file"]).unwrap_err().kind, UsageKind::UnknownOption);
		assert_eq!(parse_all(&["-x"]).unwrap_err().kind, UsageKind::UnknownOption);
		assert_eq!(parse_all(&["/BOGUS"]).unwrap_err().kind, UsageKind::UnknownOption);
		assert_eq!(parse_all(&["--PLAIN"]).unwrap_err().kind, UsageKind::UnknownOption);
	}

	#[test]
	fn a_missing_value_is_an_error() {
		assert_eq!(
			parse_all(&["--logo"]),
			Err(UsageError {
				kind: UsageKind::MissingValue,
				index: 0
			})
		);
		assert_eq!(parse_all(&["--colour="]).unwrap_err().kind, UsageKind::MissingValue);
		assert_eq!(parse_all(&["/LOGO:"]).unwrap_err().kind, UsageKind::MissingValue);
	}

	#[test]
	fn a_bad_value_is_an_error() {
		assert_eq!(parse_all(&["--logo", "penguin"]).unwrap_err().kind, UsageKind::BadValue);
		assert_eq!(parse_all(&["--colour", "rainbow"]).unwrap_err().kind, UsageKind::BadValue);
		assert_eq!(parse_all(&["--plain=yes"]).unwrap_err().kind, UsageKind::BadValue);
	}

	#[test]
	fn bytes_that_are_not_utf8_do_not_panic() {
		let odd: [&[u8]; 4] = [b"\xff\xfe", b"--logo", b"\x80", b"--colour=\xff"];
		assert!(parse(odd.iter().copied()).is_err());
		assert!(parse([&b"--"[..], b"-", b"/", b""].into_iter()).is_err());
	}
}
