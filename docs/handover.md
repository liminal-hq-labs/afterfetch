# Hand-over to reactos-liminal and to the organization

What has to change in `liminal-hq-labs/reactos-liminal` to use this afterfetch, and what the move of this repository into the `liminal-hq-labs` organization needs. Nothing here has been done: `reactos-liminal` has not been edited and no remote repository exists (AF-D2). Each choice that is the owner's is an **OWNER DECISION**, with the default this document assumes.

## Table of contents

- [What reactos-liminal has today](#what-reactos-liminal-has-today)
- [What reactos-liminal must change](#what-reactos-liminal-must-change)
- [How the Win32 binary is built and verified](#how-the-win32-binary-is-built-and-verified)
- [What the move into the organization needs](#what-the-move-into-the-organization-needs)
- [Owner decisions](#owner-decisions)

## What reactos-liminal has today

- `tools/afterfetch/`, its own Rust project (about 8.5 KB, `no_std`, a Pentium target, ANSI imports on `imports.allow`) with `build.sh [OUT_DIR]` writing `dist/afterfetch.exe`, `check.sh [--reproducible]`, and its own `rust-toolchain.toml` (1.98.0 with `rust-src` and `llvm-tools`).
- `recipes/afterglow.toml`: `[builds.afterfetch]` with `source = "../tools/afterfetch"` and a pinned `sha256` (`63d628cb…`), then `[[files]]` entries: `built:afterfetch` to `ReactOS/system32/afterfetch.exe`, `guest/cmd/afterfetch.cmd` (the batch fallback) and four one-line shims, `neofetch.cmd`, `fastfetch.cmd`, `screenfetch.cmd` and `winfetch.cmd`, each `afterfetch.exe %*`. `recipes/afterglow-compact.toml` repeats the entries; both lock files (`recipes/*.lock.json`) carry the build's hash and the file hashes.
- `reactos_liminal/recipe.py` (the `builds` schema), `reactos_liminal/doctor.py` (checks that the pinned toolchain in `tools/afterfetch` has `rust-src` and `rust-lld`), `.github/workflows/ci.yml` (job `afterfetch`, in the shared `ci-rust` image, running `bash check.sh --reproducible` in `tools/afterfetch`), `.gitignore` (`/tools/afterfetch/target/`, `dist/`), and documents that describe it: `docs/afterfetch.md`, `docs/pipeline.md`, `docs/recipes.md`, `docs/decisions.md`, `SPEC.md`, `README.md`, `AGENTS.md`, `CLAUDE.md`.

## What reactos-liminal must change

1. **Where the program comes from. OWNER DECISION** (default: option A). The afterfetch repository is outside `reactos-liminal`, so `[builds.afterfetch] source` cannot be a relative folder any more.
   - **A. A pinned checkout, built from source** (like the `afterglow` input: a repository, a commit and the tarball's SHA-256). Add an `[inputs.afterfetch]` table with `repo = "liminal-hq-labs/afterfetch"`, `commit`, and `tar_sha256`, extend the `inputs` schema in `reactos_liminal/recipe.py`, and point `[builds.afterfetch] source` at the checkout's root with the script below. The build stays from source and reproducible, as today.
   - **B. A pinned release asset.** Add an input with the asset's URL and SHA-256 (`https://github.com/liminal-hq-labs/afterfetch/releases/download/v0.1.0/afterfetch-0.1.0-windows-i586.exe`, as `reactos_iso` is pinned) and write it to the image with `[[files]]`. Nothing is built in `reactos-liminal`, so `rustup` and the toolchain check go away; it needs the release to exist first (below).
   - **C. Vendor the subset.** Copy `platforms/win32`, `crates/afterfetch-core` and `crates/afterfetch-art` with a `PROVENANCE.md`. It keeps the build in place and duplicates code, which AF-D48's committed `crates` link was made to avoid.
2. **The build command (option A).** `scripts/reactos-build.sh [OUT_DIR]` in this repository builds the Pentium Win32 program and writes `OUT_DIR/afterfetch.exe`, the same call shape and file name as `tools/afterfetch/build.sh`, so only the path changes. It runs `platforms/win32/build.sh i586` and honours `CARGO_TARGET_DIR`.
3. **Re-measure the pin.** `./rl pin` writes the new `[builds.afterfetch] sha256` and refreshes `recipes/*.lock.json`. The build is byte-identical across directories (AF-D48), so the pin is stable: at the current `main` the i586 program is 24,576 bytes with SHA-256 `0d76a501221493eedca604ca1531992ebc598abcbdeb60b850ba41897e4eda3f` (`scripts/repro-win32.sh`). It grew from about 8.5 KB because it now reads a CPU row and the 9x kernel name, carries all eight logos and draws in VT colour on the 64-bit builds.
4. **The toolchain check.** `doctor.py`'s check reads `tools/afterfetch`; it must read the checkout (option A) or be dropped (option B). The pinned toolchain is this repository's root `rust-toolchain.toml` (Rust 1.98.0 with `rustfmt`, `clippy`, `rust-src`, `llvm-tools`, and the many targets, of which the Win32 build needs none installed: its target is a JSON file and `core` is built from source).
5. **The CI job.** Replace `working-directory: tools/afterfetch` by a checkout of the pinned afterfetch commit (`actions/checkout` with `repository` and `ref`), then run `bash platforms/win32/build.sh i586`, `bash platforms/win32/check.sh i586` and `bash scripts/repro-win32.sh i586`. The shellcheck line that lists `tools/afterfetch/*.sh` goes. Remove `tools/afterfetch/` and its two `.gitignore` lines once the image builds from the new source.
6. **The shims. OWNER DECISION** (default: drop `%*` from the four shims). The old program ignored its arguments, so `neofetch --plain` printed the same picture. The new one parses them (`--plain`, `--json`, `/NOLOGO`, ...) and exits 2 with a message on an option it does not know, so a habit such as `neofetch --off` now errors. Keep `%*` on `afterfetch.cmd` only if it is wanted there.
7. **The output the image prints** gains a `CPU:` row (and a `Host:` row where Windows says one), and the title rule takes the architecture accent. The ReactOS golden is still byte-equal for the picture: the core's test `plain_win32_logo_matches_the_reactos_liminal_output` holds `--logo win32 --plain` to the text this repository's ReactOS image printed. Update the sample in `docs/afterfetch.md` and the `Display:` and `Memory:` descriptions (memory is read with `GlobalMemoryStatusEx` where present, so a machine over 4 GB is no longer capped).
8. **The version.** `afterfetch --version` prints `afterfetch 0.1.0 (windows-i586)`; record it where `docs/afterfetch.md` says "about 8.5 KB" and in the guest test, if it checks the first line.
9. **Documents.** `docs/afterfetch.md` (the program, colours, rows, "how it reaches the image", "other names"), `docs/pipeline.md`, `docs/recipes.md`, `docs/decisions.md`, `SPEC.md`, `README.md`, `AGENTS.md` and `CLAUDE.md` name `tools/afterfetch`; each follows the change. The batch fallback, `guest/cmd/afterfetch.cmd`, does not change.
10. **Do not** edit this repository's Win32 shell to suit the image: its behaviour is in `SPEC.md`, and a difference is a decision here (`docs/decisions.md`).

## How the Win32 binary is built and verified

- **Built:** `platforms/win32/build.sh i586` (Rust 1.98.0, `core` from source for a Pentium with MMX, SSE and x87 off, `rust-lld` as `lld-link`, `/SUBSYSTEM:CONSOLE,4.0 /FIXED`, no C runtime, ANSI imports only, `panic=immediate-abort`). The 64-bit builds are `x86_64` and `aarch64` (a ReactOS image is x86, so it needs only i586).
- **Verified here:** `platforms/win32/check.sh i586` (PE structure, imports against `imports.allow`, no instruction a Pentium lacks, 24 KiB budget with no spare bytes), `scripts/repro-win32.sh` (three directories, byte for byte), the host tests of `platforms/win32/src` (`make test-win32`), and `scripts/smoke/wine.sh` (the shape of `--plain`, `--json`, the exit codes and a pseudo-terminal colour run under Wine).
- **Verified in reactos-liminal:** the ReactOS guest is the real-console proof. The core's goldens hold the picture; the guest run holds the real facts and the console attributes. `make smoke-wine` does not replace it, and Windows 95 and a real Windows 10 console are still unverified here.

## What the move into the organization needs

Nothing below has been done. **Do not create or push a remote until the owner decides** (AF-D2).

**Repository.** Create `liminal-hq-labs/afterfetch` (public, from the Labs template if the owner uses one), add it as `origin`, push `main` with the history. The default branch is `main`, merges are merge commits (AGENTS.md), branches are deleted on merge. The repository's Labs status and topics follow `labs-charter.md` in `liminal-hq-labs/.github`. Dependabot stays off. No secrets are needed by any job below.

**CI** (`.github/workflows/ci.yml`, one job per check, shared images from `liminal-hq/.github` pinned by digest). Today it has the placeholder set: Markdown lint, licence headers, `fmt`, `clippy`, tests, `cargo deny`, and the `macos` job (build, structure check, run both architectures, reproducibility). The full set `make check` runs locally, as jobs:

- **Unix builds and structure checks** for every target of `make build-unix check-unix` (Linux x86_64, i586, ARM64, ARMv7, ARMv6, MIPS, MIPSel, RISC-V 64, PowerPC 64 LE, FreeBSD x86_64 and ARM64): the MIPS builds need `-Zbuild-std` and `RUSTC_BOOTSTRAP=1`, which `build.sh` sets. A matrix job on the `ci-rust` image, or `ci-cross` if the owner proposes it (AF-D18).
- **Linux smokes** (`scripts/smoke/linux-native.sh`): native x86_64 and i586 on the runner, the rest under `qemu-user-static` (`apt-get install qemu-user-static` on the runner; not in the container image).
- **FreeBSD** (`scripts/smoke/freebsd-serial.py` boots the official VM image; hosted runners have no reliable KVM, so use TCG with a longer timeout, or `vmactions/freebsd-vm`), as one job.
- **Win32** (`make fmt-win32 clippy-win32 test-win32 check-win32 repro-win32`), **Wine smokes** (`scripts/smoke/wine.sh`, `wine16.sh`; Wine from the distribution's package), and a **Windows ARM64** job on `windows-11-arm` that builds `aarch64` and runs it (AF-D5, never run yet).
- **DOS** (`make check-dos smoke-dos repro-dos`; `tools/fetch-tools.sh` downloads DOSBox Staging and CWSDPMI with checked sums), **Windows 3.1** (`check-win16 smoke-wine16 repro-win16`), **Desk** (`fmt-desk clippy-desk test-desk check-desk repro-desk`). `make smoke-w31` needs Docker, a DOSBox-X image and a Windows 3.11 tree, which are not in CI: it stays a local, once-per-release run.
- **Art** (`make art-check test-art`; needs `rsvg-convert`) and `make test` for the host crates and the preview tool.

**Releases** (`.github/workflows/release.yml`). Like every Liminal HQ product: it runs on a pushed tag `v*` (and `workflow_dispatch` for a dry run), has `permissions: contents: read` at the top and `contents: write` only in the publish job, builds every target in parallel jobs, and has **no protected environment** and no approval gate. The publish job downloads the artifacts, writes `SHA256SUMS`, and creates the GitHub Release with `gh release create` and the generated notes. File names follow SPEC section 12: `afterfetch-<version>-<target>` for Unix, the same with `.exe` for Win32 (`afterfetch-0.1.0-windows-i586.exe`), `AFETCH.EXE` and `AFETCHW.EXE` in `afterfetch-<version>-dos-i386.zip` with the licence files, `AFETCH.DMD` with the note that Desk has no loader, and the preview contact sheet. Nothing tags, publishes or dispatches a workflow until the owner asks; the first tag is `v0.1.0`. The macOS programs are ad-hoc signed by the linker and not notarized, the Windows programs are unsigned (SmartScreen may warn), and the release notes say so.

**Also.** Move `docs/` text that says "no remote" (AGENTS.md, `docs/landing.md`) to the pull-request flow, with the PR title and body formats in AGENTS.md. Add the project to the organization's index (`liminal-hq-labs.github.io`). The `ci-cross` and `ci-art` image proposals, if wanted, are written for `liminal-hq/.github`, not here.

## Owner decisions

| Question | Default taken | Where to swap |
|---|---|---|
| How does reactos-liminal get the program: a pinned checkout, a release asset, or a vendored copy? | A pinned checkout built from source (`scripts/reactos-build.sh`) | Section 1 above; `[inputs.afterfetch]` and `[builds.afterfetch]` in the recipes |
| Do the `neofetch`, `fastfetch`, `screenfetch` and `winfetch` shims keep `%*`? | No: the new program errors on an option it does not know | `guest/cmd/*.cmd` in reactos-liminal |
| Create the `liminal-hq-labs/afterfetch` repository and push? | No: nothing is created | The owner, AF-D2 |
| Add the full CI job set and the release workflow now or after the first push? | After the repository exists; this document lists them | `.github/workflows/` |
| First version and tag | 0.1.0, `v0.1.0`, not tagged | The owner |
