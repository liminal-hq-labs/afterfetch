// The contact sheet: one tile for each of the eight logos, each tile the full output on that logo's own system with a caption saying
// whether a real binary exists for it, in a grid of two columns.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::colour::{Rgb, Theme};
use crate::grid::Grid;
use crate::image::{CELL_HEIGHT, CELL_WIDTH, Canvas};
use crate::render::{Request, render_grid};

const MARGIN: usize = 24;
const GAP: usize = 24;
const PADDING: usize = 12;
const CAPTION: usize = CELL_HEIGHT + 12;
const PAGE: Rgb = [0x0b, 0x0d, 0x12];
const BAR: Rgb = [0x1d, 0x21, 0x2b];
const FRAME: Rgb = [0x2a, 0x2f, 0x3b];
const TITLE: Rgb = [0xe6, 0xe9, 0xf0];
const NOTE: Rgb = [0x8f, 0x98, 0xaa];

/// What a request's caption says about the build.
pub fn build_note(request: &Request) -> String {
	match request.system.real_target(request.arch) {
		Some(target) => format!("real build: {target}"),
		None => "preview only".to_string(),
	}
}

/// The title of a tile: the system and its architecture.
pub fn tile_title(request: &Request) -> String {
	format!("{} {}", request.system.name, request.arch)
}

/// A contact sheet of `requests`, under a heading, `per_row` tiles to a row.
pub fn contact_sheet(heading: &str, requests: &[Request], per_row: usize) -> Canvas {
	let grids: Vec<Grid> = requests.iter().map(render_grid).collect();
	let columns = requests.iter().map(|request| request.columns).max().unwrap_or(80);
	let grid_rows = grids.iter().map(Grid::height).max().unwrap_or(0);
	let tile_width = columns * CELL_WIDTH + 2 * PADDING;
	let tile_height = CAPTION + grid_rows * CELL_HEIGHT + 2 * PADDING;
	let per_row = per_row.max(1);
	let tile_rows = requests.len().div_ceil(per_row);
	let across = per_row.min(requests.len().max(1));
	let heading_height = CELL_HEIGHT + GAP;
	let width = 2 * MARGIN + across * tile_width + (across - 1) * GAP;
	let height = 2 * MARGIN + heading_height + tile_rows * tile_height + tile_rows.saturating_sub(1) * GAP;
	let mut canvas = Canvas::new(width, height, PAGE);
	canvas.draw_text(MARGIN, MARGIN, heading, TITLE, PAGE);
	for (position, (request, grid)) in requests.iter().zip(&grids).enumerate() {
		let x = MARGIN + (position % per_row) * (tile_width + GAP);
		let y = MARGIN + heading_height + (position / per_row) * (tile_height + GAP);
		let theme = Theme::for_depth(grid.depth);
		canvas.fill_rect(x, y, tile_width, tile_height, FRAME);
		canvas.fill_rect(x + 1, y + 1, tile_width - 2, tile_height - 2, theme.background);
		canvas.fill_rect(x + 1, y + 1, tile_width - 2, CAPTION - 1, BAR);
		let title = tile_title(request);
		canvas.draw_text(x + PADDING, y + 6, &title, TITLE, BAR);
		let note = build_note(request);
		let note_x = (x + tile_width).saturating_sub(PADDING + note.len() * CELL_WIDTH);
		canvas.draw_text(
			note_x.max(x + PADDING + (title.len() + 2) * CELL_WIDTH),
			y + 6,
			&note,
			NOTE,
			BAR,
		);
		canvas.draw_grid(x + PADDING, y + CAPTION + PADDING, grid, columns, theme);
	}
	canvas
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::systems::{SHEET_SYSTEMS, System};

	fn sheet_requests(columns: usize) -> Vec<Request> {
		SHEET_SYSTEMS
			.iter()
			.map(|name| Request {
				columns,
				..Request::new(System::find(name).unwrap())
			})
			.collect()
	}

	#[test]
	fn the_sheet_shows_each_of_the_eight_logos_once() {
		let mut shown: Vec<&str> = sheet_requests(80)
			.iter()
			.map(|request| afterfetch_core::default_logo(request.system.platform))
			.collect();
		shown.sort_unstable();
		let mut all: Vec<&str> = afterfetch_art::LOGOS.iter().map(|logo| logo.name).collect();
		all.sort_unstable();
		assert_eq!(shown, all);
	}

	#[test]
	fn a_sheet_is_the_same_every_time_and_has_the_tile_grid_size() {
		let requests = sheet_requests(80);
		let first = contact_sheet("afterfetch", &requests, 2);
		assert_eq!(first, contact_sheet("afterfetch", &requests, 2));
		assert!(first.width > 2 * 80 * CELL_WIDTH);
		assert_eq!(first.to_png(), contact_sheet("afterfetch", &requests, 2).to_png());
	}

	#[test]
	fn notes_say_which_builds_are_real() {
		let linux = Request::new(System::find("linux").unwrap());
		assert_eq!(build_note(&linux), "real build: linux-x86_64");
		let macos = Request::new(System::find("macos").unwrap());
		assert_eq!(build_note(&macos), "preview only");
	}
}
