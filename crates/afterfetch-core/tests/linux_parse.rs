// Runs the Linux parsers over captured files from real systems, at several chunk sizes, since a shell reads each file in pieces.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::Buf;
use afterfetch_core::parse::cpuinfo::CpuInfo;
use afterfetch_core::parse::lines::for_each_line;
use afterfetch_core::parse::meminfo::MemInfo;
use afterfetch_core::parse::os_release::OsRelease;
use afterfetch_core::parse::{model, passwd, uptime};

fn fixture(path: &str) -> Vec<u8> {
	let full = format!("{}/tests/fixtures/linux/{path}", env!("CARGO_MANIFEST_DIR"));
	std::fs::read(&full).unwrap_or_else(|error| panic!("{full}: {error}"))
}

const CHUNKS: [usize; 5] = [1, 7, 64, 512, 4096];

fn os_name(file: &str, chunk: usize) -> String {
	let mut release = OsRelease::new();
	for_each_line::<192>(&fixture(file), chunk, &mut |line| release.feed_line(line));
	let mut out = Buf::<64>::new();
	release.describe(&mut out);
	String::from_utf8(out.as_bytes().to_vec()).unwrap()
}

fn cpu_name(file: &str, chunk: usize) -> String {
	let mut info = CpuInfo::new();
	for_each_line::<192>(&fixture(file), chunk, &mut |line| info.feed_line(line));
	let mut out = Buf::<64>::new();
	info.describe(&mut out);
	String::from_utf8(out.as_bytes().to_vec()).unwrap()
}

fn memory(file: &str, chunk: usize) -> (u64, u64) {
	let mut info = MemInfo::new();
	for_each_line::<192>(&fixture(file), chunk, &mut |line| info.feed_line(line));
	(info.total_bytes(), info.available_bytes())
}

#[test]
fn os_release_files_give_their_pretty_names() {
	for chunk in CHUNKS {
		assert_eq!(os_name("os-release/arch", chunk), "Arch Linux");
		assert_eq!(os_name("os-release/debian", chunk), "Debian GNU/Linux 13 (trixie)");
		assert_eq!(os_name("os-release/alpine", chunk), "Alpine Linux v3.22");
	}
}

#[test]
fn a_weird_os_release_falls_back_to_name_and_version() {
	// An empty `PRETTY_NAME=` counts as unset, and the CRLF ends, single quotes, escapes and trailing comment all parse.
	for chunk in CHUNKS {
		assert_eq!(os_name("os-release/weird", chunk), "Odd OS 4.2 \"beta\"");
	}
}

#[test]
fn cpuinfo_files_of_each_shape_give_a_cpu_row() {
	for chunk in CHUNKS {
		assert_eq!(
			cpu_name("cpuinfo/x86_64", chunk),
			"13th Gen Intel(R) Core(TM) i7-1365U (4 cores)"
		);
		assert_eq!(cpu_name("cpuinfo/aarch64-pi4", chunk), "Cortex-A72 (4 cores)");
		assert_eq!(
			cpu_name("cpuinfo/armv6-pi-zero", chunk),
			"ARMv6-compatible processor rev 7 (v6l)"
		);
		assert_eq!(cpu_name("cpuinfo/mips-openwrt", chunk), "MIPS 74Kc V4.12");
	}
}

#[test]
fn meminfo_files_give_total_and_available() {
	for chunk in CHUNKS {
		assert_eq!(memory("meminfo/x86_64", chunk), (32_641_236 * 1024, 19_070_112 * 1024));
		assert_eq!(
			memory("meminfo/mips-openwrt", chunk),
			(59_972 * 1024, (31_236 + 2_688 + 11_476) * 1024)
		);
	}
}

#[test]
fn uptime_and_passwd_fixtures_parse() {
	assert_eq!(uptime::seconds(&fixture("uptime/x86_64")), Some(350_735));
	let passwd_file = fixture("passwd/sample");
	let mut found = None;
	for_each_line::<128>(&passwd_file, 5, &mut |line| {
		if let Some(name) = passwd::name_for_uid(line, 1000) {
			found = Some(name.to_vec());
		}
	});
	assert_eq!(found.as_deref(), Some(&b"scott"[..]));
}

#[test]
fn a_board_model_is_cleaned() {
	let mut out = Buf::<64>::new();
	model::clean(b"TP-Link TL-WR1043ND v3\0", &mut out);
	assert_eq!(out.as_bytes(), b"TP-Link TL-WR1043ND v3");
}
