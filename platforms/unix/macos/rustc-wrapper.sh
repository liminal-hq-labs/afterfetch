#!/usr/bin/env bash
# Runs rustc with SDKROOT pointed at the empty `no-sdk` directory, except for build scripts. Used as RUSTC_WRAPPER by `build.sh` for the macOS targets.
#
# rustc looks for an SDK to name in the binary, and there is none (AF-D5). A build script is a host program linked by the runner's own `cc`, which does need the real SDK, so it
# must not see the empty one.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rustc="$1"
shift
case " $* " in
*" --crate-name build_script_build "*) unset SDKROOT ;;
*) export SDKROOT="$here/no-sdk" ;;
esac
exec "$rustc" "$@"
