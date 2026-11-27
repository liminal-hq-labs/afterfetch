#!/usr/bin/env bash
# Runs AFETCHW.EXE under Wine in `/DUMP` mode and checks its output by shape (`shape.py plain --win16`), then opens its window on a virtual X
# server of its own and checks that the screen is not blank. Usage: wine16.sh
#
# Wine here is a smoke test only: it is more forgiving than Windows 3.11, and reports itself as Windows 3.95 (the shape does not look at the
# version). The prefix is its own, under ~/.cache/afterfetch/wine-win16, and the X server is Xvfb on its own display; nothing touches the host's
# desktop. Skips, and says so, when the build, Wine or Xvfb is missing.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exe="$root/platforms/win16/dist/AFETCHW.EXE"
for tool in wine Xvfb import identify magick; do
	command -v "$tool" >/dev/null || { echo "SKIP smoke-wine16: $tool not found; unverified"; exit 0; }
done
[ -f "$exe" ] || { echo "SKIP smoke-wine16: no AFETCHW.EXE"; exit 0; }

export WINEPREFIX="${WINE_PREFIX_DIR:-$HOME/.cache/afterfetch/wine-win16}"
export WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml=" DISPLAY="${WINE16_DISPLAY:-:95}"
scratch="$(mktemp -d)"
Xvfb "$DISPLAY" -screen 0 800x600x24 -nolisten tcp >/dev/null 2>&1 &
xvfb_pid=$!
wine_pid=""
cleanup() {
	[ -z "$wine_pid" ] || kill "$wine_pid" 2>/dev/null || true
	wineserver -k 2>/dev/null || true
	kill "$xvfb_pid" 2>/dev/null || true
	rm -rf "$scratch"
}
trap cleanup EXIT
sleep 1
[ -d "$WINEPREFIX/drive_c" ] || wine wineboot -i >/dev/null 2>&1

python3 -I "$root/scripts/smoke/shape.py" plain "$exe" "$root/tests/smoke/win16.labels" --win16 \
	--runner "$root/platforms/win16/tools/wine-runner.sh"

cp "$exe" "$WINEPREFIX/drive_c/windows/AFETCHW.EXE"
wine 'C:\windows\AFETCHW.EXE' >/dev/null 2>&1 &
wine_pid=$!
sleep 8
import -window root "$scratch/window.png"
# A window with the picture in it has the navy of the Dusk table and several other colours; a failed start is the X server's black.
colours="$(identify -format %k "$scratch/window.png")"
navy="$(magick "$scratch/window.png" txt:- | grep -c '#0A1230' || true)"
if [ "$colours" -lt 8 ] || [ "$navy" -lt 1000 ]; then
	echo "FAIL: the window is not drawn ($colours colours, $navy Dusk navy pixels)" >&2
	exit 1
fi
echo "window drawn on the virtual display: $colours colours, $navy Dusk navy pixels: ok"
