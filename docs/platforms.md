# Platforms

How each target is built, where its facts come from, how it is tested on the development machine, and what stays unverified. [SPEC.md](../SPEC.md) §13 says what each target is held to; this page says how.

**Status.** Linux x86_64 and i586 are built and pass their checks on this machine: structure, size, the native smoke (plain, JSON and a pseudo-terminal colour run) and two-directory reproducibility; i586 also runs under `qemu-i386-static`. The Win32 i586 and x86_64 builds pass their checks and run under Wine, and the ARM64 one builds and passes its structure checks, unverified locally (AF-D5). Every other target below is not started, and nothing has run on real hardware.

## Targets

Tier is Rust's support tier. Tier 3 and the custom targets have no prebuilt `core`, so they build it with `-Zbuild-std=core,compiler_builtins` under `RUSTC_BOOTSTRAP=1`, as every sibling project already does. The `musl` triples do not link musl: the Unix programs are `no_std` and `no_main` and link nothing; the triple only picks a static-friendly default.

| Target name | Rust target | Tier | `core` | Link | Local test | Unverified |
|---|---|---|---|---|---|---|
| `linux-x86_64` | `x86_64-unknown-linux-musl` | 2 | prebuilt | `rust-lld`, static, no libc (see [Unix](#linux-and-freebsd)) | Native | — |
| `linux-i586` | `i586-unknown-linux-musl` | 2 | prebuilt | as above | Native, and `qemu-i386-static` where the kernel cannot run 32-bit programs | — |
| `linux-aarch64` | `aarch64-unknown-linux-musl` | 2 | prebuilt | as above | `qemu-aarch64-static`, as a Cortex-A72 | Real kernel and hardware |
| `linux-armv7` | `armv7-unknown-linux-musleabihf` | 2 | prebuilt | as above, Thumb code | `qemu-arm-static`, as a Cortex-A7 | Real kernel and hardware |
| `linux-armv6` | `arm-unknown-linux-musleabihf` | 2 | prebuilt | as above, Thumb code | `qemu-arm-static`, as an ARM1176 | Real kernel and hardware; ARMv5 soft-float boards are not a target |
| `linux-mips`, `linux-mipsel` | `mips-unknown-linux-musl`, `mipsel-unknown-linux-musl` | 3 | build-std | as above, `asm_experimental_arch` | `qemu-mips-static`, `qemu-mipsel-static`, as a 24Kc | Real kernel and hardware; size budget 44 KiB (AF-D22) |
| `linux-mips64el` (optional) | `mips64el-unknown-linux-muslabi64` | 3 | build-std | as above | `qemu-mips64el-static` | Real kernel and hardware |
| `linux-riscv64` | `riscv64gc-unknown-linux-musl` | 2 | prebuilt | as above | `qemu-riscv64-static`, as `rv64` | Built and run under qemu-user locally; real hardware unverified |
| `linux-ppc64le` | `powerpc64le-unknown-linux-musl` | 2 | prebuilt | as above (inline assembly is stable on this target; no `asm_experimental_arch`) | `qemu-ppc64le-static`, as a POWER8 | Built and run under qemu-user locally; real hardware unverified; size budget 44 KiB (AF-D54) |
| `freebsd-x86_64` | `x86_64-unknown-freebsd` | 2 | prebuilt | `rust-lld`, static, no libc, FreeBSD ELF brand | FreeBSD VM under QEMU | Hardware; go/no-go AF-D23 |
| `freebsd-i686` | `i686-unknown-freebsd` | 2 | prebuilt | as above | Same VM, through `COMPAT_FREEBSD32` | Hardware; a native i386 install |
| `freebsd-aarch64` | `aarch64-unknown-freebsd` | 3 | build-std | as above | Structure only | Running it (AF-D6) |
| `macos-aarch64` | `aarch64-apple-darwin` | 1 | prebuilt | `rust-lld` darwin flavour against a hand-authored `libSystem.tbd`, ad-hoc signed | Structure only | Running it, until the GitHub macOS runner (AF-D5) |
| `macos-x86_64` | `x86_64-apple-darwin` | 2 | prebuilt | as above | Structure only | Running it, until the macOS runner under Rosetta 2 |
| `windows-i586` | `platforms/win32/i586-win32.json` (reshaped `i586-pc-windows-msvc`, Pentium, no SSE, soft float) | custom | build-std | `rust-lld` as `lld-link`, `raw-dylib` ANSI imports, no C runtime, `/SUBSYSTEM:CONSOLE,4.0 /FIXED` | Wine; Windows 95 in DOSBox-X once per release | A real Windows 95 to 11 console's colours (covered by goldens and the ReactOS guest) |
| `windows-x86_64` | `platforms/win32/x86_64-win32.json` (`x86_64-pc-windows-msvc`, plain x86-64, soft float, no unwind tables) | custom | build-std | as above, `/SUBSYSTEM:CONSOLE,5.02`, relocatable with `/DYNAMICBASE /HIGHENTROPYVA`, VT colour (AF-D52) | Wine (WoW64) | A real Windows 10 console |
| `windows-aarch64` | `platforms/win32/aarch64-win32.json` (`aarch64-pc-windows-msvc`) | custom | build-std | as above, `/SUBSYSTEM:CONSOLE,6.0`, relocatable, its own `__chkstk` (`rt_probe.rs`) | Structure only; **unverified locally**, nothing has run it | Running it, until the GitHub Windows ARM64 runner (AF-D5) |
| `dos-i386` | `platforms/dos/dos-elf.json` (the installer's, `cpu` lowered to `i386`, no x87) | custom | build-std | `rust-lld` with `link.ld`, then `elf2go32.py` behind `CWSDSTUB.EXE` | DOSBox Staging; FreeDOS in QEMU once per release | Real hardware; MDA and Hercules beyond DOSBox's `machine=hercules` |
| `win16-i386` | `platforms/win16/win16.json` (rust-win16's 16-bit code target) | custom | build-std | `rust-lld`, then `ne_pack.py` into a Windows 3.1 NE file | Wine `/DUMP`; Windows for Workgroups 3.11 in DOSBox-X | Windows 3.0, standard and real mode, a 286, real hardware |
| `desk-i486` | Desk's `i486-desk-none` (`i486-unknown-none-elf`, copied) | custom | build-std | `rust-lld` with Desk's `module.ld` at `0x0040_0000` and `--emit-relocs`, then `dmdpack pack` | Structure and host twin; Desk's emulators once Desk has a loader | Everything that runs, until the loader exists; real hardware |

Every release build uses `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` and `-Z location-detail=none`, and `scripts/repro.sh` builds it twice in different directories and compares with `cmp`. Size budgets are in SPEC.md §11.

## Win32

Source `platforms/win32`, its own Cargo workspace, from the reactos-liminal afterfetch with `PROVENANCE.md`.

- **Build:** `platforms/win32/build.sh <i586|x86_64|aarch64>` writes `dist/<arch>/afterfetch.exe`; `check.sh <arch>` checks it. Sizes (AF-D47, AF-D52): i586 24,576 of 24,576 bytes (no spare), x86_64 30,208 and ARM64 30,720 of 32,768. `panic=immediate-abort` on all three. `scripts/repro-win32.sh` builds each in three directories and compares them byte for byte.
- **Facts:** `GetUserNameA`, `GetComputerNameA`, `GetVersionExA` (the Kernel row says `Windows 95 4.00.950` on 9x and `NT 5.2.3790 SP3` on the NT family), registry `ProductName` and `BuildLab`, `GetSystemInfo`, `GetTickCount64` and `GlobalMemoryStatusEx` through `GetProcAddress` with `GetTickCount` and `GlobalMemoryStatus` as the fallback, `GetSystemMetrics`, `GetDeviceCaps`, `%COMSPEC%`, and the CPU row: the `CPUID` brand string by `core::arch` on x86 and x86_64 (32-bit builds test the EFLAGS ID bit first, so a 486 without CPUID is safe), else the registry `HARDWARE\DESCRIPTION\System\CentralProcessor\0` `ProcessorNameString` (the only source on ARM64), else the CPUID vendor and family of a Pentium with no brand string. Under Wine the brand string is the host's and the memory and uptime are the host's. ReactOS: its own System Properties dialog reads the same registry value, and the CPUID brand comes first anyway; the ReactOS guest has not been run for this row.
- **Colour:** the Pentium build writes console attributes on a console and nothing on a pipe. The two 64-bit builds first ask a console for `ENABLE_VIRTUAL_TERMINAL_PROCESSING` and `SetConsoleOutputCP(65001)`; when it accepts, the depth hint is true colour, the core's `AnsiTerminal` writes the escapes, the logo is drawn in half blocks, and the console's mode and code page are put back at exit. A pipe never gets escapes (AF-D52).
- **Imports:** ANSI names only, every one exported by Windows 95's system DLLs, listed in `imports.allow`; one list for all three arches, with the four console calls only the 64-bit builds make marked `64`. No `…W` import.
- **Structure checks:** machine type, PE flavour and subsystem version per arch (4.0, 5.02, 6.0), no relocations on i586, while x86_64 and ARM64 stay relocatable with `DYNAMIC_BASE`, `HIGH_ENTROPY_VA` and `NX_COMPAT` (ARM64 requires it), the section list, imports against the allow-list, and on i586 no instruction a Pentium lacks.
- **Local test:** `scripts/smoke/wine.sh` runs i586 and x86_64 under Wine 11.21 (WoW64) on its own Xvfb: `--plain` and `--json` on a pipe checked by `scripts/smoke/shape.py` (the CPU row must be there, without a NUL, and a machine over 6 GiB must report more than 4 GiB), then a console run under a pseudo-terminal, where i586 shows its console attributes as Wine turns them into escapes and x86_64 shows true colour and UTF-8 half blocks (Wine's console passes VT through). Windows 95 runs once per release in the DOSBox-X test bed (`/mnt/games/media/win95-testbed`, image `liminal-win95-dosbox-x:local`), writing `C:\OUT.TXT`, read back with mtools.
- **Unverified locally:** ARM64 (Wine cannot run an ARM64 PE on x86; it runs on a GitHub Windows ARM64 runner after the move, AF-D5), a real Windows 10 console's VT path, Windows 95 and ReactOS for the new rows.

## Linux and FreeBSD

Source `platforms/unix`, the crate `afterfetch-unix` in the root workspace, with `arch/*.rs` per architecture and `os/{linux,freebsd}.rs`.

- **Link:** `rust-lld` as `ld.lld`, `relocation-model=static`, `-static`, `--gc-sections`, `--build-id=none`, `-znorelro`, `--entry=_start`. No libc, no crt objects. `_start` is a `global_asm!` stub per arch that hands the initial stack (`argc`, `argv`, `envp`) to `rt.rs`, and each arch has one `syscall3` stub, since the program uses at most three system call arguments (AF-D38). 32-bit Arm builds Thumb code (`+thumb-mode`) to stay inside the budget and keeps r7 free by saving it in r12 around each call (AF-D51). MIPS and FreeBSD ARM64 add build-std with `compiler-builtins-mem`.
- **Linux facts:** `/etc/os-release`, `/proc/meminfo`, `/proc/cpuinfo`, `/proc/uptime`, `/etc/passwd`, `/sys/devices/virtual/dmi/id/product_name`, `/proc/device-tree/model`, `uname`, `sysinfo`, `TIOCGWINSZ`, environment (including `XDG_CURRENT_DESKTOP` for the Desktop row, AF-D35); the fallbacks are AF-D39. The parsers live in the core and are tested against captured files from real systems (x86_64, a Raspberry Pi 4, a Pi Zero, an OpenWrt MIPS router), because `/proc/cpuinfo` under qemu-user shows the host's processor.
- **FreeBSD facts:** `__sysctl` (names resolved through the name-to-OID MIB), `clock_gettime`, `kenv` for the SMBIOS product (omitted when restricted), `TIOCGETA`, `TIOCGWINSZ`. Errors come back in the carry flag. LLVM sets the FreeBSD `EI_OSABI` for a `*-freebsd` triple; a structure check asserts it, and a `.note.tag` ABI note is added if the kernel wants one.
- **Structure checks:** `llvm-readelf`: static, no `PT_INTERP`, no `DT_NEEDED`, the right machine and OSABI.
- **Local test, Linux:** x86_64 and i586 natively (plain, JSON, and a colour run inside `script -qc` for a pseudo-terminal), through `scripts/smoke/linux-native.sh`; i586 falls back to `qemu-i386-static` when the kernel cannot run it. All pass today. The Arm builds always run under `qemu-aarch64-static` or `qemu-arm-static`, named on the command line (no binfmt needed), with `QEMU_CPU` set to a Raspberry Pi-like processor (`cortex-a72`, `cortex-a7`, `arm1176`), so an instruction the real core lacks faults here too; they pass plain, JSON, exit 74 on `/dev/full` and the four colour depths through `script`. That proves the instruction sequence and the system call interface, not the parsers on real hardware: qemu-user answers `uname`, `/proc` and `/etc/os-release` from the host (the smoke shows the host as `aarch64` or `armv7l`). The others through qemu-user-static 11.1.3: binfmt rules for each emulator are registered on this machine, but `scripts/smoke/qemu-user.sh` maps each triple to `/usr/bin/qemu-<arch>-static` and calls it by name, so the smoke does not depend on them. Static no-libc binaries need no sysroot, so qemu-user is the whole test bed.
- **Local test, FreeBSD:** `scripts/freebsd-vm.sh fetch` downloads the official `FreeBSD-<release>-amd64-BASIC-CLOUDINIT-ufs.qcow2.xz` once into `~/.cache/afterfetch/freebsd/` (about 0.6 GB; pinned by name and SHA-256, checked against the release `CHECKSUM.SHA256`; `df -h /home` first; one image only). Not fetched yet. `qemu-system-x86_64 -m 1024 -smp 2` boots it with a snapshot overlay so the base never changes; the binary goes in on a small FAT image made with mtools and runs over the serial console under `scripts/smoke/freebsd-serial.py`, always inside `flock ~/.cache/reactos-liminal/qemu.lock`. The i686 build runs in the same guest.
- **Verified locally, Arm:** built and run under qemu-user only. Real Raspberry Pi hardware and kernels are unverified.
- **Verified locally, MIPS:** `mips` (big-endian) and `mipsel` build on the pinned 1.98.0 with `-Zbuild-std=core,compiler_builtins` and `RUSTC_BOOTSTRAP=1`, and run under qemu-user as a 24Kc (MIPS32 release 2, the oldest core the target's baseline allows): plain, `--json`, exit 74 on `/dev/full` and the colour depths through a pseudo-terminal. Real routers, kernels and OpenWrt are unverified; the OpenWrt `cpu model` shape is covered by the core's captured file.
- **Verified locally, RISC-V and PowerPC:** `riscv64` and `ppc64le` build on the pinned 1.98.0 from the prebuilt libraries (no `build-std`, no `RUSTC_BOOTSTRAP`) and run under qemu-user as `rv64` and `power8`: plain shape, `--json`, exit 74 on `/dev/full`, the four colour depths, and byte-identical builds in three directories. RISC-V is 30.0 KB; PowerPC is 42.5 KB (AF-D54).
- **Unverified locally:** real RISC-V and PowerPC kernels and hardware; real ARM and MIPS kernels and hardware (qemu-user uses the host kernel); FreeBSD ARM64, since there is no `qemu-system-aarch64`. Later options: OpenWrt `malta` under `qemu-system-mipsel`, and `vmactions/freebsd-vm` in CI for x86_64.

## macOS

Source `platforms/unix` with `os/macos.rs`.

- **Why different:** macOS has no stable system-call ABI, so the program calls `libSystem`. Apple's SDK is licensed for Apple hardware, so the build uses none: `platforms/unix/macos/libSystem.tbd` is a hand-authored text stub listing only the symbols the program calls.
- **Link:** `rust-lld` darwin flavour, entry `_main` through `LC_MAIN`, `-platform_version macos 11.0` on arm64 and `10.12` on x86_64, `-dead_strip`. lld ad-hoc signs arm64 output, which arm64 macOS requires, and derives `LC_UUID` from the content, so the output is reproducible.
- **Facts:** `sysctlbyname` (`kern.osproductversion`, `kern.osrelease`, `kern.hostname`, `hw.model`, `machdep.cpu.brand_string`, `hw.ncpu`, `hw.memsize`, `hw.pagesize`, `vm.page_free_count`, `kern.boottime`), `gettimeofday`, `ioctl`, environment. There is no Display row: no CoreGraphics.
- **Structure checks:** `llvm-objdump --macho --private-headers` (one `LC_LOAD_DYLIB`, libSystem; `LC_MAIN`; `LC_CODE_SIGNATURE` on arm64; the `LC_BUILD_VERSION` minimum) and `llvm-nm -u` against the stub's symbols.
- **Unverified locally:** running it, at all. A GitHub `macos-15` runner runs both arches (x86_64 under Rosetta 2) after the move to GitHub; that first run is the go/no-go. If dyld rejects the stub, the fallback is Apple's own linker on the runner.

## DOS

Source `platforms/dos`, its own workspace. The runtime (`start`, `dpmi`, `dos`, `env`, `panic`, `cleanup`, no heap) is copied from liminal-installer's `dos/rt`, which came from Desk, with `PROVENANCE.md`.

- **Facts:** `INT 21h` (`30h`, `3306h`, `4452h`, `4400h`, `5E00h`), `INT 2Fh AX=1600h`, `INT 12h`, `INT 15h` (`E801h`, `88h`), DPMI `INT 31h AX=0500h`, `INT 10h` (`1A00h`, `4F00h`), the BIOS data area and ROM bytes through `dpmi::linear_to_ptr`, EFLAGS and `CPUID` by inline assembly, the PSP environment.
- **Output:** plain through `INT 21h AH=40h` to handle 1, so `AFETCH > OUT.TXT` and `| MORE` work. Colour through BIOS `INT 10h` (`09h` character and attribute, `02h`/`03h` cursor, `06h` scroll), which respects the active page and works on MDA, CGA, EGA, VGA and in a Windows 3.1 DOS box.
- **Tools:** `platforms/dos/tools/fetch-tools.sh`, copied from the installer with the same URLs and SHA-256s, fetches DOSBox Staging 0.83.0 and CWSDPMI r7b into `platforms/dos/vendor/`, or into `DOS_VENDOR=~/.cache/afterfetch/dos-vendor` to share one copy. DOSBox Staging is not on `PATH` here; a fetched copy also sits in `~/.cache/li-polish-dosvendor/dosbox/`.
- **Local test:** `tools/dosrun.sh dist AFETCH.EXE /PLAIN` runs headless DOSBox Staging (`SDL_VIDEODRIVER=dummy`, `cputype=486`), captures standard output to `D:\STDOUT.TXT` and the exit code through an `IF ERRORLEVEL` ladder, then the shape check. A colour run sets `AF_DUMP=D:\SCREEN.TXT` and compares the screen read back with the `console16` golden. Once per release, FreeDOS in `qemu-system-i386` from a copy of `~/.cache/quill16-try/fl-qemu.img` (never the original), under the QEMU lock, checks a non-DOSBox BIOS; a Windows 3.11 DOS-box run uses the Windows 3.1 harness below.
- **Unverified locally:** real hardware; a 386 without a coprocessor beyond DOSBox's `cputype`.

## Windows 3.1

Source `platforms/win16`, its own workspace. The `w16` runtime and the packer tools (`ne_pack.py`, `check_segments.py`, `gen_imports.py`, `imports.tbl`) are copied from `rust-win16` with `PROVENANCE.md`. Quill's lessons are taken as given: import stubs save full 32-bit registers, a 32-bit DPMI alias of the stack, jump tables off, small frames.

- **Facts:** `GetVersion`, `GetWinFlags`, `GetFreeSpace`, `GetTickCount`, `GetFreeSystemResources`, `GetSystemMetrics`, `GetDeviceCaps`, `GetPrivateProfileString` on `SYSTEM.INI`.
- **Packing:** `ne_pack.py --module AFETCHW --stack 0x2000 --heap 0x0800 --segments res/afetchw.segments.json`; one code segment, `MAIN`, budget 20,000 bytes.
- **Local test:** `tools/e2e-wine.sh` (`xvfb-run wine AFETCHW.EXE /DUMP=…`, shape check). Manually, Windows for Workgroups 3.11 in DOSBox-X: the tree from `~/Downloads/win31.zip` unpacked to a scratch copy in `~/.cache/afterfetch/w31/` (never the shared one), an image `afterfetch-dosbox-x:local` built from a copy of the installer's `retro/Dockerfile`, its own Xvfb display, and screenshots of that display only, saved to `~/Documents/Liminal HQ/Afterfetch/renders/w31/`, at S3 800 × 600 in 256 colours and VGA 640 × 480 in 16. Wine is a smoke test only; it is more forgiving than Windows 3.11.
- **Unverified locally:** Windows 3.0, standard and real mode, a 286, real hardware.

## Desk

Source `platforms/desk`, its own workspace. Desk's `module/guest`, `module/format`, `module/pack`, `desk.magic`, the module target JSON and `module.ld` are copied into `platforms/desk/vendor/` from Desk at a pinned commit with `PROVENANCE.md`; the build never writes into Desk's checkout. `dmdpack` is built from the copy as a host tool. A module may not use `dos/rt` (no interrupts, no DPMI), so Desk shares no runtime with the DOS build, only the core.

- **Facts:** none read directly; a module never touches hardware. Until Desk offers `desk_sys_info` (AF-D26), the module reports the OS row only.
- **Output:** plain lines through `desk_log`.
- **Host checks, in `make check`:** `dmdpack info AFETCH.DMD` against a golden (a module, SDK 1, importing exactly `desk_log` and `desk_panic`); `dmdpack cross` linking at `0x0040_0000`, `0x0080_0000`, `0x0040_3000` and `0x4040_0000` with equal images and relocation tables; `file -b -m vendor/desk.magic`; a 486 instruction scan; a build-host path scan; size. A host twin of `desk_log` runs the module's logic and compares it with the `desk` goldens.
- **Preview:** `cargo run -p afterfetch-preview -- --system desk` renders what the module prints (the OS row alone, until AF-D26) beside the `desk` logo, as text, ANSI, a picture or a page (AF-D50, [docs/preview.md](preview.md)).
- **Emulator, once Desk has a loader:** a Desk image built from a copy of Desk's tree in `~/.cache/afterfetch/desk-src` at a pinned commit (never `~/source/liminal-hq/desk`). Native track: Desk's `tools/native-run.sh` (headless QEMU with a 486, serial transcript, `isa-debug-exit` code 33 for a pass), wrapped in `flock ~/.cache/reactos-liminal/qemu.lock` because Desk's own scripts take no lock. Hosted track: `DMDRUN.EXE AFETCH.DMD` in DOSBox Staging through `dosrun.sh`, once `DMDRUN.EXE` exists. Both compare their lines with the plain golden.
- **Unverified:** everything that runs, until Desk's loader exists (Checkpoint D, AF-D28); real hardware.

## Test beds on this machine

| Tool | Where | Used by |
|---|---|---|
| Rust 1.98.0, `cargo-nextest`, `rust-lld` | rustup | everything |
| qemu-user-static 11.1.3 | `/usr/bin/qemu-*-static`, binfmt rules registered | Linux i586 (`qemu-i386-static`), ARM, MIPS, RISC-V, PowerPC |
| `qemu-system-x86_64`, `qemu-system-i386` | system | FreeBSD VM, FreeDOS, Desk native track |
| FreeBSD amd64 cloud image | `~/.cache/afterfetch/freebsd/` (to be fetched) | FreeBSD |
| Wine 11.21 (WoW64) | system | Win32 i586 and x86_64, Windows 3.1 smoke |
| DOSBox Staging 0.83.0 | fetched by `fetch-tools.sh` | DOS, Desk hosted track |
| DOSBox-X | Docker images | Windows 3.11, Windows 95 |
| Windows for Workgroups 3.11 tree | `~/Downloads/win31.zip` | Windows 3.1 manual run |
| FreeDOS floppy | `~/.cache/quill16-try/fl-qemu.img` (copied, never modified) | DOS once per release |

Not available here, and so never verified locally: `qemu-system-aarch64`, `qemu-system-mips` and `qemu-system-arm`; any macOS or Windows on ARM machine; real retro hardware.
