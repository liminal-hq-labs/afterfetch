// The parts of the Windows 3.1 shell that need no Windows to run: the cell grid the window draws and the text of each row. They live in a
// library so the host can test them; everything that calls Windows is in the program itself.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(test), no_std)]

pub mod attribute;
pub mod grid;
pub mod rows;
