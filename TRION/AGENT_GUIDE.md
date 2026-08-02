# Trion Agent Guide

Instructions for AI agents (Claude Code, Copilot, etc.) working in this repository.

## What this repo is

A fork of [kscalelabs/kbot](https://github.com/kscalelabs/kbot) — an open-source humanoid robot. The fork adds a `TRION/` folder for our own experimental features. Everything outside `TRION/` mirrors upstream and should stay untouched unless the task is explicitly about changing upstream architecture.

## Hard rules

1. **New code goes in `TRION/`.** Never create Trion-specific files at the repo root or inside upstream folders (`assets/`, `electrical/`, `mechanical/`, or the submodules).
2. **Do not modify submodules** (`kos-kbot/`, `ksim-kbot/`, `kbot-inference/`) or commit submodule pointer bumps unless the user explicitly asks. They are pinned to upstream commits on purpose.
3. **Work on the `Trion` branch** (or a feature branch off it). Do not commit to `master` — it tracks upstream K-Bot.
4. **Keep the upstream diff clean.** Before editing any file outside `TRION/`, confirm with the user that they intend an upstream-facing change.
5. **Respect licensing.** Software here is GPL v3; hardware designs are CERN-OHL-S. Don't introduce incompatible license code.

## Orientation

| Path              | What it is                                          |
| ----------------- | --------------------------------------------------- |
| `TRION/`          | Our code and docs — the only place agents add files |
| `kos-kbot/`       | Submodule: robot operating system (Rust/KOS)        |
| `ksim-kbot/`      | Submodule: simulation and RL training               |
| `kbot-inference/` | Submodule: model inference                          |
| `mechanical/`     | Hardware design docs + CAD links                    |
| `electrical/`     | Electrical design docs                              |

Submodules may be uninitialized (empty folders). Initialize with `git submodule update --init` only if the task requires reading their code.

## Conventions

Git workflow details (remotes, branch model, submodule handling, upstream sync) live in [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) — follow it.

- Docs in Markdown, placed in `TRION/docs/`.
- Scripts in `TRION/scripts/`, feature code in `TRION/src/`, prototypes in `TRION/experiments/`.
- Commit messages: concise, imperative mood (e.g. `Add teleop latency probe`). Prefix Trion feature branches with `trion/`.
- Before finishing a task, run `git status` and verify nothing outside `TRION/` changed unintentionally.
