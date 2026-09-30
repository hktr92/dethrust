# Codex run: 33 — car mechanics data

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Extend existing car parsing with the minimum original mechanics data required by M2. The source parser belongs in `dethrace-formats` and remains Bevy-free.

Use upstream `LoadCar` / `ReadMechanicsData` and the M2 audit. Do not port the full `tCar_spec`.

## Required neutral representation

Parse the actual M2-relevant original values, such as where applicable:

- mass;
- center of mass;
- inertia-related values;
- wheel positions/references;
- driven/non-driven wheel radius/circumference;
- steerable-wheel information;
- suspension parameters;
- grip/traction parameters;
- braking parameters;
- engine/torque/revs values;
- gearing/transmission values needed for movement;
- aerodynamic/downforce values only if materially required;
- coordinate/scale data required by simulation.

The exact field list must come from the actual reference and assets, not this prompt's guesses.

Do not parse damage, powerups, AI personality, cockpit UI, or unrelated gameplay merely because those sections are nearby.

Use semantic section readers/skippers, not absolute line numbers. Preserve original units where practical and make simulation-unit conversion explicit.

Reject malformed values/sections with useful context.

## Tests

Add synthetic mechanics-section tests, malformed/truncated tests, and opt-in original-asset tests for the canonical starting player car. Verify selected parsed values are finite/sensible and compare representative values to the Dethrace loader where practical.

A concise `dethrace-tools car mechanics <car>` command is allowed if useful.

## Commit

Suggested: `feat(formats): parse car mechanics data`
