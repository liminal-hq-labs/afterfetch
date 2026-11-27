#!/usr/bin/env bash
# Checks a built AFETCH.EXE: a DOS executable with the go32 stub, the 386 ELF it was packed from, only 386 instructions, no floating point,
# and within its size budget. Usage: check.sh [AFETCH.EXE]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exe="${1:-$here/dist/AFETCH.EXE}"
elf="${CARGO_TARGET_DIR:-$here/target}/dos-elf/release/afetch"
budget=65536 # SPEC section 11: 64 KiB with the CWSDPMI stub included

fail() {
	echo "FAIL check dos: $*" >&2
	exit 1
}

[ -f "$exe" ] || fail "$exe does not exist"
[ "$(head -c 2 "$exe")" = MZ ] || fail "not a DOS executable (no MZ header)"
grep -aq "go32stub" "$exe" || fail "the go32 stub is not there"
size="$(stat -c %s "$exe")"
[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"

if [ -f "$elf" ]; then
	header="$(llvm-readelf -h "$elf")"
	grep -q "Class:.*ELF32" <<<"$header" || fail "the ELF is not 32-bit"
	grep -q "Machine:.*Intel 80386" <<<"$header" || fail "the ELF is not for an 80386"
	! llvm-readelf -l "$elf" | grep -q INTERP || fail "the ELF has a PT_INTERP"
	bash "$here/tools/scan-386.sh" "$elf"
else
	echo "note: $elf not found, so the instruction scan was skipped"
fi
echo "ok check dos: $size bytes of $budget"
