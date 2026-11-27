#!/usr/bin/env bash
# Runs both Windows 3.1 builds on Windows for Workgroups 3.11 in DOSBox-X (in Docker, on a virtual display of its own) and checks them.
# Usage: w31.sh [RENDER_DIR]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Three runs, each about a minute: the window program (a screenshot, which must show the Dusk navy and several colours), the same program in
# `/DUMP` mode (its plain text, checked for the labels and the 3.11 facts), and the DOS program in a DOS box (its screen read back from video
# memory, which must say it is in Windows 3.11). The screenshots go to RENDER_DIR (default ~/Documents/Liminal HQ/Afterfetch/renders/w31).
# Not part of `make check`: it needs Docker, the `quill16-dosbox-x:local` image and a Windows 3.11 tree (W31_TREE, never modified: it is copied).
# Skips, and says so, when any is missing.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
render="${1:-$HOME/Documents/Liminal HQ/Afterfetch/renders/w31}"
tree="${W31_TREE:-$HOME/.cache/quill16-play/w31-base}"
if ! command -v docker >/dev/null || ! docker image inspect "${DOSBOX_IMAGE:-quill16-dosbox-x:local}" >/dev/null 2>&1 || [ ! -d "$tree/WINDOWS" ]; then
	echo "SKIP smoke-w31: needs Docker, the DOSBox-X image and a Windows 3.11 tree at $tree; unverified"
	exit 0
fi
for exe in platforms/win16/dist/AFETCHW.EXE platforms/dos/dist/AFETCH.EXE; do
	[ -f "$root/$exe" ] || { echo "SKIP smoke-w31: $exe is not built"; exit 0; }
done
run="$root/platforms/win16/tools/run-w31.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$render"

# 1. The window.
"$run" --keep "$scratch/window" 'D:\AFETCHW.EXE' "$scratch/window" 45
colours="$(identify -format %k "$scratch/window-45.png")"
# The tree's 16-bit driver rounds the Dusk navy (#0A1230), so any dark blue counts.
navy="$(magick "$scratch/window-45.png" -format %c histogram:info:- | tr -d '(),:' | awk '$2 < 48 && $3 < 64 && $4 > $2 + 20 { total += $1 } END { print total + 0 }')"
if [ "$colours" -lt 12 ] || [ "$navy" -lt 1000 ]; then
	echo "FAIL smoke-w31: the window is not drawn ($colours colours, $navy Dusk navy pixels)" >&2
	exit 1
fi
cp "$scratch/window-45.png" "$render/afetchw-window.png"
echo "window on Windows 3.11: $colours colours, $navy Dusk navy pixels: ok"

# 2. The plain text of the same program.
"$run" --keep "$scratch/dump" 'D:\AFETCHW.EXE /DUMP=D:\W16OUT.TXT' "$scratch/dump" 40
[ -f "$scratch/dump/W16OUT.TXT" ] || { echo "FAIL smoke-w31: no /DUMP file" >&2; exit 1; }
tr -d '\r' <"$scratch/dump/W16OUT.TXT" >"$scratch/dump/out.txt"
for label in 'OS:' 'Kernel:' 'Uptime:' 'CPU:' 'Memory:' 'Display:' 'Shell:' 'Resources:'; do
	grep -q "$label" "$scratch/dump/out.txt" || { echo "FAIL smoke-w31: no $label row in /DUMP" >&2; exit 1; }
done
grep -q '386 enhanced mode' "$scratch/dump/out.txt" || { echo "FAIL smoke-w31: Windows did not say enhanced mode" >&2; exit 1; }
echo "/DUMP on Windows 3.11: every row and enhanced mode: ok"

# 3. The DOS program in a DOS box.
"$run" --keep "$scratch/box" 'D:\DOSBOX.BAT' "$scratch/box" 45
[ -f "$scratch/box/SCREEN.TXT" ] || { echo "FAIL smoke-w31: the DOS program wrote no screen dump" >&2; exit 1; }
cp "$scratch/box-45.png" "$render/afetch-dos-in-win311.png"
python3 -I "$root/scripts/smoke/dos-screen.py" "$scratch/box/SCREEN.TXT" "$root/tests/smoke/dos.labels"
grep -aq 'Win 3.11' "$scratch/box/SCREEN.TXT" || { echo "FAIL smoke-w31: the DOS box did not say Windows 3.11" >&2; exit 1; }
echo "DOS program in a Windows 3.11 DOS box: ok"
