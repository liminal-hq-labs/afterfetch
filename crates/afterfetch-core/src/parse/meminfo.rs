// `/proc/meminfo`: `Key:   value kB` lines. The memory row is what is available of what is installed.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{leading_number, split_pair};

/// The meminfo values afterfetch uses, in kilobytes.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct MemInfo {
	total: u64,
	available: Option<u64>,
	free: u64,
	buffers: u64,
	cached: u64,
}

impl MemInfo {
	pub const fn new() -> Self {
		MemInfo {
			total: 0,
			available: None,
			free: 0,
			buffers: 0,
			cached: 0,
		}
	}

	pub fn feed_line(&mut self, line: &[u8]) {
		let Some((key, value)) = split_pair(line, b':') else { return };
		let Some(kilobytes) = leading_number(value) else { return };
		match key {
			b"MemTotal" => self.total = kilobytes,
			b"MemAvailable" => self.available = Some(kilobytes),
			b"MemFree" => self.free = kilobytes,
			b"Buffers" => self.buffers = kilobytes,
			b"Cached" => self.cached = kilobytes,
			_ => {}
		}
	}

	/// Installed memory in bytes; 0 when the file gave none.
	pub fn total_bytes(&self) -> u64 {
		self.total.saturating_mul(1024)
	}

	/// Memory a program can use, in bytes: `MemAvailable`, or on a kernel older than 3.14 free memory plus buffers and cache.
	pub fn available_bytes(&self) -> u64 {
		let kilobytes = self
			.available
			.unwrap_or_else(|| self.free.saturating_add(self.buffers).saturating_add(self.cached));
		kilobytes.min(self.total).saturating_mul(1024)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn available_memory_is_used_when_the_kernel_reports_it() {
		let mut info = MemInfo::new();
		for line in [
			&b"MemTotal:       16384000 kB"[..],
			b"MemFree:         1000 kB",
			b"MemAvailable:   8192000 kB",
		] {
			info.feed_line(line);
		}
		assert_eq!(info.total_bytes(), 16_384_000 * 1024);
		assert_eq!(info.available_bytes(), 8_192_000 * 1024);
	}

	#[test]
	fn an_old_kernel_adds_up_free_buffers_and_cache() {
		let mut info = MemInfo::new();
		for line in [
			&b"MemTotal: 62000 kB"[..],
			b"MemFree: 10000 kB",
			b"Buffers: 2000 kB",
			b"Cached: 8000 kB",
		] {
			info.feed_line(line);
		}
		assert_eq!(info.available_bytes(), 20_000 * 1024);
	}

	#[test]
	fn available_never_exceeds_the_total_and_junk_is_ignored() {
		let mut info = MemInfo::new();
		for line in [&b"MemTotal: 100 kB"[..], b"MemAvailable: 900 kB", b"garbage", b"Cached: lots"] {
			info.feed_line(line);
		}
		assert_eq!(info.available_bytes(), 100 * 1024);
		assert_eq!(MemInfo::new().total_bytes(), 0);
	}
}
