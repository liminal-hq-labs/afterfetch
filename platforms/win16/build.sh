#!/usr/bin/env bash
# Builds AFETCHW.EXE, the Windows 3.1 window program: Rust to an ELF with rust-lld, then tools/ne_pack.py to an NE executable.
# Usage: build.sh [build|clippy|--regen]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Writes dist/AFETCHW.EXE and dist/AFETCHW.MAP. `--regen` rebuilds src/imports.rs from tools/imports.tbl first. `core` is built from source for
# the 16-bit target (-Zbuild-std); the options are unstable, so RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain accept them. The linker
# flags keep each code section apart, with its relocations, for the packer (see link.ld). The source path is remapped so two builds in
# different directories give the same bytes. Set CARGO_TARGET_DIR to keep the build output outside the tree.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
action="${1:-build}"

flags=(
	-C link-arg=-T"$here/link.ld"
	-C link-arg=--no-dynamic-linker
	-C link-arg=--unique
	-C link-arg=--emit-relocs
	-C link-arg=--noinhibit-exec
	"--remap-path-prefix=$here=/afterfetch-win16"
	"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
)
export CARGO_ENCODED_RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS="$(printf '%s\x1f' "${flags[@]}")"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"

target_dir="${CARGO_TARGET_DIR:-$here/target}"
cargo_args=(--release --locked --target "$here/win16.json" -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec --bin afetchw)

case "$action" in
--regen)
	python3 tools/gen_imports.py tools/imports.tbl src/imports.rs
	;;
clippy)
	# The three allowed lints are in the runtime copied from Quill (src/lib.rs, src/fault.rs), which is kept as it is (PROVENANCE.md).
	cargo clippy "${cargo_args[@]}" -- -D warnings -A function-casts-as-integer -A asm-sub-register -A clippy::manual-c-str-literals
	;;
build)
	mkdir -p "$target_dir"
	if ! cargo build "${cargo_args[@]}" 2>"$target_dir/build.log"; then
		cat "$target_dir/build.log" >&2
		exit 1
	fi
	mkdir -p dist
	python3 tools/ne_pack.py --module AFETCHW --stack 0x3000 --heap 0x0800 --shadow 64 --segments res/afetchw.segments.json \
		--map dist/AFETCHW.MAP "$target_dir/win16/release/afetchw" dist/AFETCHW.EXE
	echo "build: dist/AFETCHW.EXE ($(wc -c <dist/AFETCHW.EXE) bytes)"
	;;
*)
	echo "usage: build.sh [build|clippy|--regen]" >&2
	exit 2
	;;
esac
