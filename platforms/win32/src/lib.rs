// The parts of the Win32 shell that need no Windows to run: splitting the command line and turning a style into a console attribute.
// They live in a library so the host can test them; everything that calls Windows is in the program itself.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

pub mod attribute;
pub mod cmdline;
