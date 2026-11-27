# Local gate for afterfetch. `make check` is the gate before every commit.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

export CARGO_BUILD_JOBS ?= 4

.PHONY: check check-fast headers fmt clippy test deny art art-check test-art clippy-unix build-unix check-unix smoke-linux-native repro-unix

UNIX_TARGETS := x86_64-unknown-linux-musl i586-unknown-linux-musl

# Every later work package adds its builds, size checks and smokes to `check`.
check: check-fast deny clippy-unix build-unix check-unix smoke-linux-native repro-unix

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

# Unix (Linux x86_64 and i586): lint, build, structure and size checks, the smokes that run here, and reproducibility.
clippy-unix:
	for target in $(UNIX_TARGETS); do bash platforms/unix/build.sh $$target clippy || exit 1; done

build-unix:
	for target in $(UNIX_TARGETS); do bash platforms/unix/build.sh $$target || exit 1; done

check-unix: build-unix
	for target in $(UNIX_TARGETS); do bash platforms/unix/check.sh $$target || exit 1; done

smoke-linux-native: build-unix
	bash scripts/smoke/linux-native.sh

repro-unix:
	bash scripts/repro-unix.sh $(UNIX_TARGETS)
