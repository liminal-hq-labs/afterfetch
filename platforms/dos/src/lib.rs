// The parts of the DOS shell that need no DOS to run: splitting the command tail, turning a style into a text-mode attribute, and the text
// of the facts. They live in a library so the host can test them; everything that calls DOS or the BIOS is in the program itself.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

pub mod attribute;
pub mod cpu;
pub mod rows;
pub mod tail;
