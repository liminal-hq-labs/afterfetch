// Colour depths, and the order in which a depth is chosen from the arguments, the environment and what the terminal can do.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::ColourMode;

/// How much colour the output can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
	/// No colour and no escape sequences: a pipe, a file or `--plain`.
	None,
	/// Bold and dim only.
	Mono,
	/// 16 colours as ANSI SGR codes (red is 1, blue is 4).
	Ansi16,
	/// 256 colours as `38;5;n` and `48;5;n`.
	Ansi256,
	/// 24-bit colour as `38;2;r;g;b`.
	TrueColour,
	/// 16 colours as console attributes (blue is 1, red is 4): Windows consoles and DOS.
	Console16,
}

impl Depth {
	/// True when tones are drawn as colours (a solid sun, swatches), as opposed to plain or by weight alone.
	pub const fn has_colour(self) -> bool {
		matches!(self, Depth::Ansi16 | Depth::Ansi256 | Depth::TrueColour | Depth::Console16)
	}
}

/// What `resolve` needs to know. A shell fills it from its environment and its terminal.
#[derive(Clone, Copy, Debug)]
pub struct DepthInputs {
	pub mode: ColourMode,
	/// `--plain` was given.
	pub plain: bool,
	/// `NO_COLOR` is set and not empty.
	pub no_color: bool,
	/// `CLICOLOR_FORCE` or `FORCE_COLOR` is set.
	pub force: bool,
	/// Standard output is a terminal.
	pub is_tty: bool,
	/// The depth the platform detects for itself: from `COLORTERM` and `TERM` on Unix, `Console16` on a Windows console or DOS colour
	/// mode, `None` when it cannot show colour.
	pub hint: Depth,
}

/// The depth to draw at, decided once at start:
///
/// 1. `--plain` or `--colour never` give `None`.
/// 2. An explicit depth (`mono`, `16`, `256`, `truecolour`) is used as asked, on a pipe too. `16` becomes `Console16` where the
///    platform's hint is `Console16`.
/// 3. `--colour always` takes the detected depth even on a pipe, and `Ansi16` when nothing is detected.
/// 4. Otherwise `NO_COLOR` gives `None`, then a pipe gives `None` unless forced, then the detected depth.
pub const fn resolve(inputs: DepthInputs) -> Depth {
	if inputs.plain {
		return Depth::None;
	}
	match inputs.mode {
		ColourMode::Never => Depth::None,
		ColourMode::Mono => Depth::Mono,
		ColourMode::Ansi16 => {
			if matches!(inputs.hint, Depth::Console16) {
				Depth::Console16
			} else {
				Depth::Ansi16
			}
		}
		ColourMode::Ansi256 => Depth::Ansi256,
		ColourMode::TrueColour => Depth::TrueColour,
		ColourMode::Always => match inputs.hint {
			Depth::None => Depth::Ansi16,
			hint => hint,
		},
		ColourMode::Auto => {
			if inputs.no_color || (!inputs.is_tty && !inputs.force) {
				Depth::None
			} else {
				inputs.hint
			}
		}
	}
}

/// The Unix detection: `COLORTERM` of `truecolor` or `24bit` gives true colour, `TERM=dumb` gives none, a `TERM` ending `-256color`
/// gives 256 colours, and anything else (the Linux console included) 16.
pub fn unix_hint(colorterm: Option<&[u8]>, term: Option<&[u8]>) -> Depth {
	if colorterm.is_some_and(|value| value.eq_ignore_ascii_case(b"truecolor") || value.eq_ignore_ascii_case(b"24bit")) {
		return Depth::TrueColour;
	}
	match term {
		Some(b"dumb") => Depth::None,
		Some(name) if name.ends_with(b"-256color") => Depth::Ansi256,
		_ => Depth::Ansi16,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn inputs(mode: ColourMode, hint: Depth) -> DepthInputs {
		DepthInputs {
			mode,
			plain: false,
			no_color: false,
			force: false,
			is_tty: true,
			hint,
		}
	}

	#[test]
	fn auto_uses_the_detected_depth_on_a_terminal() {
		assert_eq!(resolve(inputs(ColourMode::Auto, Depth::Ansi256)), Depth::Ansi256);
		assert_eq!(resolve(inputs(ColourMode::Auto, Depth::Console16)), Depth::Console16);
	}

	#[test]
	fn a_pipe_gets_no_colour_unless_forced() {
		let pipe = DepthInputs {
			is_tty: false,
			..inputs(ColourMode::Auto, Depth::TrueColour)
		};
		assert_eq!(resolve(pipe), Depth::None);
		assert_eq!(resolve(DepthInputs { force: true, ..pipe }), Depth::TrueColour);
		assert_eq!(
			resolve(DepthInputs {
				mode: ColourMode::Always,
				..pipe
			}),
			Depth::TrueColour
		);
	}

	#[test]
	fn no_color_beats_auto_but_not_an_explicit_choice() {
		let quiet = DepthInputs {
			no_color: true,
			..inputs(ColourMode::Auto, Depth::TrueColour)
		};
		assert_eq!(resolve(quiet), Depth::None);
		assert_eq!(
			resolve(DepthInputs {
				mode: ColourMode::Always,
				..quiet
			}),
			Depth::TrueColour
		);
		assert_eq!(
			resolve(DepthInputs {
				mode: ColourMode::Ansi256,
				..quiet
			}),
			Depth::Ansi256
		);
	}

	#[test]
	fn plain_and_never_always_win() {
		let hinted = inputs(ColourMode::TrueColour, Depth::TrueColour);
		assert_eq!(resolve(DepthInputs { plain: true, ..hinted }), Depth::None);
		assert_eq!(
			resolve(DepthInputs {
				mode: ColourMode::Never,
				..hinted
			}),
			Depth::None
		);
	}

	#[test]
	fn explicit_depths_are_taken_on_a_pipe() {
		let pipe = |mode| DepthInputs {
			is_tty: false,
			..inputs(mode, Depth::None)
		};
		assert_eq!(resolve(pipe(ColourMode::Mono)), Depth::Mono);
		assert_eq!(resolve(pipe(ColourMode::Ansi16)), Depth::Ansi16);
		assert_eq!(resolve(pipe(ColourMode::Ansi256)), Depth::Ansi256);
		assert_eq!(resolve(pipe(ColourMode::TrueColour)), Depth::TrueColour);
	}

	#[test]
	fn sixteen_is_console_sixteen_on_a_console_platform() {
		assert_eq!(resolve(inputs(ColourMode::Ansi16, Depth::Console16)), Depth::Console16);
		assert_eq!(resolve(inputs(ColourMode::Ansi16, Depth::TrueColour)), Depth::Ansi16);
	}

	#[test]
	fn always_falls_back_to_sixteen_colours() {
		assert_eq!(resolve(inputs(ColourMode::Always, Depth::None)), Depth::Ansi16);
	}

	#[test]
	fn unix_detection_reads_colorterm_then_term() {
		assert_eq!(unix_hint(Some(b"truecolor"), Some(b"xterm")), Depth::TrueColour);
		assert_eq!(unix_hint(Some(b"24bit"), None), Depth::TrueColour);
		assert_eq!(unix_hint(None, Some(b"xterm-256color")), Depth::Ansi256);
		assert_eq!(unix_hint(None, Some(b"dumb")), Depth::None);
		assert_eq!(unix_hint(None, Some(b"linux")), Depth::Ansi16);
		assert_eq!(unix_hint(None, None), Depth::Ansi16);
	}

	#[test]
	fn only_colour_depths_have_colour() {
		assert!(!Depth::None.has_colour());
		assert!(!Depth::Mono.has_colour());
		assert!(Depth::Console16.has_colour());
		assert!(Depth::TrueColour.has_colour());
	}
}
