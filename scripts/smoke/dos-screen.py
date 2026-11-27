# Checks a text screen read back from DOS video memory (`AF_DUMP`): the facts are there, each label has its colour, and the sun is drawn
# in the fill attributes of the console16 golden. Usage: dos-screen.py <SCREEN.TXT> <labels file>
#
# The dump is the characters of each row, a blank line, then each row's attributes as two hex digits per cell. The colours are the Dusk
# attributes of `crates/afterfetch-core/tests/golden/dos/console16-80.txt`: gold OS (0E), orange Kernel (06), coral Host (0C), rose CPU and
# Memory (04), lilac Display (0D), violet Shell (05), and the sun's fills (67 and C7).
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import sys

COLOURS = {"OS:": 0x0E, "Kernel:": 0x06, "Host:": 0x0C, "CPU:": 0x04, "Memory:": 0x04, "Display:": 0x0D, "Shell:": 0x05}


def main(path, labels_path):
	with open(path, "rb") as handle:
		raw = handle.read().replace(b"\r", b"").split(b"\n")
	rows = [line.decode("cp437") for line in raw[:25]]
	attributes = [[int(line[at:at + 2], 16) for at in range(0, len(line) - 1, 2)] for line in raw[26:51]]
	if len(rows) != 25 or len(attributes) != 25:
		sys.exit("FAIL: the dump is not 25 rows of characters and 25 of attributes")
	# The last block of rows that holds the OS label is the picture; an earlier screen may hold the command that ran it.
	with open(labels_path, encoding="utf-8") as handle:
		wanted = [line.strip().rstrip("?") for line in handle if line.strip() and not line.strip().endswith("?")]
	for label in wanted:
		found = [(number, row.find(label)) for number, row in enumerate(rows) if row[38:].startswith(label)]
		if not found:
			sys.exit(f"FAIL: no row starts with {label!r} at column 38")
		number, _ = found[-1]
		if label in COLOURS and attributes[number][38] != COLOURS[label]:
			sys.exit(f"FAIL: {label} is drawn in attribute {attributes[number][38]:02X}, not {COLOURS[label]:02X}")
	fills = {value for row in attributes for value in row}
	if not {0x67, 0xC7} <= fills:
		sys.exit("FAIL: the sun's fill attributes 67 and C7 are missing from the screen")
	print(f"screen read back: {len(wanted)} labels with their colours, and the sun's fills: ok")


if __name__ == "__main__":
	main(sys.argv[1], sys.argv[2])
