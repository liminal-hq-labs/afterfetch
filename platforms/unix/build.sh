#!/usr/bin/env bash
# Builds, checks or lints the Unix shell for one target. Usage: build.sh <target> [build|clippy]
#
# The flags make a static program with no C library: no start files, no dynamic loader, no relocations, and an entry point of our own. The
# macOS programs are the exception: they link `libSystem` and nothing else.
# The source path is remapped so two builds in different directories give the same bytes.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

target="${1:?usage: build.sh <target> [build|clippy]}"
action="${2:-build}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

common=(
	-C linker=rust-lld
	-C force-unwind-tables=no
	"--remap-path-prefix=$root=/afterfetch"
	"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
)

case "$target" in
*-apple-darwin)
	# macOS: lld's Mach-O driver against the hand-written `libSystem.tbd` (no Apple SDK, AF-D5), entry `_main` through `LC_MAIN`. arm64 macOS needs a signature, which lld adds itself; the position-independent default model is what macOS requires.
	flags=("${common[@]}"
		-C linker-flavor=ld64.lld
		-C "link-arg=-L$here/macos"
		-C link-arg=-lSystem
		-C link-arg=-no_data_const
		-C link-arg=-no_uuid
	)
	# rustc adds the minimum system version itself (10.12 on x86-64, 11.0 on ARM64) and `-dead_strip`. It looks for an SDK to name in the
	# binary; there is none, so it is pointed at an empty directory instead of failing to find Xcode.
	export SDKROOT="$here/macos/no-sdk"
	;;
*)
	flags=("${common[@]}"
		-C linker-flavor=ld.lld
		-C relocation-model=static
		-C target-feature=+crt-static
		-C link-arg=-static
		-C link-arg=--gc-sections
		-C link-arg=--build-id=none
		-C link-arg=-znorelro
		-C link-arg=-znoseparate-code
		-C link-arg=--entry=_start
		-C link-arg=--strip-all
		-C link-arg=--no-eh-frame-hdr
	)
	# FreeBSD's kernel will not map two segments that share a page, so it has its own layout (AF-D55).
	case "$target" in
	*-freebsd) flags+=(-C "link-arg=-T$here/link-freebsd.ld") ;;
	*) flags+=(-C "link-arg=-T$here/link.ld") ;;
	esac
	# Linux's musl targets would link their own start files; the flag says not to. The FreeBSD targets have none, and reject the flag.
	case "$target" in
	*-linux-*) flags+=(-C link-self-contained=no) ;;
	esac
	;;
esac

# 32-bit Arm builds use Thumb code, which is denser than Arm code and keeps the program inside its size budget (AF-D47).
case "$target" in
arm-* | armv7-*) flags+=(-C target-feature=+thumb-mode) ;;
esac

# MIPS is tier 3: there is no prebuilt standard library, so `core` and `compiler_builtins` are built from the pinned toolchain's own source.
# `RUSTC_BOOTSTRAP=1` allows the unstable flags on 1.98.0 without a nightly toolchain (AF-D22).
cargo_extra=()
case "$target" in
mips-* | mipsel-*)
	export RUSTC_BOOTSTRAP=1
	cargo_extra+=(-Zbuild-std=core,compiler_builtins)
	;;
esac

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export CARGO_ENCODED_RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS="$(printf '%s\x1f' "${flags[@]}")"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"

cd "$root"
case "$action" in
build) cargo build --release --locked ${cargo_extra[@]+"${cargo_extra[@]}"} -p afterfetch-unix --target "$target" ;;
clippy) cargo clippy --release --locked ${cargo_extra[@]+"${cargo_extra[@]}"} -p afterfetch-unix --target "$target" -- -D warnings ;;
*) echo "unknown action: $action" >&2; exit 2 ;;
esac
