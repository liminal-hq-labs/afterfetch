/* The layout of the FreeBSD program: the file and program headers, code and read-only data in one read-and-execute segment, and writable
   data in a second segment on its own pages. FreeBSD's kernel maps each segment's pages itself and refuses two segments that share
   one, so the headers cannot be a segment of their own at the start of the code's page, as they are on Linux (`link.ld`).

   (c) Copyright 2026 Liminal HQ, Scott Morris
   SPDX-License-Identifier: Apache-2.0 OR MIT */
ENTRY(_start)
PHDRS
{
	text PT_LOAD FILEHDR PHDRS FLAGS(5);
	data PT_LOAD FLAGS(6);
	stack PT_GNU_STACK FLAGS(6);
}
SECTIONS
{
	. = 0x400000 + SIZEOF_HEADERS;
	.text : { *(.text .text.*) *(.rodata .rodata.*) *(.data.rel.ro .data.rel.ro.*) } :text
	. = ALIGN(0x1000) + (. & 0xfff);
	.data : { *(.data .data.*) *(.got .got.plt) } :data
	.bss : { *(.bss .bss.*) *(COMMON) } :data
	/DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.note*) *(.comment) }
}
