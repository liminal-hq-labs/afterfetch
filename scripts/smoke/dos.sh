#!/usr/bin/env bash
# Runs AFETCH.EXE under headless DOSBox Staging (a 486, `cputype = 486`, so an instruction a 486 lacks faults) and checks it. Usage: dos.sh
#
# Plain, `/JSON` and the slash options go through `shape.py plain --dos`, the DOS runner capturing standard output to a file; the colour run
# leaves the output on the console and reads the screen back from video memory (`AF_DUMP`), then checks each label's attribute against the
# console16 golden's; and a dump file that cannot be created (the Z: drive is read-only) must exit 74. Skips, and says so, when the DOS tools
# or the build are absent.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exe="$root/platforms/dos/dist/AFETCH.EXE"
export DOS_VENDOR="${DOS_VENDOR:-$HOME/.cache/afterfetch/dos-vendor}"
if [ ! -f "$exe" ] || [ -z "$(ls "$DOS_VENDOR"/dosbox/*/dosbox 2>/dev/null)" ]; then
	echo "SKIP smoke-dos: no AFETCH.EXE or no DOSBox in $DOS_VENDOR (run platforms/dos/tools/fetch-tools.sh); unverified"
	exit 0
fi
run="$root/platforms/dos/tools/dosrun.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cp "$exe" "$scratch/AFETCH.EXE"

python3 -I "$root/scripts/smoke/shape.py" plain "$scratch/AFETCH.EXE" "$root/tests/smoke/dos.labels" --dos \
	--runner "$root/platforms/dos/tools/dos-runner.sh"

DOS_CONSOLE=1 DOS_ENV='AF_DUMP=D:\SCREEN.TXT' DOS_KEEP="$scratch/keep" "$run" "$scratch" AFETCH.EXE
python3 -I "$root/scripts/smoke/dos-screen.py" "$scratch/keep/SCREEN.TXT" "$root/tests/smoke/dos.labels"

set +e
DOS_CONSOLE=1 DOS_ENV='AF_DUMP=Z:\NOPE.TXT' "$run" "$scratch" AFETCH.EXE >/dev/null 2>&1
code=$?
set -e
[ "$code" = 74 ] || { echo "FAIL: an unwritable dump exited $code, not 74" >&2; exit 1; }
echo "exit 74 on an output failure: ok"
