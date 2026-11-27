// Tells the crate when it is being built as a raw-syscall Linux or FreeBSD binary. On any other target (the host's own, for `cargo check` of the
// workspace) the crate is an empty program, so the workspace builds everywhere.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
	println!("cargo:rustc-check-cfg=cfg(raw_linux)");
	println!("cargo:rustc-check-cfg=cfg(raw_freebsd)");
	println!("cargo:rustc-check-cfg=cfg(raw_unix)");
	println!("cargo:rustc-check-cfg=cfg(arm_v7)");
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=link.ld");
	println!("cargo:rerun-if-changed=link-freebsd.ld");
	let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
	let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
	if os == "linux" && env == "musl" {
		println!("cargo:rustc-cfg=raw_linux");
		println!("cargo:rustc-cfg=raw_unix");
	}
	if os == "freebsd" {
		println!("cargo:rustc-cfg=raw_freebsd");
		println!("cargo:rustc-cfg=raw_unix");
	}
	// Rust does not say which Arm architecture version a target is, so the triple does: `armv7-...` is ARMv7, plain `arm-...` is ARMv6.
	if std::env::var("TARGET").unwrap_or_default().starts_with("armv7") {
		println!("cargo:rustc-cfg=arm_v7");
	}
}
