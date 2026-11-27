#!/usr/bin/env bash
# The FreeBSD test bed (AF-D6): fetches the official VM image, or runs the binary in it. Usage: freebsd-vm.sh fetch | run [binary]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# `fetch` downloads `FreeBSD-<release>-amd64-BASIC-CLOUDINIT-ufs.qcow2.xz` into ~/.cache/afterfetch/freebsd, checks it against the release's
# `CHECKSUM.SHA256` from the same directory and unpacks it (6 GB virtual, 2.5 GB on disk; check `df -h /home` first). `run` boots it under
# a private QEMU with the disk in snapshot mode, so the base image never changes, and drives it with `scripts/smoke/freebsd-serial.py`.
# Nothing here captures a desktop or injects keys.

set -euo pipefail

release="${FREEBSD_RELEASE:-15.1-RELEASE}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cache="${HOME}/.cache/afterfetch/freebsd"
name="FreeBSD-${release}-amd64-BASIC-CLOUDINIT-ufs.qcow2"
base="https://download.freebsd.org/releases/VM-IMAGES/${release}/amd64/Latest"

case "${1:-}" in
fetch)
	mkdir -p "$cache"
	cd "$cache"
	if [ -f "$name" ]; then
		echo "already fetched: $cache/$name"
		exit 0
	fi
	curl -fsSL -o CHECKSUM.SHA256 "$base/CHECKSUM.SHA256"
	curl -fSL -O "$base/$name.xz"
	grep "($name.xz)" CHECKSUM.SHA256 | sed 's/^SHA256 (\(.*\)) = \(.*\)$/\2  \1/' | sha256sum -c -
	xz -dk "$name.xz"
	echo "ready: $cache/$name"
	;;
run)
	binary="${2:-$root/target/x86_64-unknown-freebsd/release/afterfetch}"
	[ -f "$cache/$name" ] || { echo "no image: run '$0 fetch' first" >&2; exit 1; }
	mkdir -p "${HOME}/.cache/reactos-liminal"
	# A private QEMU on a snapshot of a read-only image, but the lock keeps it from running beside another agent's VM.
	exec flock -w 1800 "${HOME}/.cache/reactos-liminal/qemu.lock" \
		python3 -I "$root/scripts/smoke/freebsd-serial.py" "$binary" "$root/tests/smoke/freebsd.labels" "$cache/$name"
	;;
*)
	echo "usage: $0 fetch | run [binary]" >&2
	exit 2
	;;
esac
