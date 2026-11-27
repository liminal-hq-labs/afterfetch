// A client for the DPMI host (CWSDPMI): INT 31h services and the flat address space the runtime builds on them.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The stub's data selector has a base (the start of the image's block) and a limit of just that block. `init` raises the limit of DS
// to 4 GiB, so a pointer to any linear address is that address minus the base, wrapping, which is exactly what the segment arithmetic
// undoes. The first megabyte (the PSP, the environment, the BIOS data area and the video memory) is mapped at linear 0 by CWSDPMI, so it
// is reachable the same way.

use core::arch::asm;
use core::sync::atomic::{AtomicU32, Ordering};

/// The registers a DPMI call takes and returns. Only the general registers an INT 31h function uses.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Regs {
	eax: u32,
	ebx: u32,
	ecx: u32,
	edx: u32,
	esi: u32,
	edi: u32,
	/// Out: 1 if the carry flag was set (the call failed and AX holds the error code).
	carry: u32,
}

/// A DPMI call the host refused. Nothing here acts on the error code, so it is not kept.
#[derive(Clone, Copy)]
pub struct Error;

/// `INT 31h` with every register loaded from and stored to `registers`. ESI, EDI and EBX cannot be named as operands of inline assembly
/// on x86-32, so the registers go through a structure the block loads from and stores to, saving what the caller expects preserved.
fn int31(registers: &mut Regs) {
	// SAFETY: the block saves and restores every register it touches that the compiler did not give it (EBX, ESI, EDI), reads and writes
	// only `*registers`, and leaves the stack balanced. INT 31h itself is a call to the host, which preserves the segment registers.
	unsafe {
		asm!(
			"push ebx",
			"push esi",
			"push edi",
			"push edx",
			"mov eax, [edx]",
			"mov ebx, [edx + 4]",
			"mov ecx, [edx + 8]",
			"mov esi, [edx + 16]",
			"mov edi, [edx + 20]",
			"mov edx, [edx + 12]",
			"int 0x31",
			"pushfd",
			"push edx",
			"mov edx, [esp + 8]",
			"mov [edx], eax",
			"mov [edx + 4], ebx",
			"mov [edx + 8], ecx",
			"mov [edx + 16], esi",
			"mov [edx + 20], edi",
			"pop dword ptr [edx + 12]",
			"pop eax",
			"and eax, 1",
			"mov [edx + 24], eax",
			"add esp, 4",
			"pop edi",
			"pop esi",
			"pop ebx",
			inout("edx") registers as *mut Regs => _,
			out("eax") _,
			out("ecx") _,
		);
	}
}

fn call(eax: u32, ebx: u32, ecx: u32, edx: u32, esi: u32, edi: u32) -> Result<Regs, Error> {
	let mut registers = Regs {
		eax,
		ebx,
		ecx,
		edx,
		esi,
		edi,
		carry: 0,
	};
	int31(&mut registers);
	if registers.carry != 0 { Err(Error) } else { Ok(registers) }
}

/// The linear address of the start of the image. Set by `init`.
static BASE: AtomicU32 = AtomicU32::new(0);

/// Turns a linear address into a pointer through the 4 GiB data segment.
pub fn linear_to_ptr<T>(linear: u32) -> *mut T {
	linear.wrapping_sub(BASE.load(Ordering::Relaxed)) as usize as *mut T
}

/// Brings up the client: records the base, lifts the limit of DS to 4 GiB and reloads the segment registers.
///
/// # Safety
/// Call once, first, from `rt_entry`, with DS holding the stub's data selector.
pub unsafe fn init() {
	let ds: u16;
	// SAFETY: reading a segment register has no effect.
	unsafe { asm!("mov {0:x}, ds", out(reg) ds, options(nomem, nostack, preserves_flags)) };
	let Ok(base) = segment_base(ds) else {
		crate::rt::dos::die(b"dpmi: 0006h failed\r\n")
	};
	BASE.store(base, Ordering::Relaxed);
	if call(0x0008, u32::from(ds), 0xFFFF, 0xFFFF, 0, 0).is_err() {
		crate::rt::dos::die(b"dpmi: 0008h failed\r\n");
	}
	// SAFETY: reloading DS, ES and SS with the selector they already hold, so the CPU picks up the new limit. SS is reloaded with a stack
	// pointer already inside the old limit, so the instruction after cannot fault.
	unsafe {
		asm!("mov ds, {0:x}", "mov es, {0:x}", "mov ss, {0:x}", in(reg) ds, options(nostack, preserves_flags));
	}
}

/// DPMI 0006h: the linear base of a selector.
pub fn segment_base(selector: u16) -> Result<u32, Error> {
	let regs = call(0x0006, u32::from(selector), 0, 0, 0, 0)?;
	Ok((regs.ecx << 16) | (regs.edx & 0xFFFF))
}

/// DPMI 0400h: the processor type, 2 for a 286, 3 for a 386, 4 for a 486 and 5 or more for a Pentium or later.
pub fn processor() -> u8 {
	call(0x0400, 0, 0, 0, 0, 0).map(|regs| regs.ecx as u8).unwrap_or(0)
}

/// DPMI 0500h: the free memory information, a 48-byte block of 32-bit words; a word of all ones is unknown. ES is the data selector, so
/// the address of the block is the offset EDI takes.
pub fn free_memory(block: &mut [u32; 12]) -> bool {
	call(0x0500, 0, 0, 0, 0, block.as_mut_ptr() as usize as u32).is_ok()
}

/// DPMI 0100h: allocates conventional memory. Returns the real-mode segment and the protected-mode selector.
pub fn alloc_dos_memory(paragraphs: u16) -> Result<(u16, u16), Error> {
	let regs = call(0x0100, u32::from(paragraphs), 0, 0, 0, 0)?;
	Ok((regs.eax as u16, regs.edx as u16))
}

/// The largest block of conventional memory, in paragraphs: a request for more than exists fails and reports the most there is in BX.
pub fn largest_dos_block() -> u16 {
	let mut registers = Regs {
		eax: 0x0100,
		ebx: 0xFFFF,
		..Regs::default()
	};
	int31(&mut registers);
	if registers.carry != 0 { registers.ebx as u16 } else { 0 }
}

/// The real-mode register block of DPMI 0300h: 0x32 bytes, in the order the host defines.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RealRegs {
	pub edi: u32,
	pub esi: u32,
	pub ebp: u32,
	pub reserved: u32,
	pub ebx: u32,
	pub edx: u32,
	pub ecx: u32,
	pub eax: u32,
	pub flags: u16,
	pub es: u16,
	pub ds: u16,
	pub fs: u16,
	pub gs: u16,
	pub ip: u16,
	pub cs: u16,
	pub sp: u16,
	pub ss: u16,
}

/// DPMI 0300h: runs a real-mode interrupt with the given register block (the host supplies the stack when SS:SP are zero).
pub fn simulate_interrupt(vector: u8, registers: &mut RealRegs) -> Result<(), Error> {
	call(0x0300, u32::from(vector), 0, 0, 0, registers as *mut RealRegs as usize as u32).map(|_| ())
}
