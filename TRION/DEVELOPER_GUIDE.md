# Trion Developer Guide

Trion is our experimental layer on top of the upstream [K-Bot](https://github.com/kscalelabs/kbot) project from K-Scale Labs. This guide explains how the repo is organized, how the branching model works, and how to develop features here without polluting the upstream project.

## Repository layout

```
kbot_Trion/
├── assets/           # Upstream: images and media
├── electrical/       # Upstream: electrical design docs
├── mechanical/       # Upstream: mechanical design docs
├── kos-kbot/         # Upstream submodule: robot operating system (KOS)
├── ksim-kbot/        # Upstream submodule: simulation & RL training
├── kbot-inference/   # Upstream submodule: model inference
└── TRION/            # OURS: all Trion-specific code, docs, and experiments
```

**Rule of thumb:** everything outside `TRION/` mirrors upstream K-Bot. Everything inside `TRION/` is ours. This separation keeps our diff against upstream clean, so we can:

1. Pull upstream changes into `master` without merge conflicts.
2. Cherry-pick or open PRs to the original K-Bot project from a clean base.
3. Iterate freely on our own features without worrying about upstream structure.

## Branching model

| Branch   | Purpose                                                                 |
| -------- | ----------------------------------------------------------------------- |
| `master` | Tracks upstream K-Bot. Only architecture/structural changes we intend to keep in sync with (or contribute back to) upstream land here. |
| `Trion`  | Our integration/testing branch. New features are built and tested here inside `TRION/` before anything is considered for merging. |

### Workflow

1. Branch off `Trion` for a feature: `git checkout -b trion/<feature-name> Trion`.
2. Keep all new code, scripts, configs, and docs inside `TRION/`.
3. Open a PR (or merge) back into `Trion` once the feature works.
4. If a change is genuinely an improvement to K-Bot itself (not Trion-specific), make it outside `TRION/` on a branch off `master`, and consider contributing it upstream to [kscalelabs/kbot](https://github.com/kscalelabs/kbot).

### Syncing with upstream

Add the upstream remote once:

```bash
git remote add upstream https://github.com/kscalelabs/kbot.git
```

Then to sync:

```bash
git checkout master
git fetch upstream
git merge upstream/master
git checkout Trion
git merge master   # bring upstream updates into Trion
```

## Working with submodules

The three software components are git submodules pinned to specific commits. After cloning:

```bash
git submodule update --init --recursive
```

To update all submodules to their latest master (as upstream recommends):

```bash
git submodule foreach 'git checkout master && git pull origin master'
```

Do **not** commit submodule pointer changes casually — they change which version of KOS/sim/inference the repo references. Only bump them deliberately.

If we need to modify a submodule's code for Trion, prefer one of these (in order):

1. Wrap/extend it from code living in `TRION/` instead of patching the submodule.
2. Fork the submodule repo, and point `.gitmodules` at our fork on the `Trion` branch only.

## TRION/ folder conventions

Suggested structure as the folder grows:

```
TRION/
├── DEVELOPER_GUIDE.md   # this file
├── AGENT_GUIDE.md       # guidance for AI agents working in this repo
├── docs/                # design notes, experiment writeups
├── src/                 # Trion feature code
├── scripts/             # tooling, automation, setup scripts
└── experiments/         # throwaway prototypes (safe to delete)
```

Create these directories as needed — don't add empty placeholders.

## Licensing note

Upstream software is GPL v3 and hardware is CERN-OHL-S (see `LICENSE` and `LICENSE-HW` at the repo root). Code in `TRION/` that links against or derives from upstream software inherits GPL v3 obligations — keep this in mind before adding proprietary code here.
