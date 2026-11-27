# Local gate for afterfetch. `make check` is the gate before every commit.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

export CARGO_BUILD_JOBS ?= 4

.PHONY: check check-fast headers fmt clippy test deny art art-check test-art clippy-unix build-unix check-unix smoke-linux-native smoke-freebsd smoke-macos repro-unix
.PHONY: preview sheet
.PHONY: fmt-win32 clippy-win32 test-win32 build-win32 check-win32 smoke-wine repro-win32
.PHONY: fmt-dos clippy-dos test-dos build-dos check-dos smoke-dos repro-dos

UNIX_TARGETS := x86_64-unknown-linux-musl i586-unknown-linux-musl aarch64-unknown-linux-musl armv7-unknown-linux-musleabihf arm-unknown-linux-musleabihf mips-unknown-linux-musl mipsel-unknown-linux-musl riscv64gc-unknown-linux-musl powerpc64le-unknown-linux-musl x86_64-unknown-freebsd aarch64-unknown-freebsd x86_64-apple-darwin aarch64-apple-darwin
WIN32_ARCHES := i586 x86_64 aarch64

# Every later work package adds its builds, size checks and smokes to `check`.
check: check-fast deny clippy-unix build-unix check-unix smoke-linux-native smoke-freebsd repro-unix fmt-win32 clippy-win32 test-win32 build-win32 check-win32 smoke-wine repro-win32 fmt-dos clippy-dos test-dos build-dos check-dos smoke-dos repro-dos

check-fast: headers fmt clippy test test-art art-check

headers:
	bash scripts/check-headers.sh

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --locked -- -D warnings

test:
	cargo nextest run --workspace --locked --no-tests=pass

deny:
	cargo deny check

# Art: regenerate the logo blobs, palette tables and logo index from the masters (needs rsvg-convert), check that the committed
# ones are current, and run the generator's own tests. `make art` writes crates/afterfetch-art/data and src/generated.
art:
	python3 art/scripts/afterfetch_art.py

art-check:
	python3 art/scripts/afterfetch_art.py --check

test-art:
	python3 -m unittest discover -s art/scripts/tests

# Unix (Linux, FreeBSD and macOS): lint, build, structure and size checks, the smokes that run here, and reproducibility.
clippy-unix:
	for target in $(UNIX_TARGETS); do bash platforms/unix/build.sh $$target clippy || exit 1; done

build-unix:
	for target in $(UNIX_TARGETS); do bash platforms/unix/build.sh $$target || exit 1; done

check-unix: build-unix
	for target in $(UNIX_TARGETS); do bash platforms/unix/check.sh $$target || exit 1; done

smoke-linux-native: build-unix
	bash scripts/smoke/linux-native.sh

# FreeBSD (AF-D6, AF-D55): the x86_64 build runs inside the official VM image under QEMU; skipped, and said so, until `scripts/freebsd-vm.sh fetch` has run.
smoke-freebsd: build-unix
	@if [ -f "$$HOME/.cache/afterfetch/freebsd/FreeBSD-15.1-RELEASE-amd64-BASIC-CLOUDINIT-ufs.qcow2" ]; then bash scripts/freebsd-vm.sh run; else echo "SKIP smoke-freebsd: no VM image (run scripts/freebsd-vm.sh fetch); unverified"; fi

# macOS (AF-D56): the structure checks above are all that runs here. `make smoke-macos` is for a macOS machine and is what the CI runner calls.
smoke-macos:
	bash scripts/smoke/macos.sh

repro-unix:
	bash scripts/repro-unix.sh $(UNIX_TARGETS)

# Win32 (Pentium, x86-64 and ARM64): its own workspace (the shared crates reach it through a symlink, AF-D48), so its formatting, lints and host tests run from its folder. Then the build, the structure, import and
# Pentium checks with the size budget, the Wine smoke, and reproducibility.
fmt-win32:
	cd platforms/win32 && cargo fmt --all -- --check

clippy-win32:
	for arch in $(WIN32_ARCHES); do bash platforms/win32/build.sh $$arch clippy || exit 1; done

test-win32:
	cd platforms/win32 && cargo nextest run --lib --locked --no-tests=pass --target $$(rustc -vV | sed -n 's/^host: //p')

build-win32:
	for arch in $(WIN32_ARCHES); do bash platforms/win32/build.sh $$arch || exit 1; done

check-win32: build-win32
	for arch in $(WIN32_ARCHES); do bash platforms/win32/check.sh $$arch || exit 1; done

smoke-wine: build-win32
	bash scripts/smoke/wine.sh

repro-win32:
	bash scripts/repro-win32.sh

# DOS (AF-D57): its own workspace over the shared crates, like Win32. `build-dos` fetches the DOS tools (DOSBox Staging and CWSDPMI) into
# ~/.cache/afterfetch/dos-vendor once, then builds AFETCH.EXE; `smoke-dos` runs it under headless DOSBox; `repro-dos` builds it in three directories.
fmt-dos:
	cd platforms/dos && cargo fmt --all -- --check

clippy-dos:
	bash platforms/dos/build.sh clippy

test-dos:
	cd platforms/dos && cargo nextest run --lib --locked --no-tests=pass --target $$(rustc -vV | sed -n 's/^host: //p')

build-dos:
	bash platforms/dos/tools/fetch-tools.sh
	bash platforms/dos/build.sh

check-dos: build-dos
	bash platforms/dos/check.sh

smoke-dos: build-dos
	bash scripts/smoke/dos.sh

repro-dos:
	bash platforms/dos/tools/fetch-tools.sh
	bash scripts/repro-dos.sh

# Preview (AF-D50): what afterfetch looks like on any supported system, from sample facts. Host only; never part of a size gate or a
# release. `make preview SYSTEM=macos COLOUR=256 WIDTH=100` prints one system; `make sheet` writes the contact sheet of the eight logos.
SYSTEM ?= linux
COLOUR ?= truecolour
WIDTH ?= 80
SHEET ?= target/preview/contact-sheet.png

preview:
	cargo run -q -p afterfetch-preview --locked -- --system $(SYSTEM) --colour $(COLOUR) --width $(WIDTH) --format ansi

sheet:
	mkdir -p $(dir $(SHEET))
	cargo run -q -p afterfetch-preview --locked -- --sheet $(SHEET) --colour $(COLOUR) --width $(WIDTH)
	@echo "wrote $(SHEET)"
