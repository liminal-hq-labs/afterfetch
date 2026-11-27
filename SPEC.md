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

To be written.

## Interfaces

To be written.

## Settings and defaults

To be written.

## Security and privacy

To be written.

## Limits

To be written.
