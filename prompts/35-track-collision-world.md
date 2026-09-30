# Codex run: 35 — Maim Street static collision world

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Create the static collision/query representation required for the player vehicle to interact with Maim Street. Do not implement propulsion or player input yet.

Use the M2 collision audit, existing M1 track visual graph, and upstream world/raycast/collision behavior.

Do not assume every rendered triangle should automatically behave identically as collision geometry. Inspect original actor/material/face semantics where relevant.

## Requirements

Build a reusable static collision layer supporting the queries required by the selected M2 architecture, such as:

- downward wheel/ground ray or shape queries;
- surface normal;
- contact point/distance;
- chassis/environment overlap or sweep queries needed later;
- source material/surface identity where available.

If prompt 32 selected a physics crate, integrate it narrowly here. Otherwise build the smallest correct query structure needed by M2.

Do not introduce dynamic opponent/car-vs-car collision.

Reuse neutral parsed track data from M1. Do not reparse DAT/ACT/MAT through a second path.

Keep collision identity tied to original source actors/models/materials for debugging.

## Debugging

Add switchable visualization for collision geometry/bounds, wheel-query rays/contact points, and normals where useful. Keep it off by default.

## Tests

Add synthetic plane/wall/query fixtures, hit/miss and transform tests, an opt-in original Maim Street collision-graph build test, and robust handling of degenerate geometry.

## Commit

Suggested: `feat(game): build static track collision world`
