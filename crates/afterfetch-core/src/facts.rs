// The facts: the user and host name for the title, and one row per kind of fact. A platform fills what it can honestly read and leaves
// the rest empty, and an empty row is left out of the output and is `null` in JSON.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::format::Buf;
use crate::palette::Tone;

/// The kinds of fact, in the order they are printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactKey {
	Os,
	Kernel,
	/// The machine's model (`Mac15,6`), not its network name.
	Host,
	Uptime,
	Cpu,
	Memory,
	Display,
	/// The desktop environment (`GNOME`), where the platform has one and says so.
	Desktop,
	Terminal,
	Shell,
	/// Windows 3.1's free system resources.
	Resources,
}

/// How many kinds of fact there are.
pub const ROW_COUNT: usize = 11;

impl FactKey {
	/// Every key, in print order.
	pub const ALL: [FactKey; ROW_COUNT] = [
		FactKey::Os,
		FactKey::Kernel,
		FactKey::Host,
		FactKey::Uptime,
		FactKey::Cpu,
		FactKey::Memory,
		FactKey::Display,
		FactKey::Desktop,
		FactKey::Terminal,
		FactKey::Shell,
		FactKey::Resources,
	];

	/// The label printed before the value, with its colon.
	pub const fn label(self) -> &'static str {
		match self {
			FactKey::Os => "OS:",
			FactKey::Kernel => "Kernel:",
			FactKey::Host => "Host:",
			FactKey::Uptime => "Uptime:",
			FactKey::Cpu => "CPU:",
			FactKey::Memory => "Memory:",
			FactKey::Display => "Display:",
			FactKey::Desktop => "Desktop:",
			FactKey::Terminal => "Terminal:",
			FactKey::Shell => "Shell:",
			FactKey::Resources => "Resources:",
		}
	}

	/// The tone of the label.
	pub const fn tone(self) -> Tone {
		match self {
			FactKey::Os => Tone::Gold,
			FactKey::Kernel => Tone::Orange,
			FactKey::Host | FactKey::Uptime => Tone::Coral,
			FactKey::Cpu | FactKey::Memory => Tone::Rose,
			FactKey::Display | FactKey::Desktop | FactKey::Terminal => Tone::Lilac,
			FactKey::Shell => Tone::Violet,
			FactKey::Resources => Tone::Periwinkle,
		}
	}

	/// The key in `--json` output. These are stable.
	pub const fn json_key(self) -> &'static str {
		match self {
			FactKey::Os => "os",
			FactKey::Kernel => "kernel",
			FactKey::Host => "model",
			FactKey::Uptime => "uptime",
			FactKey::Cpu => "cpu",
			FactKey::Memory => "memory",
			FactKey::Display => "display",
			FactKey::Desktop => "desktop",
			FactKey::Terminal => "terminal",
			FactKey::Shell => "shell",
			FactKey::Resources => "resources",
		}
	}

	const fn slot(self) -> usize {
		self as usize
	}
}

/// All the facts. Every buffer starts empty.
#[derive(Clone, Copy, Default)]
pub struct Facts {
	pub user: Buf<32>,
	/// The machine's network name.
	pub host: Buf<32>,
	rows: [Buf<64>; ROW_COUNT],
}

impl Facts {
	pub const fn new() -> Self {
		Facts {
			user: Buf::new(),
			host: Buf::new(),
			rows: [Buf::new(); ROW_COUNT],
		}
	}

	/// The buffer for one row, for a platform to fill.
	pub fn row_mut(&mut self, key: FactKey) -> &mut Buf<64> {
		&mut self.rows[key.slot()]
	}

	/// The value of one row; empty when the platform could not report it.
	pub fn row(&self, key: FactKey) -> &[u8] {
		self.rows[key.slot()].as_bytes()
	}

	/// True when a row has a value to show.
	pub fn has(&self, key: FactKey) -> bool {
		!self.rows[key.slot()].is_empty()
	}

	/// True when there is a user or a host name for the title.
	pub fn has_title(&self) -> bool {
		!self.user.is_empty() || !self.host.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_new_set_of_facts_is_empty() {
		let facts = Facts::new();
		assert!(!facts.has_title());
		for key in FactKey::ALL {
			assert!(!facts.has(key), "{key:?}");
		}
	}

	#[test]
	fn rows_are_independent() {
		let mut facts = Facts::new();
		facts.row_mut(FactKey::Kernel).push(b"6.1");
		facts.row_mut(FactKey::Shell).push(b"zsh");
		assert_eq!(facts.row(FactKey::Kernel), b"6.1");
		assert_eq!(facts.row(FactKey::Shell), b"zsh");
		assert!(!facts.has(FactKey::Os));
	}

	#[test]
	fn keys_are_unique_and_in_slot_order() {
		for (position, key) in FactKey::ALL.into_iter().enumerate() {
			assert_eq!(key.slot(), position);
			for other in &FactKey::ALL[position + 1..] {
				assert_ne!(key.json_key(), other.json_key());
				assert_ne!(key.label(), other.label());
			}
		}
	}
}
