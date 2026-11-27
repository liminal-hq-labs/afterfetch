# Afterfetch for Windows 3.1

`AFETCHW.EXE`: a window program for Windows 3.1 and later, `no_std` Rust for 16-bit code segments packed into an NE executable. The core draws into a grid of character cells; the window paints each run of one colour with one `TextOut`. `/DUMP=<file>` writes the plain text instead and opens no window.

```sh
platforms/win16/build.sh     # dist/AFETCHW.EXE (build.sh clippy lints instead)
platforms/win16/check.sh     # NE executable, 40 KiB budget, segments, nothing newer than a 386
bash scripts/smoke/wine16.sh # shape under Wine, and a drawn window on a virtual display
bash scripts/smoke/w31.sh    # Windows for Workgroups 3.11 in DOSBox-X (Docker); not part of make check
```

The DOS build, `platforms/dos`, also runs in a Windows 3.1 DOS box. See [docs/platforms.md](../../docs/platforms.md#windows-31) for the facts and the tests, [PROVENANCE.md](PROVENANCE.md) for what was copied, and AF-D58.
