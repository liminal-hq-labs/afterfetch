#!/usr/bin/env bash
# Checks the structure of afterfetch.exe: a 32-bit Intel console program (subsystem 4.0, so Windows 95 and ReactOS load it), fixed at its
# base with no relocations, only code and read-only sections, and no C runtime. Usage: check-pe.sh <exe>
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

exe="${1:?usage: check-pe.sh <exe>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/objdump.sh"
objdump="$(find_objdump)"

fail() { echo "FAIL check-pe: $*" >&2; exit 1; }
[ -f "$exe" ] || fail "$exe does not exist"

if command -v file >/dev/null; then
	case "$(file -b "$exe")" in
	"PE32 executable for MS Windows 4.00 (console), Intel i386"* | "PE32 executable (console) Intel 80386, for MS Windows"*) ;;
	*) fail "file says: $(file -b "$exe")" ;;
	esac
fi

header="$("$objdump" -p "$exe")"
field() { awk -v key="$1" '$1 == key {print $2}' <<<"$header" | head -1; }
[ "$(field MajorSubsystemVersion)" = 4 ] && [ "$(field MinorSubsystemVersion)" = 0 ] || fail "the subsystem version is not 4.0"
grep -q "relocations stripped" <<<"$header" || fail "the executable has relocations (it is built /FIXED)"
grep -qi "Subsystem.*console\|Subsystem *00000003" <<<"$header" || fail "not a console program"

# Sections: code and read-only data only. A `.data` section would mean writable data the program should not have, and the
# `/MERGE:.data=.rdata` in .cargo/config.toml would make it fault, so the list must stay these two.
sections="$("$objdump" -h "$exe" | awk '$1 ~ /^[0-9]+$/ {print $2}' | tr '\n' ' ')"
[ "$sections" = ".text .rdata " ] || fail "sections are '$sections', expected '.text .rdata '"

dlls="$(awk '/DLL Name:/ {print tolower($3)}' <<<"$header" | sort -u | tr '\n' ' ')"
case "$dlls" in
*msvcrt* | *ucrt* | *vcruntime* | *api-ms-*) fail "imports a C runtime: $dlls" ;;
esac
echo "ok check-pe: PE32 console, subsystem 4.0, no relocations, sections .text .rdata, imports from $dlls"
