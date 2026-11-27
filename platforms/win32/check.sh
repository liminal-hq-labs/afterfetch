#!/usr/bin/env bash
# Checks a built Win32 executable: structure, imports, Pentium instructions, and the size budget. Usage: check.sh [exe]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exe="${1:-$here/dist/afterfetch.exe}"
budget=24576

bash "$here/tools/check-pe.sh" "$exe"
bash "$here/tools/check-imports.sh" "$exe"
bash "$here/tools/scan-pentium.sh" "$exe"
size="$(stat -c %s "$exe")"
if [ "$size" -gt "$budget" ]; then
	echo "FAIL check: $size bytes is over the $budget byte budget" >&2
	exit 1
fi
echo "ok check win32 i586: $size bytes of $budget"
