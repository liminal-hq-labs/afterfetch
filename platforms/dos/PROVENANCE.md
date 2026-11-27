# Provenance

The Rust-on-DOS foundation in this folder is copied from the DOS runtime of [liminal-installer](https://github.com/liminal-hq-labs/liminal-installer) (`dos/`), which copied it from [Desk](https://github.com/liminal-hq/desk)'s `dos/` and its `tools/fetch-tools.sh`. House rules allow copies with provenance, so the files are kept close to the original and every change is listed here.

- **Source:** liminal-installer `dos/` at `37af845`, itself from Desk at `4a021a7`.
- **Licence:** Apache-2.0 OR MIT, the same as this repository; every copied file keeps its header.

## What was copied

| Here | From | Changes |
|---|---|---|
| `dos-elf.json` | `dos/dos-elf.json` | `cpu` lowered from `i486` to `i386`, soft float (`+soft-float`, `-x87`, `rustc-abi x86-softfloat`), as the Win32 shell's Pentium target does |
| `link.ld` | `dos/link.ld` | Code and read-only data are separate output sections in the first segment, so a scan of `.text` does not read data as instructions |
| `src/rt/start.rs` | `dos/rt/src/start.rs` | The register snapshot is dropped; the stack is 64 KiB; the entry calls `crate::run_program` |
| `src/rt/dpmi.rs` | `dos/rt/src/dpmi.rs` | Only what the program uses (`init`, base and limit, processor, free memory, DOS memory, real-mode interrupts); errors carry no code |
| `src/rt/dos.rs` | `dos/rt/src/dos.rs` | No `alloc`, no `File`; a 1 KiB transfer buffer; `write`, `create`, `close`, `real_interrupt` |
| `src/rt/env.rs` | `dos/rt/src/env.rs` | No `alloc`: the command tail and the environment block as slices |
| `src/rt/panic.rs` | `dos/rt/src/panic.rs` | Exits with the shell crash code (70) and prints nothing |
| `src/rt/bios.rs` | `setup/src/video.rs` (the `int10` idea) | New: a macro for the register-only interrupts the facts need |
| `tools/elf2go32.py` | `dos/tools/elf2go32.py` | None. Packs the linked ELF behind a go32 stub |
| `tools/dosrun.sh` | `dos/tools/dosrun.sh` | The default vendor folder is `~/.cache/afterfetch/dos-vendor` |
| `tools/fetch-tools.sh` | `dos/tools/fetch-tools.sh` | The same URLs and SHA-256 sums; the same default folder |

Not copied: the heap, the x87 and `io` modules, `cleanup`, and everything the installer's `setup` and `li-retro` do. This repository's own code is the rest of `src/` (the program, the facts, the terminal, the library the host tests), `build.sh`, `check.sh` and `tools/scan-386.sh`, `tools/dos-runner.sh`.

## What differs in use

DJGPP's stub is not needed: `AFETCH.EXE` is packed behind CWSDPMI's own `CWSDSTUB.EXE`, the stub with the DPMI host inside it, so the program runs from a bare floppy on a machine with no DPMI host.
