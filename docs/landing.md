# How work lands

Afterfetch has no remote and no pull requests yet, so work lands locally. The merge-commit rule in `AGENTS.md` resumes with pull requests after the move to GitHub (AF-D11).

- `main` is the only long-lived branch. Each work package is a short series of small Conventional Commits (`git commit -F <file>`, markdown body with bold labels, hard-wrapped, `Co-Authored-By` naming the agent's own model: Sonnet builders sign as Sonnet, Opus designers as Opus). Never `git add -A` or `git commit -a`; files are added by name.
- **One builder per area at a time.** Areas: `core` (`crates/afterfetch-core`), `art` (`art/`, `crates/afterfetch-art`), `unix`, `win32`, `dos`, `win16`, `repo` (root files, Makefile, scripts, docs, and the host-only `crates/afterfetch-preview`). A work package names its areas, and two packages touching one area never run together.
- **When only one builder is active**, it works in the main checkout `~/source/liminal-hq-labs/afterfetch` and commits straight onto `main`. **When two or three run at once**, each works in its own worktree `~/source/worktrees/afterfetch/<wp-id>` on a local branch `wp/<wp-id>`. Build output stays in each worktree's `target/`, or `CARGO_TARGET_DIR=~/.cache/afterfetch/target-<wp-id>` if two worktrees would otherwise contend for memory.
- **The gate.** `make check` passes in the builder's tree before every commit, and the commit message's body says which smokes ran and which were skipped. The orchestrator (or an Opus reviewer) then reads the diff, and the series lands with `git rebase main` followed by `git -C ~/source/liminal-hq-labs/afterfetch merge --ff-only wp/<wp-id>`. History stays linear, because with no pull requests a merge commit would record nothing. Then `git worktree remove`, `git worktree prune` and `git branch -d wp/<wp-id>`.
- `make check` is re-run on `main` after each landing.

## Limits for parallel work

- At most three builders and two planners or designers at once.
- Heavy tools (cargo, QEMU, DOSBox, Wine) run one at a time per agent, with `CARGO_BUILD_JOBS=4`. QEMU runs only under `flock ~/.cache/reactos-liminal/qemu.lock`.
- Large files (the FreeBSD image, DOS vendor tools, the Windows 3.1 scratch tree, DOSBox-X image layers) live in `~/.cache/afterfetch/`. Check `df -h /home` before a download over 500 MB and `free -g` before a `/tmp` write over 1 GB.
- No host desktop capture or key injection: every GUI run uses its own Xvfb display.
- Never kill processes by name; kill only a PID the agent started.
- Never push, tag, publish or dispatch workflows.
