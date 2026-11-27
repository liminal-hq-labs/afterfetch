// AFETCHW.EXE: an Afterglow sunset and a few facts about this system, in a window, for Windows 3.1 and later.
//
// A 16-bit Windows program, `no_std` and `no_main`, packed into an NE executable by `tools/ne_pack.py`. The core draws into a grid of
// character cells; the window paints each run of one attribute with one `TextOut` in the system's fixed font, in the Dusk colours (GDI picks
// the nearest of 16 on a 16-colour driver and the exact colour on 256 colours and up). `/DUMP=<file>` writes the plain text to a file
// instead and shows no window, which is how the headless tests read it.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

mod facts;

use core::ptr::addr_of_mut;

use afetchw_rows::grid::Grid;
use afterfetch_core::palette::DUSK;
use afterfetch_core::run::run;
use afterfetch_core::{Style, Terminal};
use facts::{COLUMNS, Windows};
use w16::*;

export_proc!(AFETCHWPROC, window_proc);

const ROWS: usize = 20;
const WM_KEYDOWN: u32 = 0x0100;
const WM_LBUTTONDOWN: u32 = 0x0201;
const VK_ESCAPE: u32 = 0x1B;
const VK_RETURN: u32 = 0x0D;
const SYSTEM_FIXED_FONT: u32 = 16;
/// `WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE`: a window that does not resize.
const WINDOW_STYLE: u32 = 0x10CA_0000;
const CW_USEDEFAULT: u32 = 0x8000;
const SM_CYCAPTION: u32 = 4;
const SM_CXDLGFRAME: u32 = 7;
const SM_CYDLGFRAME: u32 = 8;
/// The Dusk navy as a COLORREF, which is blue, green, red from the high byte down.
const NAVY: u32 = 0x0030_120A;

static mut GRID: Grid<COLUMNS, ROWS> = Grid::new();
static mut LINE: [u8; 128] = [0; 128];
static mut SPANS: [(u8, u8); 12] = [(0, 0); 12];
static mut PS: PaintStruct = PaintStruct {
	hdc: 0,
	erase: 0,
	rect: [0; 4],
	restore: 0,
	inc_update: 0,
	reserved: [0; 16],
};
static mut CLASS: WndClass = WndClass {
	style: 0,
	wndproc: 0,
	cls_extra: 0,
	wnd_extra: 0,
	instance: 0,
	icon: 0,
	cursor: 0,
	background: 0,
	menu: 0,
	class_name: 0,
};
static mut MSG: Msg = Msg {
	hwnd: 0,
	message: 0,
	wparam: 0,
	lparam: 0,
	time: 0,
	pt: [0; 2],
};

fn grid() -> &'static mut Grid<COLUMNS, ROWS> {
	// SAFETY: one task, one thread; the window procedure and `w16_main` never run at the same moment.
	unsafe { &mut *addr_of_mut!(GRID) }
}

/// A Dusk slot as a COLORREF.
fn colour(slot: u8) -> u32 {
	let rgb = DUSK[usize::from(slot & 0x0F)];
	((rgb >> 16) & 0xFF) | (rgb & 0xFF00) | ((rgb & 0xFF) << 16)
}

/// The width and height of one character cell in the fixed font.
fn cell(hdc: u32) -> (u32, u32) {
	// SAFETY: the string is one character and ends in a NUL.
	let extent = unsafe { GetTextExtent(hdc, cs!("W"), 1) };
	(extent & 0xFFFF, extent >> 16)
}

/// Paints the grid: each run of cells with one attribute is one `TextOut`.
fn paint(hdc: u32) {
	// SAFETY: plain GDI calls on a device context Windows gave this window.
	unsafe {
		SelectObject(hdc, GetStockObject(SYSTEM_FIXED_FONT));
		let (width, height) = cell(hdc);
		let grid = grid();
		for row in 0..grid.rows_used() {
			let (characters, attributes) = (grid.characters(row), grid.attributes(row));
			let mut start = 0;
			while start < COLUMNS {
				let attribute = attributes[start];
				let mut end = start + 1;
				while end < COLUMNS && attributes[end] == attribute {
					end += 1;
				}
				SetTextColor(hdc, colour(attribute & 0x0F));
				SetBkColor(hdc, colour(attribute >> 4));
				TextOut(
					hdc,
					start as u32 * width,
					row as u32 * height,
					characters[start..].as_ptr() as usize as u32,
					(end - start) as u32,
				);
				start = end;
			}
		}
	}
}

extern "C" fn window_proc(hwnd: u32, msg: u32, wparam: u32, lparam: u32) -> u32 {
	// SAFETY: Windows calls this with the window it made; the paint structure is ours alone.
	unsafe {
		match msg {
			WM_PAINT => {
				let ps = addr_of_mut!(PS);
				let hdc = BeginPaint(hwnd, ps as usize as u32);
				paint(hdc);
				EndPaint(hwnd, ps as usize as u32);
				0
			}
			// Esc, Enter or a click ends it.
			WM_KEYDOWN if wparam == VK_ESCAPE || wparam == VK_RETURN => {
				DestroyWindow(hwnd);
				0
			}
			WM_LBUTTONDOWN => {
				DestroyWindow(hwnd);
				0
			}
			WM_DESTROY => {
				PostQuitMessage(0);
				0
			}
			_ => DefWindowProc(hwnd, msg, wparam, lparam),
		}
	}
}

/// Splits the command line on blanks into `SPANS` and takes out `/DUMP=file`, returning the file's name (NUL-terminated, in `LINE`).
fn parse() -> (usize, Option<usize>) {
	// SAFETY: the line and the spans are this function's alone, and the run only reads them.
	unsafe {
		let line = &mut *addr_of_mut!(LINE);
		let spans = &mut *addr_of_mut!(SPANS);
		let length = command_line(&mut line[..127]);
		let (mut count, mut dump, mut at) = (0, None, 0);
		while at < length && count < spans.len() {
			while at < length && line[at] == b' ' {
				at += 1;
			}
			let start = at;
			while at < length && line[at] != b' ' {
				at += 1;
			}
			if start == at {
				break;
			}
			if line[start..at].len() > 6 && line[start..start + 6].eq_ignore_ascii_case(b"/DUMP=") {
				line[at.min(127)] = 0;
				dump = Some(start + 6);
			} else {
				spans[count] = (start as u8, at as u8);
				count += 1;
			}
		}
		(count, dump)
	}
}

/// The file `/DUMP=` names, written as the core produces the text: a buffer, DOS line ends, and a note of whether every write worked.
struct DumpFile {
	handle: u32,
	buffer: [u8; 128],
	length: usize,
	failed: bool,
}

impl DumpFile {
	fn flush(&mut self) {
		if self.length > 0 && !self.failed {
			// SAFETY: the pointer and length describe the buffer, and the handle is an open file.
			let sent = unsafe {
				_lwrite(
					self.handle,
					self.buffer.as_ptr() as usize as u32,
					self.length as u32,
				)
			};
			self.failed = sent as usize != self.length;
		}
		self.length = 0;
	}

	fn write(&mut self, bytes: &[u8]) {
		for &byte in bytes {
			if self.length == self.buffer.len() {
				self.flush();
			}
			self.buffer[self.length] = byte;
			self.length += 1;
		}
	}
}

/// The terminal the core draws through: the window's grid of cells, or the plain text file.
struct Out {
	file: Option<DumpFile>,
}

impl Terminal for Out {
	fn write(&mut self, bytes: &[u8]) {
		match &mut self.file {
			Some(file) => file.write(bytes),
			None => grid().write(bytes),
		}
	}

	fn set_style(&mut self, style: Style) {
		if self.file.is_none() {
			grid().set_style(style);
		}
	}

	fn end_line(&mut self) {
		match &mut self.file {
			Some(file) => file.write(b"\r\n"),
			None => grid().end_line(),
		}
	}

	fn flush(&mut self) -> bool {
		match &mut self.file {
			Some(file) => {
				file.flush();
				// SAFETY: the handle is an open file.
				unsafe { _lclose(file.handle) };
				!file.failed
			}
			None => true,
		}
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn w16_main(hinst: u32, hprev: u32, _show: u32) -> u32 {
	let (count, dump) = parse();
	let mut platform = Windows {
		window: dump.is_none(),
	};
	let file = dump.map(|path| {
		// SAFETY: `path` is the offset of a NUL-terminated name in `LINE`.
		let name = unsafe { addr_of_mut!(LINE).cast::<u8>().add(path) };
		// SAFETY: as above; the call makes (or empties) the file.
		let handle = unsafe { _lcreat(name as usize as u32, 0) };
		DumpFile {
			handle,
			buffer: [0; 128],
			length: 0,
			failed: handle == 0xFFFF,
		}
	});
	let mut out = Out { file };
	// SAFETY: the spans index into `LINE`, which lives for the whole run.
	let code = unsafe {
		let line = &*addr_of_mut!(LINE);
		let spans = &*addr_of_mut!(SPANS);
		run(
			&mut platform,
			&mut out,
			spans[..count]
				.iter()
				.map(|&(start, end)| &line[usize::from(start)..usize::from(end)]),
		)
	};
	if dump.is_some() {
		return u32::from(code);
	}
	// SAFETY: the window class and message structures are this task's alone.
	unsafe {
		if hprev != 0 {
			return 0;
		}
		let class = addr_of_mut!(CLASS);
		(*class).wndproc = far(code_selector(), AFETCHWPROC as *const () as usize as u32);
		(*class).instance = hinst as u16;
		(*class).cursor = LoadCursor(0, 32512) as u16;
		(*class).background = CreateSolidBrush(NAVY) as u16;
		(*class).class_name = far(data_selector(), cs!("AfterfetchWindow"));
		if RegisterClass(class as usize as u32) == 0 {
			return 1;
		}
		let probe = GetDC(0);
		SelectObject(probe, GetStockObject(SYSTEM_FIXED_FONT));
		let (width, height) = cell(probe);
		ReleaseDC(0, probe);
		let rows = grid().rows_used().max(1) as u32;
		let frame_x = GetSystemMetrics(SM_CXDLGFRAME);
		let frame_y = GetSystemMetrics(SM_CYDLGFRAME);
		let hwnd = CreateWindow(
			cs!("AfterfetchWindow"),
			cs!("Afterfetch"),
			WINDOW_STYLE,
			CW_USEDEFAULT,
			0,
			COLUMNS as u32 * width + 2 * frame_x,
			rows * height + 2 * frame_y + GetSystemMetrics(SM_CYCAPTION) + 1,
			0,
			0,
			hinst,
			0,
		);
		if hwnd == 0 {
			return 2;
		}
		ShowWindow(hwnd, SW_SHOW);
		UpdateWindow(hwnd);
		let msg = addr_of_mut!(MSG) as usize as u32;
		while GetMessage(msg, 0, 0, 0) != 0 {
			TranslateMessage(msg);
			DispatchMessage(msg);
		}
		u32::from((*addr_of_mut!(MSG)).wparam)
	}
}
