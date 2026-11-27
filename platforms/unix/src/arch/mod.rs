// Per-processor pieces: the entry stub, the system call instruction, and on Linux the system call numbers and the terminal `ioctl`
// requests. FreeBSD has the same numbers on every processor, so its files hold the stub and the call only.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(all(raw_linux, target_arch = "aarch64"))]
mod aarch64;
#[cfg(all(raw_linux, target_arch = "arm"))]
mod arm;
#[cfg(all(raw_linux, target_arch = "mips"))]
mod mips;
#[cfg(all(raw_linux, target_arch = "powerpc64"))]
mod powerpc64;
#[cfg(all(raw_linux, target_arch = "riscv64"))]
mod riscv64;
#[cfg(all(raw_linux, target_arch = "x86"))]
mod x86;
#[cfg(all(raw_linux, target_arch = "x86_64"))]
mod x86_64;

#[cfg(all(raw_linux, target_arch = "aarch64"))]
pub use aarch64::*;
#[cfg(all(raw_linux, target_arch = "arm"))]
pub use arm::*;
#[cfg(all(raw_linux, target_arch = "mips"))]
pub use mips::*;
#[cfg(all(raw_linux, target_arch = "powerpc64"))]
pub use powerpc64::*;
#[cfg(all(raw_linux, target_arch = "riscv64"))]
pub use riscv64::*;
#[cfg(all(raw_linux, target_arch = "x86"))]
pub use x86::*;
#[cfg(all(raw_linux, target_arch = "x86_64"))]
pub use x86_64::*;

#[cfg(all(raw_freebsd, target_arch = "aarch64"))]
mod freebsd_aarch64;
#[cfg(all(raw_freebsd, target_arch = "x86_64"))]
mod freebsd_x86_64;

#[cfg(all(raw_freebsd, target_arch = "aarch64"))]
pub use freebsd_aarch64::*;
#[cfg(all(raw_freebsd, target_arch = "x86_64"))]
pub use freebsd_x86_64::*;
