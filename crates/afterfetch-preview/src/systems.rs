// The systems the preview knows: what each one is, how its terminal behaves, and the sample facts for every architecture it has a
// build or a plan for. A system shows only the rows SPEC §4 says it can honestly read, in the same text the platform's own formatting
// would give, and the sample values are realistic rather than real.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use afterfetch_core::format::{Buf, format_cpu, format_display, format_resources, format_terminal};
use afterfetch_core::{Depth, FactKey};

/// What a system is and how its terminal behaves.
#[derive(Clone, Copy, Debug)]
pub struct System {
	pub name: &'static str,
	/// One line for `--list`.
	pub summary: &'static str,
	/// The name the platform shell passes to `default_logo`.
	pub platform: &'static str,
	/// The architectures the sample covers; the first is the default.
	pub arches: &'static [&'static str],
	/// The depth the terminal detects.
	pub hint: Depth,
	/// The terminal draws UTF-8 half blocks.
	pub utf8: bool,
	/// Arguments the program always runs with on this system.
	pub forced: &'static [&'static str],
	/// The architectures with a real binary today, as the target name SPEC §2 gives them.
	pub real: &'static [(&'static str, &'static str)],
}

const UNIX_ARGS: &[&str] = &[];

/// Every system, in the order the contact sheet and `--all` show them.
pub const SYSTEMS: [System; 11] = [
	System {
		name: "linux",
		summary: "Linux on x86_64, i586, ARM, MIPS, RISC-V and PowerPC",
		platform: "linux",
		arches: &[
			"x86_64", "i586", "aarch64", "armv7l", "armv6l", "mips", "mipsel", "mips64el", "riscv64", "ppc64le",
		],
		hint: Depth::TrueColour,
		utf8: true,
		forced: UNIX_ARGS,
		real: &[("x86_64", "linux-x86_64"), ("i586", "linux-i586")],
	},
	System {
		name: "freebsd",
		summary: "FreeBSD on x86_64, i686 and aarch64",
		platform: "freebsd",
		arches: &["x86_64", "i686", "aarch64"],
		hint: Depth::Ansi256,
		utf8: true,
		forced: UNIX_ARGS,
		real: &[],
	},
	System {
		name: "macos",
		summary: "macOS on Apple silicon and Intel",
		platform: "macos",
		arches: &["aarch64", "x86_64"],
		hint: Depth::TrueColour,
		utf8: true,
		forced: UNIX_ARGS,
		real: &[],
	},
	System {
		name: "win32",
		summary: "Windows 10 and 11 in a console window (console attributes, AF-D45)",
		platform: "win32",
		arches: &["x86_64", "aarch64", "i586"],
		hint: Depth::Console16,
		utf8: false,
		forced: UNIX_ARGS,
		real: &[("i586", "windows-i586")],
	},
	System {
		name: "win95",
		summary: "Windows 95 in an MS-DOS Prompt (console attributes)",
		platform: "win32",
		arches: &["i586"],
		hint: Depth::Console16,
		utf8: false,
		forced: UNIX_ARGS,
		real: &[("i586", "windows-i586")],
	},
	System {
		name: "reactos",
		summary: "ReactOS in its console, the facts the Win32 goldens use",
		platform: "win32",
		arches: &["i586"],
		hint: Depth::Console16,
		utf8: false,
		forced: UNIX_ARGS,
		real: &[("i586", "windows-i586")],
	},
	System {
		name: "dos",
		summary: "MS-DOS 6.22 on a 486 with VGA",
		platform: "dos",
		arches: &["i386"],
		hint: Depth::Console16,
		utf8: false,
		forced: UNIX_ARGS,
		real: &[],
	},
	System {
		name: "win16",
		summary: "Windows for Workgroups 3.11 in 386 enhanced mode",
		platform: "win16",
		arches: &["i386"],
		hint: Depth::Console16,
		utf8: false,
		forced: UNIX_ARGS,
		real: &[],
	},
	System {
		name: "desk",
		summary: "Desk, native: the module's plain output with the OS row only",
		platform: "desk",
		arches: &["i486"],
		hint: Depth::None,
		utf8: false,
		forced: &["--plain"],
		real: &[],
	},
	System {
		name: "desk-hosted",
		summary: "Desk, hosted on DOS",
		platform: "desk",
		arches: &["i486"],
		hint: Depth::None,
		utf8: false,
		forced: &["--plain"],
		real: &[],
	},
	System {
		name: "other",
		summary: "A system with no logo of its own (NetBSD here), drawn with the Afterglow sun",
		platform: "other",
		arches: &["x86_64"],
		hint: Depth::Ansi256,
		utf8: true,
		forced: UNIX_ARGS,
		real: &[],
	},
];

/// The systems the contact sheet shows: one for each of the eight logos, in logo order.
pub const SHEET_SYSTEMS: [&str; 8] = ["dos", "win16", "win32", "linux", "freebsd", "macos", "desk", "other"];

impl System {
	/// The system named `name`, or one of its other spellings.
	pub fn find(name: &str) -> Option<&'static System> {
		let lower = name.to_ascii_lowercase();
		let canonical = match lower.as_str() {
			"windows" | "win" | "win11" | "win10" | "windows11" => "win32",
			"windows95" | "win9x" => "win95",
			"windows3" | "win3" | "win31" | "windows31" | "win311" => "win16",
			"bsd" => "freebsd",
			"mac" | "osx" | "darwin" => "macos",
			"msdos" | "freedos" => "dos",
			"desk-native" | "deskos" => "desk",
			"unknown" | "netbsd" | "generic" => "other",
			other => other,
		};
		SYSTEMS.iter().find(|system| system.name == canonical)
	}

	/// The architecture named `name` among this system's own, accepting the usual alternative spellings.
	pub fn find_arch(&self, name: &str) -> Option<&'static str> {
		let lower = name.to_ascii_lowercase();
		let own = |wanted: &str| self.arches.iter().copied().find(|arch| *arch == wanted);
		if let Some(arch) = own(&lower) {
			return Some(arch);
		}
		let canonical = match lower.as_str() {
			"amd64" | "x64" | "x86-64" => "x86_64",
			"arm64" => "aarch64",
			"armv7" | "arm" => "armv7l",
			"armv6" => "armv6l",
			"mips64" => "mips64el",
			"ppc64" | "powerpc64le" => "ppc64le",
			"i386" | "i486" | "i686" | "x86" => {
				return own("i586")
					.or_else(|| own("i686"))
					.or_else(|| own("i386"))
					.or_else(|| own("i486"));
			}
			other => other,
		};
		own(canonical)
	}

	/// The target name SPEC §2 gives this system on `arch`: `linux-i586`, `windows-i586`, `dos-i386`.
	pub fn target(&self, arch: &str) -> String {
		match self.name {
			"linux" => format!(
				"linux-{}",
				match arch {
					"armv7l" => "armv7",
					"armv6l" => "armv6",
					other => other,
				}
			),
			"freebsd" => format!("freebsd-{arch}"),
			"macos" => format!("macos-{arch}"),
			"win32" | "win95" | "reactos" => format!("windows-{arch}"),
			"dos" => "dos-i386".to_string(),
			"win16" => "win16-i386".to_string(),
			"desk" | "desk-hosted" => "desk-i486".to_string(),
			_ => format!("other-{arch}"),
		}
	}

	/// The target name of the real binary for `arch`, when there is one.
	pub fn real_target(&self, arch: &str) -> Option<&'static str> {
		self.real.iter().find(|(name, _)| *name == arch).map(|(_, target)| *target)
	}
}

/// The sample facts of one machine: the title halves, and the rows that have a value, in print order.
#[derive(Clone, Debug, Default)]
pub struct Sample {
	pub user: String,
	pub host: String,
	/// The architecture as the system itself names it (`x86_64`, `i686`, `armv7l`), which the core shows in JSON and reads for the
	/// title rule's accent.
	pub arch: String,
	/// Whole seconds since boot, when the system reports an uptime.
	pub uptime_seconds: Option<u64>,
	/// Available and total memory in bytes, when the system reports both.
	pub memory: Option<(u64, u64)>,
	pub rows: Vec<(FactKey, String)>,
}

impl Sample {
	fn new(user: &str, host: &str) -> Sample {
		Sample {
			user: user.to_string(),
			host: host.to_string(),
			..Sample::default()
		}
	}

	fn arch(mut self, name: &str) -> Sample {
		self.arch = name.to_string();
		self
	}

	/// Uptime from days, hours and minutes.
	fn up(mut self, days: u64, hours: u64, minutes: u64) -> Sample {
		self.uptime_seconds = Some(days * 86_400 + hours * 3_600 + minutes * 60);
		self
	}

	/// Free and total memory in mebibytes.
	fn mem(mut self, free_mib: u64, total_mib: u64) -> Sample {
		self.memory = Some((free_mib << 20, total_mib << 20));
		self
	}

	fn row(mut self, key: FactKey, value: impl Into<String>) -> Sample {
		self.rows.push((key, value.into()));
		self
	}

	/// The value of a row, if the sample has one.
	pub fn get(&self, key: FactKey) -> Option<&str> {
		self.rows.iter().find(|(row, _)| *row == key).map(|(_, value)| value.as_str())
	}
}

fn text<const N: usize>(buffer: &Buf<N>) -> String {
	String::from_utf8_lossy(buffer.as_bytes()).into_owned()
}

fn cpu(brand: &str, cores: u32) -> String {
	let mut out = Buf::<64>::new();
	format_cpu(&mut out, brand.as_bytes(), cores);
	text(&out)
}

fn display(width: u32, height: u32, bits: u32) -> String {
	let mut out = Buf::<64>::new();
	format_display(&mut out, width, height, bits);
	text(&out)
}

fn terminal(name: &str, columns: usize) -> String {
	let mut out = Buf::<64>::new();
	format_terminal(&mut out, name.as_bytes(), columns as u32, 30);
	text(&out)
}

fn resources(percent: u32) -> String {
	let mut out = Buf::<64>::new();
	format_resources(&mut out, percent);
	text(&out)
}

/// The sample facts for `system` on `arch`, for a terminal `columns` wide. An architecture the system does not have falls back to its
/// first.
pub fn sample(system: &System, arch: &str, columns: usize) -> Sample {
	let arch = system
		.arches
		.iter()
		.copied()
		.find(|own| *own == arch)
		.unwrap_or(system.arches[0]);
	match system.name {
		"linux" => linux(arch, columns),
		"freebsd" => freebsd(arch, columns),
		"macos" => macos(arch, columns),
		"win32" => windows(arch),
		"win95" => windows_95(),
		"reactos" => reactos(),
		"dos" => dos(),
		"win16" => windows_3(),
		"desk" => desk("native"),
		"desk-hosted" => desk("hosted on DOS"),
		_ => other(columns),
	}
}

fn linux(arch: &str, columns: usize) -> Sample {
	use FactKey::*;
	let os = |name: &str, machine: &str| format!("{name} ({machine})");
	match arch {
		"i586" => Sample::new("scott", "pentium")
			.arch("i586")
			.row(Os, os("Debian GNU/Linux 6.0 (squeeze)", "i586"))
			.row(Kernel, "Linux 2.6.32-5-486")
			.up(0, 3, 12)
			.row(Cpu, cpu("Intel(R) Pentium(R) CPU 233MHz MMX", 1))
			.mem(31, 62)
			.row(Terminal, terminal("linux", 80))
			.row(Shell, "bash"),
		"aarch64" => Sample::new("pi", "raspberrypi")
			.row(Os, os("Debian GNU/Linux 12 (bookworm)", "aarch64"))
			.arch("aarch64")
			.row(Kernel, "Linux 6.6.51+rpt-rpi-v8")
			.row(Host, "Raspberry Pi 4 Model B Rev 1.4")
			.up(11, 6, 2)
			.row(Cpu, cpu("Cortex-A72", 4))
			.mem(2_410, 3_794)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		"armv7l" => Sample::new("pi", "raspberrypi")
			.row(Os, os("Raspbian GNU/Linux 12 (bookworm)", "armv7l"))
			.arch("armv7l")
			.row(Kernel, "Linux 6.6.51+rpt-rpi-v7")
			.row(Host, "Raspberry Pi 3 Model B Plus Rev 1.3")
			.up(2, 19, 40)
			.row(Cpu, cpu("Cortex-A53", 4))
			.mem(612, 926)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		"armv6l" => Sample::new("pi", "zero")
			.row(Os, os("Raspbian GNU/Linux 11 (bullseye)", "armv6l"))
			.arch("armv6l")
			.row(Kernel, "Linux 6.1.21+")
			.row(Host, "Raspberry Pi Zero W Rev 1.1")
			.up(0, 7, 5)
			.row(Cpu, cpu("ARM1176", 1))
			.mem(208, 427)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		"mips" | "mipsel" => Sample::new("root", "OpenWrt")
			.row(Os, os("OpenWrt 23.05.5", arch))
			.arch(arch)
			.row(Kernel, "Linux 5.15.167")
			.row(Host, "TP-Link TL-WR1043ND v3")
			.up(41, 3, 9)
			.row(Cpu, cpu("MIPS 74Kc V4.12", 1))
			.mem(30, 59)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "ash"),
		"mips64el" => Sample::new("scott", "loongson")
			.row(Os, os("Debian GNU/Linux 12 (bookworm)", "mips64"))
			.arch("mips64")
			.row(Kernel, "Linux 6.1.0-18-5kc-malta")
			.row(Host, "Loongson-3A4000 Mini PC")
			.up(5, 1, 30)
			.row(Cpu, cpu("ICT Loongson-3 V0.9 FPU V0.1", 4))
			.mem(5_120, 7_890)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		"riscv64" => Sample::new("scott", "visionfive")
			.row(Os, os("Debian GNU/Linux 13 (trixie)", "riscv64"))
			.arch("riscv64")
			.row(Kernel, "Linux 6.12.43+deb13-riscv64")
			.row(Host, "StarFive VisionFive 2 v1.3B")
			.up(8, 22, 14)
			.row(Cpu, cpu("sifive,u74-mc", 4))
			.mem(6_020, 7_950)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		"ppc64le" => Sample::new("scott", "talos")
			.row(Os, os("Debian GNU/Linux 13 (trixie)", "ppc64le"))
			.arch("ppc64le")
			.row(Kernel, "Linux 6.12.43+deb13-powerpc64le")
			.row(Host, "T2P9D01 REV 1.00")
			.up(63, 4, 52)
			.row(Cpu, cpu("POWER9, altivec supported", 16))
			.mem(60_300, 64_100)
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
		_ => Sample::new("scott", "dusk")
			.row(Os, os("Debian GNU/Linux 13 (trixie)", "x86_64"))
			.arch("x86_64")
			.row(Kernel, "Linux 6.12.43+deb13-amd64")
			.row(Host, "ThinkPad T14 Gen 3")
			.up(2, 4, 17)
			.row(Cpu, cpu("AMD Ryzen 7 PRO 6850U with Radeon Graphics", 16))
			.mem(9_830, 15_360)
			.row(Desktop, "GNOME")
			.row(Terminal, terminal("xterm-256color", columns))
			.row(Shell, "bash"),
	}
}

fn freebsd(arch: &str, columns: usize) -> Sample {
	use FactKey::*;
	let (machine, host, brand, cores, free, total) = match arch {
		"i686" => (
			"i386",
			"Standard PC (i440FX + PIIX, 1996)",
			"Intel(R) Pentium(R) 4 CPU 2.80GHz",
			1,
			410,
			1_007,
		),
		"aarch64" => ("aarch64", "Raspberry Pi 4 Model B Rev 1.5", "Cortex-A72", 4, 3_120, 3_950),
		_ => (
			"amd64",
			"Standard PC (Q35 + ICH9, 2009)",
			"AMD Ryzen 7 5800U with Radeon Graphics",
			4,
			1_410,
			3_990,
		),
	};
	Sample::new("scott", "beastie")
		.row(Os, format!("FreeBSD 14.3-RELEASE ({machine})"))
		.arch(machine)
		.row(Kernel, "14.3-RELEASE 199506")
		.row(Host, host)
		.up(6, 11, 48)
		.row(Cpu, cpu(brand, cores))
		.mem(free, total)
		.row(Terminal, terminal("xterm-256color", columns))
		.row(Shell, "sh")
}

fn macos(arch: &str, columns: usize) -> Sample {
	use FactKey::*;
	if arch == "x86_64" {
		return Sample::new("scott", "mbp")
			.row(Os, "macOS 14.7.1 (x86_64)")
			.arch("x86_64")
			.row(Kernel, "Darwin 23.6.0")
			.row(Host, "MacBookPro16,1")
			.up(4, 20, 3)
			.row(Cpu, cpu("Intel(R) Core(TM) i9-9980HK CPU @ 2.40GHz", 16))
			.mem(4_200, 16_384)
			.row(Terminal, terminal("Apple_Terminal", columns))
			.row(Shell, "zsh");
	}
	Sample::new("scott", "mbp")
		.row(Os, "macOS 15.7.1 (arm64)")
		.arch("arm64")
		.row(Kernel, "Darwin 24.6.0")
		.row(Host, "Mac15,6")
		.up(9, 2, 31)
		.row(Cpu, cpu("Apple M3 Pro", 11))
		.mem(6_400, 18_432)
		.row(Terminal, terminal("iTerm.app", columns))
		.row(Shell, "zsh")
}

fn windows(arch: &str) -> Sample {
	use FactKey::*;
	// The program lower-cases both names, and names the processor architecture the way `GetSystemInfo` does.
	let (product, kernel, machine, brand, cores) = match arch {
		"aarch64" => (
			"Windows 11 Pro",
			"NT 10.0.26100",
			"aarch64",
			"Snapdragon(R) X Elite - X1E78100 - Qualcomm(R) Oryon(TM) CPU",
			12,
		),
		"i586" => (
			"Windows 10 Pro",
			"NT 10.0.19045",
			"x86",
			"Intel(R) Core(TM)2 Duo CPU E6600 @ 2.40GHz",
			2,
		),
		_ => (
			"Windows 11 Pro",
			"NT 10.0.26100",
			"x86_64",
			"AMD Ryzen 7 PRO 6850U with Radeon Graphics",
			16,
		),
	};
	Sample::new("scott", "dusk-pc")
		.row(Os, format!("{product} ({machine})"))
		.arch(machine)
		.row(Kernel, kernel)
		.up(1, 7, 55)
		.row(Cpu, cpu(brand, cores))
		.mem(9_100, 15_800)
		.row(Display, display(2560, 1600, 32))
		.row(Shell, "cmd.exe")
}

fn windows_95() -> Sample {
	use FactKey::*;
	Sample::new("scott", "win95")
		.row(Os, "Windows 95 (x86)")
		.arch("x86")
		.row(Kernel, "Windows 95 4.00.950")
		.up(0, 2, 41)
		.row(Cpu, cpu("Pentium (x86 level 5)", 1))
		.mem(14, 32)
		.row(Display, display(640, 480, 8))
		.row(Shell, "command.com")
}

/// The facts the core's goldens use: what reactos-liminal's afterfetch printed.
fn reactos() -> Sample {
	use FactKey::*;
	Sample::new("administrator", "reactos")
		.row(Os, "ReactOS 0.4.16-x86")
		.row(Kernel, "NT 5.2.3790 SP3")
		.arch("x86")
		.up(0, 0, 1)
		.mem(387, 511)
		.row(Display, "1024 x 768, 32-bit")
		.row(Shell, "cmd.exe")
}

fn dos() -> Sample {
	use FactKey::*;
	Sample::new("scott", "")
		.row(Os, "MS-DOS 6.22")
		.arch("i386")
		.row(Kernel, "DOS 6.22, loaded high")
		.row(Host, "BIOS 06/06/92, model FC")
		.row(Cpu, cpu("486 class, FPU", 1))
		.mem(11, 15)
		.row(Display, "VGA, 80 x 25 text, 256 KB")
		.row(Shell, "command.com")
}

fn windows_3() -> Sample {
	use FactKey::*;
	Sample::new("scott", "WFW311")
		.row(Os, "Windows for Workgroups 3.11")
		.arch("i386")
		.row(Kernel, "386 enhanced mode")
		.up(0, 3, 12)
		.row(Cpu, cpu("486, FPU", 1))
		.row(Memory, "5 MB free")
		.row(Display, display(800, 600, 8))
		.row(Shell, "progman.exe")
		.row(Resources, resources(72))
}

/// Desk offers a module no system information today (AF-D26), so the module reports the OS row alone.
fn desk(track: &str) -> Sample {
	Sample::new("", "")
		.arch("i486")
		.row(FactKey::Os, format!("Desk 0.1.0 ({track})"))
}

fn other(columns: usize) -> Sample {
	use FactKey::*;
	Sample::new("scott", "puffy")
		.row(Os, "NetBSD 10.1 (amd64)")
		.arch("amd64")
		.row(Kernel, "NetBSD 10.1")
		.up(12, 5, 20)
		.row(Cpu, cpu("Intel(R) Xeon(R) E-2288G CPU @ 3.70GHz", 16))
		.mem(24_100, 31_900)
		.row(Terminal, terminal("xterm-256color", columns))
		.row(Shell, "ksh")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_system_and_architecture_has_a_sample_that_fits_a_row() {
		for system in &SYSTEMS {
			for arch in system.arches {
				let facts = sample(system, arch, 100);
				assert!(!facts.rows.is_empty(), "{} {arch}", system.name);
				for (key, value) in &facts.rows {
					assert!(value.len() <= 64, "{} {arch} {key:?}: {value}", system.name);
					assert!(value.is_ascii(), "{} {arch} {key:?}", system.name);
				}
			}
		}
	}

	#[test]
	fn names_and_architectures_accept_their_common_spellings() {
		assert_eq!(System::find("Windows").unwrap().name, "win32");
		assert_eq!(System::find("mac").unwrap().name, "macos");
		assert!(System::find("plan9").is_none());
		let linux = System::find("linux").unwrap();
		assert_eq!(linux.find_arch("amd64"), Some("x86_64"));
		assert_eq!(linux.find_arch("arm64"), Some("aarch64"));
		assert_eq!(linux.find_arch("i686"), Some("i586"));
		assert_eq!(linux.find_arch("sparc"), None);
		assert_eq!(System::find("freebsd").unwrap().find_arch("i386"), Some("i686"));
		assert_eq!(System::find("dos").unwrap().find_arch("i386"), Some("i386"));
	}

	#[test]
	fn each_sheet_system_exists_and_the_default_architecture_is_first() {
		for name in SHEET_SYSTEMS {
			assert!(System::find(name).is_some(), "{name}");
		}
		assert_eq!(System::find("linux").unwrap().arches[0], "x86_64");
	}

	#[test]
	fn desk_and_dos_report_only_what_they_can_read() {
		let desk = sample(System::find("desk").unwrap(), "i486", 80);
		assert_eq!(desk.rows.len(), 1);
		assert_eq!(desk.get(FactKey::Os), Some("Desk 0.1.0 (native)"));
		assert_eq!(System::find("desk").unwrap().target("i486"), "desk-i486");
		assert!(desk.user.is_empty() && desk.host.is_empty());
		let dos = sample(System::find("dos").unwrap(), "i386", 80);
		assert!(dos.uptime_seconds.is_none());
		for key in [FactKey::Desktop, FactKey::Terminal, FactKey::Resources] {
			assert!(dos.get(key).is_none(), "{key:?}");
		}
	}

	#[test]
	fn the_real_builds_are_linux_and_win32() {
		let linux = System::find("linux").unwrap();
		assert_eq!(linux.real_target("x86_64"), Some("linux-x86_64"));
		assert_eq!(linux.real_target("i586"), Some("linux-i586"));
		assert_eq!(linux.real_target("aarch64"), None);
		assert_eq!(System::find("win32").unwrap().real_target("i586"), Some("windows-i586"));
	}
}
