#!/usr/bin/env python3
# Checks the shape of afterfetch's output without comparing values, which differ on every machine. Usage:
#   shape.py plain <binary> <labels file> [--runner <emulator>] [--windows [--golden <file>] | --dos | --win16]
#   shape.py colour <capture file> <none|16|256|truecolour>
# `--windows`: CR LF line ends, no desktop row, no escapes on a pipe at any depth; `--golden` compares the win32 logo's columns.
# `--win16`: no memory total (Windows 3.1 reports only the free global heap), Windows 3.1's resources row, no title without Windows for
# Workgroups, and no exit status: a usage error is only a message in the output file.
# `--dos`: no uptime, no escapes on a pipe at any depth, a display row allowed; the runner has already turned DOS line ends into plain ones.
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
# A labels file lists one label per line in print order. A trailing `?` marks a label that may be absent.

import json
import os
import re
import subprocess
import sys

# Schema 1 (SPEC section 6): `schema` first, the rows, then the build's identity and the raw numbers.
JSON_KEYS = [
	"schema", "user", "hostname", "os", "kernel", "model", "uptime", "cpu", "memory", "display", "desktop", "terminal", "shell",
	"resources", "target", "arch", "uptime_seconds", "memory_available_bytes", "memory_total_bytes",
]
JSON_NUMBERS = ["uptime_seconds", "memory_available_bytes", "memory_total_bytes"]
JSON_REQUIRED = ["os", "kernel"]
MAX_COLUMNS = 80


class Failure(Exception):
	pass


def expect(condition, message):
	if not condition:
		raise Failure(message)


def run(command, extra_environment=None):
	environment = dict(os.environ)
	environment.pop("NO_COLOR", None)
	environment.pop("CLICOLOR_FORCE", None)
	environment.pop("FORCE_COLOR", None)
	environment.setdefault("TERM", "xterm-256color")
	environment.update(extra_environment or {})
	return subprocess.run(command, capture_output=True, env=environment, timeout=30, check=False)


def read_labels(path):
	labels = []
	with open(path, encoding="utf-8") as handle:
		for line in handle:
			line = line.strip()
			if line:
				labels.append((line.rstrip("?"), line.endswith("?")))
	return labels


WINDOWS = False
DOS = False
WIN16 = False
LOGO_COLUMNS = 42
LOGO_ROWS = 12


def lines_of(output, what):
	"""The output with Windows line ends turned into plain ones, after checking that every line end is the right kind."""
	if WINDOWS:
		expect(output.count(b"\n") == output.count(b"\r\n"), f"{what}: a bare line feed on a Windows line")
		return output.replace(b"\r\n", b"\n")
	return output


def check_plain_text(output, what):
	output = lines_of(output, what)
	expect(all(byte < 0x80 for byte in output), f"{what}: a byte of 0x80 or more in plain output")
	expect(b"\x1b" not in output, f"{what}: an escape sequence in plain output")
	expect(b"\x00" not in output, f"{what}: a NUL byte in plain output")
	expect(b"\r" not in output, f"{what}: a stray carriage return")
	expect(output.endswith(b"\n"), f"{what}: output does not end in a newline")
	for number, line in enumerate(output.decode("ascii").split("\n")[:-1], 1):
		expect(len(line) <= MAX_COLUMNS, f"{what}: line {number} is {len(line)} columns")


def check_labels(lines, labels):
	position = 0
	for label, optional in labels:
		found = next((at for at in range(position, len(lines)) if lines[at].startswith(label)), None)
		if found is None:
			expect(optional, f"label {label!r} missing or out of order")
		else:
			position = found + 1


def without_uptime(output):
	"""The output's lines but the uptime, which may tick over to the next minute between two runs, and the memory, whose free figure moves."""
	return [line for line in output.split(b"\n") if b"Uptime:" not in line and b"Memory:" not in line]


def check_golden(command, golden_path):
	with open(golden_path, encoding="ascii") as handle:
		golden = handle.read().split("\n")[:LOGO_ROWS]
	drawn = run(command + ["--logo", "win32", "--plain"])
	expect(drawn.returncode == 0, f"--logo win32 --plain exited {drawn.returncode}")
	got = lines_of(drawn.stdout, "golden").decode("ascii").split("\n")[:LOGO_ROWS]
	expect(len(got) == LOGO_ROWS, "fewer logo rows than the golden")
	for number, (want, have) in enumerate(zip(golden, got), 1):
		expect(want[:LOGO_COLUMNS].rstrip() == have[:LOGO_COLUMNS].rstrip(), f"logo row {number}: {have[:LOGO_COLUMNS]!r} is not {want[:LOGO_COLUMNS]!r}")
	print("logo equals the reactos-liminal golden")


def check_plain(command, labels_path, golden_path=None):
	labels = read_labels(labels_path)

	facts = run(command + ["--plain", "--no-logo"])
	expect(facts.returncode == 0, f"--plain --no-logo exited {facts.returncode}")
	check_plain_text(facts.stdout, "facts")
	lines = lines_of(facts.stdout, "facts").decode("ascii").split("\n")[:-1]
	expect(len(lines) >= 4, "too few lines")
	if WIN16 and lines[0].startswith("OS:"):
		# Windows 3.1 without Windows for Workgroups has no user or computer name, so there is no title and no rule.
		check_labels(lines, labels)
	else:
		expect(re.fullmatch(r"\S+@\S+|\S+", lines[0]) is not None, f"title line {lines[0]!r}")
		expect(set(lines[1]) == {"-"}, f"rule line {lines[1]!r}")
		check_labels(lines[2:], labels)

	picture = run(command + ["--plain"])
	expect(picture.returncode == 0, f"--plain exited {picture.returncode}")
	check_plain_text(picture.stdout, "picture")
	expect(len(lines_of(picture.stdout, "picture").splitlines()) >= len(lines), "the picture has fewer lines than the facts")

	piped = run(command)
	expect(without_uptime(piped.stdout) == without_uptime(picture.stdout), "a pipe does not get the same output as --plain")

	data = run(command + ["--json"])
	expect(data.returncode == 0, f"--json exited {data.returncode}")
	one_line = lines_of(data.stdout, "--json")
	expect(one_line.count(b"\n") == 1 and one_line.endswith(b"\n"), "--json is not one line")
	decoded = json.loads(data.stdout, object_pairs_hook=lambda pairs: pairs)
	expect([key for key, _ in decoded] == JSON_KEYS, f"--json keys {[key for key, _ in decoded]}")
	values = dict(decoded)
	expect(values["schema"] == 1, f"--json schema is {values['schema']!r}")
	for key, value in values.items():
		if key == "schema" or key in JSON_NUMBERS:
			continue
		expect(value is None or isinstance(value, str), f"--json {key} is {value!r}")
	for key in JSON_NUMBERS:
		value = values[key]
		expect(value is None or (isinstance(value, int) and not isinstance(value, bool) and value >= 0), f"--json {key} is {value!r}")
	if not DOS:
		expect(values["uptime_seconds"] is not None, "--json has no uptime_seconds")
	if WIN16:
		expect(values["memory_available_bytes"] and values["memory_total_bytes"] is None, "--json memory numbers")
	else:
		expect(values["memory_total_bytes"] and values["memory_available_bytes"] <= values["memory_total_bytes"], "--json memory numbers")
	expect(values["arch"], "--json has no arch")
	expect(values["target"] and re.fullmatch(r"[a-z0-9]+-[a-z0-9_]+", values["target"]), f"--json target {values['target']!r}")
	for key in JSON_REQUIRED:
		expect(values[key], f"--json {key} is empty")
	if WINDOWS:
		expect(values["desktop"] is None, "a Windows console program named a desktop")
		expect(values["cpu"], "a Windows program printed no CPU row")
		expect("\x00" not in values["cpu"], "the CPU row carries a NUL")
		expect(values["target"].startswith("windows-"), f"--json target {values['target']!r} is not a Windows target")
		# A machine with more than 4 GiB shows it: `GlobalMemoryStatusEx` is asked for, not the saturating `GlobalMemoryStatus`.
		with open("/proc/meminfo", encoding="ascii") as meminfo:
			host_kib = int(next(line for line in meminfo if line.startswith("MemTotal:")).split()[1])
		if host_kib > 6 * 1024 * 1024:
			expect(values["memory_total_bytes"] > 4294967295, f"memory_total_bytes {values['memory_total_bytes']} is capped at 4 GiB")
	elif not (DOS or WIN16):
		expect(values["display"] is None, "a console program reported a display")
	if DOS:
		expect(values["uptime"] is None and values["terminal"] is None and values["desktop"] is None, "DOS reported a fact it cannot know")
		expect(values["cpu"] and values["memory_total_bytes"], "DOS reported no CPU or memory")
		expect(values["target"] == "dos-i386", f"--json target {values['target']!r}")
	if WIN16:
		expect(values["resources"] and values["display"] and values["target"] == "win16-i386", "Windows 3.1 reported too little")
		expect(values["desktop"] is None and values["terminal"] is None, "Windows 3.1 reported a terminal")
	else:
		expect(values["resources"] is None, "a Windows 3.1 fact on Unix")

	version = run(command + ["--version"])
	expect(version.returncode == 0 and version.stdout.startswith(b"afterfetch "), "--version")
	expect(version.stdout.decode("ascii").rstrip().endswith(f"({values['target']})"), f"--version {version.stdout!r} does not name the JSON target")
	expect(run(command + ["--help"]).returncode == 0, "--help")
	bad = run(command + ["--no-such-option"])
	# DOS has no way to capture standard error under the harness, so there only the exit code and the empty standard output are checked.
	if WIN16:
		# A Windows 3.1 program has no exit status Wine hands back, and no standard error: the message is in the output file.
		expect(b"unknown option" in bad.stdout, "a bad option gives a message")
	else:
		expect(bad.returncode == 2 and bad.stdout == b"" and (bad.stderr or DOS), "a bad option gives exit 2 and a message on stderr")

	if WINDOWS or DOS or WIN16:
		# A pipe never gets escape sequences from the Win32 or the DOS program, whatever depth is asked for (the console path is attributes).
		for depth in ("16", "256", "truecolour", "mono"):
			asked = run(command + ["--colour", depth])
			expect(b"\x1b" not in asked.stdout, f"--colour {depth} wrote an escape sequence to a pipe")
		slash = run(command + ["/PLAIN", "/NOLOGO"])
		expect(without_uptime(slash.stdout) == without_uptime(facts.stdout), "/PLAIN /NOLOGO differs from --plain --no-logo")
		if golden_path:
			check_golden(command, golden_path)
	else:
		desktop = run(command + ["--json"], {"XDG_CURRENT_DESKTOP": "Sunset"})
		expect(json.loads(desktop.stdout)["desktop"] == "Sunset", "XDG_CURRENT_DESKTOP is not reported")
		quiet = run(command + ["--json"], {"XDG_CURRENT_DESKTOP": ""})
		expect(json.loads(quiet.stdout)["desktop"] is None, "an empty XDG_CURRENT_DESKTOP gave a row")

		if os.path.exists("/dev/full"):
			with open("/dev/full", "wb") as full:
				failed = subprocess.run(command + ["--plain"], stdout=full, stderr=subprocess.DEVNULL, check=False)
		else:
			# macOS and FreeBSD have no /dev/full: a closed standard output is a write that fails.
			failed = subprocess.run(command + ["--plain"], stderr=subprocess.DEVNULL, preexec_fn=lambda: os.close(1), check=False)
		expect(failed.returncode == 74, f"output to a full device exited {failed.returncode}, not 74")

		forced = run(command + ["--colour", "truecolour"], {"NO_COLOR": "1"})
		expect(b"\x1b[0m" in forced.stdout and b"38;2;" in forced.stdout, "--colour truecolour did not draw colour on a pipe")
	print("plain shape: ok")


DEPTH_MARKS = {"16": (b"\x1b[9", b"38;5;", b"38;2;"), "256": (b"38;5;",), "truecolour": (b"38;2;",)}


def check_colour(capture_path, depth):
	with open(capture_path, "rb") as handle:
		captured = handle.read().replace(b"\r\n", b"\n")
	if depth == "none":
		expect(b"\x1b" not in captured, "colour drawn at depth none")
	else:
		expect(b"\x1b[0m" in captured, "no reset sequence")
		expect(captured.rstrip().endswith(b"\x1b[0m") or captured.endswith(b"\n"), "output ends oddly")
		if depth == "16":
			expect(re.search(rb"\x1b\[(3[0-7]|9[0-7])m", captured) is not None, "no 16-colour sequence")
			expect(b"38;5;" not in captured and b"38;2;" not in captured, "a deeper colour at depth 16")
		elif depth == "256":
			expect(b"38;5;" in captured and b"38;2;" not in captured, "not 256-colour")
		else:
			expect(b"38;2;" in captured, "not true colour")
	print(f"colour {depth}: ok")


def main(argv):
	try:
		if len(argv) >= 4 and argv[1] == "plain":
			global WINDOWS, DOS, WIN16
			WINDOWS = "--windows" in argv
			DOS = "--dos" in argv
			WIN16 = "--win16" in argv
			golden = argv[argv.index("--golden") + 1] if "--golden" in argv else None
			command = argv[2:3]
			if "--runner" in argv:
				command = [argv[argv.index("--runner") + 1]] + command
			check_plain(command, argv[3], golden)
		elif len(argv) == 4 and argv[1] == "colour":
			check_colour(argv[2], argv[3])
		else:
			print(__doc__, file=sys.stderr)
			return 2
	except Failure as failure:
		print(f"FAIL: {failure}", file=sys.stderr)
		return 1
	return 0


if __name__ == "__main__":
	sys.exit(main(sys.argv))
