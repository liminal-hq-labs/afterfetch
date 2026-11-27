// Windows 3.1 as the core sees it: the `Platform` that reads the facts from KERNEL, USER and GDI, and SYSTEM.INI.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afetchw_rows::rows;
use afterfetch_core::format::{c_str, format_display, format_resources, format_shell};
use afterfetch_core::{Buf, Depth, FactKey, Facts, Platform, TerminalInfo, default_logo};
use w16::*;

/// The number of text columns the window and the picture are laid out for.
pub const COLUMNS: usize = 80;

const SM_CXSCREEN: u32 = 0;
const SM_CYSCREEN: u32 = 1;
const BITSPIXEL: u32 = 12;
const PLANES: u32 = 14;
/// `GetFreeSystemResources`: the lowest of the GDI and USER heaps' free percentages.
const GFSR_SYSTEMRESOURCES: u32 = 0;

/// Reads `[section] key` of SYSTEM.INI into `out`; false when it is absent or empty.
fn system_ini<const N: usize>(
	section: &'static [u8],
	key: &'static [u8],
	out: &mut Buf<N>,
) -> bool {
	let mut value = [0u8; 64];
	// SAFETY: the names end in NULs, the default is an empty string, and the buffer length passed is the buffer's.
	let length = unsafe {
		GetPrivateProfileString(
			section.as_ptr() as usize as u32,
			key.as_ptr() as usize as u32,
			cs!(""),
			value.as_mut_ptr() as usize as u32,
			value.len() as u32,
			cs!("system.ini"),
		)
	} as usize;
	out.push(&value[..length.min(value.len())]);
	!out.is_empty()
}

/// Windows 3.1 as the core asks about it. `window` is true when the picture goes in a window, false when it goes to a file.
pub struct Windows {
	pub window: bool,
}

impl Windows {
	fn fill_names(facts: &mut Facts) {
		if system_ini(b"Network\0", b"UserName\0", &mut facts.user) {
			facts.user.make_ascii_lowercase();
		}
		if system_ini(b"Network\0", b"ComputerName\0", &mut facts.host) {
			facts.host.make_ascii_lowercase();
		}
	}

	fn fill_display(facts: &mut Facts) {
		// SAFETY: asks about the screen; the device context is released.
		unsafe {
			let (width, height) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
			let hdc = GetDC(0);
			let bits = GetDeviceCaps(hdc, BITSPIXEL) * GetDeviceCaps(hdc, PLANES);
			ReleaseDC(0, hdc);
			format_display(facts.row_mut(FactKey::Display), width, height, bits);
		}
	}
}

impl Platform for Windows {
	fn facts(&mut self, facts: &mut Facts) {
		// SAFETY: these calls take no pointers.
		let (version, flags, free, ticks, resources) = unsafe {
			(
				GetVersion(),
				GetWinFlags(),
				GetFreeSpace(0),
				GetTickCount(),
				GetFreeSystemResources(GFSR_SYSTEMRESOURCES),
			)
		};
		Windows::fill_names(facts);
		let os = facts.row_mut(FactKey::Os);
		rows::format_os(os, version as u16, (version >> 16) as u16);
		let arch = rows::arch_name(flags);
		facts.arch.push(arch);
		facts.row_mut(FactKey::Os).push(b" (").push(arch).push(b")");
		rows::format_mode(facts.row_mut(FactKey::Kernel), flags);
		facts.set_uptime(u64::from(ticks / 1000));
		rows::format_cpu(facts.row_mut(FactKey::Cpu), flags);
		// Windows 3.1 reports the free global heap and no total, so the row has the one figure and the JSON has no total.
		rows::format_free(facts.row_mut(FactKey::Memory), free);
		facts.memory_available_bytes = Some(u64::from(free));
		Windows::fill_display(facts);
		let mut shell = Buf::<64>::new();
		if system_ini(b"boot\0", b"shell\0", &mut shell) {
			format_shell(facts.row_mut(FactKey::Shell), c_str(shell.as_bytes()));
		}
		format_resources(facts.row_mut(FactKey::Resources), resources);
	}

	fn terminal(&mut self) -> TerminalInfo {
		TerminalInfo {
			is_tty: self.window,
			columns: COLUMNS,
			depth_hint: Depth::Console16,
			utf8: false,
		}
	}

	fn env(&self, _name: &[u8]) -> Option<&[u8]> {
		None
	}

	fn default_logo(&self) -> &'static str {
		default_logo("win16")
	}

	fn target_name(&self, out: &mut Buf<32>) {
		out.push(b"win16-i386");
	}
}
