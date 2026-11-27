# Afterfetch specification

This is a skeleton. The full product definition (command line, facts, output rules, JSON keys, exit codes, logo names) is written in the SPEC work package.

## Purpose

A tiny system-information command with an Afterglow sun for every system, from DOS to macOS.

## Users and scenarios

- Anyone who wants a quick, honest summary of the system in front of them, on any of the supported platforms.

## Non-goals

- Reporting a fact a system cannot honestly read: that row is omitted.
- Per-architecture artwork: the architecture is text plus an accent colour.

## Components

`afterfetch-core` (portable, `no_std`), `afterfetch-art` (generated logos and palette) and a thin shell per platform family.

## Behaviour

### Command line

Options are GNU-style long options, parsed in the core from raw argument bytes without allocating. A later argument wins over an earlier one. `--logo=win32` and `--logo win32` are the same. The slash forms DOS and Windows users type are accepted everywhere, in any case (`/PLAIN`, `/NOLOGO`, `/LOGO:name`, `/COLOUR:mode`, `/MONO`, `/JSON`, `/VERSION`, `/?`).

| Option | Meaning |
|---|---|
| `--plain` | No colour and the ASCII form of the logo: the output a pipe gets |
| `--no-logo` | The facts alone |
| `--logo <name>` | `auto` (default: the logo for this system), `list` (print the names), or one of `afterglow`, `dos`, `win16`, `win32`, `linux`, `freebsd`, `macos`, `desk` |
| `--colour <mode>`, `--color <mode>` | `auto` (default), `never`, `always`, `mono`, `16`, `256`, `truecolour` (also `truecolor`, `24bit`) |
| `--json` | The facts as one JSON object on one line: no logo, no colour |
| `-h`, `--help` | Usage, then exit 0 |
| `-V`, `--version` | `afterfetch 0.1.0 (<os>-<arch>)`, then exit 0 |

Until the art generator lands, every logo name draws the `win32` picture.

Exit codes: 0 done, 2 usage error (an unknown option, a missing value or a value that is not a logo or colour mode; the message names the argument's position and goes to the error stream), 70 crash (set by a shell's panic handler).

### Facts

The output is a title (`user@host`) and a rule of dashes under it, then one labelled row for each fact the platform could report, in this order, then (with colour) a gap and two rows of eight swatches of the console colours. A fact a system cannot honestly report is **left out**: its row is not printed, rows below close up, and a title with no user and no host is left out with its rule (AF-D20). With only one of user and host, the title is that one.

| Row | Label | JSON key | Tone |
|---|---|---|---|
| title | `user@host` | `user`, `hostname` | gold, slate, lilac |
| OS | `OS:` | `os` | gold |
| Kernel | `Kernel:` | `kernel` | orange |
| Host (the machine's model) | `Host:` | `model` | coral |
| Uptime | `Uptime:` | `uptime` | coral |
| CPU | `CPU:` | `cpu` | rose |
| Memory | `Memory:` | `memory` | rose |
| Display | `Display:` | `display` | lilac |
| Terminal | `Terminal:` | `terminal` | lilac |
| Shell | `Shell:` | `shell` | violet |
| Resources (Windows 3.1) | `Resources:` | `resources` | periwinkle |

Values are display strings: uptime reads `2d 3h 59m`, memory `387 MB free of 511 MB` (`12.4 GB free of 15.6 GB` from 4 GiB of total memory), display `1024 x 768, 32-bit`, terminal `xterm-256color, 120 x 30`, resources `72% free`. The values are never wrapped: a value too long for the line is cut at a character boundary.

### Layout

The logo is drawn on the left and the facts on the right, starting on the logo's second row, with the information column fixed by the logo (column 42 for `win32`). The logo is left out, and the facts stand alone, when the terminal is narrower than the information column plus 34. A width of 0 (unknown, as on a pipe) is taken as 80. When there are more fact lines than logo rows, they continue below the logo in the same column. Plain output has no trailing blank lines and no trailing spaces.

### Colour depth

The depth is decided once at start, in this order:

1. `--plain` or `--colour never`: none, with no escape sequences and the ASCII logo (the sun is `#`).
2. `--colour mono`, `16`, `256` or `truecolour`: that depth, on a pipe too. `16` means console attributes where the platform is a Windows console or DOS.
3. `--colour always`: the depth the terminal is detected to support, even on a pipe, and 16 colours when none is detected.
4. `NO_COLOR` set and not empty: none.
5. Output that is not a terminal: none, unless `CLICOLOR_FORCE` or `FORCE_COLOR` is set and not empty.
6. Otherwise the detected depth. On Unix, `COLORTERM` of `truecolor` or `24bit` gives true colour, `TERM=dumb` none, a `TERM` ending `-256color` 256 colours, and anything else 16.

With colour, the sun is drawn as solid cells (a space on a coloured background) and the swatches are added. In mono, tones are drawn by weight (the sun and horizon bold, the deep water dim) and there are no swatches. Every ANSI line ends with a reset (`ESC [ 0 m`) when it was styled, so a scroll never paints the next line.

### JSON

`--json` prints one line: `{"user":…,"hostname":…,"os":…,"kernel":…,"model":…,"uptime":…,"cpu":…,"memory":…,"display":…,"terminal":…,"shell":…,"resources":…}`. The keys and their order are stable. Every value is a string, or `null` when the platform cannot report it. Valid UTF-8 passes through; any other byte from 0x80 up is written as the `\u00XX` escape of the Latin-1 character with that number, so the output is always valid JSON.

## Interfaces

`afterfetch-core` is `no_std`, allocates nothing and does not panic on any input. A platform shell is an entry point, its system calls and two trait implementations, handed to `afterfetch_core::run(platform, terminal, argv)`, where `argv` is the arguments after the program's name.

- `Platform` fills `Facts` with what it can read, reports `TerminalInfo` (is it a terminal, its width, the colour depth it supports and whether it draws UTF-8), answers environment lookups, names the default logo and may name its target for `--version`.
- `Terminal` takes `write(bytes)`, `set_style(Style)` and `end_line()`. A `Style` is a foreground and background `Paint` (the terminal's own colour, a `Tone`, or one of the 16 console slots for swatches) and a `Weight`. A console shell turns a style into an attribute call. `AnsiTerminal` turns styles into escape sequences for `Mono`, `Ansi16`, `Ansi256` and `TrueColour` over any `ByteSink`, ending lines with `\n` or `\r\n` as the shell chooses.
- `Depth` is `None`, `Mono`, `Ansi16` (SGR numbering: red is 1, blue is 4), `Ansi256`, `TrueColour` and `Console16` (console numbering: blue is 1, red is 4).
- Console slots hold the Afterglow Dusk table, and every `Tone` maps to a slot: gold 14, orange 6, coral 12, rose 4, lilac 13, violet 5, periwinkle 1, slate 8. `Ansi16` swaps the red and blue bits, `Ansi256` takes the nearest cube or grey entry, `TrueColour` the table's RGB.

## Settings and defaults

To be written.

## Security and privacy

To be written.

## Limits

To be written.
