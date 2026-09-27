# Codex run: 11 — Bevy asset bridge for FLIC

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Begin M0b by bridging `dethrace-formats` into Bevy without moving parsing logic into Bevy-facing crates.

Before changing dependencies, determine the currently selected/pinned Bevy version for the project. Keep it fixed for Milestone 0 rather than tracking a moving pre-release.

## Scope

Implement in `dethrace-assets`:
- Carmageddon game-directory path handling sufficient for menu assets;
- a Bevy-facing FLIC asset representation or loader;
- indexed/palette to GPU-friendly image conversion;
- explicit presentation option for index-0 transparency when an overlay requires it.

Do not change generic FLIC decode semantics in `dethrace-formats` to implement menu transparency.

Do not implement main-menu behavior yet.

## Game directory

The app should ultimately accept an explicit original game directory. Keep path resolution deterministic for now; do not recreate every Dethrace configuration fallback.

Validate useful markers such as expected `ANIM`/`DATA` content and return a useful error when the path is wrong.

## Tests

Keep source-format tests outside Bevy.

Add focused tests for:
- path mapping;
- indexed/palette conversion;
- opaque versus index-0-transparent conversion.

## Commit

Suggested message:

```text
feat(assets): add Bevy FLIC asset bridge
```
