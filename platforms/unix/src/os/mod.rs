// The operating systems this shell knows: Linux and FreeBSD, each a raw-syscall program. macOS calls libSystem, which has no stable system call interface.
// `system` is the one this build is for, so the runtime and the terminal do not name an operating system.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(raw_freebsd)]
pub mod freebsd;
#[cfg(raw_linux)]
pub mod linux;
#[cfg(raw_macos)]
pub mod macos;

#[cfg(raw_freebsd)]
pub use freebsd as system;
#[cfg(raw_linux)]
pub use linux as system;
#[cfg(raw_macos)]
pub use macos as system;
