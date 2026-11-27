#!/usr/bin/env bash
# Runs the macOS binary on a macOS machine (a GitHub macOS runner) and checks its output by shape. Usage: macos.sh [arm64|x86_64]
#
# Not runnable on this development machine: there is no macOS here (AF-D5, AF-D56). Plain and --json come from a pipe (`shape.py plain`);
# colour from BSD `script`, which gives the program a pseudo-terminal. x86_64 runs under Rosetta 2 on an Arm runner, as the same program.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
shape="$root/scripts/smoke/shape.py"
labels="$root/tests/smoke/macos.labels"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

arches=("$@")
[ "${#arches[@]}" -gt 0 ] || arches=(arm64 x86_64)

for arch in "${arches[@]}"; do
	case "$arch" in
	arm64) triple=aarch64-apple-darwin ;;
	x86_64) triple=x86_64-apple-darwin ;;
	*) echo "macos.sh: unknown arch $arch" >&2; exit 2 ;;
	esac
	binary="$root/target/$triple/release/afterfetch"
	echo "== $arch"
	python3 -I "$shape" plain "$binary" "$labels"
	for depth in 16 256 truecolour none; do
		case "$depth" in
		16) term=xterm colorterm="" ;;
		256) term=xterm-256color colorterm="" ;;
		truecolour) term=xterm-256color colorterm=truecolor ;;
		none) term=dumb colorterm="" ;;
		esac
		# BSD `script` takes the file first and the command after it, and needs a terminal on standard input to copy; `</dev/null` is enough
		# because it only reads input to forward it.
		env -u NO_COLOR -u CLICOLOR_FORCE -u FORCE_COLOR TERM="$term" COLORTERM="$colorterm" \
			script -q "$scratch/$arch-$depth.txt" "$binary" </dev/null >/dev/null
		python3 -I "$shape" colour "$scratch/$arch-$depth.txt" "$depth"
	done
done
