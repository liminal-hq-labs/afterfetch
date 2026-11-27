#!/usr/bin/env python3
# Generates the afterfetch logo blobs, palette tables and logo index from the masters, the palette and the manifest.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Standard library only on the core path, plus `rsvg-convert` on PATH (the one external tool). Usage:
#
#   python3 art/scripts/afterfetch_art.py                  write the generated files into crates/afterfetch-art
#   python3 art/scripts/afterfetch_art.py --check          generate into a temp dir and fail if the committed files differ
#   python3 art/scripts/afterfetch_art.py --preview [DIR]  also write per-depth PNGs and a contact sheet (needs Pillow)
#
# Inputs: art/logos.toml (the manifest), art/palette/afterglow-dusk.toml, art/masters/<logo>.svg (and, when it exists,
# <logo>-low.svg) and art/ascii/<logo>.txt. Outputs: crates/afterfetch-art/data/<logo>.afl, and
# crates/afterfetch-art/src/generated/{palette,index}.rs. docs/logo-format.md is the blob and docs/art.md the design.
#
# A logo is generated when its master exists; a logo whose master does not exist yet (status "brief") is skipped, so the
# generator runs unchanged as masters land. Output is deterministic: no timestamps, manifest order throughout.

import argparse
import filecmp
import re
import struct
import subprocess
import sys
import tempfile
import tomllib
import zlib
from dataclasses import dataclass, field
from pathlib import Path

ART_DIR = Path(__file__).resolve().parent.parent
REPO_DIR = ART_DIR.parent
CRATE_DIR = REPO_DIR / "crates" / "afterfetch-art"
MANIFEST_PATH = ART_DIR / "logos.toml"
PALETTE_PATH = ART_DIR / "palette" / "afterglow-dusk.toml"

MAGIC = b"AFL1"
KIND_PIXELS = 0x01
KIND_PIXELS_LOW = 0x02
KIND_ASCII = 0x03
FORMAT_MAX_WIDTH = 48
FORMAT_MAX_ASCII_ROWS = 24
MAX_RUN = 16
ALPHA_THRESHOLD = 128
MAX_REPORTED_PIXELS = 10

# Where the palette file departs from "nearest match" or "same colour as the neighbour", on purpose. Each is checked, so a
# change to the palette that breaks an exception fails the build instead of passing silently (docs/art.md, Palette).
XTERM256_EXCEPTIONS = {
	# Amber's nearest entry, 215, is orange's: five distinct sun bands matter more than the closest match.
	"amber": 221,
}
CONSOLE16_FOLLOWS = {
	# At 16 colours amber and peach become orange (the sun is gold crown, orange body, coral foot), and night falls back to
	# slate by role because its nearest console colour is the background.
	"amber": "orange",
	"peach": "orange",
	"night": "slate",
}
# Tones with no 16-colour entry, dropped (drawn transparent) at 16 colours and in mono.
DROPPED_AT_16 = {"soot"}
# The console indexes the reactos-liminal afterfetch used (tools/afterfetch/src/palette.rs at d6c72ae); the ReactOS image
# must look exactly as it did, so these never move.
REACTOS_CONSOLE16 = {
	"gold": 14,
	"orange": 6,
	"coral": 12,
	"rose": 4,
	"lilac": 13,
	"violet": 5,
	"periwinkle": 1,
	"slate": 8,
}
MONO_LEVELS = ("bright", "normal", "dim")
XTERM_CUBE_STEPS = (0, 95, 135, 175, 215, 255)


class ArtError(Exception):
	"""A problem with the art, the palette or the manifest, reported without a traceback."""


# ----------------------------------------------------------------------------------------------------------------------
# Colour: sRGB to CIE Lab (D65) and the CIE76 distance


def parse_hex_colour(text):
	match = re.fullmatch(r"#([0-9a-fA-F]{6})", text)
	if match is None:
		raise ArtError(f"not a #rrggbb colour: {text!r}")
	value = int(match.group(1), 16)
	return (value >> 16 & 0xFF, value >> 8 & 0xFF, value & 0xFF)


def hex_colour(rgb):
	return "#%02x%02x%02x" % rgb


def rgb_to_lab(rgb):
	def linear(channel):
		fraction = channel / 255
		return fraction / 12.92 if fraction <= 0.04045 else ((fraction + 0.055) / 1.055) ** 2.4

	red, green, blue = (linear(channel) for channel in rgb)
	x_value = 0.4124564 * red + 0.3575761 * green + 0.1804375 * blue
	y_value = 0.2126729 * red + 0.7151522 * green + 0.0721750 * blue
	z_value = 0.0193339 * red + 0.1191920 * green + 0.9503041 * blue

	def pivot(ratio):
		return ratio ** (1 / 3) if ratio > 216 / 24389 else (24389 / 27 * ratio + 16) / 116

	pivot_x = pivot(x_value / 0.95047)
	pivot_y = pivot(y_value / 1.0)
	pivot_z = pivot(z_value / 1.08883)
	return (116 * pivot_y - 16, 500 * (pivot_x - pivot_y), 200 * (pivot_y - pivot_z))


def lab_distance(first, second):
	return sum((one - other) ** 2 for one, other in zip(first, second)) ** 0.5


def xterm_rgb(index):
	"""The colour of xterm-256 entry 16 to 255 (the cube and the grey ramp)."""
	if index >= 232:
		grey = 8 + 10 * (index - 232)
		return (grey, grey, grey)
	cube = index - 16
	return (
		XTERM_CUBE_STEPS[cube // 36],
		XTERM_CUBE_STEPS[cube // 6 % 6],
		XTERM_CUBE_STEPS[cube % 6],
	)


def swap_bits_0_and_2(value):
	"""Console order to SGR order: red and blue exchange places."""
	return (value & 0b1010) | (value >> 2 & 1) | (value & 1) << 2


# ----------------------------------------------------------------------------------------------------------------------
# The palette and the manifest


@dataclass
class Tone:
	id: int
	name: str
	letter: str
	rgb: tuple
	xterm256: int
	console16: int
	ansi16: int
	mono: str
	delta_e_xterm256: float
	lab: tuple = field(default=(0.0, 0.0, 0.0), compare=False)


@dataclass
class Palette:
	tones: list
	tables: dict
	arch_accent: dict
	mono_preview: dict

	def by_name(self, name):
		for tone in self.tones:
			if tone.name == name:
				return tone
		raise ArtError(f"no tone named {name!r} in {PALETTE_PATH.name}")

	def by_letter(self, letter):
		for tone in self.tones:
			if tone.letter == letter:
				return tone
		return None

	def by_id(self, tone_id):
		for tone in self.tones:
			if tone.id == tone_id:
				return tone
		raise ArtError(f"no tone with id {tone_id} in {PALETTE_PATH.name}")


def load_palette(path=PALETTE_PATH):
	with open(path, "rb") as palette_file:
		data = tomllib.load(palette_file)
	tones = []
	for entry in data["tone"]:
		rgb = parse_hex_colour(entry["rgb"])
		tones.append(
			Tone(
				id=entry["id"],
				name=entry["name"],
				letter=entry["letter"],
				rgb=rgb,
				xterm256=entry["xterm256"],
				console16=entry.get("console16"),
				ansi16=entry.get("ansi16"),
				mono=entry.get("mono"),
				delta_e_xterm256=entry.get("delta-e", {}).get("xterm256"),
				lab=rgb_to_lab(rgb),
			)
		)
	tables = {name: [parse_hex_colour(text) for text in values] for name, values in data["tables"].items() if name != "mono-preview"}
	mono_preview = {level: parse_hex_colour(text) for level, text in data["tables"]["mono-preview"].items()}
	palette = Palette(tones, tables, dict(data["arch-accent"]), mono_preview)
	validate_palette(palette)
	return palette


def nearest_xterm256(lab):
	candidates = range(16, 256)
	return min(candidates, key=lambda index: lab_distance(lab, rgb_to_lab(xterm_rgb(index))))


def validate_palette(palette):
	"""Checks the palette file against its own rules and the documented exceptions."""
	problems = []
	seen_ids, seen_letters, seen_names = set(), set(), set()
	for tone in palette.tones:
		if not 1 <= tone.id <= 15:
			problems.append(f"{tone.name}: id {tone.id} is outside 1 to 15")
		if tone.id in seen_ids or tone.letter in seen_letters or tone.name in seen_names:
			problems.append(f"{tone.name}: id, letter or name repeats")
		seen_ids.add(tone.id)
		seen_letters.add(tone.letter)
		seen_names.add(tone.name)
		if len(tone.letter) != 1 or not tone.letter.isalpha():
			problems.append(f"{tone.name}: letter {tone.letter!r} is not one letter")
		if not 16 <= tone.xterm256 <= 255:
			problems.append(f"{tone.name}: xterm256 {tone.xterm256} is outside 16 to 255")
		if (tone.console16 is None) != (tone.ansi16 is None):
			problems.append(f"{tone.name}: console16 and ansi16 must both be present or both absent")
		if tone.console16 is not None:
			if not 0 <= tone.console16 <= 15:
				problems.append(f"{tone.name}: console16 {tone.console16} is outside 0 to 15")
			elif tone.ansi16 != swap_bits_0_and_2(tone.console16):
				problems.append(f"{tone.name}: ansi16 {tone.ansi16} is not console16 {tone.console16} with bits 0 and 2 swapped")
		if tone.mono is not None and tone.mono not in MONO_LEVELS:
			problems.append(f"{tone.name}: mono {tone.mono!r} is not one of {MONO_LEVELS}")
		nearest = nearest_xterm256(tone.lab)
		expected = XTERM256_EXCEPTIONS.get(tone.name, nearest)
		if tone.xterm256 != expected:
			problems.append(f"{tone.name}: xterm256 {tone.xterm256} is not the nearest entry {nearest} and is not a documented exception")
		measured = lab_distance(tone.lab, rgb_to_lab(xterm_rgb(tone.xterm256)))
		if tone.delta_e_xterm256 is not None and abs(measured - tone.delta_e_xterm256) > 0.15:
			problems.append(f"{tone.name}: delta-e xterm256 is {tone.delta_e_xterm256} in the file but {measured:.1f} measured")
	by_name = {tone.name: tone for tone in palette.tones}
	for name, follows in CONSOLE16_FOLLOWS.items():
		if name in by_name and follows in by_name and by_name[name].console16 != by_name[follows].console16:
			problems.append(f"{name}: console16 must equal {follows}'s ({by_name[follows].console16}), a documented exception")
	for name in DROPPED_AT_16:
		if name in by_name and (by_name[name].console16 is not None or by_name[name].mono is not None):
			problems.append(f"{name}: must have no console16, ansi16 or mono, so it is dropped at 16 colours")
	for name, index in REACTOS_CONSOLE16.items():
		if name in by_name and by_name[name].console16 != index:
			problems.append(f"{name}: console16 must stay {index}, the index the reactos-liminal afterfetch used")
	for architecture, tone_name in palette.arch_accent.items():
		if tone_name not in by_name:
			problems.append(f"arch-accent {architecture}: no tone named {tone_name!r}")
	if problems:
		raise ArtError("palette problems:\n  " + "\n  ".join(problems))


@dataclass
class LogoSpec:
	name: str
	systems: list
	status: str
	master: Path
	ascii_path: Path
	width: int
	height: int
	ascii_width: int
	ascii_rows: int
	tone_names: list
	width_exception: str = ""


@dataclass
class Manifest:
	limits: dict
	defaults: dict
	logos: list


def load_manifest(path=MANIFEST_PATH):
	with open(path, "rb") as manifest_file:
		data = tomllib.load(manifest_file)
	art_dir = path.parent
	logos = []
	for entry in data["logo"]:
		logos.append(
			LogoSpec(
				name=entry["name"],
				systems=list(entry["systems"]),
				status=entry["status"],
				master=art_dir / entry["master"],
				ascii_path=art_dir / entry["ascii"],
				width=entry["width"],
				height=entry["height"],
				ascii_width=entry["ascii-width"],
				ascii_rows=entry["ascii-rows"],
				tone_names=list(entry.get("tones", [])),
				width_exception=entry.get("ascii-width-exception", ""),
			)
		)
	names = [logo.name for logo in logos]
	if len(set(names)) != len(names):
		raise ArtError("art/logos.toml: a logo name repeats")
	for platform, logo_name in data["defaults"].items():
		if logo_name not in names:
			raise ArtError(f"art/logos.toml: default for {platform} is {logo_name!r}, which is not a logo")
	return Manifest(data["limits"], dict(data["defaults"]), logos)


# ----------------------------------------------------------------------------------------------------------------------
# PNG: the stdlib decoder for rsvg-convert's output


def read_png(data):
	"""Decodes an 8-bit, non-interlaced PNG to a list of rows of (r, g, b, a) tuples."""
	if data[:8] != b"\x89PNG\r\n\x1a\n":
		raise ArtError("not a PNG file")
	position = 8
	header = None
	palette_entries = []
	palette_alpha = []
	compressed = bytearray()
	while position + 8 <= len(data):
		length, kind = struct.unpack(">I4s", data[position : position + 8])
		body = data[position + 8 : position + 8 + length]
		position += 12 + length
		if kind == b"IHDR":
			header = struct.unpack(">IIBBBBB", body)
		elif kind == b"PLTE":
			palette_entries = [tuple(body[offset : offset + 3]) for offset in range(0, len(body), 3)]
		elif kind == b"tRNS":
			palette_alpha = list(body)
		elif kind == b"IDAT":
			compressed += body
		elif kind == b"IEND":
			break
	if header is None:
		raise ArtError("PNG has no IHDR")
	width, height, bit_depth, colour_type, _compression, _filter, interlace = header
	if bit_depth != 8 or interlace != 0 or colour_type not in (0, 2, 3, 4, 6):
		raise ArtError(f"unsupported PNG (bit depth {bit_depth}, colour type {colour_type}, interlace {interlace})")
	channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[colour_type]
	stride = width * channels
	raw = zlib.decompress(bytes(compressed))
	if len(raw) != height * (stride + 1):
		raise ArtError("PNG data has the wrong length")
	previous = bytearray(stride)
	rows = []
	for row_index in range(height):
		start = row_index * (stride + 1)
		filter_type = raw[start]
		line = bytearray(raw[start + 1 : start + 1 + stride])
		unfilter_line(filter_type, line, previous, channels)
		rows.append(pixels_from_line(line, width, channels, colour_type, palette_entries, palette_alpha))
		previous = line
	return rows


def unfilter_line(filter_type, line, previous, bytes_per_pixel):
	for index in range(len(line)):
		left = line[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
		above = previous[index]
		upper_left = previous[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
		if filter_type == 0:
			continue
		if filter_type == 1:
			predictor = left
		elif filter_type == 2:
			predictor = above
		elif filter_type == 3:
			predictor = (left + above) // 2
		elif filter_type == 4:
			estimate = left + above - upper_left
			distance_left = abs(estimate - left)
			distance_above = abs(estimate - above)
			distance_upper_left = abs(estimate - upper_left)
			if distance_left <= distance_above and distance_left <= distance_upper_left:
				predictor = left
			elif distance_above <= distance_upper_left:
				predictor = above
			else:
				predictor = upper_left
		else:
			raise ArtError(f"PNG filter type {filter_type} is not defined")
		line[index] = (line[index] + predictor) & 0xFF


def pixels_from_line(line, width, channels, colour_type, palette_entries, palette_alpha):
	pixels = []
	for column in range(width):
		sample = line[column * channels : (column + 1) * channels]
		if colour_type == 6:
			pixels.append((sample[0], sample[1], sample[2], sample[3]))
		elif colour_type == 2:
			pixels.append((sample[0], sample[1], sample[2], 255))
		elif colour_type == 0:
			pixels.append((sample[0], sample[0], sample[0], 255))
		elif colour_type == 4:
			pixels.append((sample[0], sample[0], sample[0], sample[1]))
		else:
			red, green, blue = palette_entries[sample[0]]
			alpha = palette_alpha[sample[0]] if sample[0] < len(palette_alpha) else 255
			pixels.append((red, green, blue, alpha))
	return pixels


def write_png(rows):
	"""Encodes rows of (r, g, b, a) as an RGBA PNG; used by the tests and the preview fallbacks."""
	height = len(rows)
	width = len(rows[0])
	raw = bytearray()
	for row in rows:
		raw.append(0)
		for pixel in row:
			raw += bytes(pixel)

	def chunk(kind, body):
		return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

	return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


# ----------------------------------------------------------------------------------------------------------------------
# Rendering and quantising a master


def renderer_version():
	try:
		result = subprocess.run(["rsvg-convert", "--version"], capture_output=True, text=True, check=True)
	except (OSError, subprocess.CalledProcessError) as error:
		raise ArtError(f"rsvg-convert is needed and could not be run: {error}") from error
	return "rsvg-convert " + result.stdout.splitlines()[0].split()[-1]


def render_master(svg_path, width, height):
	"""Renders a master at one pixel per logo pixel, as rows of (r, g, b, a)."""
	command = ["rsvg-convert", "--width", str(width), "--height", str(height), "--background-color", "none", "--format", "png", str(svg_path)]
	try:
		result = subprocess.run(command, capture_output=True, check=True)
	except OSError as error:
		raise ArtError(f"rsvg-convert is needed and could not be run: {error}") from error
	except subprocess.CalledProcessError as error:
		raise ArtError(f"rsvg-convert failed on {svg_path}: {error.stderr.decode(errors='replace').strip()}") from error
	rows = read_png(result.stdout)
	if len(rows) != height or len(rows[0]) != width:
		raise ArtError(f"{svg_path}: rendered {len(rows[0])} x {len(rows)}, expected {width} x {height}")
	return rows


def quantise(rows, logo, palette, max_delta_e, warnings):
	"""Maps rendered pixels to palette indexes of the logo (0 transparent), failing on anything off palette."""
	logo_tones = [palette.by_name(name) for name in logo.tone_names]
	exact = {tone.rgb: index for index, tone in enumerate(logo_tones, start=1)}
	every_tone = {tone.rgb: tone for tone in palette.tones}
	problems = []
	off_exact = 0
	indexes = []
	for row_number, row in enumerate(rows):
		index_row = []
		for column, (red, green, blue, alpha) in enumerate(row):
			if alpha < ALPHA_THRESHOLD:
				index_row.append(0)
				continue
			rgb = (red, green, blue)
			if rgb in exact:
				index_row.append(exact[rgb])
				continue
			where = f"{logo.master.name} pixel ({column}, {row_number}) {hex_colour(rgb)}"
			if rgb in every_tone:
				problems.append(f"{where} is the tone {every_tone[rgb].name}, which is not in {logo.name}'s tones")
				index_row.append(0)
				continue
			lab = rgb_to_lab(rgb)
			best_index, best_distance = min(
				((index, lab_distance(lab, tone.lab)) for index, tone in enumerate(logo_tones, start=1)),
				key=lambda pair: pair[1],
			)
			if best_distance > max_delta_e:
				problems.append(f"{where} is off palette: nearest is {logo_tones[best_index - 1].name} at delta E {best_distance:.1f} (limit {max_delta_e})")
			else:
				off_exact += 1
			index_row.append(best_index)
		indexes.append(index_row)
	if problems:
		shown = problems[:MAX_REPORTED_PIXELS]
		more = len(problems) - len(shown)
		raise ArtError("\n  ".join(shown) + (f"\n  and {more} more" if more else ""))
	if off_exact:
		warnings.append(f"{logo.master.name}: {off_exact} pixel(s) were not exact tone fills and were snapped to the nearest tone")
	return indexes


# ----------------------------------------------------------------------------------------------------------------------
# The hand-drawn ASCII form


def parse_ascii_form(text, logo, palette):
	"""Parses art/ascii/<logo>.txt into rows of (art, tone indexes), checking it against the format's rules."""
	lines = text.split("\n")
	if lines and lines[-1] == "":
		lines.pop()
	where = logo.ascii_path.name
	if len(lines) != 2 * logo.ascii_rows:
		raise ArtError(f"{where}: {len(lines)} lines, expected {2 * logo.ascii_rows} (an art line and a tone line for each of {logo.ascii_rows} rows)")
	letters = {}
	for index, name in enumerate(logo.tone_names, start=1):
		letters[palette.by_name(name).letter] = index
	problems = []
	rows = []
	for row in range(logo.ascii_rows):
		art_line = lines[2 * row]
		tone_line = lines[2 * row + 1]
		width = max(len(art_line), len(tone_line))
		art_line = art_line.ljust(width)
		tone_line = tone_line.ljust(width)
		tones = []
		for column in range(width):
			character, letter = art_line[column], tone_line[column]
			position = f"{where}:{2 * row + 1}:{column + 1}"
			if not 0x20 <= ord(character) <= 0x7E:
				problems.append(f"{position}: character {character!r} is outside 0x20 to 0x7E")
				tones.append(0)
			elif character == " " and letter != " ":
				problems.append(f"{position}: a space in the art has tone {letter!r}")
				tones.append(0)
			elif character != " " and letter == " ":
				problems.append(f"{position}: {character!r} has no tone")
				tones.append(0)
			elif character == " ":
				tones.append(0)
			elif letter in letters:
				tones.append(letters[letter])
			elif palette.by_letter(letter) is not None:
				problems.append(f"{position}: tone {palette.by_letter(letter).name} ({letter!r}) is not in {logo.name}'s tones")
				tones.append(0)
			else:
				problems.append(f"{position}: {letter!r} is not a tone letter")
				tones.append(0)
		art = art_line.rstrip(" ")
		if len(art) > logo.ascii_width:
			problems.append(f"{where}:{2 * row + 1}: row is {len(art)} columns, over the limit of {logo.ascii_width}")
		rows.append((art, tones[: len(art)]))
	if problems:
		shown = problems[:MAX_REPORTED_PIXELS]
		more = len(problems) - len(shown)
		raise ArtError("\n  ".join(shown) + (f"\n  and {more} more" if more else ""))
	return rows


# ----------------------------------------------------------------------------------------------------------------------
# Packing a blob


def pack_runs(indexes):
	"""Run bytes for a sequence of palette indexes: (run - 1) << 4 | index, at most 16 pixels a byte."""
	packed = bytearray()
	position = 0
	while position < len(indexes):
		value = indexes[position]
		run = 1
		while position + run < len(indexes) and indexes[position + run] == value and run < MAX_RUN:
			run += 1
		packed.append((run - 1) << 4 | value)
		position += run
	return bytes(packed)


def pack_art(art):
	"""The art bytes of an ASCII row: characters, with a repeat byte for each run after its first character."""
	packed = bytearray()
	position = 0
	while position < len(art):
		character = art[position]
		run = 1
		while position + run < len(art) and art[position + run] == character:
			run += 1
		packed.append(ord(character))
		remaining = run - 1
		while remaining > 0:
			repeat = min(remaining, 128)
			packed.append(0x80 | (repeat - 1))
			remaining -= repeat
		position += run
	return bytes(packed)


def pack_ascii_row(art, tones):
	packed_art = pack_art(art)
	if len(packed_art) > 255:
		raise ArtError("an ASCII row packs to more than 255 bytes")
	return bytes([len(packed_art)]) + packed_art + pack_runs(tones)


def pack_section(kind, payload):
	if len(payload) > 0xFFFF:
		raise ArtError("a section is longer than 65,535 bytes")
	return bytes([kind]) + struct.pack("<H", len(payload)) + payload


def pack_blob(width, height, tone_ids, ascii_width, ascii_rows, pixel_rows, low_rows=None, ascii_form=None):
	"""A whole .afl blob; ascii_form is a list of (art, tone indexes), or None for a logo with no ASCII form."""
	header = MAGIC + bytes([width, height, ascii_width, ascii_rows, len(tone_ids)]) + bytes(tone_ids)
	blob = header + pack_section(KIND_PIXELS, b"".join(pack_runs(row) for row in pixel_rows))
	if low_rows is not None:
		blob += pack_section(KIND_PIXELS_LOW, b"".join(pack_runs(row) for row in low_rows))
	if ascii_form is not None:
		blob += pack_section(KIND_ASCII, b"".join(pack_ascii_row(art, tones) for art, tones in ascii_form))
	return blob


# ----------------------------------------------------------------------------------------------------------------------
# The reference decoder: the same checks docs/logo-format.md gives the core's reader, used to validate every blob


class BlobError(ArtError):
	"""A blob that breaks docs/logo-format.md."""


@dataclass
class DecodedLogo:
	width: int
	height: int
	ascii_width: int
	ascii_rows: int
	palette: list
	pixels: list
	low: list
	ascii_art: list
	ascii_tones: list
	sections: dict


def decode_runs(data, row_length, where):
	indexes = []
	for byte in data:
		run = (byte >> 4) + 1
		indexes += [byte & 0x0F] * run
	if len(indexes) != row_length:
		raise BlobError(f"{where}: runs add up to {len(indexes)}, expected {row_length}")
	return indexes


def decode_pixel_section(payload, width, height, palette_len, where):
	rows = []
	position = 0
	for row in range(height):
		total = 0
		start = position
		while total < width:
			if position >= len(payload):
				raise BlobError(f"{where}: row {row} runs past the end of the section")
			total += (payload[position] >> 4) + 1
			position += 1
		if total != width:
			raise BlobError(f"{where}: row {row} runs add up to {total}, expected {width}")
		indexes = decode_runs(payload[start:position], width, f"{where} row {row}")
		if any(index > palette_len for index in indexes):
			raise BlobError(f"{where}: row {row} has a palette index above {palette_len}")
		rows.append(indexes)
	if position != len(payload):
		raise BlobError(f"{where}: bytes after the last row")
	return rows


def decode_ascii_section(payload, ascii_width, ascii_rows, palette_len):
	art_rows, tone_rows = [], []
	position = 0
	for row in range(ascii_rows):
		where = f"ASCII row {row}"
		if position >= len(payload):
			raise BlobError(f"{where}: missing")
		art_len = payload[position]
		position += 1
		if position + art_len > len(payload):
			raise BlobError(f"{where}: art runs past the end of the section")
		art = ""
		for byte in payload[position : position + art_len]:
			if 0x20 <= byte <= 0x7E:
				art += chr(byte)
			elif byte >= 0x80:
				if not art:
					raise BlobError(f"{where}: starts with a repeat")
				art += art[-1] * ((byte & 0x7F) + 1)
			else:
				raise BlobError(f"{where}: byte {byte:#04x} is not allowed")
			if len(art) > ascii_width:
				raise BlobError(f"{where}: art is longer than {ascii_width}")
		position += art_len
		tones = []
		while len(tones) < len(art):
			if position >= len(payload):
				raise BlobError(f"{where}: tones run past the end of the section")
			byte = payload[position]
			position += 1
			tones += [byte & 0x0F] * ((byte >> 4) + 1)
		if len(tones) != len(art):
			raise BlobError(f"{where}: tone runs add up to {len(tones)}, expected {len(art)}")
		for character, index in zip(art, tones):
			if index > palette_len:
				raise BlobError(f"{where}: palette index above {palette_len}")
			if (character == " ") != (index == 0):
				raise BlobError(f"{where}: a space and its index disagree")
		art_rows.append(art)
		tone_rows.append(tones)
	if position != len(payload):
		raise BlobError("ASCII section: bytes after the last row")
	return art_rows, tone_rows


def decode_blob(blob):
	"""Decodes and validates a blob exactly as docs/logo-format.md requires; raises BlobError on any breach."""
	if len(blob) < 10 or blob[:4] != MAGIC:
		raise BlobError("bad magic or shorter than a header")
	width, height, ascii_width, ascii_rows, palette_len = blob[4:9]
	if not 1 <= width <= FORMAT_MAX_WIDTH:
		raise BlobError(f"width {width} is outside 1 to {FORMAT_MAX_WIDTH}")
	if not 2 <= height <= FORMAT_MAX_WIDTH or height % 2:
		raise BlobError(f"height {height} is not even and within 2 to {FORMAT_MAX_WIDTH}")
	if ascii_width > FORMAT_MAX_WIDTH or ascii_rows > FORMAT_MAX_ASCII_ROWS or (ascii_width == 0) != (ascii_rows == 0):
		raise BlobError("ASCII size is out of range, or only one of its width and rows is zero")
	if not 1 <= palette_len <= 15:
		raise BlobError(f"palette_len {palette_len} is outside 1 to 15")
	if len(blob) < 9 + palette_len:
		raise BlobError("shorter than its palette")
	palette = list(blob[9 : 9 + palette_len])
	if any(not 1 <= tone_id <= 15 for tone_id in palette) or len(set(palette)) != palette_len:
		raise BlobError("palette names tone 0, a tone above 15, or repeats a tone")
	position = 9 + palette_len
	sections = {}
	previous_kind = 0
	while position < len(blob):
		if position + 3 > len(blob):
			raise BlobError("a section header is cut short")
		kind = blob[position]
		length = struct.unpack("<H", blob[position + 1 : position + 3])[0]
		if kind <= previous_kind:
			raise BlobError("sections are out of order or a kind repeats")
		if kind >= 0x80:
			raise BlobError(f"section kind {kind:#04x} must be understood and is not")
		if kind == 0x00:
			raise BlobError("section kind 0x00 is not defined")
		if position + 3 + length > len(blob):
			raise BlobError("a section runs past the end of the blob")
		sections[kind] = (position + 3, length)
		previous_kind = kind
		position += 3 + length
	if KIND_PIXELS not in sections:
		raise BlobError("PIXELS is missing")
	if (KIND_ASCII in sections) != (ascii_width > 0):
		raise BlobError("ASCII section and ascii_width disagree")

	def payload_of(kind):
		offset, length = sections[kind]
		return blob[offset : offset + length]

	pixels = decode_pixel_section(payload_of(KIND_PIXELS), width, height, palette_len, "PIXELS")
	low = decode_pixel_section(payload_of(KIND_PIXELS_LOW), width, height, palette_len, "PIXELS_LOW") if KIND_PIXELS_LOW in sections else None
	art_rows, tone_rows = ([], [])
	if KIND_ASCII in sections:
		art_rows, tone_rows = decode_ascii_section(payload_of(KIND_ASCII), ascii_width, ascii_rows, palette_len)
	return DecodedLogo(width, height, ascii_width, ascii_rows, palette, pixels, low, art_rows, tone_rows, sections)


# ----------------------------------------------------------------------------------------------------------------------
# Building every logo


@dataclass
class BuiltLogo:
	spec: LogoSpec
	blob: bytes
	decoded: DecodedLogo


def check_limits(logo, limits):
	problems = []
	if not 1 <= logo.width <= limits["max-width"] or not 2 <= logo.height <= limits["max-height"] or logo.height % 2:
		problems.append(f"size {logo.width} x {logo.height} is outside the limits ({limits['max-width']} x {limits['max-height']}, height even)")
	if logo.ascii_width > limits["max-ascii-width"] and not logo.width_exception:
		problems.append(f"ascii-width {logo.ascii_width} is over {limits['max-ascii-width']} and has no ascii-width-exception")
	if logo.ascii_rows > limits["max-ascii-rows"]:
		problems.append(f"ascii-rows {logo.ascii_rows} is over {limits['max-ascii-rows']}")
	if len(logo.tone_names) > 15 or len(set(logo.tone_names)) != len(logo.tone_names):
		problems.append("tones is longer than 15 or repeats a tone")
	if problems:
		raise ArtError(f"{logo.name}: " + "; ".join(problems))


def used_tone_ids(pixel_indexes, ascii_rows, palette_len):
	used = set()
	for row in pixel_indexes:
		used.update(row)
	for tones in ascii_rows:
		used.update(tones)
	used.discard(0)
	return {index for index in used if index <= palette_len}


def build_logo(logo, palette, manifest, warnings):
	"""Builds one logo, or returns None when its master is not drawn yet."""
	if not logo.master.exists():
		if logo.status == "drawn":
			raise ArtError(f"{logo.name}: status is drawn but {logo.master} does not exist")
		return None
	check_limits(logo, manifest.limits)
	if not logo.tone_names:
		raise ArtError(f"{logo.name}: art/logos.toml needs a `tones` list for a logo with a master")
	max_delta_e = manifest.limits["max-delta-e"]
	try:
		pixel_rows = quantise(render_master(logo.master, logo.width, logo.height), logo, palette, max_delta_e, warnings)
		low_master = logo.master.with_name(logo.master.stem + "-low.svg")
		low_rows = None
		if low_master.exists():
			low_logo = LogoSpec(**{**logo.__dict__, "master": low_master})
			low_rows = quantise(render_master(low_master, logo.width, logo.height), low_logo, palette, max_delta_e, warnings)
		ascii_form = None
		if logo.ascii_path.exists():
			ascii_form = parse_ascii_form(logo.ascii_path.read_text(encoding="utf-8"), logo, palette)
		elif logo.status == "drawn":
			raise ArtError(f"status is drawn but {logo.ascii_path} does not exist")
		else:
			warnings.append(f"{logo.name}: {logo.ascii_path.name} does not exist yet, so the blob has no ASCII form")
	except ArtError as error:
		raise ArtError(f"{logo.name}:\n  {error}") from error
	tone_ids = [palette.by_name(name).id for name in logo.tone_names]
	used = used_tone_ids(pixel_rows + (low_rows or []), [tones for _, tones in (ascii_form or [])], len(tone_ids))
	for index, name in enumerate(logo.tone_names, start=1):
		if index not in used:
			warnings.append(f"{logo.name}: tone {name} is listed in tones but neither the master nor the ASCII form uses it")
	blob = pack_blob(
		logo.width,
		logo.height,
		tone_ids,
		logo.ascii_width if ascii_form else 0,
		logo.ascii_rows if ascii_form else 0,
		pixel_rows,
		low_rows,
		ascii_form,
	)
	decoded = decode_blob(blob)
	if decoded.pixels != pixel_rows or (low_rows is not None and decoded.low != low_rows):
		raise ArtError(f"{logo.name}: the blob does not decode to the pixels it was made from")
	if ascii_form is not None and (decoded.ascii_art != [art for art, _ in ascii_form] or decoded.ascii_tones != [tones for _, tones in ascii_form]):
		raise ArtError(f"{logo.name}: the blob does not decode to the ASCII form it was made from")
	return BuiltLogo(logo, blob, decoded)


def check_budgets(built, limits):
	problems = []
	for item in built:
		if len(item.blob) > limits["blob-bytes"]:
			problems.append(f"{item.spec.name}: {len(item.blob)} bytes is over the {limits['blob-bytes']} byte blob budget")
	total = sum(len(item.blob) for item in built)
	if total > limits["total-bytes"]:
		problems.append(f"all blobs together: {total} bytes is over the {limits['total-bytes']} byte budget")
	if problems:
		raise ArtError("size budget broken:\n  " + "\n  ".join(problems))


# ----------------------------------------------------------------------------------------------------------------------
# Generated Rust

GENERATED_NOTE = "Generated by art/scripts/afterfetch_art.py from art/logos.toml and art/palette/afterglow-dusk.toml.\nDo not edit; run `make art`."


def rust_header(renderer):
	lines = GENERATED_NOTE.split("\n") + [f"Renderer: {renderer}"]
	return "".join(f"// {line}\n" for line in lines) + "\n"


def rust_option(value, wrap="{}"):
	return "None" if value is None else "Some(" + wrap.format(value) + ")"


def write_palette_rs(palette, renderer):
	out = [rust_header(renderer), "use crate::{Mono, Tone};\n\n"]
	out.append("/// Every tone, in id order. Ids are 1 to 15 and never change meaning; 0 is transparent in a blob.\n")
	out.append(f"pub const TONES: [Tone; {len(palette.tones)}] = [\n")
	for tone in sorted(palette.tones, key=lambda item: item.id):
		red, green, blue = tone.rgb
		out.append("\tTone {\n")
		out.append(f"\t\tid: {tone.id},\n")
		out.append(f'\t\tname: "{tone.name}",\n')
		out.append(f"\t\tletter: b'{tone.letter}',\n")
		out.append(f"\t\trgb: [{red:#04x}, {green:#04x}, {blue:#04x}],\n")
		out.append(f"\t\txterm256: {tone.xterm256},\n")
		out.append(f"\t\tconsole16: {rust_option(tone.console16)},\n")
		out.append(f"\t\tansi16: {rust_option(tone.ansi16)},\n")
		out.append(f"\t\tmono: {rust_option(tone.mono and tone.mono.capitalize(), 'Mono::{}')},\n")
		out.append("\t},\n")
	out.append("];\n\n")
	out.append("/// The accent tone of the title rule for each architecture name, as in `[arch-accent]` of the palette file.\n")
	out.append(f"pub const ARCH_ACCENTS: [(&str, u8); {len(palette.arch_accent)}] = [\n")
	for architecture, tone_name in palette.arch_accent.items():
		out.append(f'\t("{architecture}", {palette.by_name(tone_name).id}),\n')
	out.append("];\n")
	return "".join(out)


def rust_span(span):
	return f"Section {{ offset: {span[0]}, len: {span[1]} }}"


def rust_section(span):
	return "None" if span is None else f"Some({rust_span(span)})"


def write_index_rs(built, manifest, renderer):
	out = [rust_header(renderer), "use crate::{LogoEntry, Section};\n\n"]
	out.append("/// Every generated logo, in `--logo list` order. A section's `offset` is where its payload starts in `blob`.\n")
	out.append("pub const LOGOS: &[LogoEntry] = &[\n")
	for item in built:
		spec, decoded = item.spec, item.decoded
		systems = ", ".join(f'"{system}"' for system in spec.systems)
		pixels = decoded.sections[KIND_PIXELS]
		out.append("\tLogoEntry {\n")
		out.append(f'\t\tname: "{spec.name}",\n')
		out.append(f"\t\tsystems: &[{systems}],\n")
		out.append(f'\t\tblob: include_bytes!("../../data/{spec.name}.afl"),\n')
		out.append(f"\t\twidth: {decoded.width},\n")
		out.append(f"\t\theight: {decoded.height},\n")
		out.append(f"\t\tascii_width: {decoded.ascii_width},\n")
		out.append(f"\t\tascii_rows: {decoded.ascii_rows},\n")
		out.append(f"\t\tpalette_len: {len(decoded.palette)},\n")
		out.append(f"\t\tpixels: {rust_span(pixels)},\n")
		out.append(f"\t\tlow: {rust_section(decoded.sections.get(KIND_PIXELS_LOW))},\n")
		out.append(f"\t\tascii: {rust_section(decoded.sections.get(KIND_ASCII))},\n")
		out.append("\t},\n")
	out.append("];\n\n")
	out.append("/// The logo each platform shell names in `default_logo`, as in `[defaults]` of the manifest.\n")
	out.append(f"pub const DEFAULTS: [(&str, &str); {len(manifest.defaults)}] = [\n")
	for platform, logo_name in manifest.defaults.items():
		out.append(f'\t("{platform}", "{logo_name}"),\n')
	out.append("];\n")
	return "".join(out)


# ----------------------------------------------------------------------------------------------------------------------
# Generating into a directory, and checking one


def generate(out_dir, manifest=None, palette=None, quiet=False):
	"""Writes data/*.afl and src/generated/*.rs under out_dir; returns the list of built logos."""
	manifest = manifest or load_manifest()
	palette = palette or load_palette()
	declared = manifest.limits["renderer"]
	found = renderer_version()
	warnings = []
	if found != declared:
		warnings.append(f"renderer is {found}, the committed blobs were made with {declared}; comparing anyway")
	built = []
	for logo in manifest.logos:
		item = build_logo(logo, palette, manifest, warnings)
		if item is None:
			warnings.append(f"{logo.name}: no master yet, skipped")
		else:
			built.append(item)
	if not built:
		raise ArtError("no logo has a master")
	check_budgets(built, manifest.limits)
	data_dir = out_dir / "data"
	generated_dir = out_dir / "src" / "generated"
	data_dir.mkdir(parents=True, exist_ok=True)
	generated_dir.mkdir(parents=True, exist_ok=True)
	for stale in sorted(data_dir.glob("*.afl")):
		if stale.stem not in {item.spec.name for item in built}:
			stale.unlink()
	for item in built:
		(data_dir / f"{item.spec.name}.afl").write_bytes(item.blob)
	(generated_dir / "palette.rs").write_text(write_palette_rs(palette, declared), encoding="utf-8", newline="\n")
	(generated_dir / "index.rs").write_text(write_index_rs(built, manifest, declared), encoding="utf-8", newline="\n")
	if not quiet:
		for warning in warnings:
			print(f"warning: {warning}", file=sys.stderr)
		for item in built:
			print(f"{item.spec.name}: {len(item.blob)} bytes")
		print(f"total: {sum(len(item.blob) for item in built)} of {manifest.limits['total-bytes']} bytes")
	return built


def generated_files(root):
	files = {}
	for pattern in ("data/*.afl", "src/generated/*.rs"):
		for path in sorted(root.glob(pattern)):
			files[path.relative_to(root).as_posix()] = path
	return files


def check_committed(crate_dir=CRATE_DIR):
	"""Regenerates into a temp dir and returns the names of files that differ from the committed ones."""
	with tempfile.TemporaryDirectory(prefix="afterfetch-art-") as temp_name:
		temp_dir = Path(temp_name)
		generate(temp_dir, quiet=True)
		fresh = generated_files(temp_dir)
		committed = generated_files(crate_dir)
		differences = []
		for name in sorted(set(fresh) | set(committed)):
			if name not in committed:
				differences.append(f"{name}: missing from the crate")
			elif name not in fresh:
				differences.append(f"{name}: committed but no longer generated")
			elif not filecmp.cmp(fresh[name], committed[name], shallow=False):
				differences.append(f"{name}: differs from a fresh generation")
		return differences


# ----------------------------------------------------------------------------------------------------------------------
# Previews: needs Pillow, which is only for review and never for the build

PREVIEW_BACKGROUND = {"truecolour": "#050507", "xterm256": "#050507", "console16": "#0a1230", "ansi16": "#050507", "mono": "#000000"}
PREVIEW_DEPTHS = ("truecolour", "xterm256", "console16", "ansi16", "mono")
PREVIEW_PIXEL = 8
PREVIEW_CELL_WIDTH = 8
PREVIEW_CELL_HEIGHT = 16


def tone_colour_at_depth(tone, depth, palette):
	"""The colour a tone shows at a depth, or None when the depth drops the tone."""
	if depth == "truecolour":
		return tone.rgb
	if depth == "xterm256":
		return xterm_rgb(tone.xterm256)
	if depth == "console16":
		return None if tone.console16 is None else palette.tables["dusk-console"][tone.console16]
	if depth == "ansi16":
		return None if tone.ansi16 is None else palette.tables["afterglow-terminal"][tone.ansi16]
	return None if tone.mono is None else palette.mono_preview[tone.mono]


def write_previews(built, palette, out_dir):
	"""Writes <logo>-<depth>.png at 8 x 16 px a cell, and contact-sheet.png; skips with a message without Pillow."""
	try:
		from PIL import Image, ImageDraw
	except ImportError:
		print("previews skipped: Pillow is not installed (pip install pillow); the build does not need it", file=sys.stderr)
		return False
	out_dir.mkdir(parents=True, exist_ok=True)
	sheet_rows = []
	for item in built:
		decoded = item.decoded
		images = []
		for depth in PREVIEW_DEPTHS:
			picture = decoded.low if depth in ("console16", "ansi16", "mono") and decoded.low is not None else decoded.pixels
			image = Image.new("RGB", (decoded.width * PREVIEW_PIXEL, decoded.height * PREVIEW_PIXEL), PREVIEW_BACKGROUND[depth])
			drawing = ImageDraw.Draw(image)
			for row, indexes in enumerate(picture):
				for column, index in enumerate(indexes):
					colour = None if index == 0 else tone_colour_at_depth(palette.by_id(decoded.palette[index - 1]), depth, palette)
					if colour is not None:
						left, top = column * PREVIEW_PIXEL, row * PREVIEW_PIXEL
						drawing.rectangle([left, top, left + PREVIEW_PIXEL - 1, top + PREVIEW_PIXEL - 1], fill=colour)
			image.save(out_dir / f"{item.spec.name}-{depth}.png")
			images.append(image)
		if decoded.ascii_rows:
			image = Image.new("RGB", (max(decoded.ascii_width, 1) * PREVIEW_CELL_WIDTH, decoded.ascii_rows * PREVIEW_CELL_HEIGHT), PREVIEW_BACKGROUND["truecolour"])
			drawing = ImageDraw.Draw(image)
			for row, (art, tones) in enumerate(zip(decoded.ascii_art, decoded.ascii_tones)):
				for column, (character, index) in enumerate(zip(art, tones)):
					if index:
						colour = palette.by_id(decoded.palette[index - 1]).rgb
						left, top = column * PREVIEW_CELL_WIDTH, row * PREVIEW_CELL_HEIGHT
						if character == "#":
							drawing.rectangle([left, top, left + PREVIEW_CELL_WIDTH - 1, top + PREVIEW_CELL_HEIGHT - 1], fill=colour)
						else:
							drawing.text((left, top), character, fill=colour)
			image.save(out_dir / f"{item.spec.name}-ascii.png")
			images.append(image)
		sheet_rows.append((item.spec.name, images))
	gap = 12
	row_height = max(image.height for _, images in sheet_rows for image in images) + gap
	sheet_width = max(sum(image.width + gap for image in images) for _, images in sheet_rows) + gap
	sheet = Image.new("RGB", (sheet_width, row_height * len(sheet_rows) + gap), "#222222")
	for row, (_, images) in enumerate(sheet_rows):
		left = gap
		for image in images:
			sheet.paste(image, (left, gap + row * row_height))
			left += image.width + gap
	sheet.save(out_dir / "contact-sheet.png")
	print(f"previews written to {out_dir}")
	return True


# ----------------------------------------------------------------------------------------------------------------------


def main(argv=None):
	parser = argparse.ArgumentParser(description="Generate the afterfetch logo blobs, palette tables and logo index.")
	parser.add_argument("--out", type=Path, default=CRATE_DIR, help="crate directory to write into (default: crates/afterfetch-art)")
	parser.add_argument("--check", action="store_true", help="generate into a temp dir and fail if the committed files differ")
	parser.add_argument("--preview", nargs="?", const=str(ART_DIR / "previews"), metavar="DIR", help="also write per-depth PNGs and a contact sheet (Pillow)")
	arguments = parser.parse_args(argv)
	try:
		if arguments.check:
			differences = check_committed(arguments.out)
			if differences:
				print("art-check: the committed generated files are out of date; run `make art`:", file=sys.stderr)
				for difference in differences:
					print(f"  {difference}", file=sys.stderr)
				return 1
			print("art-check: ok")
			return 0
		built = generate(arguments.out)
		if arguments.preview:
			write_previews(built, load_palette(), Path(arguments.preview))
	except ArtError as error:
		print(f"error: {error}", file=sys.stderr)
		return 1
	return 0


if __name__ == "__main__":
	sys.exit(main())
