# Art: one sun, many horizons

Every afterfetch logo is the same Afterglow sun. What changes from system to system is the horizon it sits on, the time of day and the way it is drawn, so a row of logos reads as one family, and none borrows a mark that belongs to someone else. This page is the design language; [logo-format.md](logo-format.md) is the blob the art becomes, `art/palette/afterglow-dusk.toml` holds every colour and `art/logos.toml` every logo.

## Contents

- [The constants](#the-constants)
- [The grid](#the-grid)
- [A day around the family](#a-day-around-the-family)
- [Palette](#palette)
- [The sun at each depth](#the-sun-at-each-depth)
- [Forms](#forms)
- [The drawn logos](#the-drawn-logos)
- [Briefs for the other logos](#briefs-for-the-other-logos)
- [Reviewing a master](#reviewing-a-master)

## The constants

Some things never change, so the family holds together however different the horizons are.

- **One sun.** A disc, half set or rising, centred on the logo's middle column, cut flat by its horizon. Its bands always run from the crown down: gold `#ffd08e`, then orange `#ffaa40`, then coral `#f995a6` at the horizon, with amber `#ffbd67` and peach `#fca073` between them where there are colours to spare.
- **One rose line.** The horizon is a rose `#f43f5e` line one pixel high. It is the only rose in most logos, and it is in every one.
- **Stepped, never smooth.** Gradients are flat bands with hard edges, as in Afterglow's stepped wallpapers and the boot logo. There is no dithering and no antialiasing: a terminal cell cannot show either.
- **Light from the upper left.** Any highlight (a lit cap, a bevel, a glint) sits up and to the left, as on the Afterglow Start orb.
- **Transparent sky.** The background is the terminal's own. A logo draws only what it means, so it sits on a black, navy or void terminal alike.
- **No text and no marks.** No letters, no wordmark, no penguin, daemon, apple, window flag, or any shape taken from a system's logo. A system is told by its time of day and horizon, not by its trademark.

```text
          .  \ | /  .          sky: sparks, rays, stars (per logo)
           .:######:.          gold    crown
         :############:        amber   (truecolour and 256 only)
        :##############:       orange  body
        :##############:       peach   (truecolour and 256 only)
       :################:      coral   foot, at the horizon
  ----------------------------  rose    horizon, one pixel
        ~~~~   ~~~~~~           below the line: per logo (water, ridge, tiles, nothing)
```

## The grid

A logo is pixel art on a fixed grid, so the generator can turn a master into cells with no judgement of its own.

- **36 × 24 logo pixels** at most: 36 columns and 12 rows of terminal cells, each cell an upper and a lower pixel (a half block). A logo, a gap of two and 34 columns of facts then fit 80 columns.
- **Masters are 360 × 240 units**, 10 units to the pixel, drawn inside one `scale(10)` group so every coordinate in the file is a whole pixel. Shapes are rectilinear paths on pixel edges (a disc is its stepped outline, sampled at pixel centres); `shape-rendering="crispEdges"` keeps the renderer from blending at the edges, so `rsvg-convert --width 36 --height 24` gives exactly one tone per pixel.
- **Fills are tone colours only**, written as the tone's `rgb` from the palette. No strokes, gradients, opacity, filters, text or embedded images. Anything else is transparent.
- **Think in cells.** A horizontal line one pixel high on an even row is the upper half of a cell (`▀`) and on an odd row the lower half (`▄`). Thin features (the horizon, ripples, rays) sit on purpose on one half; the sun's bands may change colour mid-cell, which the half-block form shows as two colours in one cell.
- **Centre.** A logo with a one-pixel feature on its axis (a ray) uses odd widths centred on pixel column 17; one without may use even widths centred on the line between columns 17 and 18.
- **Hand-authored ASCII.** Every logo also has an ASCII form, `art/ascii/<logo>.txt`, at most 36 × 12, drawn by hand: automatic ASCII from 36 pixels looks poor. The file alternates an art line and a tone line of the same length; a tone line names each character's tone by its letter, and a space in the art is a space in the tones. `#` is the sun's body and is drawn as a filled cell on consoles.

## A day around the family

The logos walk through one day, so each system gets a light of its own and the set reads as a sequence.

| Logo | Systems | Time of day | Horizon and drawing |
|---|---|---|---|
| `dos` | MS-DOS, PC DOS, FreeDOS, DR-DOS | dawn | the sun rising through CRT scanlines, chunky 2 × 1 pixels, only the 16 VGA colours |
| `win16` | Windows 3.0, 3.1, 3.11 | morning | a bevelled sun, raised like a button, over a grid of tiles |
| `macos` | macOS | golden hour | a soft glass sun over still water, one reflection |
| `afterglow` | anything else | sunset | the brand sun half set on a bare rose line |
| `win32` | Windows 95 to 11, ReactOS | dusk | the sun over water, ripples fading from lilac to slate |
| `freebsd` | FreeBSD | last light | a deep ember sun behind a low ridge |
| `linux` | Linux | night | the sun set onto a shell-prompt horizon under a violet, starry sky |
| `desk` | Desk OS | after dark | the sun wearing Desk's stepped halo on the Void, over its stepped horizon |

## Palette

Every colour a logo may use is a **tone** in `art/palette/afterglow-dusk.toml`: a stable id (1 to 15; 0 is transparent), a letter for ASCII tone lines, the design colour, and what the tone shows at each depth. The colours come from Afterglow (`liminal-hq/afterglow-themes` at `84a8d5c`): the brand tokens for the sun and horizon, and the Dusk console table that the ReactOS image writes to `HKCU\Console` for the rest.

| Id | Tone | Letter | Colour | Console | ANSI | 256 | Mono | Role |
|---|---|---|---|---|---|---|---|---|
| 1 | gold | `g` | `#ffd08e` | 14 | 11 | 222 | bright | sun's crown |
| 2 | amber | `a` | `#ffbd67` | 6 | 3 | 221 | bright | crown to body |
| 3 | orange | `o` | `#ffaa40` | 6 | 3 | 215 | bright | sun's body |
| 4 | peach | `h` | `#fca073` | 6 | 3 | 216 | normal | body to foot |
| 5 | coral | `c` | `#f995a6` | 12 | 9 | 211 | normal | sun at the horizon |
| 6 | rose | `r` | `#f43f5e` | 4 | 1 | 203 | normal | the horizon line |
| 7 | lilac | `l` | `#e4b8f8` | 13 | 13 | 183 | normal | stars, near water |
| 8 | violet | `v` | `#c98be6` | 5 | 5 | 176 | normal | mid water, night sky |
| 9 | periwinkle | `p` | `#7f95f2` | 1 | 4 | 104 | dim | deep water |
| 10 | slate | `s` | `#7c8796` | 8 | 8 | 102 | dim | deepest water |
| 11 | cream | `w` | `#fff7ea` | 15 | 15 | 231 | bright | highlights |
| 12 | night | `n` | `#4a4580` | 8 | 8 | 61 | dim | silhouettes |
| 13 | teal | `t` | `#62c6d6` | 3 | 6 | 116 | normal | RISC-V accent |
| 14 | umber | `u` | `#7d5422` | 6 | 3 | 94 | dim | Desk halo, middle |
| 15 | soot | `d` | `#372612` | — | — | 235 | — | Desk halo, outer |

- **Console** is the attribute index on Win32 consoles, DOS and Windows 3.1 at 16 colours. The eight tones of the reactos-liminal afterfetch keep its indexes exactly (gold 14, orange 6, coral 12, rose 4, lilac 13, violet 5, periwinkle 1, slate 8), so on the ReactOS image every one is its Afterglow colour with ΔE 0, and the generator's test holds them there.
- **ANSI** is the same colour in SGR order, which swaps red and blue: it is always the console index with bits 0 and 2 exchanged. On a terminal using the Afterglow terminal palette those slots are Afterglow's own colours (bright red is coral exactly, red is rose exactly).
- **256** is the xterm cube or grey ramp, never 0 to 15, which belong to the user's theme. Most are the nearest entry; amber (221, ΔE 18.7) is not, because its nearest, 215, is orange's, and five distinct sun bands matter more than the closest match.
- **Night** has no honest 16-colour match: the nearest console colour is the background. It falls back to slate by role. **Soot** has no 16-colour entry at all and is dropped (drawn transparent) at 16 colours and in mono. **Umber** and **soot** are provisional until the `desk` master is drawn.
- **Mono** is bold, plain or faint on Unix and the intensity bit on DOS and MDA: the crown and body are bright, so a green-screen still shows a sun.

The fact labels walk down the same sunset (gold, orange, coral, rose, lilac, violet, periwinkle), as they do in the ReactOS build, and the title rule takes the architecture's accent (`[arch-accent]` in the palette: x86 gold, ARM periwinkle, MIPS violet, RISC-V teal, POWER lilac).

## The sun at each depth

The sun's gradient has five bands where the terminal has the colours, and three where it has 16.

| Depth | Crown | | Body | | Foot | Horizon |
|---|---|---|---|---|---|---|
| Truecolour | gold `#ffd08e` | amber `#ffbd67` | orange `#ffaa40` | peach `#fca073` | coral `#f995a6` | rose `#f43f5e` |
| 256 | 222 `#ffd787` | 221 `#ffd75f` | 215 `#ffaf5f` | 216 `#ffaf87` | 211 `#ff87af` | 203 `#ff5f5f` |
| 16, Dusk console | 14 gold | 6 orange | 6 orange | 6 orange | 12 coral | 4 rose |
| 16, Afterglow terminal | bright yellow `#fddc87` | yellow `#fbbf24` | yellow | yellow | bright red `#f995a6` | red `#f43f5e` |
| 16, stock VGA | yellow `#ffff55` | brown `#aa5500` | brown | brown | light red `#ff5555` | red `#aa0000` |
| Mono | bright | bright | bright | plain | plain | plain |

At 16 colours amber and peach both become orange: the crown stays gold, the middle of the disc is one orange body and the foot is coral, which is the Afterglow boot logo's proportion (an orange sun with a pale gold cap). Amber is a near tie between gold (ΔE 15.5) and orange (ΔE 15.3); peach is nearer gold by Lab distance (24.6) than orange (27.8), but a gold band below orange would turn the sun upside down, so the 16-colour column follows the band order, not the nearest match.

**Legibility at 16 colours.** On the Dusk console (navy background) every sun and horizon tone holds at least 5:1: gold 12.8, orange 9.7, coral 8.6, rose 5.0. Neighbouring bands are far apart (gold to orange ΔE 30.8, orange to coral 59.5, coral to rose 38.4), so the steps read even in a small raster font. On a stock console the sun still reads as a sunset (yellow, brown, light red), but its edges weaken: rose becomes dark red (2.7:1 on black under VGA, 1.9:1 under the legacy Windows table), and the deep water fades almost to black (periwinkle 1.6:1). That suits the water, which should fade; the horizon is the cost of not owning the palette, and VT truecolour on Windows 10 and later avoids it.

**ΔE notes.** Every tone's distances are in the palette file: to its 256 entry, to its console entry under the Dusk console, the Afterglow terminal, the stock Windows console and stock VGA tables, and its contrast on navy and black. The stock tables are there to show what a user without Afterglow sees, not to be matched.

## Forms

The same blob draws three ways (details in [logo-format.md](logo-format.md#depth-forms)):

- **Half block** — two pixels per cell with `▀ ▄ █`, at any depth. The main form on Unix UTF-8 terminals, DOS (CP437) and Win32 in VT mode.
- **Cells** — the ASCII form with every `#` drawn as a filled cell. The form for legacy Win32 consoles, where a glyph cannot be trusted; on the ReactOS image it is exactly what the reactos-liminal afterfetch draws.
- **ASCII** — the ASCII form as plain text, for pipes, `--plain` and `--colour never`.

## The drawn logos

### `afterglow`

The mark itself, for any system without a logo of its own: the brand sun half set on a bare rose line, nothing else. The disc has radius 12 and is centred on the horizon at row 18, so it is 24 pixels across at its foot and 12 high, in five bands of three, two, two, two and three rows; the line runs 34 pixels. The empty rows above and below keep the sun level with the middle of the facts.

```text
            .:########:.
          .:############:.
        :##################:
       :####################:
      :######################:
      :######################:
 ----------------------------------
```

### `win32`

The reactos-liminal sunset, the picture the ReactOS image already shows, redrawn as a master at twice its vertical resolution: each text row is two pixel rows. Lilac sparks and three gold rays above; a disc of radius 9.5 on the horizon at row 14, gold for five rows, orange for two and coral for two; short rose bars either side of its foot; the rose horizon across 35 pixels; then four rows of ripples on even rows, so each is a thin upper half-cell, fading from lilac and violet through periwinkle to slate. The picture is centred on column 17, where the text sits a few columns right of its own sun; nothing else moved. Its ASCII form is the reactos-liminal text byte for byte, 40 columns wide, the one exception to the 36-column limit, so the ReactOS console shows exactly what it does today.

```text
                 .   .   .
           .      \  :  /      .
               .   \ : /   .
                .:########:.
            .:##############:.
           :##################:
       ====:##################:====
      ----------------------------------
       ~~~~  ~~~~~~~~~  ~~~~~~~  ~~~~
         ~~~~~~~  ~~~~~~~~~~  ~~~~~~
            ~~~~~~~~~~  ~~~~~~~~~
               ~~~~~~~~~~~~~~
```

## Briefs for the other logos

These are drawn in AF-WP16, on the same grid and constants. Each brief says what to draw; a brief may change when the master shows it should.

**`dos` — dawn.** The sun rising, with only its upper half or less above the line, drawn in chunky square pixels two columns wide and one row high (one half-block cell is two pixels tall, so each chunk is a 2 × 1 block that looks square in a terminal). Horizontal CRT scanlines cut across the disc: every other pixel row of the sun is left out from about halfway up, so the sun dissolves into stripes towards the crown, which says text-mode monitor without any logo. Only gold, orange, coral and rose, the tones that land on the standard VGA set honestly; the sky stays empty. The ASCII form uses `=` for scanlines and `#` for the disc.

**`win16` — morning.** A full disc sitting just on the horizon, drawn as a raised 3D button: a one-pixel cream highlight on its upper-left rim and a one-pixel slate shadow on its lower-right rim over the gold, orange and coral bands, the way Windows 3.1 bevels a push button. Below the line, two rows of small square tiles (night with a cream top-left pixel) recall a Program Manager group, with no icons in them. The Windows 3.1 program draws each pixel as a 4 × 4 square in a window, so this logo can use single-pixel detail that would be lost in a terminal.

**`linux` — night.** The sun almost set: only its upper third shows above the line, in gold and orange only. The rose horizon line ends on the right in a solid cream block, a cursor, with a short `>` shape of two rose pixels at its left end, so the horizon is a shell prompt. Above, a violet night sky as a few sparse violet and lilac pixels (two or three stars, one cream), no moon, no animal.

**`freebsd` — last light.** A deeper sun: coral and rose make up most of the disc, gold only a small core at the crown, nothing amber or peach, so it reads red-orange at every depth. A low ridge of night tone crosses in front of the lower part of the disc, its silhouette a gentle two-step rise, with the rose line along its foot. The red recalls the platform's colours without taking any shape from its marks: no horns, no ball, no trident.

**`macos` — golden hour.** A soft sun just above the line using every gradient step (gold, amber, orange, peach, coral), with a pale cream crescent on its upper left as a specular glint, as if the disc were glass. Below the rose line, a single soft reflection: a shorter, narrower echo of the lower bands, broken into two or three spans. At 256 colours and truecolour it uses all five bands; at 16 it falls back to a plain sun through a `PIXELS_LOW` picture rather than a muddy collapse. No apple, no window controls, no traffic lights.

**`desk` — after dark.** The sun on the Void (the terminal background, Desk's `#050507`), wearing Desk's stepped halo: three one-pixel rings around the disc in orange, umber and soot, the focused-window border of Desk's first window. Below it, Desk's stepped Horizon as three flat bands, soot, umber and orange from bottom to top, instead of a gradient, with the rose horizon line on top. Drawn only from Desk's 16-colour set plus rose, so it is the same at 4-bit and 8-bit and in Desk's own palette; no Desk wordmark and no window chrome. The umber and soot tones are reserved for it; at 16 console colours umber becomes orange and soot drops out, so the halo thins to its brightest ring, and the master decides whether that needs a `PIXELS_LOW` picture.

## Reviewing a master

`python3 art/scripts/review_sheet.py` renders every drawn master into `~/Documents/Liminal HQ/Afterfetch/renders/`: `<logo>-<depth>.png` at 10 pixels per logo pixel for truecolour, 256, the Dusk console, the Afterglow terminal, stock VGA and mono, each on the background that kind of terminal has, and `afterfetch-logos.png`, a sheet of every logo at every depth beside its ASCII form. It fails on any fill that is not a tone. The cell-accurate previews and the blobs come from the generator, `art/scripts/afterfetch_art.py`.

Before a master lands, check that:

- every fill is a tone in the logo's `tones` list in `art/logos.toml`, and every coordinate is a whole pixel;
- `rsvg-convert --width 36 --height 24` gives only those tones and transparent (no blended edge pixels);
- the sun is recognisable at 16 colours and in mono, and the horizon line is visible on navy and on black;
- the ASCII form is at most 36 × 12, ASCII only, with a tone on every drawn character and none on a space;
- nothing in it resembles a system's trademark.
