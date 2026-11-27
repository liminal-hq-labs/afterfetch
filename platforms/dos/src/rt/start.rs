// The entry point and the contract with the go32 stub loader (CWSDPMI's `CWSDSTUB.EXE`).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The stub finds a DPMI host, switches to protected mode, allocates one memory block for the image, makes a code and a data descriptor
// with the same base for it, copies the `.text` and `.data` sections in, zeroes `.bss`, and far-jumps to the entry point. CS is the code
// selector, DS and ES the data selector, FS a selector for the stub's own data (`_GO32_StubInfo` at FS:0), and SS:ESP the stub's small
// stack, which is not usable. The direction flag is whatever DOS left, so the entry clears it. The x87 is never touched: the program
// uses none.

use core::arch::global_asm;

/// The size of the program's stack. It lives in `.bss`, so it costs address space and no file size.
const STACK_SIZE: usize = 64 * 1024;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

#[unsafe(no_mangle)]
static mut RT_STACK: Stack = Stack([0; STACK_SIZE]);

/// The part of DJGPP's `_GO32_StubInfo` (`stubinfo.h`) the program uses, copied out of the stub's data at entry. The stub's block is 0x54
/// bytes; the program only needs the selector of the PSP.
#[repr(C)]
#[derive(Clone, Copy)]
struct StubInfo {
	magic: [u8; 16],
	size: u32,
	minstack: u32,
	memory_handle: u32,
	initial_size: u32,
	minkeep: u16,
	ds_selector: u16,
	ds_segment: u16,
	psp_selector: u16,
	cs_selector: u16,
	env_size: u16,
	basename: [u8; 8],
	argv0: [u8; 16],
	dpmi_server: [u8; 16],
}

#[unsafe(no_mangle)]
static mut RT_STUB_INFO: StubInfo = StubInfo {
	magic: [0; 16],
	size: 0,
	minstack: 0,
	memory_handle: 0,
	initial_size: 0,
	minkeep: 0,
	ds_selector: 0,
	ds_segment: 0,
	psp_selector: 0,
	cs_selector: 0,
	env_size: 0,
	basename: [0; 8],
	argv0: [0; 16],
	dpmi_server: [0; 16],
};

/// The selector whose base is the program segment prefix.
pub fn psp_selector() -> u16 {
	// SAFETY: written once by `_start` before any Rust code runs, read-only afterwards.
	unsafe { (*core::ptr::addr_of!(RT_STUB_INFO)).psp_selector }
}

global_asm!(
	".text",
	".global _start",
	"_start:",
	"cld",
	// Copy the stub information block (0x54 bytes at FS:0) into the image.
	"xor esi, esi",
	"lea edi, [{info}]",
	"mov ecx, 21",
	"2:",
	"mov eax, fs:[esi]",
	"mov [edi], eax",
	"add esi, 4",
	"add edi, 4",
	"dec ecx",
	"jnz 2b",
	// ES and DS already hold the data selector; SS must too before the stack moves.
	"mov ax, ds",
	"mov es, ax",
	"mov ss, ax",
	"lea esp, [{stack} + {stack_size}]",
	"xor ebp, ebp",
	"call {entry}",
	// `rt_entry` never returns.
	"ud2",
	info = sym RT_STUB_INFO,
	stack = sym RT_STACK,
	stack_size = const STACK_SIZE,
	entry = sym rt_entry,
);

/// First Rust code after `_start`. Brings up the DPMI client, runs the program and exits with its status.
extern "C" fn rt_entry() -> ! {
	// SAFETY: the stub's selectors are valid and this is the first thing the program does; `dpmi::init` documents its needs.
	unsafe { crate::rt::dpmi::init() };
	let code = crate::run_program();
	crate::rt::dos::exit(code)
}
