#!/usr/bin/env bash
# Builds AFETCH.EXE, the DOS program, into platforms/dos/dist with no C compiler and no C library. Usage: build.sh [build|clippy]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The Rust program (`no_std`, target `dos-elf.json`, `core` built from source) is linked by rust-lld with `link.ld` into a 32-bit ELF, and
# `tools/elf2go32.py` packs that behind CWSDPMI's `CWSDSTUB.EXE`, the go32 stub with the DPMI host inside it, so the program runs on a machine
# with no DPMI host and needs no CWSDPMI.EXE beside it. The stub comes from `tools/fetch-tools.sh` (set DOS_VENDOR to share one copy, default
# ~/.cache/afterfetch/dos-vendor). The options are unstable, so RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain accept them. The source
# path is remapped so two builds in different directories give the same bytes.
set -euo pipefail

action="${1:-build}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

flags=(
	-C link-arg=-T"$here/link.ld"
	-C link-arg=-nostdlib
	-C link-arg=-static
	-C link-arg=--no-eh-frame-hdr
	-C relocation-model=static
	"--remap-path-prefix=$here=/afterfetch-dos"
	"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
)
export CARGO_ENCODED_RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS="$(printf '%s\x1f' "${flags[@]}")"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"

cargo_args=(--release --locked --target dos-elf.json -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec)

case "$action" in
clippy)
	cargo clippy "${cargo_args[@]}" -- -D warnings
	;;
build)
	vendor="${DOS_VENDOR:-$HOME/.cache/afterfetch/dos-vendor}"
	stub="$vendor/cwsdpmi/bin/CWSDSTUB.EXE"
	[ -f "$stub" ] || { echo "build: $stub not found; run platforms/dos/tools/fetch-tools.sh" >&2; exit 1; }
	cargo build "${cargo_args[@]}"
	mkdir -p dist
	python3 -I tools/elf2go32.py "$stub" "${CARGO_TARGET_DIR:-$here/target}/dos-elf/release/afetch" dist/AFETCH.EXE
	echo "build: dist/AFETCH.EXE ($(wc -c < dist/AFETCH.EXE) bytes)"
	;;
*)
	echo "usage: build.sh [build|clippy]" >&2
	exit 2
	;;
esac
