// AFETCH.EXE: an Afterglow sunset and a few facts about this system, in the screen's own colours.
//
// A 32-bit protected-mode DOS program for a 386 and later, `no_std` and `no_main`, packed behind CWSDPMI's stub so it carries its own DPMI
// host. The colours are text-mode attributes written through the BIOS; the facts come from INT 21h, the BIOS and DPMI. Plain text when the
// output is a file or a pipe.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

mod os;
mod processor;
mod rt;
mod term;

/// Called by the runtime once DPMI is up. Returns the exit status.
fn run_program() -> u8 {
	os::main()
}
