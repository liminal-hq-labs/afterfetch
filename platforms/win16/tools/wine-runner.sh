#!/usr/bin/env bash
# Runs AFETCHW.EXE under Wine in `/DUMP` mode as `shape.py`'s runner: `wine-runner.sh <AFETCHW.EXE> [arguments...]`. The program writes its
# plain text to a file in the Wine prefix instead of opening a window; this prints it with DOS line ends removed and exits with the program's
# own status. WINEPREFIX and DISPLAY come from the caller (scripts/smoke/wine16.sh sets both).
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

exe="${1:?usage: wine-runner.sh <AFETCHW.EXE> [arguments...]}"
shift
prefix="${WINEPREFIX:?WINEPREFIX is not set}"
cp "$exe" "$prefix/drive_c/windows/AFETCHW.EXE"
rm -f "$prefix/drive_c/afetchw-out.txt"
status=0
WINEDEBUG=-all wine 'C:\windows\AFETCHW.EXE' "$@" '/DUMP=C:\afetchw-out.txt' >/dev/null 2>&1 || status=$?
[ ! -f "$prefix/drive_c/afetchw-out.txt" ] || tr -d '\r' <"$prefix/drive_c/afetchw-out.txt"
exit "$status"
