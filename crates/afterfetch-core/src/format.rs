// A fixed-size text buffer and the formatting of each fact afterfetch prints. No allocation, no `core::fmt`, and division only on
// 32-bit values, so a Pentium build needs no 64-bit divide helper.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A byte string of at most `N` bytes. Pushing more than fits drops the excess, so a long name never overruns.
#[derive(Clone, Copy)]
pub struct Buf<const N: usize> {
	bytes: [u8; N],
	len: usize,
}

impl<const N: usize> Default for Buf<N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<const N: usize> Buf<N> {
	pub const fn new() -> Self {
		Buf { bytes: [0; N], len: 0 }
	}

	pub fn as_bytes(&self) -> &[u8] {
		&self.bytes[..self.len]
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	pub fn clear(&mut self) {
		self.len = 0;
	}

	/// Appends as much of `text` as fits.
	pub fn push(&mut self, text: &[u8]) -> &mut Self {
		let room = N - self.len;
		let take = text.len().min(room);
		self.bytes[self.len..self.len + take].copy_from_slice(&text[..take]);
		self.len += take;
		self
	}

	/// Appends `value` in decimal.
	pub fn push_u32(&mut self, value: u32) -> &mut Self {
		let mut digits = [0u8; 10];
		let mut count = 0;
		let mut rest = value;
		loop {
			digits[count] = b'0' + (rest % 10) as u8;
			count += 1;
			rest /= 10;
			if rest == 0 {
				break;
			}
		}
		while count > 0 {
			count -= 1;
			self.push(&digits[count..count + 1]);
		}
		self
	}

	/// Lower-cases ASCII letters in place.
	pub fn make_ascii_lowercase(&mut self) {
		self.bytes[..self.len].make_ascii_lowercase();
	}
}

/// The bytes of a NUL-terminated C string in `raw`, without the NUL (all of `raw` when it has none).
pub fn c_str(raw: &[u8]) -> &[u8] {
	let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
	&raw[..end]
}

/// Whole megabytes in `bytes`, rounded to the nearest (so a machine with 512 MB, less what the kernel keeps, reads 512 and not 511).
/// Saturates at `u32::MAX`.
pub const fn megabytes(bytes: u64) -> u32 {
	let rounded = (bytes >> 20) + ((bytes >> 19) & 1);
	if rounded > u32::MAX as u64 { u32::MAX } else { rounded as u32 }
}

/// A byte count the way the memory row shows it: whole megabytes below 4 GiB of `reference`, otherwise gigabytes to one decimal.
fn push_size<const N: usize>(out: &mut Buf<N>, bytes: u64, gigabytes: bool) {
	if gigabytes {
		// Tenths of a gigabyte from megabytes, with shifts only: 10 * mb / 1024.
		let tenths = (u64::from(megabytes(bytes)) * 10 + 512) >> 10;
		out.push_u32((tenths / 10) as u32)
			.push(b".")
			.push_u32((tenths % 10) as u32)
			.push(b" GB");
	} else {
		out.push_u32(megabytes(bytes)).push(b" MB");
	}
}

/// `0d 0h 1m` from a number of seconds since boot. Past 136 years the count stops growing.
pub fn format_uptime<const N: usize>(out: &mut Buf<N>, seconds: u64) {
	let seconds = u32::try_from(seconds).unwrap_or(u32::MAX);
	let minutes_total = seconds / 60;
	let days = minutes_total / (60 * 24);
	let hours = (minutes_total / 60) % 24;
	let minutes = minutes_total % 60;
	out.push_u32(days)
		.push(b"d ")
		.push_u32(hours)
		.push(b"h ")
		.push_u32(minutes)
		.push(b"m");
}

/// The uptime from a 32-bit millisecond tick count such as `GetTickCount`. It wraps after 49 days, and the display wraps with it.
pub fn format_uptime_ms<const N: usize>(out: &mut Buf<N>, milliseconds: u32) {
	format_uptime(out, u64::from(milliseconds / 1000));
}

/// `387 MB free of 511 MB`, or `12.4 GB free of 15.6 GB` once the total reaches 4 GiB.
pub fn format_memory<const N: usize>(out: &mut Buf<N>, free_bytes: u64, total_bytes: u64) {
	let gigabytes = total_bytes >= 4 << 30;
	push_size(out, free_bytes, gigabytes);
	out.push(b" free of ");
	push_size(out, total_bytes, gigabytes);
}

/// `1024 x 768`, then `, 32-bit` when the colour depth is known (0 means it is not).
pub fn format_display<const N: usize>(out: &mut Buf<N>, width: u32, height: u32, bits_per_pixel: u32) {
	out.push_u32(width).push(b" x ").push_u32(height);
	if bits_per_pixel != 0 {
		out.push(b", ").push_u32(bits_per_pixel).push(b"-bit");
	}
}

/// `NT 5.2.3790 SP3` from the version numbers and the service pack text Windows reports (`Service Pack 3`; empty when there is none).
/// The last word of the text is the pack's name (`3`, `6a`), so a different wording still gives the right pack.
pub fn format_kernel<const N: usize>(out: &mut Buf<N>, major: u32, minor: u32, build: u32, service_pack: &[u8]) {
	out.push(b"NT ")
		.push_u32(major)
		.push(b".")
		.push_u32(minor)
		.push(b".")
		.push_u32(build);
	let token_from = service_pack.iter().rposition(|&byte| byte == b' ').map_or(0, |at| at + 1);
	let token = &service_pack[token_from..];
	if token.first().is_some_and(|byte| byte.is_ascii_digit()) {
		out.push(b" SP").push(token);
	}
}

/// `ReactOS 0.4.16-x86` from the product name, the build lab string and the processor architecture. ReactOS's build lab reads
/// `20260829-0.4.16-release-0-g822e864.GNU_8.4.0`, and the release is the part between the date and `-release`; a system whose build
/// lab has no such part gets the product name alone. `x86` is appended to the release for an Intel 32-bit processor.
pub fn format_os<const N: usize>(out: &mut Buf<N>, product: &[u8], build_lab: &[u8], x86: bool) {
	out.push(if product.is_empty() { b"Windows" } else { product });
	let after_date = build_lab.iter().position(|&byte| byte == b'-').map(|at| &build_lab[at + 1..]);
	let release = after_date.and_then(|rest| find(rest, b"-release").map(|end| &rest[..end]));
	if let Some(release) = release.filter(|release| !release.is_empty()) {
		out.push(b" ").push(release);
		if x86 {
			out.push(b"-x86");
		}
	}
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack.windows(needle.len()).position(|window| window == needle)
}

/// The file name of the command interpreter in lower case, from a path such as `C:\ReactOS\system32\cmd.exe`. Falls back to `cmd.exe`.
pub fn format_shell<const N: usize>(out: &mut Buf<N>, comspec: &[u8]) {
	let name_from = comspec
		.iter()
		.rposition(|&byte| byte == b'\\' || byte == b'/')
		.map_or(0, |at| at + 1);
	let name = &comspec[name_from..];
	if name.is_empty() {
		out.push(b"cmd.exe");
	} else {
		out.push(name);
		out.make_ascii_lowercase();
	}
}

/// `Intel(R) Core(TM) i7 (8 cores)`: the brand string with runs of spaces collapsed and the ends trimmed, then the core count when it
/// is known and more than one (0 means it is not known).
pub fn format_cpu<const N: usize>(out: &mut Buf<N>, brand: &[u8], cores: u32) {
	let mut pending_space = false;
	for &byte in brand {
		if byte == b' ' || byte == b'\t' {
			pending_space = !out.is_empty();
		} else {
			if pending_space {
				out.push(b" ");
				pending_space = false;
			}
			out.push(&[byte]);
		}
	}
	if cores > 1 {
		out.push(b" (").push_u32(cores).push(b" cores)");
	}
}

/// `xterm-256color, 120 x 30`: the terminal's name, then its size when both numbers are known (0 means unknown).
pub fn format_terminal<const N: usize>(out: &mut Buf<N>, name: &[u8], columns: u32, rows: u32) {
	out.push(name);
	if columns != 0 && rows != 0 {
		if !out.is_empty() {
			out.push(b", ");
		}
		out.push_u32(columns).push(b" x ").push_u32(rows);
	}
}

/// `72% free` from a percentage; values over 100 read as 100.
pub fn format_resources<const N: usize>(out: &mut Buf<N>, percent_free: u32) {
	out.push_u32(percent_free.min(100)).push(b"% free");
}

#[cfg(test)]
mod tests {
	use super::*;

	fn text<const N: usize>(buffer: &Buf<N>) -> &str {
		core::str::from_utf8(buffer.as_bytes()).unwrap()
	}

	#[test]
	fn decimal_numbers() {
		let mut out = Buf::<32>::new();
		out.push_u32(0)
			.push(b",")
			.push_u32(7)
			.push(b",")
			.push_u32(40960)
			.push(b",")
			.push_u32(u32::MAX);
		assert_eq!(text(&out), "0,7,40960,4294967295");
	}

	#[test]
	fn a_full_buffer_drops_the_excess() {
		let mut out = Buf::<4>::new();
		out.push(b"abcdef").push_u32(12);
		assert_eq!(text(&out), "abcd");
	}

	#[test]
	fn c_strings_stop_at_the_nul() {
		assert_eq!(c_str(b"user\0junk"), b"user");
		assert_eq!(c_str(b"whole"), b"whole");
		assert_eq!(c_str(b"\0"), b"");
	}

	#[test]
	fn megabytes_round_to_nearest() {
		assert_eq!(megabytes(0), 0);
		assert_eq!(megabytes(512 * 1024 * 1024), 512);
		assert_eq!(megabytes(512 * 1024 * 1024 - 400 * 1024), 512);
		assert_eq!(megabytes(512 * 1024 * 1024 - 600 * 1024), 511);
		assert_eq!(megabytes(u64::from(u32::MAX)), 4096);
		assert_eq!(megabytes(u64::MAX), u32::MAX);
	}

	#[test]
	fn uptime_in_days_hours_and_minutes() {
		let mut out = Buf::<32>::new();
		format_uptime(&mut out, 0);
		assert_eq!(text(&out), "0d 0h 0m");
		out.clear();
		format_uptime(&mut out, 114);
		assert_eq!(text(&out), "0d 0h 1m");
		out.clear();
		format_uptime(&mut out, (2 * 24 * 60 + 3 * 60 + 59) * 60 + 59);
		assert_eq!(text(&out), "2d 3h 59m");
		out.clear();
		format_uptime_ms(&mut out, u32::MAX);
		assert_eq!(text(&out), "49d 17h 2m");
		out.clear();
		format_uptime(&mut out, u64::MAX);
		assert_eq!(text(&out), "49710d 6h 28m");
	}

	#[test]
	fn memory_free_of_total() {
		let mut out = Buf::<48>::new();
		format_memory(&mut out, 387 * 1024 * 1024, 511 * 1024 * 1024);
		assert_eq!(text(&out), "387 MB free of 511 MB");
	}

	#[test]
	fn large_memory_reads_in_gigabytes() {
		let mut out = Buf::<48>::new();
		format_memory(&mut out, 12 * 1024 * 1024 * 1024 + 400 * 1024 * 1024, 16 * 1024 * 1024 * 1024);
		assert_eq!(text(&out), "12.4 GB free of 16.0 GB");
		out.clear();
		format_memory(&mut out, 0, 4 << 30);
		assert_eq!(text(&out), "0.0 GB free of 4.0 GB");
	}

	#[test]
	fn display_with_and_without_depth() {
		let mut out = Buf::<32>::new();
		format_display(&mut out, 1024, 768, 32);
		assert_eq!(text(&out), "1024 x 768, 32-bit");
		out.clear();
		format_display(&mut out, 800, 600, 0);
		assert_eq!(text(&out), "800 x 600");
	}

	#[test]
	fn kernel_version_and_service_pack() {
		let mut out = Buf::<48>::new();
		format_kernel(&mut out, 5, 2, 3790, b"Service Pack 3");
		assert_eq!(text(&out), "NT 5.2.3790 SP3");
		out.clear();
		format_kernel(&mut out, 5, 1, 2600, b"");
		assert_eq!(text(&out), "NT 5.1.2600");
		out.clear();
		format_kernel(&mut out, 4, 0, 1381, b"Service Pack 6a");
		assert_eq!(text(&out), "NT 4.0.1381 SP6a");
	}

	#[test]
	fn os_name_and_release() {
		let mut out = Buf::<48>::new();
		format_os(&mut out, b"ReactOS", b"20260829-0.4.16-release-0-g822e864.GNU_8.4.0", true);
		assert_eq!(text(&out), "ReactOS 0.4.16-x86");
		out.clear();
		format_os(&mut out, b"ReactOS", b"20260829-0.4.16-release-0-g822e864", false);
		assert_eq!(text(&out), "ReactOS 0.4.16");
		out.clear();
		format_os(&mut out, b"Microsoft Windows XP", b"2600.xpsp.080413-2111", true);
		assert_eq!(text(&out), "Microsoft Windows XP");
		out.clear();
		format_os(&mut out, b"", b"", true);
		assert_eq!(text(&out), "Windows");
	}

	#[test]
	fn shell_is_the_interpreter_file_name() {
		let mut out = Buf::<32>::new();
		format_shell(&mut out, br"C:\ReactOS\System32\CMD.EXE");
		assert_eq!(text(&out), "cmd.exe");
		out.clear();
		format_shell(&mut out, b"");
		assert_eq!(text(&out), "cmd.exe");
		out.clear();
		format_shell(&mut out, br"C:\tools\");
		assert_eq!(text(&out), "cmd.exe");
	}

	#[test]
	fn cpu_brand_is_tidied_and_counts_cores() {
		let mut out = Buf::<64>::new();
		format_cpu(&mut out, b"  Intel(R)  Core(TM) i7   ", 8);
		assert_eq!(text(&out), "Intel(R) Core(TM) i7 (8 cores)");
		out.clear();
		format_cpu(&mut out, b"Pentium", 1);
		assert_eq!(text(&out), "Pentium");
	}

	#[test]
	fn terminal_name_and_size() {
		let mut out = Buf::<64>::new();
		format_terminal(&mut out, b"xterm-256color", 120, 30);
		assert_eq!(text(&out), "xterm-256color, 120 x 30");
		out.clear();
		format_terminal(&mut out, b"linux", 0, 0);
		assert_eq!(text(&out), "linux");
		out.clear();
		format_terminal(&mut out, b"", 80, 25);
		assert_eq!(text(&out), "80 x 25");
	}

	#[test]
	fn resources_are_a_percentage() {
		let mut out = Buf::<16>::new();
		format_resources(&mut out, 72);
		assert_eq!(text(&out), "72% free");
		out.clear();
		format_resources(&mut out, 400);
		assert_eq!(text(&out), "100% free");
	}
}
