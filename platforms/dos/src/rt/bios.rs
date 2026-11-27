// Software interrupts that take only registers. The DPMI host reflects them to the real-mode handler with the registers as they are, so
// nothing here may pass a pointer: a call that does goes through `dos::real_interrupt` and the transfer buffer.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::asm;

/// The registers an interrupt returns, and whether it set the carry flag.
#[derive(Clone, Copy, Default)]
pub struct Registers {
	pub eax: u32,
	pub ebx: u32,
	pub ecx: u32,
	pub edx: u32,
	pub carry: bool,
}

impl Registers {
	pub const fn ax(&self) -> u16 {
		self.eax as u16
	}
	pub const fn bx(&self) -> u16 {
		self.ebx as u16
	}
	pub const fn cx(&self) -> u16 {
		self.ecx as u16
	}
	pub const fn dx(&self) -> u16 {
		self.edx as u16
	}
}

// EBX cannot be an operand of inline assembly on x86-32, so it is loaded and saved by hand.
macro_rules! interrupt {
	($(#[$doc:meta])* $name:ident, $vector:literal) => {
		$(#[$doc])*
		pub fn $name(eax: u32, ebx: u32, ecx: u32, edx: u32) -> Registers {
			let (out_eax, out_ebx, out_ecx, out_edx, flag): (u32, u32, u32, u32, u32);
			// SAFETY: a register-only service of the BIOS or DOS; EBX is saved and restored, and nothing else is touched.
			unsafe {
				asm!(
					"push ebx",
					"mov ebx, {input_ebx}",
					concat!("int ", $vector),
					"sbb {flag}, {flag}",
					"mov {output_ebx}, ebx",
					"pop ebx",
					input_ebx = in(reg) ebx,
					output_ebx = out(reg) out_ebx,
					flag = out(reg) flag,
					inout("eax") eax => out_eax,
					inout("ecx") ecx => out_ecx,
					inout("edx") edx => out_edx,
				);
			}
			Registers { eax: out_eax, ebx: out_ebx, ecx: out_ecx, edx: out_edx, carry: flag != 0 }
		}
	};
}

interrupt!(
	/// INT 10h: video services.
	int10, "0x10"
);
interrupt!(
	/// INT 12h: the conventional memory size in kilobytes, in AX.
	int12, "0x12"
);
interrupt!(
	/// INT 15h: system services (extended memory).
	int15, "0x15"
);
interrupt!(
	/// INT 21h: DOS functions that take no pointer.
	int21, "0x21"
);
interrupt!(
	/// INT 2Fh: multiplex services.
	int2f, "0x2F"
);
