# Codex run: 40 — chassis vs world collision

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Make the player chassis collide robustly with Maim Street walls, barriers, curbs, and other static environment geometry. Do not add opponent/car-vs-car collision.

Use the M2 audit and upstream collision behavior for semantics/constants where useful, without mechanically porting the original collision architecture.

## Requirements

Implement the static chassis/environment path required for M2:
- chassis collision shape or source-derived bounds;
- contact detection;
- penetration prevention/correction;
- linear/angular collision response;
- reasonable tangent/friction response where needed;
- protection against obvious tunneling at M2 speeds;
- stable multi-contact behavior against walls/corners;
- source-surface identity retained for debugging.

Do not use visual wheel meshes as chassis collision shapes.

Prioritize stable driving behavior over final damage-era impact fidelity. M7 will decide damage/crush semantics later.

## Tests

Synthetic: floor impact, wall impact, glancing contact, corner/multi-contact, resting near a wall, reasonable high-speed collision, repeated-contact finite-state stability.

Manual: hit walls/barriers, scrape a wall, cross modest curb/road transitions, reverse away after impact.

## Commit

Suggested: `feat(game): collide player car with track`
