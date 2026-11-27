#!/usr/bin/env bash
# Checks AFETCH.DMD: the host tests of its logic, the packer's description of it, the `file` magic, relocation proved by linking at other bases,
# no instruction a module must not use, no path of the build machine in the image, and its size. Usage: check.sh [--no-build]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Nothing here runs the module: Desk has no loader that can run it yet (AF-D28, AF-D59). These are the checks Desk makes of its own test modules
# (`tools/check-modules.sh`), applied to this one; the module's logic runs on the host against a golden through a twin of `desk_log`.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
target_dir="${CARGO_TARGET_DIR:-$here/target}"
dmdpack="$target_dir/release/dmdpack"
budget=24576 # AF-D59: 24 KiB (SPEC section 11 said 16 KiB for a module with no window)

fail() {
	echo "FAIL check desk: $*" >&2
	exit 1
}
step() { printf '== %s\n' "$1"; }

step "host tests of the module's logic"
cargo test --locked -p afterfetch-desk --quiet >/dev/null
cargo fmt --all -- --check
if [ "${1:-}" != --no-build ]; then
	bash build.sh >/dev/null
fi
dmd="$here/dist/AFETCH.DMD"
elf="$here/dist/afetch.elf"
[ -f "$dmd" ] || fail "$dmd does not exist"

step "dmdpack info against the expected description"
python3 -I vendor/native-check.py tests/afetch-info.txt <("$dmdpack" info "$dmd") || fail "dmdpack info differs from tests/afetch-info.txt"

step "file(1) names it a Desk module"
[ "$(file -b -m vendor/desk.magic "$dmd")" = "$(cat tests/file-magic.txt)" ] || fail "file -m desk.magic says: $(file -b -m vendor/desk.magic "$dmd")"

step "size"
size="$(stat -c %s "$dmd")"
[ "$size" -le "$budget" ] || fail "$size bytes is over the $budget byte budget"

step "relocation: the same module linked at other bases"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
for base in 0x00800000 0x00403000 0x40400000; do
	bash build.sh --link-base "$base" --out "$scratch/$base" >/dev/null
	"$dmdpack" cross "$dmd" "$scratch/$base/AFETCH.DMD" || fail "the images at 0x00400000 and $base differ in more than their relocations"
	"$dmdpack" cross "$scratch/$base/AFETCH.DMD" "$dmd" || fail "the images at $base and 0x00400000 differ in more than their relocations"
done
bash build.sh >/dev/null

step "no cmpxchg, cli or sti, and nothing a 486 lacks"
listing="$(objdump -d "$elf")" || fail "objdump failed on $elf"
grep -q 'Disassembly of section' <<<"$listing" || fail "objdump printed no disassembly of $elf"
! grep -Eq '\b(cmpxchg|cmpxchg8b|cli|sti|cpuid|rdtsc|cmov[a-z]*|xmm[0-9]|mm[0-7])\b' <<<"$listing" || fail "$elf has an instruction a module must not use"

step "no path of the build machine in the image"
text="$(strings -a "$dmd" "$elf")"
! grep -Eq '(^|[^A-Za-z0-9_.])/(home|w|root|tmp|Users|builds?)/' <<<"$text" || fail "an image names a path of the build machine"
for dir in "$here" "$(rustc --print sysroot)" "${CARGO_HOME:-$HOME/.cargo}"; do
	! grep -qF -- "$dir" <<<"$text" || fail "an image names $dir"
done
echo "ok check desk: $size bytes of $budget, a module for the i486 with two imports (desk_log, desk_panic), position independent, no build-host path"
