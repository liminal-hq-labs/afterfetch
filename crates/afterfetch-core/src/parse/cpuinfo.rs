// `/proc/cpuinfo`: blocks of `key : value` lines, one block per logical processor. The shape differs by architecture: x86 has `model name`,
// 32-bit ARM has `model name` (the architecture revision) and `Hardware`, 64-bit ARM has only implementer and part numbers, MIPS has
// `cpu model`, RISC-V has `uarch` and PowerPC has `cpu` (`POWER9 (architected), altivec supported`).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{leading_number, split_pair};
use crate::format::{Buf, format_cpu};

/// ARM cores by `CPU part`, for the implementer `0x41` (Arm Ltd), which is what 64-bit boards report instead of a model name.
const ARM_PARTS: [(u32, &str); 14] = [
	(0xd03, "Cortex-A53"),
	(0xd04, "Cortex-A35"),
	(0xd05, "Cortex-A55"),
	(0xd07, "Cortex-A57"),
	(0xd08, "Cortex-A72"),
	(0xd09, "Cortex-A73"),
	(0xd0a, "Cortex-A75"),
	(0xd0b, "Cortex-A76"),
	(0xd0c, "Neoverse-N1"),
	(0xd40, "Neoverse-V1"),
	(0xd41, "Cortex-A78"),
	(0xd44, "Cortex-X1"),
	(0xd46, "Cortex-A510"),
	(0xd47, "Cortex-A710"),
];

const ARM_IMPLEMENTER: u32 = 0x41;

/// What is kept from a cpuinfo file: the first value of each naming key, and the number of processor blocks.
#[derive(Clone, Copy, Default)]
pub struct CpuInfo {
	model_name: Buf<64>,
	cpu_model: Buf<64>,
	uarch: Buf<32>,
	power_cpu: Buf<48>,
	hardware: Buf<48>,
	board_model: Buf<48>,
	implementer: u32,
	part: u32,
	processors: u32,
}

impl CpuInfo {
	pub const fn new() -> Self {
		CpuInfo {
			model_name: Buf::new(),
			cpu_model: Buf::new(),
			uarch: Buf::new(),
			power_cpu: Buf::new(),
			hardware: Buf::new(),
			board_model: Buf::new(),
			implementer: 0,
			part: 0,
			processors: 0,
		}
	}

	pub fn feed_line(&mut self, line: &[u8]) {
		let Some((key, value)) = split_pair(line, b':') else { return };
		if key.eq_ignore_ascii_case(b"processor") {
			if leading_number(value).is_some() {
				self.processors = self.processors.saturating_add(1);
			}
		} else if key.eq_ignore_ascii_case(b"model name") {
			keep_first(&mut self.model_name, value);
		} else if key.eq_ignore_ascii_case(b"cpu model") {
			keep_first(&mut self.cpu_model, value);
		} else if key.eq_ignore_ascii_case(b"uarch") {
			keep_first(&mut self.uarch, value);
		} else if key.eq_ignore_ascii_case(b"cpu") {
			// PowerPC: `POWER9 (architected), altivec supported`; the feature list after the comma is not part of the name.
			let name = value.split(|&byte| byte == b',').next().unwrap_or(b"");
			keep_first(&mut self.power_cpu, name.trim_ascii_end());
		} else if key.eq_ignore_ascii_case(b"hardware") {
			keep_first(&mut self.hardware, value);
		} else if key == b"Model" {
			// Only the capitalised key: x86 has a numeric `model` line that is not a name.
			keep_first(&mut self.board_model, value);
		} else if key.eq_ignore_ascii_case(b"cpu implementer") {
			keep_first_number(&mut self.implementer, value);
		} else if key.eq_ignore_ascii_case(b"cpu part") {
			keep_first_number(&mut self.part, value);
		}
	}

	/// The logical processors counted so far.
	pub fn processors(&self) -> u32 {
		self.processors
	}

	/// Writes the CPU row (`Intel(R) Core(TM) i7 (8 cores)`) to `out`, or nothing when the file named no processor. The name comes from
	/// `model name`, `cpu model`, `uarch`, `cpu` (PowerPC), a known ARM part, `Hardware` or `Model`, in that order.
	pub fn describe<const N: usize>(&self, out: &mut Buf<N>) {
		let arm_part = (self.implementer == ARM_IMPLEMENTER)
			.then(|| ARM_PARTS.iter().find(|(part, _)| *part == self.part))
			.flatten()
			.map(|(_, name)| name.as_bytes());
		let name: &[u8] = [
			self.model_name.as_bytes(),
			self.cpu_model.as_bytes(),
			self.uarch.as_bytes(),
			self.power_cpu.as_bytes(),
			arm_part.unwrap_or(b""),
			self.hardware.as_bytes(),
			self.board_model.as_bytes(),
		]
		.into_iter()
		.find(|name| !name.is_empty())
		.unwrap_or(b"");
		if !name.is_empty() {
			format_cpu(out, name, self.processors);
		}
	}
}

fn keep_first<const N: usize>(target: &mut Buf<N>, value: &[u8]) {
	if target.is_empty() {
		target.push(value);
	}
}

fn keep_first_number(target: &mut u32, value: &[u8]) {
	if *target == 0 {
		let digits = value.strip_prefix(b"0x").or_else(|| value.strip_prefix(b"0X"));
		*target = match digits {
			Some(hex) => hex
				.iter()
				.map_while(|&digit| (digit as char).to_digit(16))
				.fold(0u32, |total, nibble| (total << 4) | nibble),
			None => leading_number(value).map_or(0, |number| u32::try_from(number).unwrap_or(0)),
		};
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parse::lines::for_each_line;

	fn described(text: &[u8]) -> std::string::String {
		let mut info = CpuInfo::new();
		for_each_line::<192>(text, 13, &mut |line| info.feed_line(line));
		let mut out = Buf::<64>::new();
		info.describe(&mut out);
		std::string::String::from_utf8_lossy(out.as_bytes()).into_owned()
	}

	#[test]
	fn x86_uses_the_model_name_and_counts_processors() {
		let text = b"processor\t: 0\nmodel name\t: Intel(R)  Core(TM) i7\nflags\t: fpu\n\nprocessor\t: 1\nmodel name\t: Intel(R) Core(TM) i7\n";
		assert_eq!(described(text), "Intel(R) Core(TM) i7 (2 cores)");
	}

	#[test]
	fn a_single_processor_gets_no_count() {
		assert_eq!(
			described(b"processor : 0\nmodel name : QEMU Virtual CPU\n"),
			"QEMU Virtual CPU"
		);
	}

	#[test]
	fn mips_uses_cpu_model() {
		assert_eq!(
			described(b"system type\t: Atheros AR9344 rev 2\nprocessor\t: 0\ncpu model\t: MIPS 74Kc V4.12\n"),
			"MIPS 74Kc V4.12"
		);
	}

	#[test]
	fn powerpc_uses_cpu_without_its_feature_list() {
		let text = b"processor\t: 0\ncpu\t\t: POWER9 (architected), altivec supported\nclock\t\t: 2800.000000MHz\nprocessor\t: 1\ncpu\t\t: POWER9 (architected), altivec supported\n";
		assert_eq!(described(text), "POWER9 (architected) (2 cores)");
	}

	#[test]
	fn aarch64_names_the_core_from_the_part_number() {
		let text =
			b"processor : 0\nCPU implementer : 0x41\nCPU part : 0xd08\nprocessor : 1\nCPU implementer : 0x41\nCPU part : 0xd08\n";
		assert_eq!(described(text), "Cortex-A72 (2 cores)");
	}

	#[test]
	fn an_unknown_part_falls_back_to_hardware_then_model() {
		assert_eq!(
			described(b"processor : 0\nCPU implementer : 0x41\nCPU part : 0xfff\nHardware : BCM2835\n"),
			"BCM2835"
		);
		assert_eq!(described(b"processor : 0\nModel : Some Board\nmodel : 142\n"), "Some Board");
	}

	#[test]
	fn nothing_to_name_gives_nothing() {
		assert_eq!(described(b"processor : 0\nmodel : 142\n"), "");
		assert_eq!(described(b""), "");
	}
}
