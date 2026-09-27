# Codex run: 0 — workspace setup

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Create the initial Rust multi-crate workspace and repository-level engineering baseline.

Do not implement FLIC parsing yet.

## Required workspace

Create:

```text
Cargo.toml
rust-toolchain.toml
crates/
  dethrace-app/
  dethrace-core/
  dethrace-formats/
  dethrace-assets/
  dethrace-game/
  dethrace-ui/
  dethrace-audio/
  dethrace-tools/
```

All crates should compile, but they may be intentionally minimal.

### Rules

- Use Rust edition 2024.
- Prefer a stable toolchain.
- Centralize shared package metadata in `[workspace.package]`.
- Centralize dependencies in `[workspace.dependencies]` when useful.
- Do not add Bevy yet unless it is strictly necessary for the workspace skeleton. It is acceptable and preferable for the Bevy-facing crates to remain dependency-light placeholders until M0b.
- `dethrace-core` and `dethrace-formats` must contain `#![forbid(unsafe_code)]`.
- `dethrace-formats` must not depend on Bevy.
- `dethrace-tools` must not depend on Bevy in M0a.
- Avoid placeholder abstraction layers that have no current use.

Add a small root README describing:
- this is a Rust/Bevy reimplementation using user-owned original Carmageddon data;
- Dethrace upstream is a behavioral/reference source;
- how to bootstrap `.reference/dethrace`;
- how `CARMAGEDDON_DIR` is supplied;
- current milestone: M0a formats foundation.

Run `./scripts/bootstrap-reference.sh` if the reference checkout is not present, and verify the expected upstream files exist.

## Acceptance

All of these must succeed:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Commit

Suggested message:

```text
chore: bootstrap Rust workspace
```
