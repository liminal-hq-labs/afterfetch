# The `.afl` logo format

An `.afl` blob is one logo: a picture in logo pixels, an optional second picture for 16 colours and below, and the hand-authored ASCII form. It is written by `art/scripts/afterfetch_art.py` from a master, `art/logos.toml` and `art/palette/afterglow-dusk.toml`, and read by `afterfetch-core` on every platform, including DOS and Windows 3.1. The design language is in [art.md](art.md).

The format is built for that reader: decoded row by row into a caller's buffer of 48 cells, with no allocation, no tables to build and every length checked before it is used, so a truncated or hostile blob is an error, never a panic.

## Contents

- [Layout](#layout)
- [Header](#header)
- [Sections](#sections)
- [Pixel rows](#pixel-rows)
- [ASCII rows](#ascii-rows)
- [How the generator writes a blob](#how-the-generator-writes-a-blob)
- [Depth forms](#depth-forms)
- [Size budget](#size-budget)
- [Validation](#validation)
- [Example](#example)
- [Compatibility](#compatibility)

## Layout

All numbers are bytes; multi-byte numbers are little-endian. A blob is a header followed by sections, back to back, ending exactly at the end of the blob.

```text
header    magic, sizes, palette
section   kind, length, payload     PIXELS, required
section   kind, length, payload     PIXELS_LOW, optional
section   kind, length, payload     ASCII, required when the header says there is an ASCII form
```

## Header

| Offset | Size | Field | Rule |
|---|---|---|---|
| 0 | 4 | magic | `AFL1` (`41 46 4C 31`); the digit is the format version |
| 4 | 1 | `width` | logo pixels across, 1 to 48; one column of cells each |
| 5 | 1 | `height` | logo pixels down, 2 to 48 and even; two pixels per row of cells |
| 6 | 1 | `ascii_width` | columns of the ASCII form, 0 to 48: the logo's `ascii-width` in `art/logos.toml`, not the width of its widest row; 0 means the logo has no ASCII form |
| 7 | 1 | `ascii_rows` | rows of the ASCII form, 0 to 24: the logo's `ascii-rows`; 0 exactly when `ascii_width` is 0 |
| 8 | 1 | `palette_len` | entries in the logo's palette, 1 to 15 |
| 9 | `palette_len` | `palette` | tone ids, each 1 to 15, no repeats |

The logo's palette maps a **palette index** to a **tone id**: index 0 is always transparent, index `i` (1 to `palette_len`) is the tone `palette[i - 1]`. Tone ids are the `id` column of `art/palette/afterglow-dusk.toml` and never change meaning; the colour a tone shows at each depth lives in the generated `palette.rs`, not in the blob, so one table serves every logo. The palette's order is the logo's `tones` list in `art/logos.toml`, and one palette serves the whole blob: `PIXELS`, `PIXELS_LOW` and `ASCII` all index it.

The logo limits in `art/logos.toml` (36 × 24 pixels, ASCII 36 × 12, `win32`'s ASCII 40 × 12, AF-D41) are tighter than the format's. Every drawn logo today is 36 × 24 with a 36 × 12 ASCII form, `win32`'s 40 × 12. The format allows up to 48 so the limits can move without a version change.

## Sections

Each section is `kind` (1 byte), `length` (2 bytes, the payload's size) and `length` payload bytes.

| Kind | Name | Payload | Count |
|---|---|---|---|
| `0x01` | `PIXELS` | the picture: `height` [pixel rows](#pixel-rows) | exactly one |
| `0x02` | `PIXELS_LOW` | the picture for 16 colours, mono and the cells form: `height` pixel rows | at most one |
| `0x03` | `ASCII` | exactly `ascii_rows` [ASCII rows](#ascii-rows), empty rows included | one when `ascii_width` > 0, else none |

Kind `0x00` is not defined, and a reader rejects it. Sections appear in strictly ascending `kind` order. A reader skips a section of a kind it does not know (kinds `0x04` to `0x7F`, reserved for additions that older readers may ignore); kinds `0x80` and above are reserved for additions a reader must understand, so a reader that meets one rejects the blob. With the lengths, a reader finds any section without decoding the ones before it.

## Pixel rows

A pixel row is a run of **run bytes** whose lengths add up to exactly `width`. A run byte is:

```text
bit  7 6 5 4   3 2 1 0
     run - 1   palette index
```

so one byte covers 1 to 16 pixels of one palette index, and index 0 is transparent. A longer stretch of one index is several run bytes. A run never crosses into the next row, so row `y` starts where row `y - 1` ended and every row checks its own sum. A palette index above `palette_len` is an error.

An empty 36-pixel row is three bytes (`F0 F0 30`); a row of the sun is typically four to six.

## ASCII rows

An ASCII row is its art, then its tones:

| Part | Size | Content |
|---|---|---|
| `art_len` | 1 | bytes of encoded art that follow, 0 to 255 |
| art | `art_len` | the characters, encoded as below |
| tones | varies | run bytes, as in a pixel row, adding up to the decoded art's length |

**Art.** A byte `0x20` to `0x7E` is that character. A byte `0x80` to `0xFF` repeats the character before it `(byte & 0x7F) + 1` more times (1 to 128), so a run of 34 `-` is `2D A0` and a run of 36 is `2D A2`. A repeat byte may follow another repeat byte, which repeats the same character again. A repeat byte first in a row, and any other byte value, is an error. The decoded art is at most `ascii_width` characters; the rest of the row is spaces. Trailing spaces are not stored. An empty row is the single byte `00`: no art, and so no tone bytes. Every row of the form is stored, so the section holds exactly `ascii_rows` rows even when some are empty.

**Tones.** The runs give a palette index for every decoded character, spaces included, and stop exactly at the decoded art's length. They follow the decoded characters, not the art bytes, so one tone run may cover several repeats and spaces. A space has index 0 (plain), and every other character has a non-zero index: the generator holds the ASCII form to that, as the reactos-liminal tests did, and a reader treats a breach as an error.

A `#` in the art is the sun's body. The cells form draws it as a filled cell (a space on that tone's background), which is how the ReactOS console shows it; the plain form prints it as `#`.

## How the generator writes a blob

A reader accepts any blob that meets this specification. `art/scripts/afterfetch_art.py` always writes the same bytes for the same inputs, by these rules, so the committed blobs can be checked byte for byte:

- **Header.** `ascii_width` and `ascii_rows` come from the logo's entry in `art/logos.toml`, and the palette is its `tones` list in order.
- **Sections.** `PIXELS`, then `PIXELS_LOW` only when `art/masters/<logo>-low.svg` exists, then `ASCII`. `PIXELS_LOW` is quantised against the same `tones` list and indexed through the same palette as `PIXELS`. A logo with no ASCII file yet (only while it is not drawn) gets `ascii_width` and `ascii_rows` of 0 and no `ASCII` section.
- **Pixel runs.** Each row is packed greedily from the left: the longest run of one index, at most 16, then the next. A run never crosses a row.
- **Art.** Every run of 2 or more of one character, spaces included, is the character then repeat bytes for the rest: one repeat byte for up to 128 more, and further repeat bytes for longer runs. A single character is never followed by a repeat. Trailing spaces are dropped.
- **ASCII tone runs.** Each row's tones are packed greedily across the whole decoded row, at most 16 to a byte, regardless of where the art's repeats fall.
- **Tones.** A master pixel or an ASCII tone letter that names a tone missing from the logo's `tones` list is an error, and so is a master pixel further than `max-delta-e` from every listed tone. A `tones` entry that neither the master nor the ASCII form uses is only a warning, and it stays in the palette.
- **Round trip.** The generator decodes every blob it writes with its own reader, which makes the checks of [Validation](#validation), and fails unless the result is the pixels and ASCII form it started from.

## Depth forms

The blob holds pictures; the core picks a form and a depth and turns pixels into cells.

| Form | Used for | Source | A cell is |
|---|---|---|---|
| **Half block** | Unix UTF-8 terminals, DOS (CP437), Win32 in VT mode, every colour depth | `PIXELS`, or `PIXELS_LOW` at 16 colours and mono | the pixels `(x, 2r)` above and `(x, 2r + 1)` below; see the table below |
| **Cells** | consoles where a glyph cannot be trusted (legacy Win32 consoles, non-UTF-8 terminals) | the `ASCII` section, with `#` filled; a logo with no ASCII form uses its pixels, one colour per cell: the upper pixel when it is drawn, else the lower | one character or one filled cell |
| **ASCII** | `--plain`, pipes, `--colour never` | the `ASCII` section, uncoloured | one character |

Half-block cells, with *U* the upper pixel and *L* the lower:

| *U* | *L* | Glyph | Foreground | Background |
|---|---|---|---|---|
| transparent | transparent | space | — | terminal's own |
| tone | transparent | `▀` (CP437 `DF`) | *U* | terminal's own |
| transparent | tone | `▄` (CP437 `DC`) | *L* | terminal's own |
| tone | same tone | `█` (CP437 `DB`) | *U* | terminal's own |
| tone | other tone | `▀` | *U* | *L* |

**Depth.** Each palette index becomes a tone, and the tone becomes a colour from the generated table: `rgb` at truecolour, `xterm256` at 256, `ansi16` for SGR at 16, `console16` for console attributes at 16, `mono` for bold, plain or faint. A tone with no `console16` (and so no `ansi16`) or no `mono` is drawn as transparent at that depth. That one rule is how a tone that only works with many colours (a dark glow, say) drops out at 16.

**`PIXELS_LOW`.** When dropping or merging tones is not enough, because the picture at 16 colours should be a different drawing, a logo carries a second picture. The generator makes it from `art/masters/<logo>-low.svg` when that file exists, and the core uses it at 16 colours, in mono and for the cells form of a logo with no ASCII form. No logo needs one: `macos`, the one whose brief allowed for it, still reads at 16 colours with amber and peach merged into orange.

## Size budget

`art/logos.toml` sets the budgets the generator enforces: **700 bytes** per blob and **6,144 bytes** (6 KiB) for all blobs together, which leaves room for eight logos at the per-blob limit. 7 KiB is the ceiling if the budget ever has to grow (AF-D40). Measured against this specification:

| Logo | Header | `PIXELS` | `ASCII` | Total |
|---|---|---|---|---|
| `afterglow` | 15 | 3 + 82 | 3 + 75 | 178 |
| `dos` | 13 | 3 + 80 | 3 + 58 | 157 |
| `win16` | 16 | 3 + 146 | 3 + 192 | 360 |
| `win32` | 17 | 3 + 116 | 3 + 189 | 328 |
| `linux` | 15 | 3 + 92 | 3 + 85 | 198 |
| `freebsd` | 13 | 3 + 79 | 3 + 81 | 179 |
| `macos` | 16 | 3 + 89 | 3 + 121 | 232 |
| `desk` | 18 | 3 + 165 | 3 + 205 | 394 |
| All eight | | | | 2,026 |

The ASCII form usually costs more than the picture: text has more distinct runs than stepped bands. `make art` prints each blob's size as it writes it, and `art/scripts/review_sheet.py` prints the same figures as estimates while a master is being drawn.

## Validation

A reader rejects the blob, without reading past its end, when:

- the magic is not `AFL1`, or the blob is shorter than its header;
- a header field is outside its range, or the palette repeats a tone or names tone 0 or a tone above 15;
- a section's kind is `0x00`, a section's length runs past the end, the sections are out of order, a kind repeats, `PIXELS` is missing, `ASCII` is missing while `ascii_width` > 0 (or present while it is 0), or a kind `0x80` or above appears;
- a pixel row's runs do not add up to exactly `width`, or a section holds bytes after its last row;
- the `ASCII` section holds fewer or more than `ascii_rows` rows, or an ASCII row's art decodes to more than `ascii_width` characters, holds a byte outside `0x20` to `0x7E` and `0x80` to `0xFF`, starts with a repeat, or its tone runs do not add up to its length;
- a palette index is above `palette_len`, or a space and its index disagree (a drawn character with index 0 or a space with a non-zero index).

The reader validates the whole blob once when the logo is opened, so drawing never fails part-way through a line.

## Example

A 4 × 2 logo with one tone (gold, id 1): a two-pixel bar in the middle of the upper row, and a one-row ASCII form `.##.`, all gold. The whole blob is 26 bytes.

```text
41 46 4C 31          magic AFL1
04 02 04 01          width 4, height 2, ASCII form 4 x 1
01 01                palette_len 1; index 1 is tone 1 (gold)
01 04 00             PIXELS, 4 bytes
   00 11 00          row 0: 1 transparent, 2 of index 1, 1 transparent
   30                row 1: 4 transparent
03 06 00             ASCII, 6 bytes
   04                row 0: 4 bytes of art
   2E 23 80 2E       ".", "#", repeat once more, "." -> ".##."
   31                tones: 4 characters of index 1
```

## Compatibility

`AFL1` is experimental, like the project: until the first release, a change may alter it in place, and the generator, the core's decoder and the committed blobs change together in one commit with `make art-check` clean. After the first release, an addition a reader may ignore is a new section kind below `0x80`; anything else is `AFL2`, and the core reads both until `AFL1` blobs are gone.
