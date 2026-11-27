// DOS as the core sees it: the `Platform` that reads the facts from INT 21h, the BIOS, the BIOS data area and DPMI, and the program's whole
// run. Every call is one a 386 DOS machine has; the Windows 3.1 DOS box answers the same ones.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::format::{format_cpu, format_shell};
use afterfetch_core::{Buf, Depth, FactKey, Facts, Platform, TerminalInfo, default_logo, run};
use afterfetch_dos::rows;
use afterfetch_dos::tail::Tail;

use crate::processor;
use crate::rt::dpmi::RealRegs;
use crate::rt::{bios, dos, dpmi, env};
use crate::term::{Mode, Out};

/// A DOS machine as the core asks about it.
pub struct Dos {
	environment: &'static [u8],
	console: bool,
	mode: Mode,
	/// The DOS version AH=30h reports and, where AX=3306h answers, the true one.
	version: (u8, u8),
	oem: u8,
}

/// AX=3306h in real mode: the true DOS version, and the flags word (DH bit 4: the kernel is in the high memory area). The version is None
/// where the call is not there (before DOS 5), which leaves BX unchanged or gives an impossible version.
fn true_version_and_flags() -> (Option<(u8, u8)>, u8) {
	let mut registers = RealRegs {
		eax: 0x3306,
		ebx: 0,
		edx: 0,
		..RealRegs::default()
	};
	if !dos::real_interrupt(0x21, &mut registers) || registers.eax as u8 == 0xFF {
		return (None, 0);
	}
	let (major, minor) = (registers.ebx as u8, (registers.ebx >> 8) as u8);
	(
		(major >= 5 && minor < 100).then_some((major, minor)),
		(registers.edx >> 8) as u8,
	)
}

/// True when standard output is the console: a character device (bit 7 of the device word) that is the current standard output (bit 1).
fn stdout_is_console() -> bool {
	let reply = bios::int21(0x4400, 1, 0, 0);
	!reply.carry && reply.dx() & 0x0082 == 0x0082
}

impl Dos {
	fn new() -> Dos {
		let plain = bios::int21(0x3000, 0, 0, 0);
		let (mut major, mut minor) = (plain.ax() as u8, (plain.ax() >> 8) as u8);
		// AX=3306h gives the true version, which a SETVER entry cannot disguise (DOS 5 and later). It goes through the real-mode call: the
		// host's own INT 21h hook in protected mode does not pass this one on.
		let (true_version, _) = true_version_and_flags();
		if let Some((true_major, true_minor)) = true_version {
			(major, minor) = (true_major, true_minor);
		}
		Dos {
			environment: env::block(),
			console: stdout_is_console(),
			mode: Mode::read(),
			version: (major, minor),
			oem: (plain.bx() >> 8) as u8,
		}
	}

	/// The value of an environment variable, by name in any case. DOS stores names in upper case.
	fn var(&self, name: &[u8]) -> Option<&'static [u8]> {
		self.environment
			.split(|&byte| byte == 0)
			.take_while(|entry| !entry.is_empty())
			.find_map(|entry| {
				let at = entry.iter().position(|&byte| byte == b'=')?;
				entry[..at].eq_ignore_ascii_case(name).then_some(&entry[at + 1..])
			})
			.filter(|value| !value.is_empty())
	}

	/// The version of Windows when this is a DOS box (INT 2Fh AX=1600h): AL is 0 or 80h when Windows is not running, 1 or FFh for Windows 2.x.
	fn windows(&self) -> Option<(u8, u8)> {
		let reply = bios::int2f(0x1600, 0, 0, 0);
		let (major, minor) = (reply.ax() as u8, (reply.ax() >> 8) as u8);
		(!matches!(major, 0x00 | 0x80 | 0x01 | 0xFF)).then_some((major, minor))
	}

	fn fill_names(&self, facts: &mut Facts) {
		if let Some(user) = self.var(b"USER") {
			facts.user.push(user);
			facts.user.make_ascii_lowercase();
		}
		// A network redirector keeps the machine name (INT 21h AX=5E00h, a pointer, so through the transfer buffer); else the environment.
		let mut registers = RealRegs {
			eax: 0x5E00,
			ds: dos::buffer_segment(),
			..RealRegs::default()
		};
		if dos::real_interrupt(0x21, &mut registers) && registers.flags & 1 == 0 && (registers.ecx >> 8) as u8 != 0 {
			// SAFETY: the transfer buffer holds the 16 bytes DOS wrote.
			let name = unsafe { core::slice::from_raw_parts(dos::buffer(), 15) };
			facts
				.host
				.push(name.split(|&byte| byte == 0).next().unwrap_or(b"").trim_ascii_end());
		}
		if facts.host.is_empty()
			&& let Some(name) = self.var(b"HOSTNAME").or_else(|| self.var(b"COMPUTERNAME"))
		{
			facts.host.push(name);
		}
		facts.host.make_ascii_lowercase();
	}

	fn fill_system(&self, facts: &mut Facts, processor: u8) {
		let dr_dos = !bios::int21(0x4452, 0, 0, 0).carry;
		let windows = self.windows();
		rows::format_os(
			facts.row_mut(FactKey::Os),
			rows::product(self.oem, dr_dos),
			self.version.0,
			self.version.1,
			windows,
		);
		let name = rows::arch_name(processor);
		facts.arch.push(name);
		let os = facts.row_mut(FactKey::Os);
		os.push(b" (").push(name).push(b")");
		let high = true_version_and_flags().1 & 0x10 != 0;
		rows::format_kernel(facts.row_mut(FactKey::Kernel), self.version.0, self.version.1, high);
	}

	/// The BIOS date and the model byte, in the ROM at the top of the first megabyte.
	fn fill_host(&self, facts: &mut Facts) {
		// SAFETY: the BIOS ROM is in the first megabyte, mapped at linear 0 by the DPMI host.
		let (date, model) = unsafe {
			(
				core::slice::from_raw_parts(dpmi::linear_to_ptr::<u8>(0xF_FFF5), 8),
				dpmi::linear_to_ptr::<u8>(0xF_FFFE).read_volatile(),
			)
		};
		rows::format_host(facts.row_mut(FactKey::Host), date, model);
	}

	fn fill_cpu(&self, facts: &mut Facts, processor: u8) {
		let mut brand = Buf::<64>::new();
		processor::brand(&mut brand, processor);
		let fpu = bda_equipment() & 0x0002 != 0;
		let row = facts.row_mut(FactKey::Cpu);
		if !brand.is_empty() {
			format_cpu(row, brand.as_bytes(), 0);
			return;
		}
		let mut vendor = Buf::<64>::new();
		processor::vendor_family(&mut vendor, processor);
		if vendor.is_empty() {
			rows::format_class(row, processor, fpu);
		} else {
			row.push(vendor.as_bytes());
			rows::push_fpu(row, fpu);
		}
	}

	/// Conventional memory and the extended memory the BIOS reports, then what DPMI has free.
	fn fill_memory(&self, facts: &mut Facts) {
		let conventional = u64::from(bios::int12(0, 0, 0, 0).ax()) * 1024;
		let mut extended = 0u64;
		let modern = bios::int15(0xE801, 0, 0, 0);
		if !modern.carry {
			// AX is the kilobytes from 1 MB to 16 MB, BX the 64 KB blocks above 16 MB (CX and DX repeat them where AX and BX are 0).
			let (low, high) = if modern.ax() != 0 || modern.bx() != 0 {
				(modern.ax(), modern.bx())
			} else {
				(modern.cx(), modern.dx())
			};
			extended = u64::from(low) * 1024 + u64::from(high) * 65536;
		} else {
			let old = bios::int15(0x8800, 0, 0, 0);
			if !old.carry {
				extended = u64::from(old.ax()) * 1024;
			}
		}
		let mut block = [0xFFFF_FFFFu32; 12];
		let have_dpmi = dpmi::free_memory(&mut block);
		let known = |word: u32| (word != 0xFFFF_FFFF).then_some(u64::from(word));
		let physical = if have_dpmi {
			known(block[6]).map(|pages| pages * 4096)
		} else {
			None
		};
		let free_extended = if have_dpmi {
			known(block[5]).map(|pages| pages * 4096).or_else(|| known(block[0]))
		} else {
			None
		};
		let total = conventional + extended.max(physical.unwrap_or(0));
		let free = u64::from(dpmi::largest_dos_block()) * 16 + free_extended.unwrap_or(0);
		if total > 0 {
			facts.set_memory(free.min(total), total);
		}
	}

	/// The adapter, the VESA BIOS if there is one, and the text mode.
	fn fill_display(&self, facts: &mut Facts) {
		let active = bios::int10(0x1A00, 0, 0, 0);
		let adapter = if active.ax() as u8 == 0x1A {
			// A BIOS that answers 1Ah is VGA-class even when it leaves the combination code empty (DOSBox does).
			rows::adapter_name(active.bx() as u8).or(Some(&b"VGA"[..]))
		} else if bios::int10(0x1200, 0x10, 0, 0).bx() as u8 != 0x10 {
			Some(&b"EGA"[..])
		} else if self.mode.mono {
			Some(&b"MDA"[..])
		} else {
			Some(&b"CGA"[..])
		};
		let Some(adapter) = adapter else { return };
		// The VESA BIOS information block goes to ES:DI in conventional memory.
		let buffer = dos::buffer();
		// SAFETY: the transfer buffer is 1 KiB; the block is 512 bytes.
		unsafe { core::ptr::write_bytes(buffer, 0, 512) };
		let mut registers = RealRegs {
			eax: 0x4F00,
			es: dos::buffer_segment(),
			..RealRegs::default()
		};
		let vesa = if dos::real_interrupt(0x10, &mut registers) && registers.eax as u16 == 0x004F {
			// SAFETY: the block is 512 bytes in the transfer buffer.
			let block = unsafe { core::slice::from_raw_parts(buffer, 512) };
			(&block[..4] == b"VESA").then(|| {
				(
					u16::from_le_bytes([block[4], block[5]]),
					u16::from_le_bytes([block[18], block[19]]),
				)
			})
		} else {
			None
		};
		rows::format_display(
			facts.row_mut(FactKey::Display),
			adapter,
			vesa,
			u32::from(self.mode.columns),
			u32::from(self.mode.rows),
		);
	}
}

/// The BIOS equipment word at 0040:0010: bit 1 is a math coprocessor.
fn bda_equipment() -> u16 {
	// SAFETY: the BIOS data area is in conventional memory, mapped at linear 0.
	unsafe { dpmi::linear_to_ptr::<u16>(0x410).read_volatile() }
}

impl Platform for Dos {
	fn facts(&mut self, facts: &mut Facts) {
		let processor = dpmi::processor();
		self.fill_names(facts);
		self.fill_system(facts, processor);
		self.fill_host(facts);
		// Uptime is left out: DOS has no boot clock, and the BIOS tick count resets at midnight (AF-D20).
		self.fill_cpu(facts, processor);
		self.fill_memory(facts);
		self.fill_display(facts);
		if let Some(comspec) = self.var(b"COMSPEC") {
			format_shell(facts.row_mut(FactKey::Shell), comspec);
		}
	}

	fn terminal(&mut self) -> TerminalInfo {
		TerminalInfo {
			is_tty: self.console,
			columns: usize::from(self.mode.columns),
			// A monochrome adapter has bold and normal and nothing else; every colour adapter draws the 16 text-mode colours.
			depth_hint: if self.mode.mono { Depth::Mono } else { Depth::Console16 },
			utf8: false,
		}
	}

	fn env(&self, name: &[u8]) -> Option<&[u8]> {
		self.var(name)
	}

	fn default_logo(&self) -> &'static str {
		default_logo("dos")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"dos-i386");
	}
}

/// The whole program. The core's exit code already says when the output could not be written (74): `run` asks the terminal at the end.
pub fn main() -> u8 {
	let mut platform = Dos::new();
	let dump = platform.var(b"AF_DUMP");
	let mut out = Out::new(platform.console, platform.mode, dump);
	let tail = Tail::parse(env::command_tail());
	run(&mut platform, &mut out, tail.args())
}
