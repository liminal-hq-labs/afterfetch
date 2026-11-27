#!/usr/bin/env bash
# Runs the Linux binaries and checks their output by shape. Usage: linux-native.sh [x86_64|i586|aarch64|armv7|armv6|mips|mipsel ...] (default: all)
#
# The Arm and MIPS binaries run under qemu-user, called by name, as a named core (QEMU_CPU: a Pi-like Arm core, a 24Kc for MIPS); the README says what that does not prove.
#
# Plain and --json come from a pipe (`shape.py plain`); colour from `script -qec` at 16, 256, true colour and TERM=dumb. i586 runs natively
# when the kernel allows, else under qemu-i386-static, else SKIP.
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
[ "${#arches[@]}" -gt 0 ] || arches=(x86_64 i586 aarch64 armv7 armv6 mips mipsel)

smoke_one() {
	local arch="$1"
	local triple="$arch-unknown-linux-musl"
	local runner=()
	local cpu=""
	unset QEMU_CPU
	case "$arch" in
	armv7) triple=armv7-unknown-linux-musleabihf cpu=cortex-a7 runner=(qemu-arm-static) ;;
	armv6) triple=arm-unknown-linux-musleabihf cpu=arm1176 runner=(qemu-arm-static) ;;
	aarch64) cpu=cortex-a72 runner=(qemu-aarch64-static) ;;
	mips) triple=mips-unknown-linux-musl cpu=24Kc runner=(qemu-mips-static) ;;
	mipsel) triple=mipsel-unknown-linux-musl cpu=24Kc runner=(qemu-mipsel-static) ;;
	esac
	local binary="$root/target/$triple/release/afterfetch"
	if [ ! -x "$binary" ]; then
		echo "SKIP smoke-linux-native $arch: $binary not built"
		return 0
	fi
	if [ "${#runner[@]}" -gt 0 ]; then
		if ! command -v "${runner[0]}" >/dev/null; then
			echo "SKIP smoke-linux-native $arch: ${runner[0]} is not installed (unverified)"
			return 0
		fi
		echo "note: $arch runs under ${runner[0]} with the $cpu processor model, not the kernel"
		export QEMU_CPU="$cpu"
	elif ! "$binary" --version >/dev/null 2>&1; then
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
