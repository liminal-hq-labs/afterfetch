# Afterfetch for DOS

`AFETCH.EXE`: a 32-bit protected-mode program for a 386 and later, packed behind CWSDPMI's stub so it carries its own DPMI host. No C library and no heap. Colour is text-mode attributes written through the BIOS; the facts come from INT 21h, the BIOS and DPMI.

```sh
platforms/dos/tools/fetch-tools.sh   # DOSBox Staging and CWSDPMI into ~/.cache/afterfetch/dos-vendor, once
platforms/dos/build.sh               # dist/AFETCH.EXE (build.sh clippy lints instead)
platforms/dos/check.sh               # MZ and go32 stub, 80386 ELF, no 486 or x87 instruction, 64 KiB budget
bash scripts/smoke/dos.sh            # plain, /JSON, the screen read back, exit 74, all under headless DOSBox
```

`make check` runs all of them. See [docs/platforms.md](../../docs/platforms.md#dos) for the facts and how each is read, [PROVENANCE.md](PROVENANCE.md) for what was copied, and AF-D57.
