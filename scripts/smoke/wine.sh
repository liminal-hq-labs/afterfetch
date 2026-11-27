#!/usr/bin/env bash
# Runs the Win32 executable under Wine on this machine and checks its output by shape. Usage: wine.sh [exe]
#
# Plain output, --json, the usage error and the logo against the reactos-liminal golden come from a pipe (`shape.py plain --windows`).
# Wine gets its own virtual display (Xvfb on a free number, started and stopped here) and a throwaway prefix in
# ~/.cache/afterfetch/wine, so nothing touches the real desktop or the real keyboard. The console-attribute stream is covered by the core's
# goldens and the attribute tests, since a console under Wine needs a window. Says SKIP when Wine, Xvfb or python3 is missing.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exe="${1:-$root/platforms/win32/dist/afterfetch.exe}"
shape="$root/scripts/smoke/shape.py"
labels="$root/tests/smoke/win32.labels"
golden="$root/crates/afterfetch-core/tests/golden/win32-plain-80.txt"
prefix="${HOME}/.cache/afterfetch/wine"

for tool in wine Xvfb python3; do
	if ! command -v "$tool" >/dev/null; then
		echo "SKIP smoke-wine: $tool not found"
		exit 0
	fi
done
if [ ! -f "$exe" ]; then
	echo "SKIP smoke-wine: $exe not built"
	exit 0
fi
# A prefix is a few hundred megabytes and a lot of memory while it boots; do not start one when the machine is short.
available_mb="$(awk '/MemAvailable/ {print int($2 / 1024)}' /proc/meminfo)"
if [ "$available_mb" -lt 2048 ]; then
	echo "SKIP smoke-wine: only $available_mb MB of memory is free"
	exit 0
fi

# A display number nobody uses: the first one with no X socket and no lock file.
display=""
for number in $(seq 90 130); do
	if [ ! -e "/tmp/.X11-unix/X$number" ] && [ ! -e "/tmp/.X$number-lock" ]; then
		display=":$number"
		break
	fi
done
[ -n "$display" ] || { echo "FAIL smoke-wine: no free X display number" >&2; exit 1; }

mkdir -p "$prefix"
Xvfb "$display" -screen 0 1024x768x24 -nolisten tcp >/dev/null 2>&1 &
xvfb_pid=$!
capture=""
# Only what was started here is stopped: the display by its process number, Wine's server by asking it, through this prefix alone.
cleanup() {
	[ -z "$capture" ] || rm -f "$capture"
	WINEPREFIX="$prefix" wineserver -k 2>/dev/null || true
	kill "$xvfb_pid" 2>/dev/null || true
	wait "$xvfb_pid" 2>/dev/null || true
}
trap cleanup EXIT
for _ in $(seq 50); do
	[ -e "/tmp/.X11-unix/X${display#:}" ] && break
	sleep 0.1
done
[ -e "/tmp/.X11-unix/X${display#:}" ] || { echo "FAIL smoke-wine: Xvfb did not start" >&2; exit 1; }

export DISPLAY="$display" WINEPREFIX="$prefix" WINEDEBUG=-all WINEDLLOVERRIDES="winemenubuilder.exe=d;mscoree=d;mshtml=d"
unset WAYLAND_DISPLAY
# The first run makes the prefix; give it time, and keep its noise out of the checks.
timeout 180 wine wineboot --init >/dev/null 2>&1 || true
timeout 60 wine "$exe" --version >/dev/null 2>&1 || { echo "FAIL smoke-wine: Wine cannot run $exe" >&2; exit 1; }

echo "== win32 i586 under $(wine --version)"
# Wine makes the program's standard output a pipe, which is what the plain path is for. The shape checker starts `wine <exe>` for each run.
python3 -I "$shape" plain "$exe" "$labels" --runner wine --windows --golden "$golden"

# The console path: under a pseudo-terminal Wine gives the program a console and turns its attribute calls into escape sequences, so the
# capture shows what the program asked of the console. The sun's gold must be there, the swatches' backgrounds, and a reset at the end.
if command -v script >/dev/null; then
	capture="$(mktemp)"
	TERM=xterm timeout 90 script -qec "wine $exe" /dev/null </dev/null >"$capture" || true
	python3 -I - "$capture" <<'PY'
import re
import sys

captured = open(sys.argv[1], "rb").read()
plain = re.sub(rb"\x1b\[[0-9;?]*[A-Za-z]", b"", captured)
assert b"OS:" in plain and b"Kernel:" in plain, "the console run printed no facts"
assert b"\x1b[93m" in captured, "no gold (console attribute 14) text"
assert re.search(rb"\x1b\[(10[0-7]|4[0-7])m", captured), "no background colour for the sun or the swatches"
assert captured.rstrip().endswith(b"\x1b[?25h") and b"\x1b[m" in captured, "the console's own attribute is not restored"
print("console attributes: ok")
PY
else
	echo "SKIP smoke-wine console: script not found"
fi
