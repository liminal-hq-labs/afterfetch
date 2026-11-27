# Provenance

The Win32 shell is the proven `afterfetch.exe` of reactos-liminal, ported onto the portable core.

## Source

- **Repository:** `liminal-hq-labs/reactos-liminal`, `tools/afterfetch/`.
- **Commit:** `556e35f9754e77a0189a239d366e9976ef0cad34` (`feat(afterfetch): build a coloured afterfetch.exe in Rust at image time`), the last commit to touch that folder, read at `origin/main` `fcb83d9b64f59d40fe8fa80849aadf8f0b922e3b`. The tree is identical at `d6c72ae`, the commit the plan names.
- **ReactOS code:** none. The program calls Win32 entry points by name and copies no ReactOS source.

## What came from where

| Here | There | Change |
| --- | --- | --- |
| `i586-win32.json` | `win32.json` | Renamed for the arch-named target directory. `"+soft-float"` added to `features`, which silences rustc's warning that the soft-float ABI needs the feature named; the executable is byte-identical with and without it |
| `.cargo/config.toml` | `.cargo/config.toml` | The target section is `i586-win32`. `/MERGE:.data=.rdata` added: the program has no writable data, and the empty `.data` section made the linker reserve a second page of headers (512 bytes) |
| `imports.allow` | `imports.allow` | Two entries added, `kernel32.dll!GetCommandLineA` (the arguments) and `kernel32.dll!GetFileType` (is standard output a console), for 22 in all |
| `src/sys.rs` | `src/sys.rs` | `GetFileType`, `GetCommandLineA` and their constants (`STD_ERROR_HANDLE`, `FILE_TYPE_CHAR`, the platform and architecture numbers) added |
| `src/rt.rs` | `src/rt.rs` | The panic handler exits with the core's `EXIT_CRASH`. `__chkstk` added, after the installer's Win32 runtime (`liminal-installer/win32/setup/src/rt.rs`), because the larger frame of the new program crosses a page |
| `src/os.rs` | `src/main.rs`, `src/sys.rs` (`gather`) | The fact gathering, as a `Platform` for the core: the same calls, with the formatting now in `afterfetch-core::format`, the arguments from `GetCommandLineA`, and the environment read once |
| `src/term.rs` | `src/main.rs` (`Output`) | The console output, as a `Terminal` for the core: a style becomes an attribute (`src/attribute.rs`), the line ends by restoring the console's own attribute, and the output is buffered |
| `build.sh`, `check.sh`, `tools/*.sh` | `build.sh`, `check.sh` | The same flags and checks, split into a build script, a structure script, an import script and a Pentium scan, with the size budget and the repository's reproducibility script |
| (core) | `src/lib.rs`, `logo.rs`, `palette.rs`, `render.rs`, `text.rs` | Moved into `afterfetch-core` by AF-WP02, with their tests |

## Differences in behaviour

- **Arguments.** The original took none. The core's options (`--plain`, `--logo`, `--colour`, `--json`, `/PLAIN` and the rest) are read from the command line with the C runtime's splitting rules.
- **Plain output.** A pipe or a file gets the plain ASCII picture, as before, and now also any depth the user asks for: the Win32 program writes no escape sequences, because the writer costs 1.3 KB of the 16 KiB budget. The VT path (AF-WP13) decides what a console that understands escapes gets.
- **OS row.** A system that is not a known ReactOS release gets its architecture in brackets (`Windows 10 Pro (x86)`), as the specification asks; a ReactOS release keeps `0.4.16-x86`.
- **Console test.** `GetFileType` must say a character device before `GetConsoleScreenBufferInfo` is asked, so a `NUL` output is not taken for a console.
- **Exit code.** 74 when standard output cannot be written (AF-D33); 70 on a crash, 2 for a usage error.
- **Build output.** `target/i586-win32/release/afterfetch.exe`, copied to `platforms/win32/dist/`.

## Refreshing

The ReactOS image keeps its own copy until the handover package replaces `tools/afterfetch/` with a pinned build of this folder. That handover must keep the image's output byte-identical: `--logo win32 --plain` against `crates/afterfetch-core/tests/golden/win32-plain-80.txt` is the test.
