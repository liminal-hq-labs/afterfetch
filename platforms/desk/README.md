# Afterfetch for Desk

`AFETCH.DMD`: a module for Desk's loader, `no_std` Rust packed by Desk's own packer. `app_start` prints the `desk` picture and the one fact a module can honestly report through `desk_log`.

```sh
platforms/desk/build.sh      # dist/AFETCH.DMD (build.sh clippy lints; --hosted packs for the hosted track)
platforms/desk/check.sh      # host tests, dmdpack info, file magic, relocation at three more bases, instruction and path scans, 24 KiB
bash scripts/repro-desk.sh   # byte-identical in three directories
```

**It has never run.** Desk has no loader yet (AF-D28, AF-D59): the checks are Desk's own checks of a module, and the module's logic runs on the host against a golden. See [docs/platforms.md](../../docs/platforms.md#desk), [vendor/PROVENANCE.md](vendor/PROVENANCE.md), and [docs/desk-sys-info.md](../../docs/desk-sys-info.md) for the call that would give the module more to say.
