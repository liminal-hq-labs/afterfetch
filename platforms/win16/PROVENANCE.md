# Provenance of platforms/win16's toolchain

The packer, the import generator, the linker script and the runtime in `src/lib.rs` are copies from Quill's Win16 build, `win16/` at `659a038` on Quill's `origin/main` (`~/source/experiments/quill16`, read with `git show origin/main:...`), taken here through afterglow-savers' `platforms/win16` (at `4289c59`), which copied them with its own `PROVENANCE.md`. Quill's copy descends from [`liminal-hq-labs/rust-win16`](https://github.com/liminal-hq-labs/rust-win16), branch `feat/multi-segment`, commit `4c0bac4`. They are copied rather than fetched, so a build needs nothing from the network.

## What is copied, and what changed here

| File | From | Changed here |
|---|---|---|
| `win16.json` | afterglow-savers' `platforms/win16/win16.json` | `+soft-float` added to `features`, which the compiler now asks for with `rustc-abi: softfloat` |
| `link.ld` | the same | No |
| `src/lib.rs`, `src/probe.rs`, `src/wizard.rs`, `src/fault.rs` | the same | No. `src/rustfmt.toml` (hard tabs, otherwise the defaults) keeps their formatting |
| `tools/ne_pack.py`, `tools/check_segments.py`, `tools/check_cpu.py`, `tools/check_resources.py`, `tools/gen_imports.py` | the same | No |
| `tools/ne_check.py`, `tools/ne_dump.py`, `tools/ne_addr2sym.py`, `tools/rust_demangle.py` | the same | No |
| `tests/test_ne_pack.py`, `tests/test_segments.py` | the same | No |
| `tools/imports.tbl`, `src/imports.rs` | the same, regenerated | One row added: `GetFreeSystemResources` (USER 284), whose ordinal was read from Windows 3.11's own `USER.EXE` with `tools/ne_dump.py` |
| `.cargo/config.toml` | the same | Not copied: its build-std and linker flags are in `build.sh`, so the host tests use the ordinary standard library |

This repository's own code is `src/bin/afetchw/`, `rows/`, `build.sh`, `check.sh`, `res/afetchw.segments.json`, `tools/wine-runner.sh` and `tools/run-w31.sh`.

## Refreshing

Compare the files above with the same paths in Quill's `origin/main`, copy the unchanged ones over, keep the one row in `tools/imports.tbl`, regenerate `src/imports.rs` with `bash build.sh --regen`, run `bash check.sh`, and record the new commit here.
