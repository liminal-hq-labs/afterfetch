#!/usr/bin/env bash
# Checks a built Unix binary: the right machine, static, no interpreter, no needed libraries, within its size budget, and the system
# calls in the source equal the allow-list. Usage: check.sh <target> [binary]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

target="${1:?usage: check.sh <target> [binary]}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
binary="${2:-$root/target/$target/release/afterfetch}"
budget=32768
data=""

case "$target" in
x86_64-unknown-linux-musl) class="ELF64" machine="Advanced Micro Devices X86-64" ;;
aarch64-unknown-linux-musl) class="ELF64" machine="AArch64" ;;
armv7-unknown-linux-musleabihf | arm-unknown-linux-musleabihf) class="ELF32" machine="ARM" ;;
i586-unknown-linux-musl) class="ELF32" machine="Intel 80386" ;;
mips-unknown-linux-musl) class="ELF32" machine="MIPS R3000" budget=45056 data="big endian" ;; # AF-D22: o32 code is wider than Thumb
mipsel-unknown-linux-musl) class="ELF32" machine="MIPS R3000" budget=45056 data="little endian" ;;
*) echo "check.sh: no expectations for $target" >&2; exit 2 ;;
esac

fail() {
	echo "FAIL $target: $*" >&2
	exit 1
}

[ -f "$binary" ] || fail "$binary does not exist"
header="$(llvm-readelf -h "$binary")"
grep -q "Class:.*$class" <<<"$header" || fail "not $class"
grep -q "Machine:.*$machine" <<<"$header" || fail "not $machine"
[ -z "$data" ] || grep -q "Data:.*$data" <<<"$header" || fail "not $data"
grep -q "Type:.*EXEC" <<<"$header" || fail "not a fixed-address executable (relocation-model=static)"
! llvm-readelf -l "$binary" | grep -q 'INTERP' || fail "has a PT_INTERP"
! llvm-readelf -S "$binary" | grep -q '\.interp' || fail "has an .interp section"
dynamic="$(llvm-readelf -d "$binary" 2>&1 || true)"
! grep -q 'NEEDED' <<<"$dynamic" || fail "has a DT_NEEDED"
! grep -qi 'dynamic section' <<<"$dynamic" || fail "has a dynamic section"
size="$(stat -c %s "$binary")"
[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"

source_calls="$(sed -n '/^enum Sys {/,/^}/p' "$here/src/os/linux.rs" | grep -oE '^\s+[A-Z][A-Za-z]+,' | tr -d '\t ,' | sort)"
allowed="$(grep -v '^#' "$here/syscalls.allow" | sort)"
[ "$source_calls" = "$allowed" ] || fail "the Sys enum and syscalls.allow differ"

echo "ok $target: $size bytes of $budget, static, no PT_INTERP, no DT_NEEDED"
