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
# writes <logo>-<depth>.png at 10 px per logo pixel (truecolour is the master itself); afterfetch-logos.png, a sheet
# with every depth beside the ASCII form, each on the background a terminal of that kind would have; and
# afterfetch-family.png, the logos side by side in the order of the day (docs/art.md), one row per depth. A depth is
# drawn by swapping each tone's rgb for the colour that depth shows (art/palette/afterglow-dusk.toml), which is what the
# program does cell by cell.
#
# It also checks each logo the way the generator will, and fails the run on any breach: every fill is a tone in the
# logo's `tones` list; rsvg-convert at 36 x 24 gives only those tones and transparent; the ASCII form fits its size, is
# ASCII, has a tone from the list on every drawn character and none on a space; and the estimated .afl blob (the layout
# in docs/logo-format.md, with no PIXELS_LOW) is within the per-logo and total budgets. It prints each estimate.
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
import struct
import zlib
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


# The logos in the order of the day they stand for (docs/art.md, "A day around the family").
DAY_ORDER = ["dos", "win16", "macos", "afterglow", "win32", "freebsd", "linux", "desk"]
FAMILY_DEPTHS = ["truecolour", "dusk16", "vga16", "mono"]


def read_png(path):
	"""Decode an 8-bit RGBA, non-interlaced PNG (what rsvg-convert writes) into rows of (r, g, b, a) tuples."""
	data = path.read_bytes()
	position = 8
	compressed = b""
	width = height = 0
	while position < len(data):
		length, kind = struct.unpack(">I4s", data[position : position + 8])
		chunk = data[position + 8 : position + 8 + length]
		if kind == b"IHDR":
			width, height, bit_depth, colour_type, _, _, interlace = struct.unpack(">IIBBBBB", chunk)
			if (bit_depth, colour_type, interlace) != (8, 6, 0):
				raise ValueError(f"{path}: expected 8-bit RGBA without interlace")
		elif kind == b"IDAT":
			compressed += chunk
		position += 12 + length
	raw = zlib.decompress(compressed)
	stride = width * 4
	rows = []
	previous = bytearray(stride)
	offset = 0
	for _ in range(height):
		filter_type = raw[offset]
		line = bytearray(raw[offset + 1 : offset + 1 + stride])
		offset += 1 + stride
		for index in range(stride):
			left = line[index - 4] if index >= 4 else 0
			above = previous[index]
			upper_left = previous[index - 4] if index >= 4 else 0
			if filter_type == 1:
				line[index] = (line[index] + left) & 0xFF
			elif filter_type == 2:
				line[index] = (line[index] + above) & 0xFF
			elif filter_type == 3:
				line[index] = (line[index] + (left + above) // 2) & 0xFF
			elif filter_type == 4:
				estimate = left + above - upper_left
				distances = (abs(estimate - left), abs(estimate - above), abs(estimate - upper_left))
				predictor = (left, above, upper_left)[distances.index(min(distances))]
				line[index] = (line[index] + predictor) & 0xFF
		rows.append([tuple(line[index : index + 4]) for index in range(0, stride, 4)])
		previous = line
	return rows


def run_bytes(indexes):
	"""How many run bytes a row of palette indexes takes: one per run of up to 16 equal indexes."""
	count = 0
	position = 0
	while position < len(indexes):
		run = 1
		while position + run < len(indexes) and indexes[position + run] == indexes[position] and run < 16:
			run += 1
		count += 1
		position += run
	return count


def encoded_art_bytes(art):
	"""Bytes of one ASCII row's art: a character, then a repeat byte for up to 128 more of the same."""
	count = 0
	position = 0
	while position < len(art):
		run = 1
		while position + run < len(art) and art[position + run] == art[position] and run < 129:
			run += 1
		count += 1 if run == 1 else 2
		position += run
	return count


def estimate_blob(pixel_rows, ascii_rows, tone_count):
	"""The .afl size for a logo with no PIXELS_LOW: header, PIXELS section and ASCII section."""
	header = 9 + tone_count
	pixels = 3 + sum(run_bytes(row) for row in pixel_rows)
	ascii_section = 3
	for indexes in ascii_rows:
		ascii_section += 1 + encoded_art_bytes(indexes[0]) + run_bytes(indexes[1])
	return header + pixels + ascii_section


def check_logo(logo, svg_text, palette, scratch_dir, limits):
	"""Every check the generator will make of a master and its ASCII form; returns (failures, estimated blob bytes)."""
	failures = []
	name = logo["name"]
	tones_by_name = {tone["name"]: tone for tone in palette["tone"]}
	logo_tones = logo.get("tones", [])
	unknown = [tone_name for tone_name in logo_tones if tone_name not in tones_by_name]
	if unknown or not logo_tones:
		return [f"{name}: tones list missing or names unknown tones {unknown}"], 0
	rgb_to_index = {}
	for index, tone_name in enumerate(logo_tones, start=1):
		rgb_to_index[tones_by_name[tone_name]["rgb"].lower()] = index
	for colour in FILL.findall(svg_text):
		if colour.lower() not in rgb_to_index:
			failures.append(f"{logo['master']}: fill {colour} is not a tone in the logo's tones list")

	png_path = scratch_dir / f"{name}-grid.png"
	subprocess.run(
		["rsvg-convert", "--width", str(logo["width"]), "--height", str(logo["height"]), "--background-color", "none", "--output", str(png_path), str(ART_DIR / logo["master"])],
		check=True,
	)
	pixel_rows = []
	for y, row in enumerate(read_png(png_path)):
		indexes = []
		for x, (red, green, blue, alpha) in enumerate(row):
			if alpha == 0:
				indexes.append(0)
				continue
			colour = "#%02x%02x%02x" % (red, green, blue)
			if alpha != 255 or colour not in rgb_to_index:
				failures.append(f"{name}: pixel ({x}, {y}) is {colour} at alpha {alpha}, not a tone")
				indexes.append(0)
			else:
				indexes.append(rgb_to_index[colour])
		pixel_rows.append(indexes)

	letter_to_index = {tones_by_name[tone_name]["letter"]: index for index, tone_name in enumerate(logo_tones, start=1)}
	ascii_rows = read_ascii(ART_DIR / logo["ascii"])
	ascii_limit = logo.get("ascii-width", limits["max-ascii-width"])
	if len(ascii_rows) != logo["ascii-rows"]:
		failures.append(f"{logo['ascii']}: {len(ascii_rows)} rows, the manifest says {logo['ascii-rows']}")
	encoded = []
	for row_number, (art, tones) in enumerate(ascii_rows):
		where = f"{logo['ascii']} row {row_number}"
		if len(art) > ascii_limit or (ascii_limit > limits["max-ascii-width"] and "ascii-width-exception" not in logo):
			failures.append(f"{where}: wider than {ascii_limit}")
		if not art.isascii() or any(not (" " <= character <= "~") for character in art):
			failures.append(f"{where}: not printable ASCII")
		if len(tones) != len(art):
			failures.append(f"{where}: tone line is {len(tones)} long, art is {len(art)}")
			continue
		indexes = []
		for column, (character, letter) in enumerate(zip(art, tones)):
			if character == " " and letter != " ":
				failures.append(f"{where} column {column}: a space with a tone")
			elif character != " " and letter not in letter_to_index:
				failures.append(f"{where} column {column}: tone {letter!r} is not in the logo's tones list")
			indexes.append(letter_to_index.get(letter, 0))
		encoded.append((art, indexes))
	return failures, estimate_blob(pixel_rows, encoded, len(logo_tones))


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
	limits = manifest["limits"]
	failures = []
	sheet_rows = []
	estimates = {}
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
			logo_failures, estimates[name] = check_logo(logo, svg_text, palette, scratch_dir, limits)
			failures += logo_failures
			if estimates[name] > limits["blob-bytes"]:
				failures.append(f"{name}: estimated blob {estimates[name]} bytes is over {limits['blob-bytes']}")
			for depth, _label, background in DEPTHS:
				mapping = depth_colours(palette, depth)
				variant = recolour(svg_text, mapping)
				variant = variant.replace('<g transform="scale(10)">', f'<rect width="360" height="240" fill="{background}"/>\n<g transform="scale(10)">', 1)
				variant_path = scratch_dir / f"{name}-{depth}.svg"
				variant_path.write_text(variant, encoding="utf-8")
				render(variant_path, arguments.out / f"{name}-{depth}.png")
			sheet_rows.append((logo, svg_text))

		total = sum(estimates.values())
		for name, size in estimates.items():
			print(f"review_sheet: {name:<10} about {size} bytes as .afl")
		print(f"review_sheet: all       about {total} bytes (budget {limits['total-bytes']})")
		if total > limits["total-bytes"]:
			failures.append(f"estimated blobs total {total} bytes, over {limits['total-bytes']}")
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
		family_path = scratch_dir / "afterfetch-family.svg"
		family_path.write_text(family_sheet(sheet_rows, palette, letters), encoding="utf-8")
		render(family_path, arguments.out / "afterfetch-family.png")
	print(f"review_sheet: wrote {len(sheet_rows)} logos to {arguments.out}")


def family_sheet(sheet_rows, palette, letters):
	"""The logos side by side in the order of the day, one row per depth, then their ASCII forms at Dusk 16."""
	order = {name: position for position, name in enumerate(DAY_ORDER)}
	logos = sorted(sheet_rows, key=lambda entry: order.get(entry[0]["name"], len(DAY_ORDER)))
	pixel_size = CELL_WIDTH
	tile_width = 40 * pixel_size
	tile_height = 24 * pixel_size
	gap = 16
	label_width = 220
	header_height = 64
	name_height = 28
	sheet_width = label_width + len(logos) * (tile_width + gap) + gap
	rows = [(depth, label, background, "pixels") for depth, label, background in DEPTHS if depth in FAMILY_DEPTHS]
	rows.append(("dusk16", "ASCII form, Dusk 16", "#0a1230", "ascii"))
	sheet_height = header_height + name_height + len(rows) * (tile_height + gap) + gap
	parts = [
		f'<svg xmlns="http://www.w3.org/2000/svg" width="{sheet_width}" height="{sheet_height}" viewBox="0 0 {sheet_width} {sheet_height}">',
		f'<rect width="{sheet_width}" height="{sheet_height}" fill="#12121a"/>',
		f'<text x="{gap}" y="40" fill="#e0e0e0" font-family="DejaVu Sans, sans-serif" font-size="22">afterfetch: one sun, eight horizons, from dawn to after dark</text>',
	]
	for column, (logo, _svg_text) in enumerate(logos):
		left = label_width + column * (tile_width + gap)
		parts.append(f'<text x="{left}" y="{header_height + 18}" fill="#e0e0e0" font-family="DejaVu Sans, sans-serif" font-size="16">{html.escape(logo["name"])}</text>')
	for row_number, (depth, label, background, kind) in enumerate(rows):
		top = header_height + name_height + row_number * (tile_height + gap)
		parts.append(f'<text x="{gap}" y="{top + tile_height // 2}" fill="#b4bfce" font-family="DejaVu Sans, sans-serif" font-size="14">{html.escape(label)}</text>')
		mapping = depth_colours(palette, depth)
		for column, (logo, svg_text) in enumerate(logos):
			left = label_width + column * (tile_width + gap)
			parts.append(f'<rect x="{left}" y="{top}" width="{tile_width}" height="{tile_height}" fill="{background}"/>')
			if kind == "pixels":
				body = recolour(master_body(svg_text), mapping).replace("scale(10)", f"scale({pixel_size})")
				parts.append(f'<g transform="translate({left + 2 * pixel_size} {top})">{body}</g>')
			else:
				parts.append(ascii_group(read_ascii(ART_DIR / logo["ascii"]), letters, mapping, left, top))
	parts.append("</svg>")
	return "\n".join(parts)


if __name__ == "__main__":
	main()
