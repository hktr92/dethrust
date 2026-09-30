# Codex run: 36 — player car spawn on Maim Street

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Spawn the original player car on Maim Street at the correct original start/grid transform and connect its visual hierarchy to `VehicleState`. It does not need to drive yet.

Trace starting car choice, Maim Street start transforms, actor/master/principal relationships, and the centralized coordinate conversion established in M1.

Use original data. Do not hand-place by eye.

## Required scene

Add or extend a deterministic debug route equivalent to:

```bash
cargo run -p dethrace-app -- \
  --game-dir "$CARMAGEDDON_DIR" \
  --debug-scene maim-street-drive
```

For this run it should:
- load Maim Street;
- build static collision world;
- load the starting player car;
- spawn at correct position/orientation;
- initialize `VehicleState` and converted config;
- bind presentation hierarchy to simulation pose cleanly.

The visual actor hierarchy should be presentation under a stable simulation vehicle root so later physics updates do not double-apply transforms.

Do not add drive controls yet. Do not introduce new coordinate hacks here.

## Validation

Verify correct car, position, forward direction, scale, no transform double-application, and M0/M1 scenes remain healthy.

## Commit

Suggested: `feat(game): spawn player car on Maim Street`
