// Afterfetch for Linux: no libc, no runtime, raw system calls. `rt` is the entry point, `arch` the per-processor system call stubs,
// `os::linux` the system and its facts, and `term` the output.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(raw_linux, no_std)]
#![cfg_attr(raw_linux, no_main)]

#[cfg(raw_linux)]
mod arch;
#[cfg(raw_linux)]
mod os;
#[cfg(raw_linux)]
mod rt;
#[cfg(raw_linux)]
mod term;

#[cfg(not(raw_linux))]
fn main() {}
