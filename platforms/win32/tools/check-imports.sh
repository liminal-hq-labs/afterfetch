#!/usr/bin/env bash
# Checks that every import of afterfetch.exe is on imports.allow (the functions Windows 95, NT and ReactOS all export), and that none is
# a wide-character entry point. The list and the executable must agree both ways: an allowed function that is not imported is stale.
# Usage: check-imports.sh <i586|x86_64|aarch64> <exe>
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

arch="${1:?usage: check-imports.sh <i586|x86_64|aarch64> <exe>}"
exe="${2:?usage: check-imports.sh <i586|x86_64|aarch64> <exe>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/objdump.sh"
objdump="$(find_objdump)"

fail() { echo "FAIL check-imports: $*" >&2; exit 1; }

got="$("$objdump" -p "$exe" | awk '/DLL Name:/ {dll=tolower($3)} /^ +[0-9]+ +[A-Za-z_]/ {print dll "!" $2}' | sort -u)"
# A line ending in `64` is for the 64-bit builds only.
allow="$(grep -v '^#' "$here/../imports.allow" | grep -v '^$' | { if [ "$arch" = i586 ]; then grep -v ' 64$'; else cat; fi; } | awk '{print $1}' | sort -u)"
[ -n "$got" ] || fail "no imports found (is $exe a PE file?)"

lower() { tr '[:upper:]' '[:lower:]'; }
while IFS= read -r import; do
	grep -qixF "$import" <<<"$allow" || fail "not on the allowlist: $import"
	case "$import" in *W) fail "wide-character entry point: $import" ;; esac
done <<<"$got"
stale="$(comm -13 <(lower <<<"$got" | sort -u) <(lower <<<"$allow" | sort -u) | tr '\n' ' ')"
[ -z "$stale" ] || fail "imports.allow lists functions the executable does not import: $stale"
echo "ok check-imports: $(wc -l <<<"$got") imports, all on the allowlist, none wide"
