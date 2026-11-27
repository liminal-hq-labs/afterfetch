// The `afterfetch-preview` program: see `cli` for the options.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use afterfetch_preview::cli::{self, Format};

fn main() -> ExitCode {
	let arguments: Vec<String> = std::env::args().skip(1).collect();
	let outcome = cli::parse(&arguments).and_then(|parsed| cli::execute(&parsed).map(|output| (parsed, output)));
	let (parsed, output) = match outcome {
		Ok(done) => done,
		Err(message) => {
			eprintln!("afterfetch-preview: {message}");
			return ExitCode::from(2);
		}
	};
	let binary = parsed.sheet.is_some() || parsed.format == Format::Png;
	let written = match &output.file {
		Some(path) => std::fs::write(path, &output.bytes),
		None if binary && std::io::stdout().is_terminal() => {
			eprintln!("afterfetch-preview: a PNG goes to a file: add --out <file> (or --sheet <file>)");
			return ExitCode::from(2);
		}
		None => std::io::stdout().write_all(&output.bytes),
	};
	match written {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("afterfetch-preview: {error}");
			ExitCode::from(74)
		}
	}
}
