#!/usr/bin/env bash
# Checks a built Win32 executable: structure, imports, Pentium instructions (i586 only), and the size budget.
# Usage: check.sh [i586|x86_64|aarch64] [exe]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
arch="${1:-i586}"
exe="${2:-$here/dist/$arch/afterfetch.exe}"
# The budgets of SPEC section 11 and AF-D47, AF-D52: 24 KiB for the Pentium build, 32 KiB for the 64-bit ones.
case "$arch" in
i586) budget=24576 ;;
x86_64 | aarch64) budget=32768 ;;
*) echo "usage: check.sh [i586|x86_64|aarch64] [exe]" >&2; exit 2 ;;
esac

bash "$here/tools/check-pe.sh" "$arch" "$exe"
bash "$here/tools/check-imports.sh" "$arch" "$exe"
[ "$arch" != i586 ] || bash "$here/tools/scan-pentium.sh" "$exe"
size="$(stat -c %s "$exe")"
if [ "$size" -gt "$budget" ]; then
	echo "FAIL check: $size bytes is over the $budget byte budget" >&2
	exit 1
fi
echo "ok check win32 $arch: $size bytes of $budget"
