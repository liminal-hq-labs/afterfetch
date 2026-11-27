/* The layout of the program: code and read-only data in one read-and-execute segment, writable data after it, and nothing else. The
   compiler's own pieces carry unwind tables even with panic=abort, so they are dropped here.

   (c) Copyright 2026 Liminal HQ, Scott Morris
   SPDX-License-Identifier: Apache-2.0 OR MIT */
ENTRY(_start)
SECTIONS
{
	. = 0x400000 + SIZEOF_HEADERS;
	.text : { *(.text .text.*) *(.rodata .rodata.*) }
	. = ALIGN(0x1000) + (. & 0xfff);
	.data : { *(.data .data.*) *(.got .got.plt) }
	.bss : { *(.bss .bss.*) *(COMMON) }
	/DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.ARM.exidx*) *(.ARM.extab*) *(.ARM.attributes) *(.comment) *(.note*) }
}
