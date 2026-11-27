# Preview

`afterfetch-preview` draws what afterfetch looks like on any supported system, on the development machine, from sample facts. It is the real core: the tool calls the core's `run` with a fake `Platform` that answers from a sample and a terminal that records every cell, so the picture is the program's output and cannot drift from it. It is a host-only crate, never built for a target, never shipped and outside every size budget (AF-D50, which realizes AF-D30).

## Use

```sh
cargo run -p afterfetch-preview -- --system macos --colour 256
cargo run -p afterfetch-preview -- --system linux --arch mips --colour mono --width 72
cargo run -p afterfetch-preview -- --all --arch all                    # every system and architecture, as text
cargo run -p afterfetch-preview -- --system dos --format png --out dos.png
cargo run -p afterfetch-preview -- --all --format html --out all.html   # a page for README screenshots
make preview SYSTEM=freebsd COLOUR=16 WIDTH=100
make sheet COLOUR=truecolour WIDTH=100 SHEET=~/somewhere/contact-sheet.png
```

| Option | Meaning |
|---|---|
| `--system` | `linux` (default), `freebsd`, `macos`, `win32` (Windows 10 and 11), `win95`, `reactos`, `dos`, `win16`, `desk`, `desk-hosted`, `other` (a system with no logo of its own). `--list` shows them with their architectures |
| `--arch` | The system's architecture, such as `aarch64`, `i586`, `mips`, `riscv64`; `all` with `--all` renders each |
| `--colour` | `auto` (the system's own detection), `never`, `mono`, `16`, `ansi16`, `console16`, `256`, `truecolour`. `16` is console attributes on Win32, DOS and Windows 3.1 and ANSI elsewhere, as on the real program |
| `--width` | Terminal columns, 20 to 400, default 80; the logo shows from 72 columns, or 76 for `win32` |
| `--logo`, `--no-logo`, `--cells` | A different logo, the facts alone, or the logo as cells (a terminal without UTF-8) |
| `--format` | `text` (no escapes, no trailing spaces), `ansi` (the exact bytes a terminal receives), `html`, `png`, `json` (the real `--json` output) |
| `--sheet <file.png>` | The contact sheet: each of the eight logos on its own system, with a caption saying whether a real binary exists |

Console attributes have no escape form, so `--format ansi` writes them as 16-colour ANSI, and pictures draw them in the VGA palette on a black console. Other depths draw on a dark terminal with xterm's sixteen colours.

## What the samples are

Sample facts are realistic, not real, and one table in `crates/afterfetch-preview/src/systems.rs` holds them. Each system shows only the rows SPEC §4 lets its platform read: DOS has no uptime, Win32 no host, desktop or terminal rows, Windows 3.1 adds Resources, and Desk reports the OS row alone until Desk offers `desk_sys_info` (AF-D26). Values are formatted by the core's own `format_*` functions, and the core cuts any value that does not fit beside the logo, so a long OS row loses its tail at 80 columns exactly as it would on the real program. The `reactos` sample holds the facts the core's goldens use; the preview's tests reproduce those goldens byte for byte for every logo, depth and width.

## Which targets are real

A **real build** exists today for Linux x86_64 and i586 and for Windows (Win32) i586. Every other target is **preview only**: its picture is the core's output on sample facts, and nothing has run on that system. The tile captions say which.
