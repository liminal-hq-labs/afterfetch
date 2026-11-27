#!/usr/bin/env bash
# Builds the Desk module in three different directories and compares the bytes. Usage: repro-desk.sh
#
# The three paths differ in depth and length, because a build that depends on the checkout path can hide behind two lucky ones. The copies and
# their build output live in a folder of their own under ~/.cache/afterfetch (one per run, so two runs never meet) and are removed when the
# script ends.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "${HOME}/.cache/afterfetch"
work="$(mktemp -d "${HOME}/.cache/afterfetch/repro-desk.XXXXXX")"
trap 'rm -rf "$work"' EXIT

copies=(one/afterfetch two/second-place three/a/much/deeper/directory/with-a-longer-name)
for copy in "${copies[@]}"; do
	mkdir -p "$work/$copy"
	(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -I{} cp -P --parents {} "$work/$copy/")
done
for copy in "${copies[@]}"; do
	(cd "$work/$copy" && CARGO_TARGET_DIR="$work/$copy/target" bash platforms/desk/build.sh >/dev/null 2>&1) || {
		echo "FAIL repro desk: the build in $copy failed" >&2
		exit 1
	}
done
first="$work/${copies[0]}/platforms/desk/dist/AFETCH.DMD"
for copy in "${copies[@]:1}"; do
	if ! cmp -s "$first" "$work/$copy/platforms/desk/dist/AFETCH.DMD"; then
		echo "FAIL repro desk: the build in $copy differs from the build in ${copies[0]}" >&2
		exit 1
	fi
done
echo "ok repro desk: byte-identical across ${#copies[@]} directories ($(sha256sum <"$first" | cut -d' ' -f1))"
