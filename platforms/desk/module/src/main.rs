// AFETCH.DMD: an Afterglow sunset and the one fact a Desk module can honestly report, printed through `desk_log`.
//
// A Desk module is `no_std` and `no_main` with the entry points of the loader's ABI: `app_start` runs the program and returns 0. It has no
// command line, so the core always runs as `--plain` with the `desk` logo, and no hardware access, so the facts are what the build knows
// until Desk offers `desk_sys_info` (AF-D26). The panic handler is `desk-guest`'s, which reports through `desk_panic`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

use desk_guest::{desk_app, log};

/// Sends one line to the core's log.
fn send(line: &[u8]) {
	log(core::str::from_utf8(line).unwrap_or(""));
}

fn start(_app: u32, _args: &[u8]) -> i32 {
	i32::from(afterfetch_desk::run(send))
}

desk_app!(start = start);
