# Decisions

Each decision is numbered and stays here even when superseded, with a note on what replaced it. Status is **Owner** when the owner has decided it and **Assumed** when work proceeds on a recommended default until confirmed. Open questions move here once they are answered.

| # | Decision | Choice | Status | Why |
|---|---|---|---|---|
| AF-D1 | Name | `afterfetch`; 8.3 names `AFETCH.EXE` (DOS) and `AFETCHW.EXE` (Windows 3.1) | Owner | Ties the tool to Afterglow without naming a product, and both programs can share one `C:\UTILS`. |
| AF-D2 | Repository location | `~/source/liminal-hq-labs/afterfetch`, branch `main`, no remote | Owner | The owner approved the folder; it moves to `liminal-hq-labs/afterfetch` on GitHub later. |
| AF-D3 | What the Windows 3.1 program is | Both: the DOS build in the MS-DOS Prompt (it detects Windows through `INT 2Fh AX=1600h` and says so in the OS row) and a small Win16 window program, `AFETCHW.EXE`, with `/DUMP` for tests | Owner | Windows 3.1 has no console, so the DOS build covers the prompt and a window covers the desktop. |
| AF-D4 | qemu-user | Installed: `qemu-{arm,aarch64,mips,mipsel,riscv64,ppc64le}-static` and the rest in `/usr/bin`; smokes call the emulators by name, so they work with or without binfmt rules (rules are now registered on the development machine) | Owner | Static no-libc binaries need no sysroot, so qemu-user is the whole test bed. |
| AF-D5 | macOS and Windows ARM64 | CI-only, later, after the move to GitHub; macOS is built with no SDK (hand-authored `libSystem.tbd` plus `rust-lld`) | Owner | No local hardware, and Apple's SDK licence rules out osxcross-style builds off Apple hardware. |
| AF-D6 | FreeBSD test bed | The FreeBSD amd64 cloud image is downloaded to `~/.cache/afterfetch/freebsd`; ARM64 is build-only | Owner | The official image boots under the installed `qemu-system-x86_64`; there is no `qemu-system-aarch64`. |
| AF-D7 | Unix runtime | Raw syscalls, no libc, static, on Linux and FreeBSD; `libSystem` on macOS (required) | Assumed | Small binaries with no sysroot; macOS has no stable syscall ABI. |
| AF-D8 | Extra arches | RISC-V 64 and PPC64LE after MIPS; MIPS64 optional; no s390x or LoongArch unless asked | Assumed | Both have prebuilt `core` and cheap syscall stubs. |
| AF-D9 | `--json` | Yes, with stable keys documented in `SPEC.md` | Assumed | Scripts want the facts without parsing a logo. |
| AF-D10 | Logos per binary | All logos in every binary (at most 7 KiB together, eight logos with `desk`) so `--logo` works everywhere | Assumed | Predictable behaviour across platforms for a few kilobytes. |
| AF-D11 | Landing | Straight onto `main`, `make check` before each commit; parallel builders use worktrees and fast-forward; the merge-commit rule resumes with pull requests on GitHub. See [landing.md](landing.md) | Owner | With no pull requests a merge commit would record nothing. |
| AF-D12 | Art licence | Same as the code, `Apache-2.0 OR MIT` | Assumed | One licence for the repository. |
| AF-D13 | Win32 colour | Console attributes everywhere, VT truecolour when `SetConsoleMode` accepts it; cells, not half blocks, on legacy consoles | Assumed | Raster fonts and code pages vary on legacy consoles. |
| AF-D14 | Decision ids | `AF-D<n>` in one table, never removed | Assumed | Matches the sibling projects. |
| AF-D15 | Windows 3.1 colour | Nearest of 16 VGA colours on 16-colour drivers, exact RGB on 256-colour and above | Assumed | What the display driver can honestly show. |
| AF-D16 | DOS floor | A 386 with DPMI (CWSDPMI embedded); no 8086 or 286 | Assumed | Protected mode keeps the Rust runtime simple. |
| AF-D17 | Arch marks | Text plus an accent colour; no per-architecture art | Owner | One sun per system type is enough; the OS row carries the architecture. |
| AF-D18 | CI image tiers | The owner's later job; the `ci-cross` and `ci-art` proposals stay in `docs/ci.md` only | Owner | They belong in `liminal-hq/.github`, not here. |
| AF-D19 | 32-bit x86 Linux | `i586` (Pentium, no SSE2), matching Win32 | Assumed | One 32-bit x86 floor across platforms. |
| AF-D20 | Missing facts | Left out: the row is omitted, and JSON gives `null` | Owner | A dash on every row of a platform that cannot report it is noise. |
| AF-D25 | What afterfetch is on Desk | A `.DMD` module, `AFETCH.DMD`, that prints through `desk_log` now, and a window app over Desk's SDK later; not a kernel test mode | Assumed | The owner made Desk a supported platform; a module is Desk's own program container, and a kernel test mode would mean editing Desk. |
| AF-D26 | Desk facts | Propose a `desk_sys_info` SDK call to the Desk project (`docs/desk-sys-info.md`); until then report only OS "Desk" and the track; never touch hardware from a module | Assumed | Desk exposes nothing to modules yet, and its SDK rules forbid hardware access. |
| AF-D27 | Desk code reuse | Copy `desk-guest`, `desk-module`, `dmdpack`, the module target and `module.ld` from Desk into `platforms/desk/vendor/` with `PROVENANCE.md`; no runtime shared with DOS; share only the core and a `PixelPainter` with Windows 3.1 | Assumed | Desk stays read-only, and a module may not use `dos/rt`. |
| AF-D28 | Desk fallback if Checkpoint D is no-go | Ship `afterfetch-preview --system desk` renders, the host-verified `AFETCH.DMD` marked "waiting on Desk's loader", and the DOS build for Desk's hosted machine | Assumed | Desk has no loader yet; the platform must never block the others. |
| AF-D29 | Desk logo palette | Desk's own 16-colour set (the Void, the stepped halo, the stepped horizon) plus Afterglow rose; no Desk wordmark | Assumed | Desk's identity is its halo and palette, not a logo, so the sun looks native at 4-bit and 8-bit. |
| AF-D30 | Preview tool | `afterfetch-preview`, a host-only `std` crate, never shipped; it also makes the README screenshots for every system | Assumed | It is the face of platforms that cannot run yet, and one way to picture all of them. |
| AF-D31 | Core shell interface | The core owns the whole program (`run`): a shell supplies `Platform` and `Terminal` and passes the arguments after its own name. Colour is a `Style` of `Paint` values (a tone, or one of the 16 console slots for swatches), so swatches need no extra trait method; `AnsiTerminal` encodes styles for the ANSI depths and console shells map them to attributes | Assumed | One small surface for six platforms, and every escape sequence is tested on the host. |
| AF-D32 | Logos before the art generator | Every logo name parses, and all of them draw the `win32` ASCII picture until the generated logos land. Memory is shown in gigabytes from 4 GiB of total, and all arithmetic on 64-bit values is shifts, so a Pentium build needs no 64-bit divide | Assumed | Lets `--logo` and the shells work now; the generated art replaces the picture without an interface change. |
| AF-D33 | Exit code for an output failure | 74 (I/O error, the BSD sysexits `EX_IOERR`) when output cannot be written: a closed pipe, or a `/DUMP` file that cannot be created. The codes are 0 ok, 2 usage, 70 crash, 74 output failure | Owner | Scripts can tell a failed write from a bad option or a crash. |
| AF-D34 | JSON numbers and build identity | `--json` carries raw numbers (uptime in seconds, memory in bytes) and the build's `target` and `arch` beside the display text, in a versioned schema | Owner | Scripts get numbers they can compare; the text rows stay as they are. |
| AF-D35 | Linux desktop row | Linux reports the desktop environment from `XDG_CURRENT_DESKTOP` as a row only when it is set | Owner | A desktop session shows GNOME or KDE; a console or SSH session stays tidy. |
| AF-D36 | Accent tones for the rarer architectures | Teal for RISC-V and coral for PowerPC, as accent colours beside the existing x86, ARM and MIPS accents | Owner | Each architecture stays distinct with no extra artwork (AF-D17). |

The go/no-go records (AF-D21 slice accepted, AF-D22 MIPS, AF-D23 FreeBSD, AF-D24 Windows 3.1) are written when their checkpoints are reached.

## Open questions

- Whether real-hardware runs (a Pi, an OpenWrt router, a 486) are wanted before v0.1.0.
