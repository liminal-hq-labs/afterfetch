# CLAUDE.md

A summary of [AGENTS.md](AGENTS.md), which is canonical. Keep the two in sync.

## What this is

Afterfetch is a Liminal HQ Labs experiment: A tiny system-information command with an Afterglow sun for every system, from DOS to macOS. Experimental, so anything may change. Behaviour lives in `SPEC.md`, decisions in `docs/decisions.md`.

## Rules to remember

- **Canadian English.** Real em dashes. Sentence case. No hard-wrapped markdown prose. Ship the result, not the journey: no "this PR" or reviewer names in code or docs.
- **Conventional Commits.** Body in markdown with bold labels and no headings, hard-wrapped, written to a file and committed with `git commit -F`. End with a `Co-Authored-By` trailer.
- **Git.** No remote yet, so work lands straight onto `main` after `make check` (`docs/landing.md`); parallel builders use worktrees under `~/source/worktrees/afterfetch/<wp-id>` and fast-forward. Merge commits only resume with pull requests. Never push, tag, publish or dispatch workflows unless asked. Never `git add -A` or `git commit -a`. `PLAN.md` stays out of git.
- **Areas.** One builder per area at a time: `core`, `art`, `unix`, `win32`, `dos`, `win16`, `repo` (which holds the host-only `crates/afterfetch-preview`). Sign commits with your own model in `Co-Authored-By`.
- **Formatting.** Tabs everywhere except YAML. Rust uses rustfmt with hard tabs and a 130-column width (`rustfmt.toml`).
- **Headers.** SPDX header on every source, script, YAML, TOML and SVG file: summary line, blank, "(c) Copyright 2026 Liminal HQ, Scott Morris", `SPDX-License-Identifier: Apache-2.0 OR MIT`.
- **Architecture.** Core first: `afterfetch-core` is `no_std`, allocates nothing and never panics on any input; shells are an entry point, syscalls and the `Platform` and `Terminal` traits. No libc and no Lua (`libSystem` on macOS only). Missing facts are left out of the text; their JSON keys stay, as `null`.
- **Size budgets.** Win32 i586 24 KiB, Win32 64-bit 20 KiB, Linux x86_64 and i586 32 KiB, FreeBSD 24 KiB, macOS 64 KiB, DOS 64 KiB, Windows 3.1 24 KiB, Desk 16 KiB; logos 700 bytes each and 6 KiB together (7 KiB the ceiling, AF-D40); over budget fails `make check`.
- **Art.** Vector masters in `art/masters/`, rendered by script into generated blobs and palette tables; never hand-edit generated files. Logos are 36 × 12 cells, ASCII forms at most 36 × 12 except `win32`'s 40 (AF-D41). Copied runtimes carry a `PROVENANCE.md`.
- **CI.** One job per check. Shared images come from `liminal-hq/.github`. Dependabot is off unless the project opts in.
- **Docs.** Update `SPEC.md` and `docs/decisions.md` in the same change as the behaviour.

## Commands

```sh
make check                       # the gate before every commit
make check-fast                  # headers, fmt, clippy, tests
bash scripts/check-headers.sh
bash scripts/check-banner.sh     # add --shots /tmp/shots for review frames
docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"
```
