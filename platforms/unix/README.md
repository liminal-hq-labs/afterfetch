# Afterfetch for Unix

A static program with no C library, built from raw system calls. Linux x86_64, i586, ARM64 (`aarch64`) and 32-bit Arm (`armv7`, `armv6`) build today; the layout leaves room for the other architectures, FreeBSD and macOS.

## Build and check

```sh
platforms/unix/build.sh x86_64-unknown-linux-musl    # or i586-, aarch64-unknown-linux-musl, armv7- or arm-unknown-linux-musleabihf; add `clippy` to lint instead
platforms/unix/check.sh x86_64-unknown-linux-musl    # machine, static, no PT_INTERP, no DT_NEEDED, 32 KiB budget, system call list
bash scripts/smoke/linux-native.sh                   # shape, --json and the colour depths from a pseudo-terminal; the Arm builds run under qemu-user
bash scripts/repro-unix.sh                           # two builds in two directories, compared byte for byte
```

`make check` runs all of them. `build.sh` holds every compiler and linker flag in one place, because `RUSTFLAGS` set per call is the only way to remap the source path so builds in different directories match. The crate is empty on any target that is not Linux with musl (`build.rs` sets the `raw_linux` cfg), so `cargo check --workspace` works on the host.

## Layout

- `src/rt.rs`: the Rust side of the entry stub, the arguments, the panic handler (exit code 70) and the memory routines the compiler expects from a C library.
- `src/arch/<arch>.rs`: the `_start` stub, the system call instruction, the system call numbers and the terminal `ioctl` requests.
- `src/os/linux.rs`: the `Sys` enum (every system call, kept equal to `syscalls.allow` by `check.sh`) and the `Platform` that reads `/etc/os-release`, `/proc`, `/sys` and the environment. The parsing is in `afterfetch-core::parse`.
- `src/term.rs`: the buffered output and the `Terminal` over `AnsiTerminal`. `FdSink::finish` flushes and says whether every write worked, so the core's `run` exits 74 on a failed write (AF-D33); the shell has no exit-code logic of its own.
- `link.ld`: the layout and the sections dropped.

## Adding an architecture (notes for ARM, MIPS and the rest)

- Add `src/arch/<arch>.rs` with `_start` (pass the initial stack pointer to `rt::start` as the one argument, aligned for the call), `syscall3`, the `nr` numbers and the `TCGETS` and `TIOCGWINSZ` values, then list it in `arch/mod.rs`. MIPS and PowerPC have different `ioctl` request numbers, and MIPS o32 passes errors in `$a3` with a positive result.
- `rt.rs` and `os/linux.rs` are portable. The structures (`Utsname`, `Sysinfo`) use the kernel's layout with `usize` for `long`; check the 32-bit and big-endian cases.
- Add the target to `build.sh`'s callers and `check.sh`'s table, and extend `scripts/smoke/` with a runner such as `qemu-aarch64-static`; `shape.py plain <binary> <labels> --runner <emulator>` already takes one.
- Name the target: `TARGET_ARCH` in `os/linux.rs` gives the SPEC section 2 name (`aarch64`, `armv7`, `mipsel`, `ppc64le`); `--version` and the JSON `target` print it. Fill `facts.arch` with the kernel's own machine name, unmapped, and set the raw numbers with `facts.set_uptime` and `facts.set_memory` so the JSON and the text agree.
- x86, ARM64 and 32-bit Arm have been built. `/proc/cpuinfo` is parsed for the ARM, MIPS and RISC-V shapes from captured files in `crates/afterfetch-core/tests/fixtures/linux/cpuinfo`.

## Arm

- **ARM64** has the generic system call table (no `open`, only `openat`; `ioctl` is 29, `exit_group` 94). The stub is `svc #0` with the number in x8 and arguments in x0 to x2.
- **32-bit Arm** uses the EABI table (`openat` 322, `exit_group` 248, `getuid32` 199) and `svc 0`. The two builds are Thumb code (AF-D51): Arm-mode code was over the 32 KiB budget. Thumb reserves r7, so `syscall3` saves it in r12 and loads the number through a scratch register. `_start` is a `.arm` stub that reaches the Thumb entry with `blx`. `armv7` and `armv6` differ only in the triple (`build.rs` sets `arm_v7`), for `TARGET_ARCH`.
- **Linking:** the prebuilt `compiler_builtins` for Arm carries unwind tables that name `rust_eh_personality`; `link.ld` drops `.ARM.exidx` and `.ARM.extab`, so nothing is undefined. rustc warns that `thumb-mode` is unstable; it is only a warning.
- **What the smoke proves:** `scripts/smoke/linux-native.sh aarch64 armv7 armv6` runs each binary under `qemu-aarch64-static` or `qemu-arm-static`, named on the command line so it works without binfmt, with `QEMU_CPU` set to `cortex-a72`, `cortex-a7` and `arm1176`. It checks plain output, `--json`, exit 74 on `/dev/full` and the four colour depths through `script`. That proves the instruction sequence (an instruction the real core lacks faults under the matching `QEMU_CPU`) and the system call interface. It does **not** prove the parsers on real hardware: qemu-user answers `uname`, `/proc/cpuinfo`, `/proc/meminfo` and `/etc/os-release` from the host kernel, so the rows show the host (the arch as `aarch64` or `armv7l`, an ARMv8 CPU line). The Pi 4 and Pi Zero shapes are covered by the core's captured-file tests only.

## Notes for MIPS (WP09)

- **o32 system call numbers** are offset by 4000 (`write` 4004, `read` 4003, `ioctl` 4054, `uname` 4122, `sysinfo` 4116, `openat` 4288, `exit_group` 4246, `getuid` 4024); they match the x86 numbers' order loosely, so take them from `arch/mips/include/uapi/asm/unistd.h`, not from memory.
- **Errors** come back with `$a3` non-zero and a positive errno in `$v0`: `syscall3` must read `$a3` and return `-v0` then. The call number goes in `$v0`, arguments in `$a0` to `$a2`, and the instruction is `syscall`; `$a3` is also clobbered, so declare it an output.
- **`ioctl` requests** differ: `TCGETS` is `0x540D` and `TIOCGWINSZ` is `0x40087468` on MIPS. Check `Termios` and `Winsize` sizes too (`NCCS` is 23 on MIPS).
- **Big endian** applies to `mips`, not `mipsel`: `Utsname` is bytes and fine; `Sysinfo` fields are `usize` and are read natively, so check the layout under `qemu-mips-static`.
- **Entry:** the stack holds `argc` at `$sp`; align it to 8 (o32), reserve the 16-byte argument area for the call and use `jal` with a delay slot (`.set noreorder`, or let the assembler fill it). The Thumb trick does not apply; budget is still 32 KiB.
- **Build:** tier 3, so `-Zbuild-std=core,compiler_builtins` with `RUSTC_BOOTSTRAP=1`, `asm_experimental_arch`, and `compiler-builtins-mem` for the memory routines. Add the targets to the toolchain file only if they are prebuilt (they are not). `build.sh` needs a branch for them.
- **Local test:** `qemu-mips-static` and `qemu-mipsel-static`; set `QEMU_CPU` only if a model matters (the default `24Kc` suits). The OpenWrt cpuinfo fixture already covers the `cpu model` shape.

## Notes for FreeBSD

- FreeBSD reports errors in the carry flag, not as a negative result, so `syscall3` there must return a `Result`-like pair; `os/freebsd.rs` should own the numbers and `sysctl` calls.
- A `Platform` for FreeBSD can reuse `term.rs`, `rt.rs` and the core's `parse::basename`; it does not have `/proc` by default.
