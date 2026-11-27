#!/usr/bin/env bash
# Scans the code of the DOS program's ELF for an instruction a 386 lacks (the 486 and Pentium additions) and for any x87 or vector
# instruction. CPUID is allowed: the program runs it only after the EFLAGS ID bit test says the processor has it. Usage: scan-386.sh <elf>
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

elf="${1:?usage: scan-386.sh <elf>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/../../win32/tools/objdump.sh"
objdump="$(find_objdump)"

listing="$("$objdump" -d -j .text --no-show-raw-insn "$elf" | grep -v "file format")"
[ "$(wc -l <<<"$listing")" -gt 100 ] || { echo "FAIL scan-386: no code found in $elf" >&2; exit 1; }
bad="$(grep -Ei '(xmm|ymm|zmm)[0-9]|[[:space:]]%?mm[0-7]([^0-9a-z]|$)|[[:space:]](bswap|cmpxchg[0-9a-z]*|xadd|invd|wbinvd|invlpg|rdtsc|rdmsr|wrmsr|cmov[a-z]+|emms|movbe|popcnt|lzcnt|tzcnt|prefetch[a-z0-9]*|[ls]fence|mfence|f[a-z0-9]+)[[:space:]]' <<<"$listing" || true)"
if [ -n "$bad" ]; then
	echo "FAIL scan-386: instructions a 386 lacks, or floating point:" >&2
	head -20 <<<"$bad" >&2
	exit 1
fi
echo "ok scan-386: $(wc -l <<<"$listing") lines of code, 386 instructions only"
