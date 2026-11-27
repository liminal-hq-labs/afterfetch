#!/usr/bin/env bash
# Builds the Unix binaries in three different directories and compares the bytes. Usage: repro-unix.sh [target ...]
#
# The three paths differ in depth and length. The copies and their build output live in ~/.cache/afterfetch/repro-unix and are removed when
# the script ends.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
targets=("$@")
[ "${#targets[@]}" -gt 0 ] || targets=(x86_64-unknown-linux-musl i586-unknown-linux-musl)
work="${HOME}/.cache/afterfetch/repro-unix"
rm -rf "$work"
mkdir -p "$work"
trap 'rm -rf "$work"' EXIT

copies=(one/afterfetch two/second-place three/a/much/deeper/directory/with-a-longer-name)
for copy in "${copies[@]}"; do
	mkdir -p "$work/$copy"
	(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -I{} cp -P --parents {} "$work/$copy/")
	for target in "${targets[@]}"; do
		(cd "$work/$copy" && CARGO_TARGET_DIR="$work/$copy/target" bash platforms/unix/build.sh "$target" >/dev/null 2>&1)
	done
done

status=0
for target in "${targets[@]}"; do
	first="$work/${copies[0]}/target/$target/release/afterfetch"
	for copy in "${copies[@]:1}"; do
		if ! cmp -s "$first" "$work/$copy/target/$target/release/afterfetch"; then
			echo "FAIL repro $target: the build in $copy differs from the build in ${copies[0]}" >&2
			status=1
		fi
	done
	[ "$status" -ne 0 ] || echo "ok repro $target: byte-identical across ${#copies[@]} directories"
done
exit "$status"
