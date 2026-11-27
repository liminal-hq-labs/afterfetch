#!/usr/bin/env bash
# Builds, or lints, the Win32 shell with no C compiler and no C library: Rust, rust-lld's lld-link and a Pentium target.
# Usage: build.sh [i586] [build|clippy] [--out DIR]     (the executable is copied to DIR, default platforms/win32/dist)
#
# `core` is built from source (-Zbuild-std) for a Pentium: the prebuilt one assumes SSE2, which a Pentium lacks. The options are unstable,
# so RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain accept them. The linker is the toolchain's own rust-lld. Set CARGO_TARGET_DIR to
# keep the build output outside the tree. The result is reproducible: the same source and toolchain give the same file.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

arch="i586"
action="build"
out=""
while [ $# -gt 0 ]; do
	case "$1" in
	i586) arch="$1" ;;
	build | clippy) action="$1" ;;
	--out) shift; out="${1:?--out needs a directory}" ;;
	*) echo "usage: build.sh [i586] [build|clippy] [--out DIR]" >&2; exit 2 ;;
	esac
	shift
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
out="${out:-$here/dist}"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
target="${arch}-win32"
cargo_args=(--release --locked --target "$target.json" -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec)

case "$action" in
clippy)
	cargo clippy "${cargo_args[@]}" -- -D warnings
	;;
build)
	cargo build "${cargo_args[@]}"
	mkdir -p "$out"
	cp "${CARGO_TARGET_DIR:-$here/target}/$target/release/afterfetch.exe" "$out/afterfetch.exe"
	echo "build: $out/afterfetch.exe ($(wc -c < "$out/afterfetch.exe") bytes)"
	;;
esac
