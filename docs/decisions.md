# Decisions

Each decision is numbered and stays here even when superseded, with a note on what replaced it. Status is **Owner** when the owner has decided it and **Assumed** when work proceeds on a recommended default until confirmed. Open questions move here once they are answered.

| # | Decision | Choice | Status | Why |
|---|---|---|---|---|
| AF-D1 | Name | `afterfetch`; 8.3 names `AFETCH.EXE` (DOS) and `AFETCHW.EXE` (Windows 3.1) | Owner | Ties the tool to Afterglow without naming a product, and both programs can share one `C:\UTILS`. |
| AF-D2 | Repository location | `~/source/liminal-hq-labs/afterfetch`, branch `main`, no remote | Owner | The owner approved the folder; it moves to `liminal-hq-labs/afterfetch` on GitHub later. |
| AF-D3 | What the Windows 3.1 program is | Both: the DOS build in the MS-DOS Prompt (it detects Windows through `INT 2Fh AX=1600h` and says so in the OS row) and a small Win16 window program, `AFETCHW.EXE`, with `/DUMP` for tests | Owner | Windows 3.1 has no console, so the DOS build covers the prompt and a window covers the desktop. |
| AF-D4 | qemu-user | Installed: `qemu-{arm,aarch64,mips,mipsel,riscv64,ppc64le}-static` and the rest in `/usr/bin`; no binfmt rules, smokes call the emulators directly | Owner | Static no-libc binaries need no sysroot, so qemu-user is the whole test bed. |
| AF-D5 | macOS and Windows ARM64 | CI-only, later, after the move to GitHub; macOS is built with no SDK (hand-authored `libSystem.tbd` plus `rust-lld`) | Owner | No local hardware, and Apple's SDK licence rules out osxcross-style builds off Apple hardware. |
| AF-D6 | FreeBSD test bed | The FreeBSD amd64 cloud image is downloaded to `~/.cache/afterfetch/freebsd`; ARM64 is build-only | Owner | The official image boots under the installed `qemu-system-x86_64`; there is no `qemu-system-aarch64`. |
| AF-D7 | Unix runtime | Raw syscalls, no libc, static, on Linux and FreeBSD; `libSystem` on macOS (required) | Assumed | Small binaries with no sysroot; macOS has no stable syscall ABI. |
| AF-D8 | Extra arches | RISC-V 64 and PPC64LE after MIPS; MIPS64 optional; no s390x or LoongArch unless asked | Assumed | Both have prebuilt `core` and cheap syscall stubs. |
| AF-D9 | `--json` | Yes, with stable keys documented in `SPEC.md` | Assumed | Scripts want the facts without parsing a logo. |
| AF-D10 | Logos per binary | All logos in every binary (at most 6 KiB together) so `--logo` works everywhere | Assumed | Predictable behaviour across platforms for a few kilobytes. |
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
| AF-D-Desk | Desk as a platform | Desk, Liminal HQ's Desk OS (`~/source/liminal-hq/desk`), is also a supported platform; its section is pending in the plan, and `docs/platforms.md` gets a row when that file exists | Owner (details pending) | Desk's `dos/` lineage makes a build plausible; the shape waits for the plan. |
| AF-D31 | Core shell interface | The core owns the whole program (`run`): a shell supplies `Platform` and `Terminal` and passes the arguments after its own name. Colour is a `Style` of `Paint` values (a tone, or one of the 16 console slots for swatches), so swatches need no extra trait method; `AnsiTerminal` encodes styles for the ANSI depths and console shells map them to attributes | Assumed | One small surface for six platforms, and every escape sequence is tested on the host. |
| AF-D32 | Logos before the art generator | Every logo name parses, and all of them draw the `win32` ASCII picture until the generated logos land. Memory is shown in gigabytes from 4 GiB of total, and all arithmetic on 64-bit values is shifts, so a Pentium build needs no 64-bit divide | Assumed | Lets `--logo` and the shells work now; the generated art replaces the picture without an interface change. |

The go/no-go records (AF-D21 slice accepted, AF-D22 MIPS, AF-D23 FreeBSD, AF-D24 Windows 3.1) are written when their checkpoints are reached.

## Open questions

- Whether real-hardware runs (a Pi, an OpenWrt router, a 486) are wanted before v0.1.0.
- The shape of the Desk build (AF-D-Desk).
