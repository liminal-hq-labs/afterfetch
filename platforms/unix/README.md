# Afterfetch for Unix

A static program with no C library, built from raw system calls. Linux x86_64, i586, ARM64 (`aarch64`), 32-bit Arm (`armv7`, `armv6`) and MIPS (`mips`, `mipsel`) build today; the layout leaves room for the other architectures, FreeBSD and macOS.

## Build and check

```sh
platforms/unix/build.sh x86_64-unknown-linux-musl    # or i586-, aarch64-unknown-linux-musl, armv7- or arm-unknown-linux-musleabihf; add `clippy` to lint instead
platforms/unix/check.sh x86_64-unknown-linux-musl    # machine, static, no PT_INTERP, no DT_NEEDED, 32 KiB budget, system call list
bash scripts/smoke/linux-native.sh                   # shape, --json and the colour depths from a pseudo-terminal; the Arm and MIPS builds run under qemu-user
bash scripts/repro-unix.sh                           # two builds in two directories, compared byte for byte
```

`make check` runs all of them. `build.sh` holds every compiler and linker flag in one place, because `RUSTFLAGS` set per call is the only way to remap the source path so builds in different directories match. The crate is empty on any target that is not Linux with musl (`build.rs` sets the `raw_linux` cfg), so `cargo check --workspace` works on the host.

## Layout

- `src/rt.rs`: the Rust side of the entry stub, the arguments, the panic handler (exit code 70) and the memory routines the compiler expects from a C library.
- `src/arch/<arch>.rs`: the `_start` stub, the system call instruction, the system call numbers and the terminal `ioctl` requests.
- `src/os/linux.rs`: the `Sys` enum (every system call, kept equal to `syscalls.allow` by `check.sh`) and the `Platform` that reads `/etc/os-release`, `/proc`, `/sys` and the environment. The parsing is in `afterfetch-core::parse`.
- `src/term.rs`: the buffered output and the `Terminal` over `AnsiTerminal`. `FdSink::finish` flushes and says whether every write worked, so the core's `run` exits 74 on a failed write (AF-D33); the shell has no exit-code logic of its own.
- `link.ld`: the layout and the sections dropped.

## Adding an architecture (notes for the rest)

- Add `src/arch/<arch>.rs` with `_start` (pass the initial stack pointer to `rt::start` as the one argument, aligned for the call), `syscall3`, the `nr` numbers and the `TCGETS` and `TIOCGWINSZ` values, then list it in `arch/mod.rs`. MIPS and PowerPC have different `ioctl` request numbers, and both report errors in a register flag, not a negative result (MIPS: `$a3`; PowerPC: CR0.SO).
- `rt.rs` and `os/linux.rs` are portable. The structures (`Utsname`, `Sysinfo`) use the kernel's layout with `usize` for `long`; check the 32-bit and big-endian cases.
- Add the target to `build.sh`'s callers and `check.sh`'s table, and extend `scripts/smoke/` with a runner such as `qemu-aarch64-static`; `shape.py plain <binary> <labels> --runner <emulator>` already takes one.
- Name the target: `TARGET_ARCH` in `os/linux.rs` gives the SPEC section 2 name (`aarch64`, `armv7`, `mipsel`, `ppc64le`); `--version` and the JSON `target` print it. Fill `facts.arch` with the kernel's own machine name, unmapped, and set the raw numbers with `facts.set_uptime` and `facts.set_memory` so the JSON and the text agree.
- x86, ARM64, 32-bit Arm and MIPS have been built. `/proc/cpuinfo` is parsed for the ARM, MIPS and RISC-V shapes from captured files in `crates/afterfetch-core/tests/fixtures/linux/cpuinfo`.

## Arm

- **ARM64** has the generic system call table (no `open`, only `openat`; `ioctl` is 29, `exit_group` 94). The stub is `svc #0` with the number in x8 and arguments in x0 to x2.
- **32-bit Arm** uses the EABI table (`openat` 322, `exit_group` 248, `getuid32` 199) and `svc 0`. The two builds are Thumb code (AF-D51): Arm-mode code was over the 32 KiB budget. Thumb reserves r7, so `syscall3` saves it in r12 and loads the number through a scratch register. `_start` is a `.arm` stub that reaches the Thumb entry with `blx`. `armv7` and `armv6` differ only in the triple (`build.rs` sets `arm_v7`), for `TARGET_ARCH`.
- **Linking:** the prebuilt `compiler_builtins` for Arm carries unwind tables that name `rust_eh_personality`; `link.ld` drops `.ARM.exidx` and `.ARM.extab`, so nothing is undefined. rustc warns that `thumb-mode` is unstable; it is only a warning.
- **What the smoke proves:** `scripts/smoke/linux-native.sh aarch64 armv7 armv6` runs each binary under `qemu-aarch64-static` or `qemu-arm-static`, named on the command line so it works without binfmt, with `QEMU_CPU` set to `cortex-a72`, `cortex-a7` and `arm1176`. It checks plain output, `--json`, exit 74 on `/dev/full` and the four colour depths through `script`. That proves the instruction sequence (an instruction the real core lacks faults under the matching `QEMU_CPU`) and the system call interface. It does **not** prove the parsers on real hardware: qemu-user answers `uname`, `/proc/cpuinfo`, `/proc/meminfo` and `/etc/os-release` from the host kernel, so the rows show the host (the arch as `aarch64` or `armv7l`, an ARMv8 CPU line). The Pi 4 and Pi Zero shapes are covered by the core's captured-file tests only.

## MIPS

`mips` (big-endian) and `mipsel` (little-endian) are o32 builds (AF-D22). Everything below was checked by building and running the two binaries under qemu-user, and the numbers against the `libc` crate's MIPS tables in the Cargo registry (`SYS_* = 4000 + n`); there was no kernel source to hand.

- **System call numbers** are the o32 table offset by 4000: `read` 4003, `write` 4004, `close` 4006, `getuid` 4024, `ioctl` 4054, `sysinfo` 4116, `uname` 4122, `exit_group` 4246, `openat` 4288. `getuid` is the 32-bit-id call here, as in the generic 32-bit tables.
- **Calling convention:** number in `$v0` (`$2`), arguments in `$a0` to `$a2` (`$4` to `$6`), `syscall`. `$a3` (`$7`) comes back non-zero on failure with a **positive** errno in `$v0`, so `syscall3` returns `-v0` then. The kernel keeps every register except `$v0` and `$a3`, but the stub also declares `$v1`, the temporaries and `$t8` and `$t9` as clobbered, as musl does, because a kernel may change them.
- **`ioctl` requests:** `TCGETS` is `0x540D` (x86: `0x5401`) and `TIOCGWINSZ` is `0x40087468` (x86: `0x5413`). Both work: the colour smoke only passes if `TCGETS` recognises the pseudo-terminal. `struct termios` is under 64 bytes on MIPS, so the existing buffer holds it, and `winsize` is four `u16` in the same order as everywhere.
- **Endianness:** nothing needed changing. `Utsname` is bytes; `Sysinfo` is native-endian `long` fields in the same order on both, and the uptime and memory it fills match the host's on `mips` and `mipsel`. Under qemu-user both report `uname` machine `mips` (a real `mipsel` kernel does too), so the OS row says `(mips)` for both and the target name (`linux-mips` or `linux-mipsel`) is the only thing that tells them apart; `check.sh` reads the ELF's byte order.
- **Entry:** `_start` is a `global_asm!` stub in `noreorder` with an explicit `nop` in the `jalr` delay slot, a call through `$t9` (`$25`), the stack aligned to 8 and 16 bytes reserved for the callee's argument area. It builds a non-PIC absolute address with `lui` and `addiu` (`%hi`, `%lo`), which is fine at the fixed base `0x400000`.
- **Build:** tier 3, so `build.sh` sets `RUSTC_BOOTSTRAP=1` and passes `-Zbuild-std=core,compiler_builtins` for the two targets only; `rust-src` is already in `rust-toolchain.toml`, and the targets are not (there is no prebuilt library). `asm_experimental_arch` is enabled in `main.rs` for MIPS alone. `compiler-builtins-mem` is **not** used: `rt.rs` already defines the memory routines. The default CPU is `mips32r2` with soft float.
- **Linking:** `link.ld` also drops `.pdr`, `.mdebug*` and `.gnu.attributes`, which LLD keeps otherwise (the `.pdr` alone was 25 KB of file). `.reginfo` and `.MIPS.abiflags` stay: they are 48 bytes and the kernel and tools read them.
- **Size:** 42,744 bytes each, 32.4 KB of code. That is over the 32 KiB of the other Linux builds, so `check.sh` allows MIPS 44 KiB (AF-D22). MIPS16 does not build (`compiler_builtins` has assembly it cannot encode), microMIPS is absent from the 24K and 74K cores routers use, and the usual flags (`opt-level=s`, no MIR inlining, function alignment) change nothing.
- **What the smoke proves:** `scripts/smoke/linux-native.sh mips mipsel` runs each binary under `qemu-mips-static` or `qemu-mipsel-static` with `QEMU_CPU=24Kc` (MIPS32 release 2): plain output, `--json`, exit 74 on `/dev/full` and the four colour depths through `script`. A MIPS32 release 1 core (a 4Kc) would fault on the baseline's instructions; the target is `mips32r2`. It does **not** prove the parsers on a router: qemu-user answers `/proc/cpuinfo`, `/proc/meminfo` and `/etc/os-release` from the host, so the rows show the host. The OpenWrt shape is covered by the core's captured file.
- **Not done:** `mips64` (n64) and `mips64el`; the numbers are offset by 5000 there and `mips64el-unknown-linux-muslabi64` needs its own stub.

## Notes for FreeBSD

- FreeBSD reports errors in the carry flag, not as a negative result, so `syscall3` there must return a `Result`-like pair; `os/freebsd.rs` should own the numbers and `sysctl` calls.
- A `Platform` for FreeBSD can reuse `term.rs`, `rt.rs` and the core's `parse::basename`; it does not have `/proc` by default.
