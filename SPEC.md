# Afterfetch specification

What `afterfetch` prints, how it is asked, and what each platform build promises. Why each choice was made is in [docs/decisions.md](docs/decisions.md) (cited as AF-D*n*); how each target is built and tested is in [docs/platforms.md](docs/platforms.md). When this document and the code disagree, decide which is right and fix the other in the same change.

## Status

Nothing is released. Every part may change between commits. A part marked **planned** is specified here but not implemented yet.

- **Core** (`afterfetch-core`: arguments, colour depth, facts, layout, JSON): in progress; the command line, facts, layout, depth and JSON below are implemented and host-tested, including exit code 74, JSON schema 1 and the architecture accent. The logo decoder reads the generated blobs and draws them in every form at every depth, host-tested with goldens for every logo.
- **Art** (`afterfetch-art`, the eight logos): all eight masters and ASCII forms are drawn and their blobs generated (2,026 bytes together, §7). The core decodes them (AF-D46). `umber` and `soot`, used only by `desk`, are provisional until the owner reviews that logo (AF-D42).
- **Preview** (`afterfetch-preview`, host only, AF-D50): draws the real core's output for any system from sample facts, as text, ANSI, PNG or HTML, and writes the contact sheet of the eight logos. See [docs/preview.md](docs/preview.md).
- **Platforms**: Linux x86_64, i586, aarch64, armv7 and armv6 build and pass their local checks, the Arm ones under qemu-user (§13); the Win32 i586, x86_64 and ARM64 builds exist (x86_64 runs under Wine, ARM64 is structure-checked only); every other target is not started. The order and the go/no-go checkpoints are in §13.

## 1 · Purpose and scope

`afterfetch` prints a short, honest summary of the system it runs on beside an Afterglow sun drawn for that kind of system. It is one small static program per platform, with no libc, no runtime to install and no network access.

**Non-goals.**

- Reporting a fact a system cannot honestly read. That row is left out (AF-D20).
- Per-architecture artwork. The architecture is text plus an accent colour (AF-D17).
- Configuration files, themes, plugins or a scripting language.
- Packages for any distribution or store. Releases are plain files and zips (§12).

## 2 · Program names

| Platform | File | Notes |
|---|---|---|
| Linux, FreeBSD, macOS | `afterfetch` | Static; on macOS it links `libSystem` only (AF-D7) |
| Windows 95 and later, ReactOS | `afterfetch.exe` | Console program, subsystem 4.0 on x86 |
| DOS | `AFETCH.EXE` | 8.3 name; DPMI program with CWSDPMI embedded (AF-D1, AF-D16) |
| Windows 3.1 | `AFETCHW.EXE` | 8.3 name; a small window, since Windows 3.1 has no console (AF-D3) |
| Desk | `AFETCH.DMD` | A Desk module (AF-D25) |

`AFETCH.EXE` and `AFETCHW.EXE` can sit together in one `C:\UTILS`. Under Windows 3.1, `AFETCH.EXE` also runs in the MS-DOS Prompt and says so in its OS row.

Each build has a **target name**, `<os>-<arch>`, used by `--version` and in release file names (§12): `linux-x86_64`, `linux-i586`, `linux-aarch64`, `linux-armv7`, `linux-armv6`, `linux-mips`, `linux-mipsel`, `linux-mips64el`, `linux-riscv64`, `linux-ppc64le`, `freebsd-x86_64`, `freebsd-i686`, `freebsd-aarch64`, `macos-aarch64`, `macos-x86_64`, `windows-i586`, `windows-x86_64`, `windows-aarch64`, `dos-i386`, `win16-i386`, `desk-i486`.

## 3 · Command line

Arguments are parsed by the core from raw bytes, with no allocation, the same way on every platform. Both spellings work everywhere: GNU-style long options for Unix and Windows users, slash options for DOS and Windows 3.1 users. Option names are case-insensitive in the slash form and case-sensitive in the long form; values (logo names, colour modes) are case-insensitive in both.

| Long form | Slash form | Meaning |
|---|---|---|
| `--plain` | `/PLAIN` | No colour and the ASCII form of the logo: exactly what a pipe gets |
| `--no-logo` | `/NOLOGO` | Facts only |
| `--logo <name>`, `--logo=<name>` | `/LOGO:<name>` | `auto` (the default: this system's logo, §7) or a logo name; `list` prints the names, one per line, and exits |
| `--colour <mode>`, `--colour=<mode>`; `--color` is an alias | `/COLOUR:<mode>`, `/COLOR:<mode>` | `auto` (default), `never`, `always`, `mono`, `16`, `256`, `truecolour` (`truecolor` and `24bit` are aliases) |
| | `/MONO` | The same as `--colour mono` |
| `--json` | `/JSON` | The facts as one JSON object (§6); no logo, no colour |
| `-h`, `--help` | `/?`, `/HELP` | Usage on standard output, then exit 0 |
| `-V`, `--version` | `/VERSION` | `afterfetch 0.1.0 (<target name>)`, for example `afterfetch 0.1.0 (linux-i586)`, then exit 0 |

- Options may come in any order; when one is given twice, the last wins. `--help` and `--version` win over everything else.
- There are no positional arguments. Anything that is not an option above is a usage error.
- A usage error (an unknown option, a missing value, or a value that is not a logo name or colour mode) prints one line naming the argument's position to the error stream and exits 2.
- **`--version`** prints the §2 target name of the build, named by its shell (AF-D44): the i586 Linux build prints `afterfetch 0.1.0 (linux-i586)`.

**Windows 3.1 only.** `AFETCHW.EXE` reads the same options from its command line (a Program Manager item or File > Run) and adds `/DUMP=<file>`: write what a pipe would get to `<file>` and exit without opening a window. `/DUMP` honours `/JSON`, `/NOLOGO` and `/LOGO`; it is how the program is tested. Without `/DUMP` the window always draws in colour; `/PLAIN` and `/MONO` draw the ASCII form in black on white.

**Desk.** A Desk module has no command line. `AFETCH.DMD` always runs as `--plain` with the `desk` logo (§5).

**Exit codes.**

| Code | Meaning |
|---|---|
| 0 | Done, including `--help`, `--version` and `--logo list` |
| 2 | Usage error |
| 70 | Crash: a panic ends the program with 70 on every platform, matching the Liminal Win32 programs |
| 74 | Output failure: the output could not be written (a closed pipe, or a `/DUMP` file that cannot be created); decision AF-D33. The core's `run` returns it when the terminal's final flush reports a failed write, unless the run already ends in 2 or 70; a shell returns it for a file it could not create (`output_exit`) |

## 4 · Facts

The facts are one ordered list in the core. A platform fills each row it can honestly read and leaves the rest empty; an empty row is **omitted** from the text output and is `null` in JSON (AF-D20). Values are cut to fit at a character boundary, never wrapped (§5), and each holds at most 64 bytes; the user and host names hold at most 32.

The output is, in order: the **title** (`user@host`, or whichever half exists; left out with its rule when neither does), a **rule** of dashes as long as the title, the rows below in this order (an omitted row closes up), then, with colour only, a gap and two rows of eight swatches of the console colours.

| Row | Label | Tone | JSON key |
|---|---|---|---|
| Title | — | gold user, slate `@`, lilac host | `user`, `hostname` |
| OS | `OS:` | gold | `os` |
| Kernel | `Kernel:` | orange | `kernel` |
| Host (the machine's model, not its network name) | `Host:` | coral | `model` |
| Uptime | `Uptime:` | coral | `uptime` |
| CPU | `CPU:` | rose | `cpu` |
| Memory | `Memory:` | rose | `memory` |
| Display (a graphical screen) | `Display:` | lilac | `display` |
| Desktop (the desktop environment) | `Desktop:` | lilac | `desktop` |
| Terminal (a text terminal) | `Terminal:` | lilac | `terminal` |
| Shell | `Shell:` | violet | `shell` |
| Resources (Windows 3.1 only) | `Resources:` | periwinkle | `resources` |

**Value formats.** Uptime is `2d 4h 17m`. Memory is `387 MB free of 511 MB`, switching to `12.4 GB free of 15.6 GB` once the total reaches 4 GiB. Display is `1024 x 768, 32-bit` (the depth is left off when unknown). Desktop is the platform's own name for it, as given (`GNOME`, `KDE`, `ubuntu:GNOME`). Terminal is `xterm-256color, 120 x 30` (the size is left off when unknown). CPU is the brand or model text with runs of spaces collapsed, then the core count in brackets (`(8 cores)`) when more than one core is known. Shell is the interpreter's file name, lower-cased on DOS and Windows. Resources is `72% free`.

**Architecture** appears in the OS row on every platform, from `uname().machine`, `hw.machine_arch`, `GetSystemInfo`, or the build's own `target_arch` as the last resort. A system's own name is printed as it gives it (`x86_64`, `i686`, `aarch64`, `armv7l`, `mips`), with no mapping to a list (AF-D44). The form is `<os> (<arch>)` (`Debian GNU/Linux 13 (aarch64)`), except on Win32, which keeps the ReactOS form `<product> <release>-x86` for a known release on x86 so the ReactOS image prints what it prints today (`ReactOS 0.4.16-x86`).

### What each platform reports

A blank cell means the row is omitted on that platform.

| Row | DOS | Windows 3.1 | Win32 (95 to 11, ReactOS) | Linux | FreeBSD | macOS | Desk |
|---|---|---|---|---|---|---|---|
| Title | Host from `INT 21h AX=5E00h` when a network redirector is loaded, else `%HOSTNAME%` or `%COMPUTERNAME%`; user from `%USER%` | `SYSTEM.INI [Network] UserName` and `ComputerName` (Windows for Workgroups) | `GetUserNameA`, `GetComputerNameA` | `$USER` or `$LOGNAME`, else the uid's name in `/etc/passwd`; `uname().nodename` | `$USER`, else the uid's name in `/etc/passwd`; `kern.hostname` | `$USER`; `kern.hostname` | |
| OS | `INT 21h AH=30h` and `AX=3306h` (true version); MS-DOS, PC DOS or FreeDOS from the OEM byte, DR-DOS from `AX=4452h`; "under Windows 3.1 enhanced mode" from `INT 2Fh AX=1600h` | `GetVersion()`: Windows 3.10 or 3.11 and the DOS version | Registry `ProductName` and the `BuildLab` release, or the `GetVersionExA` platform on 9x | `PRETTY_NAME` from `/etc/os-release`, else `/usr/lib/os-release` | `kern.ostype` and `kern.osrelease` | `macOS` and `kern.osproductversion` | `Desk` and its version, and the track: `native` or `hosted on DOS` |
| Kernel | DOS kernel version, and whether it is loaded high (`AX=3306h` DH bit 4) | Mode: enhanced or standard, from `GetWinFlags()` | `NT 5.2.3790 SP3` on the NT family, `Windows 95 4.00.950` on Windows 9x (4.10 is `Windows 98`, 4.90 `Windows Me`), from the `GetVersionExA` platform | `uname` sysname and release | `kern.osrelease` and `kern.osrevision` | `Darwin` and `kern.osrelease` | Native: `desk-kernel` version; hosted: DOS version and DPMI host |
| Host | BIOS date at `F000:FFF5` and model byte at `F000:FFFE`, when known | | | `/sys/devices/virtual/dmi/id/product_name`, or `/proc/device-tree/model` on ARM boards; firmware placeholders such as "To Be Filled By O.E.M." are not a model (AF-D39) | `smbios.system.product` through `kenv`, when readable | `hw.model` (`Mac15,6`) | |
| Uptime | Omitted: DOS has no boot clock, and the BIOS tick count resets at midnight | `GetTickCount()` since Windows started | `GetTickCount64` where present (Vista and later, looked up with `GetProcAddress`), else `GetTickCount`, which wraps at 49.7 days | `/proc/uptime`, else `sysinfo().uptime` | `clock_gettime(CLOCK_UPTIME)` | `kern.boottime` against `gettimeofday` | Time since Desk started, once the SDK reports it |
| CPU | 386, 486 or Pentium class from the EFLAGS AC and ID bits, then the `CPUID` vendor and brand string when present; FPU presence | 286, 386 or 486 and the coprocessor from `GetWinFlags()` | The `CPUID` brand string on x86 and x86_64 (32-bit builds test the EFLAGS ID bit first), else the registry `ProcessorNameString` (the only source on ARM), else the vendor and family of a Pentium with no brand; the count is `GetSystemInfo`'s processors (AF-D52) | `/proc/cpuinfo` `model name`, else `cpu model` (MIPS), `uarch`, a known Arm core, `Hardware` or `Model` (AF-D39); the core count | `hw.model`, `hw.ncpu` | `machdep.cpu.brand_string`, `hw.ncpu` | Once the SDK reports it; hosted: the DPMI processor byte |
| Memory | Conventional (`INT 12h`), extended (`INT 15h AX=E801h`, else `AH=88h`), free from DPMI `INT 31h AX=0500h` | `GetFreeSpace(0)`, the free global heap | `GlobalMemoryStatusEx` where present (more than 4 GB), else `GlobalMemoryStatus` | `MemAvailable` of `MemTotal` from `/proc/meminfo` (free, buffers and cache on kernels without `MemAvailable`), else `sysinfo` (AF-D39) | `vm.stats.vm.v_free_count` × `hw.pagesize` of `hw.physmem` | `vm.page_free_count` × `hw.pagesize` of `hw.memsize` | Usable and free RAM, once the SDK reports it |
| Display | Adapter from `INT 10h AX=1A00h` (VGA, EGA, MDA), VESA version and video memory from `INT 10h AX=4F00h`, text mode and rows from the BIOS data area | `GetSystemMetrics` size, `GetDeviceCaps` `BITSPIXEL × PLANES` or `NUMCOLORS` | `GetSystemMetrics`, `GetDeviceCaps` | | | | Mode and depth (`640 x 480, 8-bit`), once the SDK reports it |
| Desktop | | | | `$XDG_CURRENT_DESKTOP`, only when set and not empty (AF-D35) | | | |
| Terminal | | | | `$TERM_PROGRAM` or `$TERM`; size from `TIOCGWINSZ` | As Linux | As Linux | |
| Shell | `%COMSPEC%` | `SYSTEM.INI [boot] shell=` | `%COMSPEC%` | `$SHELL` | `$SHELL` | `$SHELL` | The shell's name, once Desk names it |
| Resources | | `GetFreeSystemResources(GFSR_SYSTEMRESOURCES)` | | | | | |

**Desk.** Desk exposes no system information to a module today, and a module must never touch hardware, so `AFETCH.DMD` reads nothing itself: until Desk offers the proposed `desk_sys_info` call (AF-D26, `docs/desk-sys-info.md`), it reports only the OS row, from what the build knows (`Desk` and the track the module was packed for). Every other Desk cell above is what that call would supply.

**Where facts are parsed.** Text formats (`/proc/meminfo`, `/proc/cpuinfo`, `os-release`, `/etc/passwd`, the sysctl strings) are parsed by the core and tested on the host against captured files from real systems. A shell only makes the system calls.

## 5 · Output

### Layout

- **Side by side.** The logo sits on the left and the title and facts start at the logo's information column, beginning on the logo's second row. The information column is the logo's widest form plus a 2-column gap: 38 for a 36-column logo, and 42 for `win32`, whose 40-column ASCII form sets it for every form, so the facts do not move between plain and colour output (AF-D41). Fact lines beyond the logo's rows continue below it in the same column. The logo is drawn beside the facts only when `columns ≥ info column + 34`: 72 columns for a 36-column logo, 76 for `win32`.
- **Narrow console.** Below that, the facts are printed alone. Values are cut at the right edge, never wrapped.
- **Width.** From `TIOCGWINSZ` (Unix), `GetConsoleScreenBufferInfo` (Win32), the BIOS data area at `0040:004A` (DOS), or 80 when it cannot be read, which includes every pipe and file.
- **`--no-logo`** prints the facts alone at any width.
- Plain output has no trailing blank lines and no trailing spaces.

### Colour depth

Decided once at start, first match wins:

1. `--plain` or `--colour never`: no colour.
2. `--colour mono`, `16`, `256` or `truecolour`: that depth, even on a pipe. `16` means console attributes where the platform's own colour is console attributes (Win32 consoles, DOS).
3. `--colour always`: the depth the platform detects, even on a pipe; 16 colours when nothing is detected.
4. `NO_COLOR` set and not empty: no colour.
5. Standard output is not a terminal (Unix `ioctl(TCGETS)` fails, Win32 `GetConsoleMode` fails, DOS `INT 21h AX=4400h` says it is not the console device): no colour, unless `CLICOLOR_FORCE` or `FORCE_COLOR` is set.
6. The platform's detection:
	- **Unix:** `COLORTERM` of `truecolor` or `24bit` gives true colour; `TERM=dumb` gives none; `TERM` ending `-256color` gives 256; anything else, the Linux console included, gives 16.
	- **Win32:** on the x86_64 and ARM64 builds, true colour through VT sequences when `SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)` succeeds (Windows 10 1511 and later, Windows Terminal); otherwise 16 console attributes (AF-D13). Windows 9x, ReactOS and the Pentium build take the attribute path, the Pentium build because its 24 KiB has no room for the escape writer (AF-D52).
	- **DOS:** 16 BIOS attributes in a colour text mode; mono in mode 7 (MDA, Hercules).
	- **Windows 3.1:** the nearest of the 16 VGA colours when the driver has 16 colours or fewer, exact RGB otherwise (AF-D15).
	- **Desk:** none; the module writes plain lines to Desk's log.

### Rules

1. **Plain output** (no colour, from any of the rules above): no escape sequences and no console calls, the ASCII form of the logo (the sun is `#`), and no swatches. Everything afterfetch draws itself is ASCII; values are written as the system gave them. Line ends are `\n` on Unix and `\r\n` on DOS and Windows.
2. **Restore.** Every line ends by restoring the console's own attribute (Win32, DOS) or with `ESC[0m` (Unix), and so does the program, so a scroll never paints the next line and the prompt keeps its colours.
3. **Mono** uses weight, not colour: the sun and horizon bold, the deep water dim (`ESC[1m`, `ESC[2m`) on Unix, the intensity bit on DOS and MDA, so the sun stays visible on a green screen. Mono has no swatches. In UTF-8 the logo's half blocks are drawn in mono too, a cell with two drawn pixels as one `█` in the weight of the upper one; elsewhere the ASCII form is drawn with its weights and the sun as `#`. A tone with no weight (soot) draws nothing.
4. **Block glyphs.** The colour logo is drawn with half blocks (`▀ ▄ █`) in UTF-8 on Unix when `LC_ALL`, `LC_CTYPE` or `LANG` names UTF-8, and otherwise as cells (spaces with a background colour). DOS uses the CP437 half blocks `0xDF 0xDC 0xDB`. With colour and no half blocks the sun is solid cells. Win32 consoles use cells, which need no font glyph, and half blocks only in VT mode when `SetConsoleOutputCP(65001)` succeeds. Each tone is drawn in the colour the art table gives it at the depth; a tone with none (night and soot at 16 colours are the cases AF-D42 names) is transparent, and two pixels that show the same colour at the depth are one `█`. A logo's `PIXELS_LOW` picture, when it has one, is used at 16 colours and in mono.
5. **DOS blink.** While drawing background-coloured cells, DOS turns blink off (`INT 10h AX=1003h BL=0`) so all 16 background colours show, and turns it back on before exit.
6. **Architecture accent.** The title's rule is drawn in the architecture's accent tone so a row of screenshots tells the arches apart: x86 and x86_64 gold, ARM (32 and 64-bit) periwinkle, MIPS violet, RISC-V teal, PowerPC coral, and slate for any other or unknown architecture (AF-D36). The family is read from the system's own name by prefix (`i686`, `armv7l`, `mipsel`, `ppc64le` and `riscv32` all count) and looked up in the art crate's accent table; the name shown is never changed. A platform that does not fill `arch` gets the slate rule.

### Windows 3.1 window

`AFETCHW.EXE` opens a small fixed-size window captioned "afterfetch". The logo is painted as 4 × 4 px pixels (144 × 96 px for a 36 × 24-pixel logo) and the facts in `OEM_FIXED_FONT`, one `TextOut` per run of one colour. Esc, Enter, a click or closing the window ends it. Ctrl+C copies the plain text form to the clipboard as `CF_TEXT`.

### Desk module

`AFETCH.DMD` renders the plain form and hands each line to `desk_log`, one log line per output line, then returns from `app_start`. A window version over Desk's SDK, drawing the logo with Desk's own palette, follows once that SDK exists (§13).

## 6 · JSON output

`--json` writes one JSON object on one line, followed by the platform's line end, with no logo and no colour. Every key always appears, in this order, on every platform. A fact the platform cannot report is `null`: the text output leaves that row out, but JSON keeps the key (AF-D20). Every other value is a string: the same text the plain output prints.

```json
{"schema":1,"user":"ada","hostname":"dusk","os":"Debian GNU/Linux 13 (x86_64)","kernel":"Linux 6.12.48","model":null,"uptime":"2d 4h 17m","cpu":null,"memory":"18.2 GB free of 31.1 GB","display":null,"desktop":"GNOME","terminal":"xterm-256color, 120 x 30","shell":"zsh","resources":null,"target":"linux-x86_64","arch":"x86_64","uptime_seconds":188220,"memory_available_bytes":19541000000,"memory_total_bytes":33400000000}
```

| Key | Value |
|---|---|
| `user`, `hostname` | The title's two halves |
| `os`, `kernel`, `model`, `uptime`, `cpu`, `memory`, `display`, `desktop`, `terminal`, `shell`, `resources` | The rows of §4 |

**Schema version 1 (AF-D34, AF-D43).** The JSON starts with a schema number and ends with the build's identity and raw numbers beside the display text:

| Key | Value |
|---|---|
| `schema` | The number `1`, first in the object, so a script can check which keys to expect |
| `target`, `arch` | The build's target name (§2) and the architecture name of §4 exactly as the system gives it (`x86_64`, `i686`, `aarch64`, `armv7l`), as strings, or `null` when the system does not say |
| `uptime_seconds` | Whole seconds since boot, a number, or `null` |
| `memory_available_bytes`, `memory_total_bytes` | The two figures of the Memory row in bytes, numbers, or `null` |

`schema` comes first and the other five follow `resources`, in the order above, so the object has 19 keys. The numbers are set together with their text rows, so the two cannot disagree. Win32 asks `GlobalMemoryStatusEx` and `GetTickCount64` where Windows has them (Windows 98 and later, and Vista and later), so a large machine's byte counts and a long uptime are exact; on Windows 95 it falls back to `GlobalMemoryStatus`, whose byte counts stop at 4,294,967,295, and `GetTickCount`, whose seconds wrap after 49 days, as the text row does.

**Encoding.** `"`, `\`, control characters and DEL are escaped. Valid UTF-8 passes through; any other byte from 0x80 up (CP437 on DOS, say) is written as the `\u00XX` escape of the Latin-1 character with that number, so the output is always valid JSON.

**Stability.** Until 0.1.0 the keys and their order may still change. From 0.1.0 on, a key is never renamed or removed, its meaning never changes, and the order stays. New keys may only be added at the end. A consumer should ignore keys it does not know and treat every key as possibly `null`. Values are display text and may change form between releases.

## 7 · Logos

Every logo is the same Afterglow sun, gold at the crown through orange to coral at the horizon, cut by a rose horizon line. What changes per system is the horizon, the time of day and the way it is drawn. No logo borrows a trademark. The art language is in `docs/art.md`, the blob format in `docs/logo-format.md`.

All eight are drawn; `docs/art.md` describes each one.

| Name | Default on | Idea | Cells (ASCII form) | Blob |
|---|---|---|---|---|
| `afterglow` | any platform without its own logo | The plain sun half set on a single rose line, at sunset | 36 × 12 (36 × 12) | 178 B |
| `dos` | DOS (`AFETCH.EXE`, also in a Windows 3.1 DOS box) | A chunky sunrise behind CRT scanlines, in colours that land on the 16 VGA colours | 36 × 12 (36 × 12) | 157 B |
| `win16` | Windows 3.1 (`AFETCHW.EXE`) | A bevelled morning sun, raised like a button, over Program Manager tiles | 36 × 12 (36 × 12) | 360 B |
| `win32` | Windows 95 to 11, ReactOS | Dusk over water: the existing ReactOS picture, unchanged | 36 × 12 (40 × 12) | 328 B |
| `linux` | Linux | Night: the sun set onto a shell-prompt horizon ending in a block cursor | 36 × 12 (36 × 12) | 198 B |
| `freebsd` | FreeBSD | Last light: a deeper ember sun behind a low ridge | 36 × 12 (36 × 12) | 179 B |
| `macos` | macOS | Golden hour: a soft glass sun with a still-water reflection | 36 × 12 (36 × 12) | 232 B |
| `desk` | Desk | After dark: the banded sun wearing Desk's stepped halo on the Void, over Desk's stepped horizon, in Desk's 16 colours | 36 × 12 (36 × 12) | 394 B |

- `--logo list` prints these names, one per line, in this order: `afterglow`, `dos`, `win16`, `win32`, `linux`, `freebsd`, `macos`, `desk`. Every binary carries every logo (AF-D10), so `--logo` works everywhere.
- **Size** (AF-D41). Every logo's picture is 36 × 24 pixels, 36 columns × 12 rows of half-block cells, and its ASCII form is at most 36 × 12. The `win32` ASCII form is the single exception: it keeps its existing 40 columns so the ReactOS output stays byte for byte the same. All blobs together are at most 6 KiB (§11).
- **Forms.** Half blocks, cells, or a hand-authored ASCII form of the same size; the ASCII form is what plain output and pipes get.
- **Architecture** is never drawn: it is text in the OS row and the accent of the rule (§5).

## 8 · Interfaces

`afterfetch-core` is `no_std`, allocates nothing and does not panic on any input. A platform shell is an entry point, its system calls and two trait implementations, handed to `afterfetch_core::run(platform, terminal, argv)`, where `argv` is the arguments after the program's name (AF-D31).

- `Platform` fills `Facts` with what it can read, reports `TerminalInfo` (is it a terminal, its width, the colour depth it supports and whether it draws UTF-8), answers environment lookups, names the default logo and may name its target for `--version`.
- `Terminal` takes `write(bytes)`, `set_style(Style)` and `end_line()`. A `Style` is a foreground and background `Paint` (the terminal's own colour, a `Tone`, or one of the 16 console slots for swatches) and a `Weight`. A console shell turns a style into an attribute call. `AnsiTerminal` turns styles into escape sequences for `Mono`, `Ansi16`, `Ansi256` and `TrueColour` over any `ByteSink`, ending lines with `\n` or `\r\n` as the shell chooses.
- `Depth` is `None`, `Mono`, `Ansi16` (SGR numbering: red is 1, blue is 4), `Ansi256`, `TrueColour` and `Console16` (console numbering: blue is 1, red is 4).
- Console slots hold the Afterglow Dusk table, and every `Tone` maps to a slot: gold 14, orange 6, coral 12, rose 4, lilac 13, violet 5, periwinkle 1, slate 8. `Ansi16` swaps the red and blue bits, `Ansi256` takes the nearest xterm cube or grey entry (never 0 to 15, which belong to the user's theme), `TrueColour` the table's RGB.
- **The logo tones** (`art/palette/afterglow-dusk.toml`, generated into `afterfetch-art`) follow the same rules, with these exceptions (AF-D42): amber's 256 entry is 221, not its nearest, 215, which is orange's, so the five sun bands stay distinct; at 16 colours amber and peach take orange's slot (6) to keep the bands in order; night and umber take slate (8) by role; soot has no 16-colour entry and is dropped at 16 colours and in mono. Umber and soot are provisional until the owner reviews the `desk` logo. The full table is in `docs/art.md`.

## 9 · Settings and defaults

There is no configuration file. The program reads only these environment variables:

| Variable | Effect |
|---|---|
| `NO_COLOR` | Set and not empty: no colour unless asked for explicitly (§5) |
| `CLICOLOR_FORCE`, `FORCE_COLOR` | Colour on a pipe |
| `COLORTERM`, `TERM` | Colour depth on Unix |
| `LC_ALL`, `LC_CTYPE`, `LANG` | UTF-8 half blocks on Unix |
| `USER`, `LOGNAME`, `SHELL`, `TERM_PROGRAM` | Facts on Unix (§4) |
| `XDG_CURRENT_DESKTOP` | The Desktop row on Linux (§4) |
| `HOSTNAME`, `COMPUTERNAME`, `USER`, `COMSPEC` | Facts on DOS; `COMSPEC` on Win32 |
| `AF_DUMP` | DOS test hook: after drawing, write the text-mode screen's characters and attributes to the named file |

Defaults: `--logo auto`, `--colour auto`, logo shown when it fits.

## 10 · Security and privacy

- No network access, no telemetry, no writes, except the file named by `/DUMP` (Windows 3.1) or `AF_DUMP` (DOS tests).
- The output includes the user and host names, which a user pasting it into a bug report may want to remove. `--json` includes them too.
- Files read: `/etc/os-release`, `/usr/lib/os-release`, `/proc/meminfo`, `/proc/cpuinfo`, `/etc/passwd` (only to name the current uid), `/proc/uptime`, `/sys/devices/virtual/dmi/id/product_name`, `/proc/device-tree/model` and, on Windows 3.1, `SYSTEM.INI`. Nothing else is opened.
- The Linux build makes only these system calls: `read`, `write`, `openat`, `close`, `ioctl` (`TCGETS`, `TIOCGWINSZ`), `uname`, `sysinfo`, `getuid`, `exit_group`. FreeBSD adds `__sysctl`, `clock_gettime` and `kenv` and uses `exit`. Win32 imports only the ANSI functions on its allow-list. The Desk module imports only `desk_log` and `desk_panic` (and `desk_sys_info` once it exists).
- The core never panics on any input, including truncated logo blobs and arbitrary argument bytes; a panic in a shell exits 70.

## 11 · Limits and size budgets

Every release binary must fit its budget; `scripts/check-sizes.sh` reads `sizes.toml` and a build over budget fails `make check`.

| Target | Budget |
|---|---|
| Win32 i586 | 24 KiB (AF-D47) |
| Win32 x86_64, ARM64 | 32 KiB (AF-D52) |
| Linux x86_64, i586, ARM64 and 32-bit Arm | 32 KiB (AF-D47) |
| FreeBSD (any arch) | 24 KiB |
| macOS (each arch) | 64 KiB |
| DOS `AFETCH.EXE` | 64 KiB, CWSDPMI stub included |
| Windows 3.1 `AFETCHW.EXE` | 24 KiB file; code segment `MAIN` 20,000 bytes; DGROUP 16 KiB |
| Desk `AFETCH.DMD` | 16 KiB (the later window version 24 KiB) |
| All logo blobs together | 6 KiB (6,144 bytes) and 700 bytes per blob, enforced by the art generator; 2,026 bytes today. 7 KiB is the ceiling if the budget ever has to grow (AF-D40) |

Other limits: a fact value is at most 64 bytes, a user or host name at most 32; a logo at most 36 × 12 cells, its ASCII form at most 36 × 12 (`win32`'s 40 × 12, AF-D41); the DOS floor is a 386 with DPMI (AF-D16), the 32-bit x86 floor elsewhere is a Pentium without SSE2 (AF-D19), Desk's is a 486, and Windows 3.1's is a 386 in enhanced mode.

## 12 · Releases and reproducibility

**Reproducible.** Every target is built twice, in different directories, by `make repro`, and the two outputs must be byte-identical (`cmp`). Builds embed no paths, times or host names: `--remap-path-prefix`, `-Z location-detail=none`, `--build-id=none` on ELF, `/Brepro` and `/DEBUG:NONE` on PE, content-hashed `LC_UUID` on Mach-O. Release zips use fixed timestamps (`TZ=UTC`, `touch -d @0`, `zip -X -D`), so two `make dist` runs give the same `SHA256SUMS`.

**Release files.** `afterfetch-<version>-<target name>` for Unix (`afterfetch-0.1.0-linux-aarch64`), the same with `.exe` for Win32 (`afterfetch-0.1.0-windows-i586.exe`), and zips for the rest: `afterfetch-<version>-dos.zip` (`AFETCH.EXE`, `README.TXT` in CP437 with CRLF), `afterfetch-<version>-win16.zip` (`AFETCHW.EXE`, `README.TXT`) and `afterfetch-<version>-desk.zip` (`AFETCH.DMD`, marked "waiting on Desk's loader" until Checkpoint D passes). `SHA256SUMS` covers all of them, sorted.

## 13 · Platform matrix and verification

What "verified" means for each target. **Local** means the program runs on this machine (natively or emulated) and passes the shape check: plain output with no escape bytes, the expected labels in order, every line within 80 columns, and `--json` parsing with the expected keys. **Structure** means built and checked without running (file format, imports, size). The **Status** column is the state today; the others show what each target is held to.

| Platform | Arch | Held to | How | Checkpoint | Status |
|---|---|---|---|---|---|
| Linux | x86_64, i586 | Local | Native, plain, JSON and a pseudo-terminal colour run; i586 also under `qemu-i386-static` | Slice (Checkpoint A) | Built and passing: structure, size, smoke and reproducibility |
| Linux | aarch64, armv7, armv6 | Local, emulated | qemu-user, called by name, as a Cortex-A72, Cortex-A7 and ARM1176 | — | Built and passing under qemu-user: structure, size, smoke and reproducibility; real kernels and hardware unverified (AF-D51) |
| Linux | mips, mipsel (mips64el optional) | Local, emulated | qemu-user, called by name, as a 24Kc | — | Built and passing under qemu-user: structure, smoke and reproducibility; 42.7 KB against a 44 KiB budget for MIPS only; real kernels and hardware unverified (AF-D22) |
| Linux | riscv64, ppc64le | Local, emulated | qemu-user | After MIPS (went, AF-D22) | Not started |
| FreeBSD | x86_64, i686 | Local, in a VM | The FreeBSD amd64 cloud image under QEMU | **Go/no-go** (AF-D23): build-only plus structure checks if the VM automation is not reliable | Not started; image not fetched |
| FreeBSD | aarch64 | Structure | ELF brand, static, size | Run unverified until an aarch64 VM or hardware exists | Not started |
| macOS | aarch64, x86_64 | Structure locally; CI later | Mach-O load commands and symbols; runs on a GitHub macOS runner after the move to GitHub (AF-D5) | **Go/no-go** on the first CI run | Not started |
| Win32 | i586, x86_64 | Local, emulated | Wine, plain and a pseudo-terminal console run (attributes for i586, true colour and half blocks for x86_64); Windows 95 in DOSBox-X once per release; the ReactOS guest in reactos-liminal is the real-console proof | Slice (Checkpoint A) for i586 | Built and checked; Windows 95, ReactOS and a real Windows 10 console not yet run |
| Win32 | aarch64 | Structure locally; CI later | PE checks; runs on a GitHub Windows ARM64 runner after the move (AF-D5) | — | Built and structure-checked; unverified locally |
| DOS | i386 | Local, emulated | DOSBox Staging, plain and colour (screen read back); FreeDOS in QEMU once per release | **Go/no-go**: plain and colour runs pass and the size fits | Not started |
| Windows 3.1 | i386 | Local, emulated | Wine `/DUMP`; Windows for Workgroups 3.11 in DOSBox-X with screenshots | **Go/no-go** (AF-D24): the DOS build is the Windows 3.1 answer if no stable window in two builder sessions | Not started |
| Desk | i486 | Structure and host | `dmdpack info` and `cross`, a 486 instruction scan, size, and the module's logic run on the host; runs in Desk's emulators once Desk has a loader | **Go/no-go** (Checkpoint D, AF-D28): re-checked each release, never blocks other platforms | Not started |

Not verified anywhere: real hardware of any kind (ARM boards, MIPS routers, a 486, MDA and Hercules adapters beyond DOSBox's emulation), Windows 3.0 and Windows 3.1 standard or real mode, and real ARM or MIPS kernels (qemu-user uses the host's).
