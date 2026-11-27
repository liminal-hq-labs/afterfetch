// The portable core of afterfetch: no platform knowledge, no allocation, no panics on any input.
//
// A shell for a platform is an entry point, its system calls, and implementations of `Platform` and `Terminal` that it hands to `run`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

pub mod args;
pub mod depth;
pub mod facts;
pub mod format;
pub mod json;
pub mod layout;
pub mod logo;
pub mod palette;
pub mod parse;
pub mod run;
pub mod sink;

pub use afterfetch_art::default_logo;
pub use args::{Action, ColourMode, LogoChoice, Options, UsageError};
pub use depth::{Depth, DepthInputs};
pub use facts::{FactKey, Facts};
pub use format::Buf;
pub use logo::{Blocks, Drawing, Logo, LogoError};
pub use palette::{Tone, Weight};
pub use run::{NAME, Platform, TerminalInfo, VERSION, run};
pub use sink::{AnsiTerminal, ByteSink, Paint, Style, Terminal};
