#!/usr/bin/env bash
# Builds, or lints, the Win32 shell with no C compiler and no C library. Usage: build.sh [i586|x86_64|aarch64] [build|clippy] [--out DIR]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Rust, rust-lld's lld-link and a custom target for each of the Pentium, x86-64 and ARM64 (`<arch>-win32.json`). The executable is copied to
# DIR, default platforms/win32/dist/<arch>. `core` is built from source (-Zbuild-std) for every arch: the prebuilt one for a Pentium assumes
# SSE2, which a Pentium lacks, and none of them carries the memory routines (`memcpy` and the rest) a program with no C library needs. The
# options are unstable, so RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain accept them. The linker is the toolchain's own rust-lld. Set
# CARGO_TARGET_DIR to keep the build output outside the tree. The result is reproducible: the same source and toolchain give the same file.

set -euo pipefail

arch="i586"
action="build"
out=""
while [ $# -gt 0 ]; do
	case "$1" in
	i586 | x86_64 | aarch64) arch="$1" ;;
	build | clippy) action="$1" ;;
	--out) shift; out="${1:?--out needs a directory}" ;;
	*) echo "usage: build.sh [i586|x86_64|aarch64] [build|clippy] [--out DIR]" >&2; exit 2 ;;
	esac
	shift
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
out="${out:-$here/dist/$arch}"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
target="${arch}-win32"
target_arg="$target.json"
cargo_args=(--release --locked --target "$target_arg" -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec)

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
