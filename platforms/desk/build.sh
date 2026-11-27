#!/usr/bin/env bash
# Builds AFETCH.DMD, the Desk module, into platforms/desk/dist with no C compiler and no C library. Usage: build.sh [build|clippy] [--hosted] [--link-base ADDR] [--out DIR]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The module (`no_std`, target `vendor/i486-desk-none.json`, `core` built from source) is linked by rust-lld with Desk's `module.ld`, and `dmdpack`,
# the host packer built from the copy of Desk's module crates, turns the ELF into `AFETCH.DMD`. `--hosted` packs it for Desk's hosted track (the OS row
# says so). `--link-base ADDR` links at another address, for the packer's relocation cross check. The options are unstable, so RUSTC_BOOTSTRAP=1 lets the
# pinned stable toolchain accept them. The paths that end up in an image are mapped to fixed names (the checkout, the toolchain's sources and the cargo
# home), so two builds in different directories give the same bytes. Set CARGO_TARGET_DIR to keep the build output outside the tree.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$here"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

action="build" features=() out="$here/dist" base=""
while [ $# -gt 0 ]; do
	case "$1" in
	build | clippy) action="$1"; shift ;;
	--hosted) features=(--features hosted); shift ;;
	--link-base) base="${2:?--link-base needs an address}"; shift 2 ;;
	--out) out="${2:?--out needs a directory}"; shift 2 ;;
	*) echo "usage: build.sh [build|clippy] [--hosted] [--link-base ADDR] [--out DIR]" >&2; exit 2 ;;
	esac
done

target_dir="${CARGO_TARGET_DIR:-$here/target}"
sysroot="$(rustc --print sysroot)"
flags=(
	-C link-arg=-nostdlib
	-C link-arg=--emit-relocs
	-C link-arg=-zmax-page-size=0x1000
	"--remap-path-prefix=$here=/afterfetch-desk"
	"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
	"--remap-path-prefix=$sysroot=/rust"
)
export CARGO_ENCODED_RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS="$(printf '%s\x1f' "${flags[@]}")"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"
[ -z "$base" ] || export DESK_LINK_BASE="$base"
module_args=(--release --locked --target "$here/vendor/i486-desk-none.json" -Zbuild-std=core,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec -p afetch-dmd "${features[@]}")

case "$action" in
clippy)
	cargo clippy "${module_args[@]}" -- -D warnings
	;;
build)
	# The packer is a host tool, built without the target flags.
	env -u CARGO_ENCODED_RUSTFLAGS cargo build --release --locked -p dmdpack
	dmdpack="$target_dir/release/dmdpack"
	cargo build "${module_args[@]}"
	mkdir -p "$out"
	elf="$target_dir/i486-desk-none/release/afetch"
	pack_args=()
	[ -z "$base" ] || pack_args=(--link-base "$base")
	"$dmdpack" pack --kind module "${pack_args[@]}" "$elf" "$out/AFETCH.DMD"
	cp "$elf" "$out/afetch.elf"
	echo "build: $out/AFETCH.DMD ($(wc -c <"$out/AFETCH.DMD") bytes)"
	;;
esac
