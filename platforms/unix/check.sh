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
macho=""
data=""
abi_flags=""
osabi=""
source_file="linux"
allow_file="syscalls.allow"

case "$target" in
x86_64-unknown-linux-musl) class="ELF64" machine="Advanced Micro Devices X86-64" ;;
x86_64-unknown-freebsd) class="ELF64" machine="Advanced Micro Devices X86-64" osabi="FreeBSD" ;; # AF-D55: 32 KiB like Linux
x86_64-apple-darwin) macho="X86_64" minos="10.12" budget=65536 ;; # SPEC section 11: 64 KiB per macOS arch
aarch64-apple-darwin) macho="ARM64" minos="11.0" budget=65536 ;;
aarch64-unknown-freebsd) class="ELF64" machine="AArch64" osabi="FreeBSD" ;;
aarch64-unknown-linux-musl) class="ELF64" machine="AArch64" ;;
armv7-unknown-linux-musleabihf | arm-unknown-linux-musleabihf) class="ELF32" machine="ARM" ;;
i586-unknown-linux-musl) class="ELF32" machine="Intel 80386" ;;
riscv64gc-unknown-linux-musl) class="ELF64" machine="RISC-V" data="little endian" ;;
powerpc64le-unknown-linux-musl) class="ELF64" machine="PowerPC64" data="little endian" abi_flags="0x2$" budget=45056 ;; # AF-D54: PowerPC code is wider (TOC addressing)
mips-unknown-linux-musl) class="ELF32" machine="MIPS R3000" budget=45056 data="big endian" ;; # AF-D22: o32 code is wider than Thumb
mipsel-unknown-linux-musl) class="ELF32" machine="MIPS R3000" budget=45056 data="little endian" ;;
*) echo "check.sh: no expectations for $target" >&2; exit 2 ;;
esac

case "$target" in
*-freebsd)
	source_file="freebsd"
	allow_file="syscalls-freebsd.allow"
	;;
esac

fail() {
	echo "FAIL $target: $*" >&2
	exit 1
}

[ -f "$binary" ] || fail "$binary does not exist"

if [ -n "$macho" ]; then
	# macOS: a Mach-O executable that links libSystem and nothing else (AF-D5, AF-D56). It cannot run here, so the structure is all that is checked.
	headers="$(llvm-objdump --macho --private-headers "$binary")"
	grep -qE "^ *MH_MAGIC_64 +$macho " <<<"$headers" || fail "not a 64-bit $macho Mach-O"
	grep -qE "EXECUTE" <<<"$headers" || fail "not an executable"
	grep -q "PIE" <<<"$headers" || fail "not position independent (macOS requires it)"
	[ "$(grep -c 'cmd LC_LOAD_DYLIB' <<<"$headers")" = 1 ] || fail "does not load exactly one library"
	grep -q "name /usr/lib/libSystem.B.dylib" <<<"$headers" || fail "does not load libSystem"
	grep -q "cmd LC_MAIN" <<<"$headers" || fail "has no LC_MAIN"
	grep -qE "(minos|^ +version) $minos\$" <<<"$headers" || fail "minimum system version is not $minos"
	if [ "$macho" = ARM64 ]; then
		grep -q "cmd LC_CODE_SIGNATURE" <<<"$headers" || fail "arm64 macOS needs a code signature, and there is none"
	fi
	size="$(stat -c %s "$binary")"
	[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"
	# Every undefined symbol is in the stub, and every function the source declares is really imported.
	stub="$(sed -n 's/^ *symbols: *\[//p;s/^ *\([_a-z].*\)\]$/\1/p' "$here/macos/libSystem.tbd" | tr -d ' ' | tr ',' '\n' | grep . | sort -u)"
	undefined="$(llvm-nm -u "$binary" | tr -d ' ' | sort -u)"
	declared="$(sed -n '/^unsafe extern "C" {/,/^}/p' "$here/src/os/macos.rs" | grep -oE 'fn [A-Za-z_]+' | sed 's/^fn /_/' | sort -u)"
	[ -z "$(comm -23 <(printf '%s\n' "$undefined") <(printf '%s\n' "$stub"))" ] || fail "an imported symbol is missing from libSystem.tbd: $(comm -23 <(printf '%s\n' "$undefined") <(printf '%s\n' "$stub") | tr '\n' ' ')"
	[ -z "$(comm -23 <(printf '%s\n' "$declared") <(printf '%s\n' "$undefined"))" ] || fail "a declared function is not imported: $(comm -23 <(printf '%s\n' "$declared") <(printf '%s\n' "$undefined") | tr '\n' ' ')"
	echo "ok $target: $size bytes of $budget, Mach-O $macho, libSystem only, minos $minos, $(wc -l <<<"$undefined" | tr -d ' ') imports"
	exit 0
fi
header="$(llvm-readelf -h "$binary")"
grep -q "Class:.*$class" <<<"$header" || fail "not $class"
grep -q "Machine:.*$machine" <<<"$header" || fail "not $machine"
[ -z "$data" ] || grep -q "Data:.*$data" <<<"$header" || fail "not $data"
[ -z "$osabi" ] || grep -q "OS/ABI:.*$osabi" <<<"$header" || fail "not a $osabi binary (EI_OSABI)"
[ -z "$abi_flags" ] || grep -qi "Flags:.*$abi_flags" <<<"$header" || fail "not ELFv2 (e_flags 2)"
grep -q "Type:.*EXEC" <<<"$header" || fail "not a fixed-address executable (relocation-model=static)"
! llvm-readelf -l "$binary" | grep -q 'INTERP' || fail "has a PT_INTERP"
! llvm-readelf -S "$binary" | grep -q '\.interp' || fail "has an .interp section"
dynamic="$(llvm-readelf -d "$binary" 2>&1 || true)"
! grep -q 'NEEDED' <<<"$dynamic" || fail "has a DT_NEEDED"
! grep -qi 'dynamic section' <<<"$dynamic" || fail "has a dynamic section"
size="$(stat -c %s "$binary")"
[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"

source_calls="$(sed -n '/^enum Sys {/,/^}/p' "$here/src/os/$source_file.rs" | grep -oE '^\s+[A-Z][A-Za-z]+,' | tr -d '\t ,' | sort)"
allowed="$(grep -v '^#' "$here/$allow_file" | sort)"
[ "$source_calls" = "$allowed" ] || fail "the Sys enum and syscalls.allow differ"

echo "ok $target: $size bytes of $budget, static, no PT_INTERP, no DT_NEEDED"
