# Afterfetch for Win32

A console program for Windows 95 and later and ReactOS, with no C runtime: `no_std`, `no_main`, an entry point of its own, and every Win32 call imported by its ANSI name. Three builds: i586 (a Pentium, no SSE), x86_64 and ARM64. ARM64 builds and passes its structure checks but has never run (AF-D5, CI only).

## Build and check

```sh
platforms/win32/build.sh i586            # or x86_64 or aarch64: build-std, rust-lld as lld-link; add `clippy` to lint instead
platforms/win32/check.sh i586            # PE structure, imports, Pentium scan (i586), size budget: 24 KiB for i586, 32 KiB for the 64-bit builds
bash scripts/smoke/wine.sh               # plain output, --json, the golden logo and a console run, for i586 and x86_64, under Wine on its own Xvfb
bash scripts/repro-win32.sh              # three builds of each arch in three directories of different depth, compared byte for byte
(cd platforms/win32 && cargo nextest run --lib --target x86_64-unknown-linux-gnu)    # the host tests
```

`make check` runs all of them. This is its own Cargo workspace, because the build-std flags and the custom target must not reach the host build. `build.sh` needs `RUSTC_BOOTSTRAP=1` (it sets it) for `-Zbuild-std`: the prebuilt `core` assumes SSE2, which a Pentium lacks, and no prebuilt one carries `memcpy` and the rest, so `core` is built from source for `<arch>-win32.json`. Each target's flags are in `.cargo/config.toml`, keyed by the JSON file's name.

## Reproducibility

The same source and toolchain give the same file in any checkout directory. Cargo hashes a path dependency into its crate metadata (and so into symbol hashes, and the order of the code) as a path relative to the workspace root, but falls back to the absolute path when the dependency sits outside that root. The core and the art crate used to be reached as `../../crates/...`, outside this workspace, so the build changed with the directory it ran in: two of five directories gave one executable and three gave another, differing in code layout and in the `/Brepro` timestamp, which is a hash of the output. `crates` here is a symlink to `../../crates`, so both crates are reached under this workspace's root, and the workspace carries the `[workspace.package]` and `[workspace.lints]` tables that the core inherits (keep them equal to the root `Cargo.toml`'s). `scripts/repro-win32.sh` builds in three directories of different depth and length so that a lucky pair cannot hide a regression.

## Layout

- `src/lib.rs`: the parts that need no Windows, so the host can test them: `cmdline` (the command line split as the C runtime splits it), `attribute` (a core `Style` as a console attribute), `kernel` (the Kernel row, `Windows 95 4.00.950` or `NT 5.2.3790 SP3`) and `cpu` (CPUID registers as text).
- `src/processor.rs`: the `CPUID` calls, with the EFLAGS ID-bit test on 32-bit x86.
- `src/main.rs`: the entry point, `start`.
- `src/os.rs`: the `Platform`, the facts from the Win32 calls, and the whole run. `Out::flush` tells the core whether every write worked, and `run` exits 74 when one failed (AF-D33).
- `src/term.rs`: the buffered output and the `Terminal`. On a console a style is `SetConsoleTextAttribute` and every line ends by restoring the console's own attribute; the program restores it again at exit. On a pipe or a file nothing is styled. The 64-bit builds first ask a console for VT processing and UTF-8 (`ConsoleMode`); when it accepts, styles go through the core's `AnsiTerminal` and the mode and code page are put back at exit. The Pentium build never takes that branch (`VT_BUILD` is a constant, so it is not linked): the writer would cost it 2 KB.
- `src/sys.rs`: the Win32 declarations, `raw-dylib`, ANSI only. `imports.allow` is the list; `tools/check-imports.sh` keeps the executable to it, both ways.
- `src/rt.rs`: the panic handler (exit 70). `src/rt_x86.rs` (Pentium only): the 64-bit division helpers and the stack probe the 32-bit MSVC ABI expects. `src/rt_probe.rs` (x86_64 and ARM64): the 64-bit `__chkstk`; the x86-64 one must keep RAX, which the caller subtracts next.
- `i586-win32.json`, `x86_64-win32.json`, `aarch64-win32.json`, `.cargo/config.toml`: the targets and the link flags (`/NODEFAULTLIB /ENTRY:start /SUBSYSTEM:CONSOLE,<4.0|5.02|6.0> /Brepro`, then `/FIXED /DYNAMICBASE:NO` for i586 and `/DYNAMICBASE /HIGHENTROPYVA` for the others; `-Z location-detail=none` and `-C panic=immediate-abort` for all). The x86-64 target is soft-float on the plain x86-64 baseline with no unwind tables (about 2 KB smaller than the stock target); the ARM64 one keeps the default features and its unwind tables, because nothing here can run it.
- `tools/`: `check-pe.sh`, `check-imports.sh` and `scan-pentium.sh`.

## Facts

User and host from `GetUserNameA` and `GetComputerNameA`; the OS from the registry (`ProductName`, and `BuildLab` for the ReactOS release) and `GetSystemInfo` for the architecture; the kernel from `GetVersionExA` (`Windows 95 4.00.950` on 9x, `NT 5.2.3790 SP3` on NT); the uptime from `GetTickCount64` where `kernel32` has it, else `GetTickCount` (it wraps at 49 days); memory from `GlobalMemoryStatusEx` where `kernel32` has it, else `GlobalMemoryStatus` (it saturates at 4 GiB, and so do the byte counts in `--json`); the CPU from the CPUID brand string, else the registry's `ProcessorNameString`, else the vendor and family of a Pentium (the count is `GetSystemInfo`'s processors, so a hyper-threaded machine counts its threads); the architecture name for `arch` and the OS row from `GetSystemInfo`; the display from `GetSystemMetrics` and `GetDeviceCaps`; the shell from `COMSPEC`. There is no host-model, desktop or terminal row. The depth hint is `Console16` (true colour once a 64-bit build's console accepts VT), the width is the console window's, and a narrow window drops the logo in the core.

## Notes for the next package

- **Size.** The Pentium build is 24,576 of 24,576 bytes: its text is padded by under 500 bytes and its read-only data by under 70, so any new code over about 480 bytes needs another 512-byte file block and fails the check. The first things to look at are the 64-bit division helpers (`rt_x86.rs`, which nothing calls but `compiler_builtins` names), the registry code, and the core's `push_size`. The 64-bit builds are 30,208 and 30,720 of 32,768 (AF-D52); the 20 KiB first planned would need a smaller core.
- `/MERGE:.data=.rdata` is for the Pentium build, which has no writable data. A mutable static would fault, so `tools/check-pe.sh` insists on the section lists it checks.
- The environment is read once for the four names the program uses (`NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `COMSPEC`), because Windows 95 has no call that returns the whole block. A fact that needs another variable adds it to `VARIABLES` in `os.rs`.
- A 64-bit build leaves the console in VT mode and UTF-8 only while it runs; a crash (exit 70) skips the restore.
- The x86-64 stack probe `__chkstk` must keep RAX. The first version did not and crashed under Wine on the first large frame.
- The 64-bit PE files stay relocatable (ARM64 requires it), so `check-pe.sh`'s no-relocations test is the i586 one.
- Not yet run: Windows 95 (DOSBox-X), the ReactOS guest for the CPU and kernel rows, a real Windows 10 console for the VT path, and ARM64 anywhere.
