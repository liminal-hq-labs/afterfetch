// A self-contained web page with a terminal-like window for each preview, for README screenshots. Half blocks are drawn as two-colour
// boxes so the picture has no gaps whatever font the browser picks.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::Write;

use crate::colour::{Rgb, Theme, style_colours};
use crate::grid::Grid;

/// One window on the page.
pub struct Window {
	/// The window's title, such as `linux x86_64`.
	pub title: String,
	/// A line under the title bar, such as which build this is.
	pub note: String,
	pub grid: Grid,
}

fn hex(colour: Rgb) -> String {
	format!("#{:02x}{:02x}{:02x}", colour[0], colour[1], colour[2])
}

fn escape(text: &str, out: &mut String) {
	for ch in text.chars() {
		match ch {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'"' => out.push_str("&quot;"),
			_ => out.push(ch),
		}
	}
}

/// The HTML for the cells of one grid: a `div` per row, runs of text in one `span` per colour pair, and a box per half block.
fn body(grid: &Grid, theme: Theme) -> String {
	let mut out = String::new();
	for row in 0..grid.height() {
		out.push_str("<div class=\"row\">");
		let mut open: Option<(Rgb, Rgb)> = None;
		for cell in &grid.rows[row] {
			let (foreground, background) = style_colours(cell.style, grid.depth, theme);
			let gradient = match cell.ch {
				'\u{2580}' => Some((foreground, background)),
				'\u{2584}' => Some((background, foreground)),
				'\u{2588}' => Some((foreground, foreground)),
				_ => None,
			};
			if let Some((top, bottom)) = gradient {
				if open.take().is_some() {
					out.push_str("</span>");
				}
				let _ = write!(out, "<i style=\"--t:{};--b:{}\"></i>", hex(top), hex(bottom));
				continue;
			}
			if open != Some((foreground, background)) {
				if open.is_some() {
					out.push_str("</span>");
				}
				let _ = write!(
					out,
					"<span style=\"color:{};background:{}\">",
					hex(foreground),
					hex(background)
				);
				open = Some((foreground, background));
			}
			escape(&cell.ch.to_string(), &mut out);
		}
		if open.is_some() {
			out.push_str("</span>");
		}
		out.push_str("</div>\n");
	}
	out
}

/// A whole page holding `windows`.
pub fn page(title: &str, windows: &[Window]) -> String {
	let mut out = String::new();
	out.push_str(
		"<!doctype html>\n<html lang=\"en-CA\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>",
	);
	escape(title, &mut out);
	out.push_str(
		"</title>\n<style>\n\
:root{color-scheme:dark}\n\
body{margin:0;padding:24px;background:#0b0d12;color:#cfd3dc;font:14px/1.4 system-ui,sans-serif}\n\
h1{font-size:18px;font-weight:600;margin:0 0 20px}\n\
.window{display:inline-block;vertical-align:top;margin:0 20px 20px 0;border-radius:10px;overflow:hidden;box-shadow:0 6px 24px #0008;border:1px solid #2a2f3b}\n\
.bar{display:flex;justify-content:space-between;gap:24px;padding:6px 12px;background:#1d212b;font-size:12px;color:#aab1c0}\n\
.bar b{font-weight:600;color:#e6e9f0}\n\
.screen{padding:10px 12px;font:15px/1.2 ui-monospace,'DejaVu Sans Mono',Menlo,Consolas,monospace}\n\
.row{height:1.2em;white-space:pre}\n\
.row span{display:inline-block;height:1.2em}\n\
i{display:inline-block;width:1ch;height:1.2em;vertical-align:top;background:linear-gradient(var(--t) 50%,var(--b) 50%)}\n\
</style>\n</head>\n<body>\n<h1>",
	);
	escape(title, &mut out);
	out.push_str("</h1>\n");
	for window in windows {
		let theme = Theme::for_depth(window.grid.depth);
		out.push_str("<section class=\"window\"><div class=\"bar\"><b>");
		escape(&window.title, &mut out);
		out.push_str("</b><span>");
		escape(&window.note, &mut out);
		let _ = write!(
			out,
			"</span></div>\n<div class=\"screen\" style=\"background:{};color:{}\">\n",
			hex(theme.background),
			hex(theme.foreground)
		);
		out.push_str(&body(&window.grid, theme));
		out.push_str("</div></section>\n");
	}
	out.push_str("</body>\n</html>\n");
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use afterfetch_core::Terminal;

	#[test]
	fn text_is_escaped_and_half_blocks_become_boxes() {
		let mut grid = Grid::default();
		grid.write("<a & b>\u{2580}".as_bytes());
		grid.end_line();
		let html = page(
			"t & <x>",
			&[Window {
				title: "w".into(),
				note: "n".into(),
				grid,
			}],
		);
		assert!(html.contains("&lt;a &amp; b&gt;"));
		assert!(html.contains("<i style=\"--t:"));
		assert!(html.contains("<title>t &amp; &lt;x&gt;</title>"));
		assert!(html.starts_with("<!doctype html>"));
	}
}
