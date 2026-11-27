# AGENTS.md

## Table of contents

- [Project status](#project-status)
- [Spelling and writing](#spelling-and-writing)
- [Markdown formatting](#markdown-formatting)
- [Authoring voice](#authoring-voice)
- [Commit messages](#commit-messages)
- [Pull requests](#pull-requests)
- [Git workflow](#git-workflow)
- [Areas and landing](#areas-and-landing)
- [Local tooling](#local-tooling)
- [Licence headers](#licence-headers)
- [Architecture rules](#architecture-rules)
- [Testing](#testing)
- [Documentation](#documentation)
- [Repository layout](#repository-layout)

## Project status

Afterfetch is a [Liminal HQ Labs](https://github.com/liminal-hq-labs) experiment: A tiny system-information command with an Afterglow sun for every system, from DOS to macOS. It is **experimental**, so interfaces, file formats and behaviour may change between any two commits. `SPEC.md` is the source of truth for behaviour, and `docs/decisions.md` records why. The [Labs charter](https://github.com/liminal-hq-labs/.github/blob/main/docs/labs-charter.md) covers what that status means and how a project graduates.

## Spelling and writing

**REQUIREMENT:** Use **Canadian English** in UI strings, comments, commit messages, pull request text and documentation: `colour`, `centre`, `behaviour`, `cancelled`, `licence` (noun) and `license` (verb), and `-ize` endings such as `customize`. Keep American spellings where an API, crate, CSS property or protocol requires them (`color`, `background-color`).

- Use a real em dash (`—`) in prose, never `--`. A command-line flag such as `--check` is not prose.
- Sentence case in the UI and in headings. Title Case for window titles.
- Write tight. No padding, no restating the heading.

## Markdown formatting

**REQUIREMENT:** Do not hard-wrap markdown prose. Write each paragraph or bullet as one line and let the renderer reflow it. This applies to `README.md`, `SPEC.md`, `docs/` and pull request descriptions. Commit message bodies are the one exception: hard-wrap them at roughly 72 characters. Code comments wrap normally.

## Authoring voice

**REQUIREMENT:** Ship the result, not how it was arrived at. Write comments, identifiers, docs and pull request text as the author of the artifact, for the person who meets it later.

- No "this PR", reviewer names or commit hashes in comments or docs. State the fact or the reasoning directly.
- When a comment has been edited more than once, rewrite it as one clean explanation.
- Commit messages and a pull request's `## Test plan` are the exceptions: they are the right place to record why a change happened and how it was checked.

## Commit messages

**Format:** Conventional Commits (`feat`, `fix`, `docs`, `test`, `ci`, `build`, `chore`, `refactor`, `perf`, `style`), with an optional scope.

- The body explains what and why, not how. Use markdown with **bold labels** and no headings. Hard-wrap.
- Backtick every code-level reference (identifiers, paths, config keys, flags, crate names). User-facing strings use quotes.
- End with a `Co-Authored-By:` trailer for the model that wrote the change.
- Write the message to a file with a single-quoted heredoc and commit with `git commit -F <file>`, so backticks and `$()` are not expanded. Check the result with `git log -1 --pretty=fuller`.

## Pull requests

- **Title:** capitalized, imperative, about 70 characters, no Conventional Commit prefix, outcome-focused.
- **Body:** `## Summary` with flat bullets that open with a **bold lead-in**, then `## Test plan` with `- [x]` or `- [ ]` items naming the commands run. Finish with what was not verified.
- **Labels:** one of `enhancement`, `bug`, `documentation`, `testing`, `ci`, `build`, `chore`, plus scope labels where useful.
- Fix a bug with a test that fails before the fix, and state exactly what was and was not run.

## Git workflow

- `main` is the default branch. While there is no remote, work lands straight onto `main` (see [Areas and landing](#areas-and-landing)). Once pull requests exist, work on topic branches (`feat/`, `fix/issue-<n>-…`, `docs/`, `chore/`, `ci/`, `build/`) and merge with merge commits only: no squash, no rebase merges, branches deleted on merge.
- **Never** push, tag, publish or dispatch workflows unless asked. The repository has no remote yet.
- **Never** run `git add -A` or `git commit -a`, because checkouts may be shared. Stage explicit paths.
- Prefer the `gh` CLI for GitHub work.
- Planning files such as `PLAN.md` stay out of git.

## Areas and landing

[`docs/landing.md`](docs/landing.md) is canonical. In short:

- **One builder per area at a time.** Areas: `core` (`crates/afterfetch-core`), `art` (`art/`, `crates/afterfetch-art`), `unix`, `win32`, `dos`, `win16` and `repo` (root files, Makefile, scripts, docs, and the host-only `crates/afterfetch-preview`). Two work packages touching one area never run together.
- **Straight onto `main`**, with `make check` passing before every commit. One active builder works in the main checkout. Two or three at once each use a worktree at `~/source/worktrees/afterfetch/<wp-id>` on `wp/<wp-id>`, then `git rebase main` and `git merge --ff-only`, and remove the worktree and branch.
- Sign each commit with the agent's own model in `Co-Authored-By`.

## Local tooling

- **Editor settings:** `.editorconfig` is authoritative: tabs everywhere except YAML, which uses spaces.
- **Rust**, if the project uses it, is formatted with rustfmt using hard tabs (`rustfmt.toml`). Strictness comes from CI: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and the test suite.
- **Shared CI images** come from [`liminal-hq/.github`](https://github.com/liminal-hq/.github) (`ci-rust`, `ci-web`, `tauri-ci-desktop` and others). Pick the leanest tier that fits and pin it by digest. A project image belongs here only when no shared tier covers it.
- **Dependabot** is off. Add `.github/dependabot.yml` only when the project opts in (see `docs/new-project.md`).

- **Toolchain:** Rust 1.98.0 (`rust-toolchain.toml`, with the components and targets the platforms need), `cargo nextest`, `cargo deny`. Set `CARGO_BUILD_JOBS=4`.
- **The gate:** `make check` is the single local gate; `make check-fast` is fmt, clippy, tests and headers. Markdown, shell and workflow checks run in Docker.
- **Large files** (FreeBSD image, DOS vendor tools, scratch trees) live in `~/.cache/afterfetch/`, never in `/tmp`.

## Licence headers

Every source, script, YAML, TOML and SVG file starts with a one-line summary, a blank comment line and the copyright block:

```rust
// One-line summary of the file.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
```

Use `#` comments for shell, YAML, TOML and Dockerfiles, and an XML comment for SVG and `.ui` files. Markdown, JSON and lockfiles don't need one. `scripts/check-headers.sh` enforces it. The project is licensed `Apache-2.0 OR MIT`.

## Architecture rules

- **Core first.** `afterfetch-core` is `no_std`, allocates nothing, never panics on any input and knows nothing about a platform. A shell is an entry point, its system calls and two small trait implementations (`Platform` and `Terminal`).
- **No libc, no Lua.** Unix programs are `no_std` and `no_main` over raw syscalls, linking nothing (`libSystem` on macOS only, because it has no stable syscall ABI). Win32 imports ANSI names from an allow-list, with no C runtime. Nothing embeds a scripting language.
- **Size budgets** are enforced by `make check` through `scripts/check-sizes.sh` reading `sizes.toml` as each target lands, and a build over budget fails the gate: Win32 i586 24 KiB, Win32 x86_64 and ARM64 20 KiB, Linux x86_64 and i586 32 KiB, FreeBSD 24 KiB, macOS 64 KiB, DOS `AFETCH.EXE` 64 KiB, Windows 3.1 `AFETCHW.EXE` 24 KiB, Desk `AFETCH.DMD` 16 KiB. The art generator enforces the logo budget: 700 bytes per blob and 6 KiB for all together, with 7 KiB the ceiling if it ever has to grow (AF-D40).
- **Missing facts are omitted.** A row a system cannot honestly report is left out of the text, and its `--json` key stays with the value `null`; every JSON key is always present.
- **Art pipeline.** Every logo is a vector master in `art/masters/`, rendered down by script into the generated `.afl` blobs and palette tables in `crates/afterfetch-art`. Never hand-edit generated files; `make art-check` diffs them. ASCII forms are hand-authored. Logos are 36 × 12 cells, and ASCII forms at most 36 × 12 except `win32`'s 40 (AF-D41).
- **Copied runtimes** (DOS from the installer and Desk, Win16 from `rust-win16`, Win32 from reactos-liminal) carry a `PROVENANCE.md` naming the source commit. No ReactOS code is copied in before the Win32 port work package.
- **Platform shells are their own workspaces** (Win32, DOS, Windows 3.1), so build-std flags never reach the host workspace. They reach the core by path.
- **SPEC first.** A change to behaviour, interfaces, settings or defaults updates `SPEC.md` in the same change.

## Testing

- `make check` — the gate; `make check-fast` — fmt, clippy, tests, headers
- `bash scripts/check-headers.sh` — SPDX headers
- `bash scripts/check-banner.sh` — hero banner rules and text fit (needs `xmllint`, `rsvg-convert`, `python3` and Chromium; `--shots DIR` writes frames to look at)
- `docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"` — markdown
- `docker run --rm -v "$PWD:/mnt" -w /mnt koalaman/shellcheck:stable scripts/*.sh` — shell scripts
- `docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint` — workflows

State exactly what was and was not run when reporting results.

## Documentation

- Update `SPEC.md` when behaviour changes, and add or amend a `docs/decisions.md` entry when a decision is made. A change that alters behaviour without touching the doc that describes it is incomplete.
- `README.md` is for people who want to use or understand the project: short, scannable, no padding. Keep architecture and history out of it, and keep the `> **Status:**` line honest.
- `CLAUDE.md` is a summary of this file. Keep them in sync.

## Repository layout

```text
README.md SPEC.md AGENTS.md CLAUDE.md LICENSE-APACHE LICENSE-MIT
.editorconfig .markdownlint.jsonc .gitignore rustfmt.toml rust-toolchain.toml deny.toml Makefile Cargo.toml
crates/        afterfetch-core (portable core), afterfetch-art (generated logos and palette) and afterfetch-preview (host-only previews, never shipped)
art/           vector masters, ASCII forms, the palette, the logo manifest and the generator
platforms/     platform shells: unix (Linux today), win32 in progress
tests/         smoke label lists
assets/        hero banner
docs/          specification notes: decisions, platforms, art, logo format, how work lands
scripts/       header check, banner check, smoke tests and reproducibility checks
.github/       CI workflow, issue and pull request templates, release notes configuration
```
