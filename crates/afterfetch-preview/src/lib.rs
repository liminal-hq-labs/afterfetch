// Afterfetch preview: draws what afterfetch looks like on any supported system, on this machine, from sample facts. It runs the real
// core, so the picture is the program's output, not a mock-up. A host tool: it is never built for a target or shipped.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod cli;
pub mod colour;
pub mod font;
pub mod grid;
pub mod html;
pub mod image;
pub mod render;
pub mod sheet;
pub mod systems;
