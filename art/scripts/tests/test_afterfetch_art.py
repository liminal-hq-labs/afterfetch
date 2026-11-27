# Unit tests for the art generator: PNG decoding, colour, the palette, packing, the reference decoder and the guards.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Run with: python3 -m unittest discover -s art/scripts/tests

import shutil
import struct
import sys
import tempfile
import unittest
import zlib
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import afterfetch_art as art  # noqa: E402

HAVE_RENDERER = shutil.which("rsvg-convert") is not None


def png_with_filter(rows, filter_type):
	"""Encodes rows of (r, g, b, a) as an RGBA PNG with every scanline using one filter type."""
	width = len(rows[0])
	bytes_per_pixel = 4
	raw = bytearray()
	previous = bytes(width * bytes_per_pixel)
	for row in rows:
		line = b"".join(bytes(pixel) for pixel in row)
		filtered = bytearray()
		for index in range(len(line)):
			left = line[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
			above = previous[index]
			upper_left = previous[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
			if filter_type == 0:
				predictor = 0
			elif filter_type == 1:
				predictor = left
			elif filter_type == 2:
				predictor = above
			elif filter_type == 3:
				predictor = (left + above) // 2
			else:
				estimate = left + above - upper_left
				distances = [abs(estimate - left), abs(estimate - above), abs(estimate - upper_left)]
				predictor = [left, above, upper_left][distances.index(min(distances))]
			filtered.append((line[index] - predictor) & 0xFF)
		raw.append(filter_type)
		raw += filtered
		previous = line

	def chunk(kind, body):
		return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

	header = struct.pack(">IIBBBBB", width, len(rows), 8, 6, 0, 0, 0)
	return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(raw))) + chunk(b"IEND", b"")


def svg_master(fills, width=4, height=2):
	"""A master of rectangles: each fill is (colour, x, y, w, h) in logo pixels, drawn at 10 units a pixel."""
	shapes = "".join(f'<rect fill="{colour}" x="{x}" y="{y}" width="{w}" height="{h}"/>' for colour, x, y, w, h in fills)
	return (
		f'<svg xmlns="http://www.w3.org/2000/svg" width="{width * 10}" height="{height * 10}" '
		f'viewBox="0 0 {width * 10} {height * 10}" shape-rendering="crispEdges">'
		f'<g transform="scale(10)">{shapes}</g></svg>'
	)


class ColourTests(unittest.TestCase):
	def test_lab_of_white_and_black(self):
		white = art.rgb_to_lab((255, 255, 255))
		self.assertAlmostEqual(white[0], 100.0, places=1)
		self.assertAlmostEqual(white[1], 0.0, places=1)
		self.assertAlmostEqual(white[2], 0.0, places=1)
		self.assertAlmostEqual(art.rgb_to_lab((0, 0, 0))[0], 0.0, places=3)

	def test_distance_of_a_colour_to_itself_is_zero(self):
		lab = art.rgb_to_lab((244, 63, 94))
		self.assertEqual(art.lab_distance(lab, lab), 0.0)

	def test_xterm_entries(self):
		self.assertEqual(art.xterm_rgb(16), (0, 0, 0))
		self.assertEqual(art.xterm_rgb(231), (255, 255, 255))
		self.assertEqual(art.xterm_rgb(222), (255, 215, 135))
		self.assertEqual(art.xterm_rgb(232), (8, 8, 8))
		self.assertEqual(art.xterm_rgb(255), (238, 238, 238))

	def test_ansi_is_console_with_red_and_blue_swapped(self):
		self.assertEqual(art.swap_bits_0_and_2(14), 11)
		self.assertEqual(art.swap_bits_0_and_2(4), 1)
		self.assertEqual(art.swap_bits_0_and_2(1), 4)
		self.assertEqual(art.swap_bits_0_and_2(6), 3)
		self.assertEqual(art.swap_bits_0_and_2(8), 8)


class PngTests(unittest.TestCase):
	def test_every_filter_type_decodes(self):
		rows = [[(x * 40 + y, 255 - x * 30, (x * y) % 256, 255 if x != 2 else 0) for x in range(6)] for y in range(5)]
		for filter_type in range(5):
			with self.subTest(filter_type=filter_type):
				self.assertEqual(art.read_png(png_with_filter(rows, filter_type)), rows)

	def test_own_encoder_round_trips(self):
		rows = [[(1, 2, 3, 255), (4, 5, 6, 0)]]
		self.assertEqual(art.read_png(art.write_png(rows)), rows)

	def test_rejects_a_bad_signature(self):
		with self.assertRaises(art.ArtError):
			art.read_png(b"not a png at all")

	def test_unsupported_bit_depth_is_an_error(self):
		def chunk(kind, body):
			return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

		data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 16, 6, 0, 0, 0)) + chunk(b"IEND", b"")
		with self.assertRaises(art.ArtError):
			art.read_png(data)


class PaletteTests(unittest.TestCase):
	@classmethod
	def setUpClass(cls):
		cls.palette = art.load_palette()

	def test_the_committed_palette_is_valid(self):
		self.assertEqual(len(self.palette.tones), 15)

	def test_documented_exceptions_are_encoded(self):
		self.assertEqual(self.palette.by_name("amber").xterm256, 221)
		self.assertEqual(self.palette.by_name("amber").console16, self.palette.by_name("orange").console16)
		self.assertEqual(self.palette.by_name("peach").console16, self.palette.by_name("orange").console16)
		self.assertEqual(self.palette.by_name("night").console16, self.palette.by_name("slate").console16)
		self.assertIsNone(self.palette.by_name("soot").console16)
		self.assertIsNone(self.palette.by_name("soot").mono)

	def test_reactos_console_indexes_are_held(self):
		for name, index in art.REACTOS_CONSOLE16.items():
			self.assertEqual(self.palette.by_name(name).console16, index, name)

	def test_a_broken_exception_fails_validation(self):
		broken = [replace(tone, console16=1, ansi16=4) if tone.name == "peach" else tone for tone in self.palette.tones]
		with self.assertRaises(art.ArtError):
			art.validate_palette(replace(self.palette, tones=broken))

	def test_ansi_must_follow_console(self):
		broken = [replace(tone, ansi16=0) if tone.name == "gold" else tone for tone in self.palette.tones]
		with self.assertRaises(art.ArtError):
			art.validate_palette(replace(self.palette, tones=broken))

	def test_an_unexplained_256_index_fails_validation(self):
		broken = [replace(tone, xterm256=100) if tone.name == "gold" else tone for tone in self.palette.tones]
		with self.assertRaises(art.ArtError):
			art.validate_palette(replace(self.palette, tones=broken))


class PackingTests(unittest.TestCase):
	def test_run_bytes(self):
		self.assertEqual(art.pack_runs([0] * 36), bytes([0xF0, 0xF0, 0x30]))
		self.assertEqual(art.pack_runs([0, 1, 1, 0]), bytes([0x00, 0x11, 0x00]))
		self.assertEqual(art.pack_runs([3] * 17), bytes([0xF3, 0x03]))

	def test_art_bytes_use_repeats(self):
		self.assertEqual(art.pack_art(".##."), bytes([0x2E, 0x23, 0x80, 0x2E]))
		self.assertEqual(art.pack_art("-" * 34), bytes([0x2D, 0x80 | 32]))
		self.assertEqual(art.pack_art("a" * 200), bytes([0x61, 0xFF, 0x80 | 70]))

	def test_the_specification_example_is_byte_for_byte(self):
		blob = art.pack_blob(4, 2, [1], 4, 1, [[0, 1, 1, 0], [0, 0, 0, 0]], None, [(".##.", [1, 1, 1, 1])])
		expected = bytes.fromhex("41464C31 04020401 0101 010400 001100 30 030600 04 2E23802E 31".replace(" ", ""))
		self.assertEqual(blob, expected)
		self.assertEqual(len(blob), 26)

	def test_round_trip_with_a_low_picture(self):
		pixels = [[(x + y) % 4 for x in range(10)] for y in range(4)]
		low = [[(x * y) % 3 for x in range(10)] for y in range(4)]
		form = [("ab  c", [1, 2, 0, 0, 3]), ("", []), ("zzzz", [1, 1, 1, 1])]
		blob = art.pack_blob(10, 4, [1, 2, 3], 6, 3, pixels, low, form)
		decoded = art.decode_blob(blob)
		self.assertEqual(decoded.pixels, pixels)
		self.assertEqual(decoded.low, low)
		self.assertEqual(decoded.ascii_art, ["ab  c", "", "zzzz"])
		self.assertEqual(decoded.ascii_tones, [[1, 2, 0, 0, 3], [], [1, 1, 1, 1]])
		self.assertEqual(sorted(decoded.sections), [1, 2, 3])

	def test_a_logo_with_no_ascii_form(self):
		blob = art.pack_blob(2, 2, [5], 0, 0, [[1, 0], [0, 1]])
		decoded = art.decode_blob(blob)
		self.assertEqual(decoded.ascii_rows, 0)
		self.assertNotIn(art.KIND_ASCII, decoded.sections)


class DecoderRejectionTests(unittest.TestCase):
	def setUp(self):
		self.blob = art.pack_blob(4, 2, [1, 2], 4, 1, [[0, 1, 2, 0], [1, 1, 1, 1]], None, [(".##.", [1, 1, 2, 2])])

	def rejects(self, blob):
		with self.assertRaises(art.BlobError):
			art.decode_blob(bytes(blob))

	def test_every_truncation_is_an_error_not_a_crash(self):
		for length in range(len(self.blob)):
			with self.subTest(length=length):
				self.rejects(self.blob[:length])

	def test_trailing_bytes_are_an_error(self):
		self.rejects(self.blob + b"\x00")

	def test_bad_magic_and_header_fields(self):
		for position, value in [(0, 0x42), (3, 0x32), (4, 0), (4, 49), (5, 3), (5, 0), (7, 0), (8, 0), (8, 16), (9, 0), (9, 16), (10, 1)]:
			with self.subTest(position=position, value=value):
				mutated = bytearray(self.blob)
				mutated[position] = value
				self.rejects(mutated)

	def test_bad_pixel_rows_and_indexes(self):
		pixels_start = 9 + 2 + 3
		mutated = bytearray(self.blob)
		mutated[pixels_start] = 0x10 | 1
		self.rejects(mutated)
		mutated = bytearray(self.blob)
		mutated[pixels_start + 1] = 0x00 | 3
		self.rejects(mutated)

	def test_bad_ascii_rows(self):
		# A repeat first in a row, a byte outside the allowed ranges, a drawn character with index 0.
		header = art.MAGIC + bytes([4, 2, 4, 1, 1, 1]) + art.pack_section(1, bytes([0x30, 0x30]))
		for ascii_payload in [bytes([1, 0x80, 0x10]), bytes([1, 0x1F, 0x00]), bytes([1, 0x41, 0x00]), bytes([1, 0x20, 0x10]), bytes([5, 0x41, 0x80, 0x80, 0x80, 0x80, 0x40])]:
			with self.subTest(payload=ascii_payload.hex()):
				self.rejects(header + art.pack_section(3, ascii_payload))

	def test_section_order_and_kinds(self):
		pixels = art.pack_section(1, bytes([0x30, 0x30]))
		header = art.MAGIC + bytes([4, 2, 0, 0, 1, 1])
		self.rejects(header)
		self.rejects(header + art.pack_section(2, bytes([0x30, 0x30])) + pixels)
		self.rejects(header + pixels + pixels)
		self.rejects(header + pixels + art.pack_section(0x80, b""))
		self.rejects(header + pixels + art.pack_section(3, b""))
		art.decode_blob(header + pixels + art.pack_section(0x10, b"anything"))


class AsciiFormTests(unittest.TestCase):
	@classmethod
	def setUpClass(cls):
		cls.palette = art.load_palette()
		cls.manifest = art.load_manifest()

	def logo(self, **changes):
		spec = replace(self.manifest.logos[0], ascii_width=6, ascii_rows=2, tone_names=["gold", "orange"], ascii_path=Path("test.txt"))
		return replace(spec, **changes)

	def parse(self, text, **changes):
		return art.parse_ascii_form(text, self.logo(**changes), self.palette)

	def test_a_good_form(self):
		rows = self.parse(" .##. \n gggg \n  --  \n  oo  \n")
		self.assertEqual(rows, [(" .##.", [0, 1, 1, 1, 1]), ("  --", [0, 0, 2, 2])])

	def test_tone_line_may_be_shorter_with_trailing_spaces_trimmed(self):
		self.assertEqual(self.parse("ab\ngg\n\n\n")[0], ("ab", [1, 1]))

	def test_wrong_line_count(self):
		with self.assertRaisesRegex(art.ArtError, "expected 4"):
			self.parse("ab\ngg\n")

	def test_too_wide(self):
		with self.assertRaisesRegex(art.ArtError, "over the limit"):
			self.parse("abcdefg\nggggggg\n\n\n")

	def test_disallowed_character(self):
		with self.assertRaisesRegex(art.ArtError, "outside 0x20"):
			self.parse("aé\ngg\n\n\n")
		with self.assertRaisesRegex(art.ArtError, "outside 0x20"):
			self.parse("a\tb\nggg\n\n\n")

	def test_drawn_character_without_a_tone_and_the_reverse(self):
		with self.assertRaisesRegex(art.ArtError, "has no tone"):
			self.parse("ab\ng \n\n\n")
		with self.assertRaisesRegex(art.ArtError, "space in the art"):
			self.parse("a \ngg\n\n\n")

	def test_unknown_letter_and_tone_not_in_the_logo(self):
		with self.assertRaisesRegex(art.ArtError, "not a tone letter"):
			self.parse("a\n!\n\n\n")
		with self.assertRaisesRegex(art.ArtError, "not in afterglow's tones|is not in"):
			self.parse("a\nr\n\n\n")

	def test_the_committed_forms_parse(self):
		for logo in self.manifest.logos:
			if logo.master.exists() and logo.ascii_path.exists():
				with self.subTest(logo=logo.name):
					art.parse_ascii_form(logo.ascii_path.read_text(encoding="utf-8"), logo, self.palette)


@unittest.skipUnless(HAVE_RENDERER, "rsvg-convert is not installed")
class MasterTests(unittest.TestCase):
	@classmethod
	def setUpClass(cls):
		cls.palette = art.load_palette()
		cls.manifest = art.load_manifest()
		cls.temp = tempfile.TemporaryDirectory()
		cls.directory = Path(cls.temp.name)

	@classmethod
	def tearDownClass(cls):
		cls.temp.cleanup()

	def make_logo(self, name, svg, tones=("gold", "orange")):
		master = self.directory / f"{name}.svg"
		master.write_text(svg, encoding="utf-8")
		return replace(
			self.manifest.logos[0],
			name=name,
			master=master,
			ascii_path=self.directory / "none.txt",
			status="brief",
			width=4,
			height=2,
			tone_names=list(tones),
		)

	def test_exact_fills_quantise_to_their_tones(self):
		logo = self.make_logo("exact", svg_master([("#ffd08e", 1, 0, 2, 1), ("#ffaa40", 0, 1, 4, 1)]))
		warnings = []
		rows = art.quantise(art.render_master(logo.master, 4, 2), logo, self.palette, 12.0, warnings)
		self.assertEqual(rows, [[0, 1, 1, 0], [2, 2, 2, 2]])
		self.assertEqual(warnings, [])

	def test_an_off_palette_colour_trips_the_guard(self):
		logo = self.make_logo("off", svg_master([("#ffd08e", 1, 0, 2, 1), ("#00ff00", 0, 1, 4, 1)]))
		with self.assertRaisesRegex(art.ArtError, r"pixel \(0, 1\) #00ff00 is off palette"):
			art.quantise(art.render_master(logo.master, 4, 2), logo, self.palette, 12.0, [])

	def test_a_near_colour_snaps_with_a_warning(self):
		logo = self.make_logo("near", svg_master([("#ffd18e", 1, 0, 2, 1)]))
		warnings = []
		rows = art.quantise(art.render_master(logo.master, 4, 2), logo, self.palette, 12.0, warnings)
		self.assertEqual(rows[0], [0, 1, 1, 0])
		self.assertEqual(len(warnings), 1)

	def test_a_tone_missing_from_the_logo_is_named(self):
		logo = self.make_logo("missing", svg_master([("#f43f5e", 0, 0, 1, 1)]))
		with self.assertRaisesRegex(art.ArtError, "tone rose, which is not in"):
			art.quantise(art.render_master(logo.master, 4, 2), logo, self.palette, 12.0, [])

	def test_build_logo_fails_on_an_off_palette_master(self):
		logo = self.make_logo("build-off", svg_master([("#123456", 0, 0, 4, 2)]))
		with self.assertRaisesRegex(art.ArtError, "off palette"):
			art.build_logo(logo, self.palette, self.manifest, [])

	def test_a_logo_with_no_master_is_skipped_unless_drawn(self):
		logo = replace(self.manifest.logos[0], master=self.directory / "absent.svg", status="brief")
		self.assertIsNone(art.build_logo(logo, self.palette, self.manifest, []))
		with self.assertRaises(art.ArtError):
			art.build_logo(replace(logo, status="drawn"), self.palette, self.manifest, [])

	def test_a_low_master_becomes_pixels_low(self):
		logo = self.make_logo("lowform", svg_master([("#ffd08e", 0, 0, 4, 2)]))
		self.directory.joinpath("lowform-low.svg").write_text(svg_master([("#ffaa40", 0, 0, 2, 2)]), encoding="utf-8")
		item = art.build_logo(logo, self.palette, self.manifest, [])
		self.assertEqual(item.decoded.pixels, [[1, 1, 1, 1]] * 2)
		self.assertEqual(item.decoded.low, [[2, 2, 0, 0]] * 2)

	def test_a_blob_over_budget_fails(self):
		item = art.build_logo(self.make_logo("budget", svg_master([("#ffd08e", 0, 0, 4, 2)])), self.palette, self.manifest, [])
		limits = {"blob-bytes": 10, "total-bytes": 6144}
		with self.assertRaisesRegex(art.ArtError, "blob budget"):
			art.check_budgets([item], limits)
		with self.assertRaisesRegex(art.ArtError, "all blobs together"):
			art.check_budgets([item], {"blob-bytes": 700, "total-bytes": 10})


@unittest.skipUnless(HAVE_RENDERER, "rsvg-convert is not installed")
class CommittedArtTests(unittest.TestCase):
	"""The real masters: blobs decode back to their pixels, runs are deterministic, and sizes match the specification."""

	@classmethod
	def setUpClass(cls):
		cls.temp = tempfile.TemporaryDirectory()
		cls.first = Path(cls.temp.name) / "first"
		cls.second = Path(cls.temp.name) / "second"
		cls.built = art.generate(cls.first, quiet=True)
		art.generate(cls.second, quiet=True)

	@classmethod
	def tearDownClass(cls):
		cls.temp.cleanup()

	def test_two_runs_are_byte_identical(self):
		first, second = art.generated_files(self.first), art.generated_files(self.second)
		self.assertEqual(sorted(first), sorted(second))
		for name in first:
			self.assertEqual(first[name].read_bytes(), second[name].read_bytes(), name)

	def test_every_blob_decodes_and_fits_its_budget(self):
		manifest = art.load_manifest()
		art.check_budgets(self.built, manifest.limits)
		for item in self.built:
			with self.subTest(logo=item.spec.name):
				decoded = art.decode_blob(item.blob)
				self.assertEqual(len(decoded.pixels), item.spec.height)
				self.assertEqual([art.load_palette().by_id(tone_id).name for tone_id in decoded.palette], item.spec.tone_names)

	def test_sizes_match_the_specification_table(self):
		sizes = {item.spec.name: len(item.blob) for item in self.built}
		self.assertEqual(sizes.get("afterglow", 178), 178)
		self.assertEqual(sizes.get("win32", 328), 328)

	def test_the_ascii_forms_decode_to_the_files(self):
		palette = art.load_palette()
		for item in self.built:
			if item.decoded.ascii_rows:
				text = item.spec.ascii_path.read_text(encoding="utf-8")
				rows = art.parse_ascii_form(text, item.spec, palette)
				self.assertEqual(item.decoded.ascii_art, [art_row for art_row, _ in rows])

	def test_the_generated_rust_names_every_logo(self):
		index = (self.first / "src" / "generated" / "index.rs").read_text(encoding="utf-8")
		for item in self.built:
			self.assertIn(f'include_bytes!("../../data/{item.spec.name}.afl")', index)
		self.assertIn("Do not edit; run `make art`.", index)

	def test_check_mode_flags_a_changed_file(self):
		crate = Path(self.temp.name) / "crate"
		art.generate(crate, quiet=True)
		self.assertEqual(art.check_committed(crate), [])
		first_blob = sorted((crate / "data").glob("*.afl"))[0]
		first_blob.write_bytes(first_blob.read_bytes() + b"x")
		self.assertEqual(len(art.check_committed(crate)), 1)


if __name__ == "__main__":
	unittest.main()
