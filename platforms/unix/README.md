# Afterfetch for Unix

A static program with no C library, built from raw system calls. Linux x86_64 and i586 build today; the layout leaves room for the other architectures, FreeBSD and macOS.

## Build and check

```sh
platforms/unix/build.sh x86_64-unknown-linux-musl    # or i586-unknown-linux-musl; add `clippy` to lint instead
platforms/unix/check.sh x86_64-unknown-linux-musl    # machine, static, no PT_INTERP, no DT_NEEDED, 24 KiB budget, system call list
bash scripts/smoke/linux-native.sh                   # shape, --json and the colour depths from a pseudo-terminal
bash scripts/repro-unix.sh                           # two builds in two directories, compared byte for byte
```

`make check` runs all of them. `build.sh` holds every compiler and linker flag in one place, because `RUSTFLAGS` set per call is the only way to remap the source path so builds in different directories match. The crate is empty on any target that is not Linux with musl (`build.rs` sets the `raw_linux` cfg), so `cargo check --workspace` works on the host.

## Layout

- `src/rt.rs`: the Rust side of the entry stub, the arguments, the panic handler (exit code 70) and the memory routines the compiler expects from a C library.
- `src/arch/<arch>.rs`: the `_start` stub, the system call instruction, the system call numbers and the terminal `ioctl` requests.
- `src/os/linux.rs`: the `Sys` enum (every system call, kept equal to `syscalls.allow` by `check.sh`) and the `Platform` that reads `/etc/os-release`, `/proc`, `/sys` and the environment. The parsing is in `afterfetch-core::parse`.
- `src/term.rs`: the buffered output and the `Terminal` over `AnsiTerminal`.
- `link.ld`: the layout and the sections dropped.

## Adding an architecture (notes for ARM, MIPS and the rest)

- Add `src/arch/<arch>.rs` with `_start` (pass the initial stack pointer to `rt::start` as the one argument, aligned for the call), `syscall3`, the `nr` numbers and the `TCGETS` and `TIOCGWINSZ` values, then list it in `arch/mod.rs`. MIPS and PowerPC have different `ioctl` request numbers, and MIPS o32 passes errors in `$a3` with a positive result.
- `rt.rs` and `os/linux.rs` are portable. The structures (`Utsname`, `Sysinfo`) use the kernel's layout with `usize` for `long`; check the 32-bit and big-endian cases.
- Add the target to `build.sh`'s callers and `check.sh`'s table, and extend `scripts/smoke/` with a runner such as `qemu-aarch64-static`; `shape.py plain <binary> <labels> --runner <emulator>` already takes one.
- Only x86 has been built so far. `/proc/cpuinfo` is parsed for the ARM, MIPS and RISC-V shapes from captured files in `crates/afterfetch-core/tests/fixtures/linux/cpuinfo`.

## Notes for FreeBSD

- FreeBSD reports errors in the carry flag, not as a negative result, so `syscall3` there must return a `Result`-like pair; `os/freebsd.rs` should own the numbers and `sysctl` calls.
- A `Platform` for FreeBSD can reuse `term.rs`, `rt.rs` and the core's `parse::basename`; it does not have `/proc` by default.
