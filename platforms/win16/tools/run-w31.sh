#!/usr/bin/env bash
# Runs a program on Windows for Workgroups 3.11 in DOSBox-X (in Docker, on its own Xvfb) and saves screenshots of that display only.
# Usage: run-w31.sh [--vga] [--keep DIR] "<windows command line>" PREFIX SECONDS...   (programs to run are copied to D: first)
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The command line is what `WIN /3` gets: `D:\AFETCHW.EXE` opens the window program, `D:\AFETCHW.EXE /DUMP=D:\OUT.TXT` writes its text to the host
# folder that is drive D, and `D:\DOSBOX.BAT` opens a DOS box that runs the DOS program (it waits at a `pause`). `--vga` uses VGA at 640 x 480 in 16
# colours instead of the tree's own S3 driver. A screenshot of the virtual display (never the host's desktop) is saved as PREFIX-<s>.png at
# each number of seconds, and with `--keep DIR` the files in drive D are copied to DIR at the end (screen dumps, dump files).
# W31_TREE (default ~/.cache/quill16-play/w31-base) is a Windows 3.11 installation as a DOS drive. It is copied first and the copy is thrown away,
# so the tree is never changed. No keys or clicks are sent. DOSBOX_IMAGE (default quill16-dosbox-x:local) is the DOSBox-X image.

set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../.." && pwd)"

vga=0 keep=""
while [ $# -gt 0 ]; do
	case "$1" in
	--vga) vga=1; shift ;;
	--keep) keep="$2"; shift 2 ;;
	*) break ;;
	esac
done
wincmd="$1" prefix="$2"
shift 2
W31_TREE="${W31_TREE:-$HOME/.cache/quill16-play/w31-base}"
[ -d "$W31_TREE/WINDOWS" ] || { echo "run-w31: no Windows tree at $W31_TREE" >&2; exit 1; }

export DISPLAY="${W31_DISPLAY:-:96}"
mkdir -p "$HOME/.cache/afterfetch"
work="$(mktemp -d "$HOME/.cache/afterfetch/w31run.XXXXXX")"
name="afetch-w31-$$"
Xvfb "$DISPLAY" -screen 0 "$([ "$vga" = 1 ] && echo 640x480 || echo 800x600)x24" -nolisten tcp >/dev/null 2>&1 &
xvfb_pid=$!
cleanup() {
	docker rm -f "$name" >/dev/null 2>&1 || true
	kill "$xvfb_pid" 2>/dev/null || true
	rm -rf "$work"
}
trap cleanup EXIT
sleep 1

cp -a --reflink=auto "$W31_TREE" "$work/c"
mkdir -p "$work/d" "$work/conf"
win="$work/c/WINDOWS"
[ ! -f "$root/platforms/win16/dist/AFETCHW.EXE" ] || cp "$root/platforms/win16/dist/AFETCHW.EXE" "$work/d/"
[ ! -f "$root/platforms/dos/dist/AFETCH.EXE" ] || cp "$root/platforms/dos/dist/AFETCH.EXE" "$work/d/"
# A DOS box that runs the DOS program with the screen dump on, then waits.
printf '@echo off\r\nset AF_DUMP=D:\\SCREEN.TXT\r\nD:\\AFETCH.EXE\r\necho done> D:\\DONE.TXT\r\npause\r\n' >"$work/d/DOSBOX.BAT"
# The owner's tree starts Notepad and Trumpet Winsock from its Startup group; a test wants a quiet desktop.
sed -i 's/^run=.*/run=/' "$win/WIN.INI"
sed -i -e '/^Group5=/d' -e '/^Group6=/d' "$win/PROGMAN.INI"
if [ "$vga" = 1 ]; then
	sed -i -e 's/^display\.drv=.*/display.drv=vga.drv/' -e 's/^display=.*/display=*vddvga/' "$win/SYSTEM.INI"
	sed -i 's/^display\.drv=S3 .*/display.drv=VGA/' "$win/SYSTEM.INI"
fi
for ini in SYSTEM.INI WIN.INI PROGMAN.INI; do sed -i 's/\r*$/\r/' "$win/$ini"; done

{
	cat <<'CONF'
[sdl]
output=surface
fullscreen=false
windowresolution=original
autolock=false
[dosbox]
CONF
	if [ "$vga" = 1 ]; then echo "machine=vgaonly"; else echo "machine=svga_s3"; fi
	cat <<'CONF'
memsize=32
[cpu]
core=normal
cputype=pentium
cycles=max
[dos]
ems=false
xms=true
umb=true
[autoexec]
CONF
	printf '%s\n' '@echo off' 'mount c /c' 'mount d /d' 'c:' 'set PATH=c:\windows;c:\utils' 'cd \windows' "win /3 $wincmd" 'exit'
} >"$work/conf/run.conf"

docker run -d --name "$name" --user "$(id -u):$(id -g)" --network host -v /tmp/.X11-unix:/tmp/.X11-unix -e DISPLAY="$DISPLAY" \
	-e SDL_AUDIODRIVER=dummy -e HOME=/tmp -v "$work/c:/c" -v "$work/d:/d" -v "$work/conf:/conf" --entrypoint dosbox-x \
	"${DOSBOX_IMAGE:-quill16-dosbox-x:local}" -conf /conf/run.conf -nomenu -fastlaunch >/dev/null

start=$(date +%s)
for at in "$@"; do
	now=$(( $(date +%s) - start ))
	[ "$at" -gt "$now" ] && sleep $(( at - now ))
	import -window root "$prefix-$at.png"
done
if [ -n "$keep" ]; then
	mkdir -p "$keep"
	cp -a "$work/d/." "$keep/"
fi
