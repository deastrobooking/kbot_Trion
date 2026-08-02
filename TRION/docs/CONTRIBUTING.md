# Contributing to kbot_Trion — Git Guide

Everything you need to know about working with git in this repository. For the robot architecture itself, see the [Developer Guide](../DEVELOPER_GUIDE.md).

## How this repo is structured

`kbot_Trion` is a **fork** of [kscalelabs/kbot](https://github.com/kscalelabs/kbot). The top-level repo is an *umbrella*: it holds hardware documentation and pins three **git submodules** that contain the actual software:

| Submodule         | Upstream repo                | What it is                          |
| ----------------- | ---------------------------- | ----------------------------------- |
| `kos-kbot/`       | `kscalelabs/kos-kbot`        | K-Scale OS platform for K-Bot (Rust) |
| `ksim-kbot/`      | `kscalelabs/ksim-kbot`       | RL training tasks (Python/JAX)       |
| `kbot-inference/` | `kscalelabs/kbot-inference`  | On-robot policy inference (Rust)     |

A submodule folder that looks **empty** just means it hasn't been initialized — the parent repo only stores a commit pointer, not the code.

## Remotes

| Remote     | URL                                          | Purpose                        |
| ---------- | -------------------------------------------- | ------------------------------ |
| `origin`   | `github.com/deastrobooking/kbot_Trion`       | Our fork — push here           |
| `upstream` | `github.com/kscalelabs/kbot`                 | Original project — fetch only  |

If `upstream` is missing on a fresh clone:

```bash
git remote add upstream https://github.com/kscalelabs/kbot.git
```

## Branch model

| Branch            | Purpose                                                       |
| ----------------- | ------------------------------------------------------------- |
| `master`          | Clean mirror of upstream K-Bot. Never commit Trion work here. |
| `Trion`           | Our integration branch. All Trion features land here.         |
| `trion/<feature>` | Short-lived feature branches, branched off `Trion`.           |

```bash
# start a feature
git checkout Trion
git pull
git checkout -b trion/my-feature

# finish a feature
git checkout Trion
git merge trion/my-feature
git push
git branch -d trion/my-feature
```

**Golden rule:** all Trion code lives under `TRION/`. Files outside `TRION/` change only when we deliberately modify K-Bot architecture itself (ideally as an upstream contribution).

## Cloning from scratch

```bash
git clone --recurse-submodules https://github.com/deastrobooking/kbot_Trion.git
cd kbot_Trion
git checkout Trion
```

### SSH vs HTTPS for submodules

`.gitmodules` uses SSH URLs (`git@github.com:...`). If you don't have GitHub SSH keys set up, cloning submodules fails with "Please make sure you have the correct access rights." Fix it by rewriting SSH → HTTPS locally (does not modify any tracked file):

```bash
git config url."https://github.com/".insteadOf git@github.com:
git submodule update --init
```

If the submodule URLs were already copied into `.git/config` with SSH, override them directly:

```bash
for s in kos-kbot ksim-kbot kbot-inference; do
  git config submodule.$s.url https://github.com/kscalelabs/$s.git
done
git submodule update --init
```

## Working with submodules

```bash
# initialize after clone (downloads the pinned commits)
git submodule update --init

# reset submodules to exactly what the parent repo pins
git submodule update

# update all submodules to their latest upstream master
git submodule foreach 'git checkout master && git pull origin master'
```

Key facts:

- The parent repo pins each submodule to an **exact commit**. `git status` in the parent shows `(new commits)` next to a submodule whenever its checked-out commit differs from the pin.
- Committing a submodule path in the parent repo **bumps the pin**. Only do this deliberately — it changes which version of KOS/sim/inference everyone gets.
- Never commit Trion code inside a submodule; we don't own those repos. If we need to patch one, fork it and point `.gitmodules` at our fork on the `Trion` branch only.

## Syncing with upstream

```bash
git checkout master
git fetch upstream
git merge upstream/master     # should always be fast-forward
git push origin master

git checkout Trion
git merge master              # bring upstream changes under our work
git push
```

Because `master` carries no Trion commits, the first merge is always clean. Conflicts, if any, surface in the second merge — where we can resolve them once.

## Contributing back to K-Scale

1. Branch off `master` (not `Trion`): `git checkout -b fix/thing master`.
2. Make the change **outside** `TRION/` — upstream shouldn't see our folder.
3. Push to `origin` and open a PR against `kscalelabs/kbot`.
4. Changes to the software itself (KOS, sim, inference) belong in PRs against those individual repos, not the umbrella.

Upstream K-Scale code style (enforced in their repos): `cargo fmt --all`, `cargo clippy`, `cargo test`; use `tracing` for logging and `eyre` for errors; **no `unwrap()` or `expect()`**.

## Commit conventions

- Imperative mood, concise subject: `Add teleop latency probe`, not `added stuff`.
- One logical change per commit.
- Run `git status` before committing and confirm nothing outside `TRION/` changed unintentionally (the [Agent Guide](../AGENT_GUIDE.md) holds AI agents to the same rule).

## .gitignore policy

The root [.gitignore](../../.gitignore) exists so the only diffs against `master` are intentional: it filters OS junk, editor folders, Python/Rust/Node build artifacts, logs, and secrets (`.env*`). `TRION/experiments/scratch/` is never tracked — use it for throwaway work.
