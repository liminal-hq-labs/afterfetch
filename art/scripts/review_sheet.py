#!/usr/bin/env python3
# Renders the logo masters for review: each master at 10x, each depth's recolouring, and one contact sheet.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Standard library only, plus rsvg-convert on PATH. Usage:
#
#   python3 art/scripts/review_sheet.py [--out DIR] [LOGO ...]
#
# DIR defaults to "~/Documents/Liminal HQ/Afterfetch/renders". For every logo in art/logos.toml that has a master, it
# writes <logo>-<depth>.png at 10 px per logo pixel (truecolour is the master itself), and afterfetch-logos.png, a sheet
# with every depth beside the ASCII form, each on the background a terminal of that kind would have. A depth is drawn by
# swapping each tone's rgb for the colour that depth shows (art/palette/afterglow-dusk.toml), which is what the program
# does cell by cell. Any fill in a master that is not a tone's rgb is reported and fails the run.
#
# This is a review aid, not the generator: the .afl blobs, palette.rs and the cell-accurate previews come from
# art/scripts/afterfetch_art.py.

import argparse
import html
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ART_DIR = Path(__file__).resolve().parent.parent
DEFAULT_OUT = Path.home() / "Documents" / "Liminal HQ" / "Afterfetch" / "renders"
PIXEL = 10
CELL_WIDTH = 8
CELL_HEIGHT = 16

# Each depth: its label, the terminal background it is shown on, and how a tone's colour is found.
DEPTHS = [
	("truecolour", "Truecolour", "#050507"),
	("xterm256", "256 colours", "#050507"),
	("dusk16", "16, Afterglow Dusk console", "#0a1230"),
	("terminal16", "16, Afterglow terminal", "#050507"),
	("vga16", "16, stock VGA", "#000000"),
	("mono", "Mono", "#000000"),
]


def xterm_colour(index):
	if index >= 232:
		grey = 8 + 10 * (index - 232)
		return "#%02x%02x%02x" % (grey, grey, grey)
	levels = [0, 95, 135, 175, 215, 255]
	value = index - 16
	return "#%02x%02x%02x" % (levels[value // 36], levels[(value // 6) % 6], levels[value % 6])


def depth_colours(palette, depth):
	"""Map each tone's rgb to the colour shown at `depth`, or None where the tone is not drawn."""
	tables = palette["tables"]
	mapping = {}
	for tone in palette["tone"]:
		design = tone["rgb"].lower()
		if depth == "truecolour":
			shown = design
		elif depth == "xterm256":
			shown = xterm_colour(tone["xterm256"])
		elif depth == "dusk16":
			shown = tables["dusk-console"][tone["console16"]] if "console16" in tone else None
		elif depth == "terminal16":
			shown = tables["afterglow-terminal"][tone["ansi16"]] if "ansi16" in tone else None
		elif depth == "vga16":
			shown = tables["stock-vga"][tone["console16"]] if "console16" in tone else None
		elif depth == "mono":
			shown = tables["mono-preview"][tone["mono"]] if "mono" in tone else None
		else:
			raise ValueError(depth)
		mapping[design] = shown
	return mapping


FILL = re.compile(r'fill="(#[0-9a-fA-F]{6})"')


def recolour(svg_text, mapping):
	def swap(match):
		shown = mapping[match.group(1).lower()]
		return 'fill="none"' if shown is None else f'fill="{shown}"'

	return FILL.sub(swap, svg_text)


def master_body(svg_text):
	"""The drawing inside the master's root element, for embedding in the sheet."""
	start = svg_text.index(">", svg_text.index("<svg")) + 1
	end = svg_text.rindex("</svg>")
	body = svg_text[start:end]
	return re.sub(r"<title>.*?</title>", "", body, flags=re.S)


def read_ascii(path):
	lines = path.read_text(encoding="ascii").split("\n")
	if lines and lines[-1] == "":
		lines.pop()
	return [(lines[index], lines[index + 1]) for index in range(0, len(lines) - 1, 2)]


def ascii_group(rows, letters, mapping, left, top):
	"""The ASCII form as a terminal shows it: a '#' is a filled cell, every other character is text in its tone."""
	parts = [f'<g transform="translate({left} {top})">']
	for row_number, (art, tones) in enumerate(rows):
		baseline = row_number * CELL_HEIGHT + 12
		for column, (character, letter) in enumerate(zip(art, tones)):
			if character == " ":
				continue
			shown = mapping.get(letters.get(letter, ""), "#e9e2d8")
			if shown is None:
				continue
			x = column * CELL_WIDTH
			if character == "#":
				parts.append(f'<rect x="{x}" y="{row_number * CELL_HEIGHT}" width="{CELL_WIDTH}" height="{CELL_HEIGHT}" fill="{shown}"/>')
			else:
				parts.append(
					f'<text x="{x}" y="{baseline}" fill="{shown}" font-family="DejaVu Sans Mono, monospace" font-size="13.3">{html.escape(character)}</text>'
				)
	parts.append("</g>")
	return "\n".join(parts)


def render(svg_path, png_path):
	subprocess.run(["rsvg-convert", "--output", str(png_path), str(svg_path)], check=True)


def main():
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
	parser.add_argument("logos", nargs="*")
	arguments = parser.parse_args()

	palette = tomllib.loads((ART_DIR / "palette" / "afterglow-dusk.toml").read_text(encoding="utf-8"))
	manifest = tomllib.loads((ART_DIR / "logos.toml").read_text(encoding="utf-8"))
	letters = {tone["letter"]: tone["rgb"].lower() for tone in palette["tone"]}
	design_colours = set(letters.values())

	chosen = [logo for logo in manifest["logo"] if (ART_DIR / logo["master"]).exists()]
	if arguments.logos:
		chosen = [logo for logo in chosen if logo["name"] in arguments.logos]
	if not chosen:
		sys.exit("review_sheet: no masters to render")

	arguments.out.mkdir(parents=True, exist_ok=True)
	failures = []
	sheet_rows = []
	with tempfile.TemporaryDirectory() as scratch:
		scratch_dir = Path(scratch)
		for logo in chosen:
			name = logo["name"]
			svg_text = (ART_DIR / logo["master"]).read_text(encoding="utf-8")
			for colour in FILL.findall(svg_text):
				if colour.lower() not in design_colours:
					failures.append(f"{logo['master']}: fill {colour} is not a palette tone")
			if failures:
				continue
			for depth, _label, background in DEPTHS:
				mapping = depth_colours(palette, depth)
				variant = recolour(svg_text, mapping)
				variant = variant.replace('<g transform="scale(10)">', f'<rect width="360" height="240" fill="{background}"/>\n<g transform="scale(10)">', 1)
				variant_path = scratch_dir / f"{name}-{depth}.svg"
				variant_path.write_text(variant, encoding="utf-8")
				render(variant_path, arguments.out / f"{name}-{depth}.png")
			sheet_rows.append((logo, svg_text))

		if failures:
			sys.exit("\n".join(failures))

		# The sheet: one band per logo, the six depths at 8 px per pixel (a half-block cell is 8 x 16), then the ASCII
		# form at Dusk 16 and in truecolour, as 8 x 16 cells.
		pixel_size = CELL_WIDTH
		tile_width = 36 * pixel_size
		tile_height = 24 * pixel_size
		gap = 24
		label_height = 28
		ascii_width = max(len(art) for logo, _ in sheet_rows for art, _ in read_ascii(ART_DIR / logo["ascii"])) * CELL_WIDTH
		columns = [(depth, label, background, "pixels") for depth, label, background in DEPTHS]
		columns += [("dusk16", "ASCII form, Dusk 16", "#0a1230", "ascii"), ("truecolour", "ASCII form, truecolour", "#050507", "ascii")]
		column_widths = [tile_width if kind == "pixels" else ascii_width for _, _, _, kind in columns]
		sheet_width = gap + sum(width + gap for width in column_widths)
		band_height = label_height + tile_height + gap + 8
		sheet_height = 64 + band_height * len(sheet_rows)
		parts = [
			f'<svg xmlns="http://www.w3.org/2000/svg" width="{sheet_width}" height="{sheet_height}" viewBox="0 0 {sheet_width} {sheet_height}">',
			f'<rect width="{sheet_width}" height="{sheet_height}" fill="#12121a"/>',
			f'<text x="{gap}" y="40" fill="#e0e0e0" font-family="DejaVu Sans, sans-serif" font-size="22">afterfetch logos: masters at each depth (8 px per logo pixel, one cell is 8 x 16)</text>',
		]
		for band_number, (logo, svg_text) in enumerate(sheet_rows):
			top = 64 + band_number * band_height
			left = gap
			ascii_rows = read_ascii(ART_DIR / logo["ascii"])
			for (depth, label, background, kind), width in zip(columns, column_widths):
				mapping = depth_colours(palette, depth)
				parts.append(f'<text x="{left}" y="{top + 18}" fill="#b4bfce" font-family="DejaVu Sans, sans-serif" font-size="14">{html.escape(logo["name"])} · {html.escape(label)}</text>')
				parts.append(f'<rect x="{left}" y="{top + label_height}" width="{width}" height="{tile_height}" fill="{background}"/>')
				if kind == "pixels":
					body = recolour(master_body(svg_text), mapping).replace("scale(10)", f"scale({pixel_size})")
					parts.append(f'<g transform="translate({left} {top + label_height})">{body}</g>')
				else:
					parts.append(ascii_group(ascii_rows, letters, mapping, left, top + label_height))
				left += width + gap
		parts.append("</svg>")
		sheet_path = scratch_dir / "afterfetch-logos.svg"
		sheet_path.write_text("\n".join(parts), encoding="utf-8")
		render(sheet_path, arguments.out / "afterfetch-logos.png")
	print(f"review_sheet: wrote {len(sheet_rows)} logos to {arguments.out}")


if __name__ == "__main__":
	main()
