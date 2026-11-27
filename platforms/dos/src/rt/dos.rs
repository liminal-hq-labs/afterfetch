// INT 21h: output and the exit call, and the transfer buffer every call that takes a pointer goes through.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// CWSDPMI does not translate pointers for a protected-mode INT 21h: the call is reflected to real mode with the registers as they are, so
// DS:EDX names whatever real-mode DS and the low 16 bits of EDX happen to address. Every call that takes a pointer therefore goes
// through a buffer in conventional memory and the DPMI real-mode call (0300h), as DJGPP's C library does: copy the data into the buffer,
// call DOS with a real-mode segment:offset, copy the result out. Calls that take no pointer (AH=4Ch, AH=3Eh) are made directly.

use core::arch::asm;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::rt::dpmi::{self, RealRegs};

/// The size of the transfer buffer in paragraphs: 1 KiB, which holds a 512-byte VBE information block and the longest write.
const BUFFER_PARAGRAPHS: u16 = 64;
/// The size of the transfer buffer in bytes.
pub const BUFFER_SIZE: usize = BUFFER_PARAGRAPHS as usize * 16;

/// The real-mode segment of the transfer buffer, 0 until first use.
static BUFFER_SEGMENT: AtomicU32 = AtomicU32::new(0);

/// The real-mode segment of the transfer buffer, allocating it on first use.
pub fn buffer_segment() -> u16 {
	let segment = BUFFER_SEGMENT.load(Ordering::Relaxed);
	if segment != 0 {
		return segment as u16;
	}
	match dpmi::alloc_dos_memory(BUFFER_PARAGRAPHS) {
		Ok((segment, _selector)) => {
			BUFFER_SEGMENT.store(u32::from(segment), Ordering::Relaxed);
			segment
		}
		Err(_) => die(b"dos: no conventional memory for the transfer buffer\r\n"),
	}
}

/// The transfer buffer as a pointer in the flat data segment (conventional memory is mapped at linear 0).
pub fn buffer() -> *mut u8 {
	dpmi::linear_to_ptr::<u8>(u32::from(buffer_segment()) << 4)
}

/// Runs a real-mode interrupt with `registers`; false when the host refuses.
pub fn real_interrupt(vector: u8, registers: &mut RealRegs) -> bool {
	dpmi::simulate_interrupt(vector, registers).is_ok()
}

/// Runs INT 21h in real mode with `registers`, and turns the carry flag into a failure.
fn real21(registers: &mut RealRegs) -> bool {
	real_interrupt(0x21, registers) && registers.flags & 1 == 0
}

/// Ends the program with `code`.
pub fn exit(code: u8) -> ! {
	loop {
		// SAFETY: AH=4Ch takes no pointer and does not return; the loop is for the impossible case that it does.
		unsafe { asm!("int 0x21", in("eax") 0x4C00 | u32::from(code), options(nostack)) };
	}
}

/// Reports a failure before the runtime is up, and exits with status 255. No allocation, no formatting.
pub fn die(message: &[u8]) -> ! {
	let _ = write(1, message);
	exit(255)
}

/// AH=40h: writes everything to a handle through the transfer buffer. False when DOS fails or writes less (the disk is full).
pub fn write(handle: u16, bytes: &[u8]) -> bool {
	for chunk in bytes.chunks(BUFFER_SIZE) {
		// SAFETY: the buffer is BUFFER_SIZE bytes of conventional memory and the chunk is no longer.
		unsafe { core::ptr::copy_nonoverlapping(chunk.as_ptr(), buffer(), chunk.len()) };
		let mut registers = RealRegs {
			eax: 0x4000,
			ebx: u32::from(handle),
			ecx: chunk.len() as u32,
			ds: buffer_segment(),
			..RealRegs::default()
		};
		if !real21(&mut registers) || registers.eax as u16 as usize != chunk.len() {
			return false;
		}
	}
	true
}

/// AH=3Ch: creates (or empties) a file and returns its handle.
pub fn create(path: &[u8]) -> Option<u16> {
	if path.len() + 1 > BUFFER_SIZE {
		return None;
	}
	// SAFETY: the length was checked against the buffer, with room for the NUL.
	unsafe {
		core::ptr::copy_nonoverlapping(path.as_ptr(), buffer(), path.len());
		buffer().add(path.len()).write(0);
	}
	let mut registers = RealRegs {
		eax: 0x3C00,
		ds: buffer_segment(),
		..RealRegs::default()
	};
	real21(&mut registers).then_some(registers.eax as u16)
}

/// AH=3Eh: closes a handle. False when DOS fails (a full disk can fail here).
pub fn close(handle: u16) -> bool {
	let mut registers = RealRegs {
		eax: 0x3E00,
		ebx: u32::from(handle),
		..RealRegs::default()
	};
	real21(&mut registers)
}
