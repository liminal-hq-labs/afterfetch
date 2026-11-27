# Provenance

The loader crates and the module build files in this folder are copied from [Desk](https://github.com/liminal-hq/desk) (`~/source/liminal-hq/desk`, read-only: nothing here writes into that checkout). House rules allow copies with provenance, so the files are kept close to the original and every change is listed here.

- **Source:** `liminal-hq/desk` at `4a021a7` ("Record the layered layout as merged in the Manual and the progress report (#100)").
- **Licence:** Apache-2.0 OR MIT, the same as this repository; every copied file keeps its header.

## What was copied

| Here | From Desk | Changes |
|---|---|---|
| `module/format/` (`desk-module`) | `module/format/` | `Cargo.toml`: `edition = "2021"` instead of inheriting the workspace's. Sources only: the tests are not copied |
| `module/pack/` (`dmdpack`) | `module/pack/` | The same `Cargo.toml` change. `src/elf.rs` accepts `R_386_PLT32` as `R_386_PC32` (a call to another object file in a static link, which a module that calls `memcmp` has), and its error text lists the type. Sources only |
| `module/guest/` (`desk-guest`) | `module/guest/` | The same `Cargo.toml` change. Sources only |
| `module/rustfmt.toml` | `module/rustfmt.toml` | The licence header added. Desk's own formatting (spaces, 130 columns) keeps the copied sources stable under `cargo fmt` |
| `module.ld` | `module/mods/module.ld` | None |
| `link.rs` | `module/mods/link.rs` | The linker script is found at `../vendor/module.ld` from the module crate |
| `i486-desk-none.json` | `kernel/i486-desk-none.json` | None |
| `desk.magic` | `tools/desk.magic` | None. `file -m` names a packed module |
| `native-check.py` | `tools/native-check.py` | None. Compares `dmdpack info` with the expected description |

The three crates are members of this workspace and not a workspace of their own (Desk's `module/Cargo.toml` is not copied): a path dependency outside the workspace puts its absolute path into the crate's identity, and the code's layout then depends on the checkout's directory.

Not copied: the test modules (`hello`, `appmain`), `tools/build-modules.sh` and `tools/check-modules.sh` (the checks are rewritten for this module in `check.sh`), and everything of Desk's kernel and shell.

## Refreshing

Compare the copied files with the same paths in Desk, copy the unchanged ones over, keep the changes above, run `bash platforms/desk/check.sh`, and record the new commit here. A change to the header table of `docs/LOADER.md` changes `desk.magic` and `tests/afetch-info.txt` in the same commit.
