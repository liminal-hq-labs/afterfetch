// The runtime for a 32-bit DOS program that runs under a DPMI host (CWSDPMI) behind a go32-style stub: no C library, no crt0, no heap.
// Copied from liminal-installer's `dos/rt`, itself from Desk, and trimmed (see PROVENANCE.md).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// One concern per module:
//
// - `start`   the entry point `_start`, the stack and the stub's information block
// - `dpmi`    a client for the DPMI host: INT 31h, the first megabyte through a flat data segment, real-mode calls
// - `dos`     INT 21h through a transfer buffer in conventional memory, and the exit call
// - `bios`    software interrupts that take only registers (INT 10h, 12h, 15h, 21h, 2Fh)
// - `env`     the command tail and the environment, read from the PSP
// - `panic`   the panic handler: exit with status 70

pub mod bios;
pub mod dos;
pub mod dpmi;
pub mod env;
mod panic;
pub mod start;
