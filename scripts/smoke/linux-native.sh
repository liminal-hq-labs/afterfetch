#!/usr/bin/env bash
# Runs the Linux binaries on this machine and checks their output by shape. Usage: linux-native.sh [x86_64|i586 ...] (default: both)
#
# Plain output and --json come from a pipe (`shape.py plain`). Colour comes from a pseudo-terminal made with `script -qec`, at 16, 256 and
# true colour and with TERM=dumb. The i586 binary runs natively when the kernel allows 32-bit programs, otherwise under qemu-i386-static,
# and otherwise the smoke says SKIP and the build is unverified.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
shape="$root/scripts/smoke/shape.py"
labels="$root/tests/smoke/linux.labels"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

if ! command -v python3 >/dev/null; then
	echo "SKIP smoke-linux-native: python3 not found"
	exit 0
fi

arches=("$@")
[ "${#arches[@]}" -gt 0 ] || arches=(x86_64 i586)

smoke_one() {
	local arch="$1"
	local binary="$root/target/$arch-unknown-linux-musl/release/afterfetch"
	local runner=()
	if [ ! -x "$binary" ]; then
		echo "SKIP smoke-linux-native $arch: $binary not built"
		return 0
	fi
	if ! "$binary" --version >/dev/null 2>&1; then
		if command -v qemu-i386-static >/dev/null && [ "$arch" = i586 ]; then
			runner=(qemu-i386-static)
			echo "note: $arch runs under qemu-i386-static, not the kernel"
		else
			echo "SKIP smoke-linux-native $arch: this kernel cannot run it and qemu-i386-static is not installed (unverified)"
			return 0
		fi
	fi
	echo "== $arch"
	if [ "${#runner[@]}" -gt 0 ]; then
		python3 -I "$shape" plain "$binary" "$labels" --runner "${runner[0]}"
	else
		python3 -I "$shape" plain "$binary" "$labels"
	fi
	if ! command -v script >/dev/null; then
		echo "SKIP smoke-linux-native $arch colour: script not found"
		return 0
	fi
	local depth term colorterm
	for depth in 16 256 truecolour none; do
		case "$depth" in
		16) term=xterm colorterm="" ;;
		256) term=xterm-256color colorterm="" ;;
		truecolour) term=xterm-256color colorterm=truecolor ;;
		none) term=dumb colorterm="" ;;
		esac
		# Wrapped in env so the variables reach the program, with COLORTERM empty when it should be unset.
		env -u NO_COLOR -u CLICOLOR_FORCE -u FORCE_COLOR TERM="$term" COLORTERM="$colorterm" \
			script -qec "${runner[*]:-} $binary" /dev/null </dev/null >"$scratch/$arch-$depth.txt"
		python3 -I "$shape" colour "$scratch/$arch-$depth.txt" "$depth"
	done
}

for arch in "${arches[@]}"; do
	smoke_one "$arch"
done
