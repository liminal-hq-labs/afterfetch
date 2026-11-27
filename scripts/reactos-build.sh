#!/usr/bin/env bash
# Builds the Pentium Win32 program for the ReactOS image: `scripts/reactos-build.sh [OUT_DIR]` writes OUT_DIR/afterfetch.exe.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The same call shape and output name as reactos-liminal's old `tools/afterfetch/build.sh`, so the image's recipe changes a path and a pin
# and nothing else (docs/handover.md). Default OUT_DIR: platforms/win32/dist/i586. Set CARGO_TARGET_DIR to keep the build output outside the tree.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$root/platforms/win32/dist/i586}"
bash "$root/platforms/win32/build.sh" i586 --out "$out"
