// Afterfetch for Linux, FreeBSD and macOS: no libc, no runtime, raw system calls. `rt` is the entry point, `arch` the per-processor system call stubs,
// `os::linux`, `os::freebsd` or `os::macos` the system and its facts, and `term` the output.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(raw_unix, no_std)]
#![cfg_attr(raw_unix, no_main)]
// MIPS inline assembly is still experimental; the build uses a nightly-capable compiler for it (AF-D22).
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]

#[cfg(any(raw_linux, raw_freebsd))]
mod arch;
#[cfg(raw_unix)]
mod os;
#[cfg(raw_unix)]
mod rt;
#[cfg(raw_unix)]
mod term;

#[cfg(not(raw_unix))]
fn main() {}
