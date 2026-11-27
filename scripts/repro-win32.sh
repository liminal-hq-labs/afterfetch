#!/usr/bin/env bash
# Builds the Win32 executable in three different directories and compares the bytes. Usage: repro-win32.sh
#
# The three paths differ in depth and length, because a build that depends on the checkout path can hide behind two lucky ones. The copies and
# their build output live in ~/.cache/afterfetch/repro-win32 and are removed when the script ends.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="${HOME}/.cache/afterfetch/repro-win32"
rm -rf "$work"
mkdir -p "$work"
trap 'rm -rf "$work"' EXIT

copies=(one/afterfetch two/second-place three/a/much/deeper/directory/with-a-longer-name)
for copy in "${copies[@]}"; do
	mkdir -p "$work/$copy"
	(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -I{} cp -P --parents {} "$work/$copy/")
	(cd "$work/$copy" && CARGO_TARGET_DIR="$work/$copy/target" bash platforms/win32/build.sh i586 --out "$work/$copy/out" >/dev/null 2>&1)
done

first="$work/${copies[0]}/out/afterfetch.exe"
for copy in "${copies[@]:1}"; do
	if ! cmp -s "$first" "$work/$copy/out/afterfetch.exe"; then
		echo "FAIL repro win32 i586: the build in $copy differs from the build in ${copies[0]}" >&2
		exit 1
	fi
done
echo "ok repro win32 i586: byte-identical across ${#copies[@]} directories ($(sha256sum <"$first" | cut -d' ' -f1))"
