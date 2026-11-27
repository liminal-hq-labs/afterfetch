// Tells the crate when it is being built as the raw-syscall Linux binary. On any other target (the host's own, for `cargo check` of the
// workspace) the crate is an empty program, so the workspace builds everywhere.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
	println!("cargo:rustc-check-cfg=cfg(raw_linux)");
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=link.ld");
	let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
	let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
	if os == "linux" && env == "musl" {
		println!("cargo:rustc-cfg=raw_linux");
	}
}
