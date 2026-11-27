// Per-processor pieces: the entry stub, the system call instruction, the system call numbers and the terminal `ioctl` requests.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "arm")]
mod arm;
#[cfg(target_arch = "mips")]
mod mips;
#[cfg(target_arch = "x86")]
mod x86;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
#[cfg(target_arch = "arm")]
pub use arm::*;
#[cfg(target_arch = "mips")]
pub use mips::*;
#[cfg(target_arch = "x86")]
pub use x86::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
