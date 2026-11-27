#!/usr/bin/env bash
# Builds, checks or lints the Unix shell for one target. Usage: build.sh <target> [build|clippy]
#
# The flags make a static program with no C library: no start files, no dynamic loader, no relocations, and an entry point of our own.
# The source path is remapped so two builds in different directories give the same bytes.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

target="${1:?usage: build.sh <target> [build|clippy]}"
action="${2:-build}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

flags=(
	-C linker=rust-lld
	-C linker-flavor=ld.lld
	-C relocation-model=static
	-C target-feature=+crt-static
	-C force-unwind-tables=no
	-C link-self-contained=no
	-C link-arg=-static
	-C link-arg=--gc-sections
	-C link-arg=--build-id=none
	-C link-arg=-znorelro
	-C link-arg=-znoseparate-code
	-C link-arg=--entry=_start
	-C link-arg=--strip-all
	-C link-arg=--no-eh-frame-hdr
	-C "link-arg=-T$here/link.ld"
	"--remap-path-prefix=$root=/afterfetch"
	"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
)

# 32-bit Arm builds use Thumb code, which is denser than Arm code and keeps the program inside its size budget (AF-D47).
case "$target" in
arm-* | armv7-*) flags+=(-C target-feature=+thumb-mode) ;;
esac

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export CARGO_ENCODED_RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS="$(printf '%s\x1f' "${flags[@]}")"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"

cd "$root"
case "$action" in
build) cargo build --release --locked -p afterfetch-unix --target "$target" ;;
clippy) cargo clippy --release --locked -p afterfetch-unix --target "$target" -- -D warnings ;;
*) echo "unknown action: $action" >&2; exit 2 ;;
esac
