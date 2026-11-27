# Afterfetch for Win32

A console program for Windows 95 and later and ReactOS, with no C runtime: `no_std`, `no_main`, an entry point of its own, and every Win32 call imported by its ANSI name. The i586 build is the one that exists today (a Pentium, no SSE); the x86_64 and ARM64 builds are a later package.

## Build and check

```sh
platforms/win32/build.sh i586            # build-std for a Pentium, rust-lld as lld-link; add `clippy` to lint instead
platforms/win32/check.sh                 # PE structure, imports, Pentium scan, 24 KiB budget
bash scripts/smoke/wine.sh               # plain output, --json, the golden logo and the console attributes, under Wine on its own Xvfb
bash scripts/repro-win32.sh              # two builds in two directories, compared byte for byte
(cd platforms/win32 && cargo nextest run --lib --target x86_64-unknown-linux-gnu)    # the host tests
```

`make check` runs all of them. This is its own Cargo workspace, because the build-std flags and the custom target must not reach the host build. `build.sh` needs `RUSTC_BOOTSTRAP=1` (it sets it) for `-Zbuild-std`: the prebuilt `core` assumes SSE2, which a Pentium lacks, so `core` is built from source for `i586-win32.json`.

## Layout

- `src/lib.rs`: the parts that need no Windows, so the host can test them: `cmdline` (the command line split as the C runtime splits it) and `attribute` (a core `Style` as a console attribute).
- `src/main.rs`: the entry point, `start`.
- `src/os.rs`: the `Platform`, the facts from the Win32 calls, and the whole run (exit code 74 when a write fails).
- `src/term.rs`: the buffered output and the `Terminal`. On a console a style is `SetConsoleTextAttribute` and every line ends by restoring the console's own attribute; the program restores it again at exit. On a pipe or a file nothing is styled.
- `src/sys.rs`: the Win32 declarations, `raw-dylib`, ANSI only. `imports.allow` is the list; `tools/check-imports.sh` keeps the executable to it, both ways.
- `src/rt.rs`: the panic handler (exit 70), the 64-bit division helpers and the stack probe the MSVC ABI expects.
- `i586-win32.json`, `.cargo/config.toml`: the target and the link flags (`/NODEFAULTLIB /ENTRY:start /SUBSYSTEM:CONSOLE,4.0 /FIXED /DYNAMICBASE:NO /Brepro`, `-Z location-detail=none`).
- `tools/`: `check-pe.sh`, `check-imports.sh` and `scan-pentium.sh`.

## Facts

User and host from `GetUserNameA` and `GetComputerNameA`; the OS from the registry (`ProductName`, and `BuildLab` for the ReactOS release) and `GetSystemInfo` for the architecture; the kernel from `GetVersionExA`; the uptime from `GetTickCount` (it wraps at 49 days); memory from `GlobalMemoryStatus` (it saturates at 4 GiB); the display from `GetSystemMetrics` and `GetDeviceCaps`; the shell from `COMSPEC`. There is no CPU, host-model, desktop or terminal row. The depth hint is `Console16`, the width is the console window's, and a narrow window drops the logo in the core.

## Notes for the x86_64 and ARM64 builds

- The 24 KiB budget (AF-D47) holds the eight logos. The stack probe, the 64-bit division helpers (nothing calls them, but `compiler_builtins` names them, so the linker needs them) and the registry code are the obvious places to look if a feature needs more.
- `/MERGE:.data=.rdata` is for the Pentium build, which has no writable data. A mutable static would fault, so `tools/check-pe.sh` insists on the two sections `.text` and `.rdata`.
- `GlobalMemoryStatus` reports at most 4 GiB; the 64-bit builds should use `GlobalMemoryStatusEx` through `GetProcAddress`, and `GetTickCount64` the same way, with the `imports.allow` list kept to what Windows 95 exports.
- The environment is read once for the four names the program uses (`NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `COMSPEC`), because Windows 95 has no call that returns the whole block. A fact that needs another variable adds it to `VARIABLES` in `os.rs`.
- Escape sequences (VT) are not written anywhere yet. `Out` in `term.rs` is the place: ask `SetConsoleMode` for `ENABLE_VIRTUAL_TERMINAL_PROCESSING`, and use the core's `AnsiTerminal` when it accepts.
- The x86_64 and ARM64 PE files stay relocatable (ARM64 requires it), so `check-pe.sh`'s no-relocations test is the i586 one.
