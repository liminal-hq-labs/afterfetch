#!/usr/bin/env bash
# Builds the Win32 executable twice, in two different directories, and compares the bytes. Usage: repro-win32.sh
#
# The copies and their build output live in ~/.cache/afterfetch/repro-win32 and are removed when the script ends.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="${HOME}/.cache/afterfetch/repro-win32"
rm -rf "$work"
mkdir -p "$work"
trap 'rm -rf "$work"' EXIT

for copy in one/afterfetch two/second-place; do
	mkdir -p "$work/$copy"
	(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -I{} cp --parents {} "$work/$copy/")
	(cd "$work/$copy" && CARGO_TARGET_DIR="$work/$copy/target" bash platforms/win32/build.sh i586 --out "$work/$copy/out" >/dev/null 2>&1)
done

if cmp -s "$work/one/afterfetch/out/afterfetch.exe" "$work/two/second-place/out/afterfetch.exe"; then
	echo "ok repro win32 i586: byte-identical across two directories ($(sha256sum <"$work/one/afterfetch/out/afterfetch.exe" | cut -d' ' -f1))"
else
	echo "FAIL repro win32 i586: the two builds differ" >&2
	exit 1
fi
