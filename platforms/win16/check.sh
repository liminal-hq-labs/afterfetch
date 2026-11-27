#!/usr/bin/env bash
# Checks a built AFETCHW.EXE: an NE executable for Windows, within its size budget, with code segments inside theirs, and only 386
# instructions. Usage: check.sh [AFETCHW.EXE]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Nothing here needs Windows, Wine or an emulator. The packer fails a build whose code segment passes its budget in res/afetchw.segments.json;
# this script adds the whole-file limit (AF-D58), the NE check, the instruction check (386 only) and the branch check (every near branch lands
# on an instruction in its own segment).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exe="${1:-$here/dist/AFETCHW.EXE}"
budget=40960

fail() {
	echo "FAIL check win16: $*" >&2
	exit 1
}

[ -f "$exe" ] || fail "$exe does not exist"
desc="$(file "$exe")"
grep -Eq 'NE (version [0-9.]+ )?for MS Windows' <<<"$desc" || fail "not an NE executable for Windows: $desc"
size="$(stat -c %s "$exe")"
[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"
python3 -I "$here/tools/check_segments.py" "$exe"
python3 -I "$here/tools/check_cpu.py" "$exe"
python3 -I "$here/tools/ne_dump.py" "$exe" | grep -qE '^(resident|non-resident) names.*AFETCHW' || fail "the module is not named AFETCHW"
echo "ok check win16: $size bytes of $budget, an NE executable named AFETCHW"
