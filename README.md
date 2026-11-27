# Afterfetch

<p align="center"><img src="assets/hero.svg" alt="Afterfetch — A tiny system-information command with an Afterglow sun for every system, from DOS to macOS" width="100%"></p>

<p align="center">
  <a href="https://github.com/liminal-hq-labs/afterfetch/actions/workflows/ci.yml"><img src="https://github.com/liminal-hq-labs/afterfetch/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <img src="https://img.shields.io/badge/status-experimental-a3e635" alt="Status: experimental">
  <img src="https://img.shields.io/badge/licence-Apache--2.0%20OR%20MIT-3fb950" alt="Licence: Apache-2.0 OR MIT">
</p>

A tiny system-information command with an Afterglow sun for every system, from DOS to macOS.

Afterfetch prints a short, neofetch-style summary of the system it runs on, beside a different Afterglow sun for each kind of system. It is one tiny static program per platform, with no libc and no runtime to install.

> **Status:** experimental, local only, no remote yet. The repository is bootstrapped and nothing runs yet; there are no releases.

## What it's for

- Seeing at a glance what a machine is, from a DOS box to a macOS laptop.
- Pasting the same plain-text summary into a bug report from any platform, or reading it as JSON in a script.
- Telling a row of screenshots apart: each system type has its own sun.

## Install and run

There are no packages yet. From source, with Rust 1.98.0 (`rust-toolchain.toml` pins it):

```sh
make check
```

## Platforms

| Platform | Architectures | Verified locally |
|---|---|---|
| Linux | x86_64, x86, ARM64, ARMv7, ARMv6, MIPS, MIPSel | planned |
| FreeBSD | x86_64, x86, ARM64 | planned |
| macOS | ARM64, x86_64 | CI only, after the move to GitHub |
| Windows (Win32) | x86, x86_64, ARM64 | planned; ARM64 CI only |
| DOS | 386 and later | planned |
| Windows 3.1 | 386 enhanced mode | planned |
| Desk | to be decided | pending |

## Documentation

- [SPEC.md](SPEC.md) — behaviour and interfaces
- [docs/decisions.md](docs/decisions.md) — decisions and why
- [docs/landing.md](docs/landing.md) — how work lands without a remote
- [AGENTS.md](AGENTS.md) — contributor and agent rules

## Contributing

There is no remote yet. The house rules are in [AGENTS.md](AGENTS.md); run `make check` before every commit.

## Part of Liminal HQ Labs

Afterfetch is a [Liminal HQ Labs](https://github.com/liminal-hq-labs) experiment. Experiments from Liminal HQ — early, rough and open. If it proves itself, it [graduates](https://github.com/liminal-hq-labs/.github/blob/main/docs/labs-charter.md#graduation) to [Liminal HQ](https://github.com/liminal-hq).

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT licence](LICENSE-MIT), at your option.
