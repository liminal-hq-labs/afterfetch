#!/usr/bin/env bash
# Runs AFETCH.EXE under headless DOSBox as `shape.py`'s runner: `dos-runner.sh <path to AFETCH.EXE> [arguments...]`. Standard output and the
# exit code come back as if the program were a native one. See dosrun.sh for what is and is not captured.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

exe="${1:?usage: dos-runner.sh <AFETCH.EXE> [arguments...]}"
shift
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export DOS_VENDOR="${DOS_VENDOR:-$HOME/.cache/afterfetch/dos-vendor}"
exec "$here/dosrun.sh" "$(dirname "$exe")" "$(basename "$exe")" "$@"
