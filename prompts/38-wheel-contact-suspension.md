# Codex run: 38 — wheel contact and suspension

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Make the spawned player car settle onto Maim Street through real wheel/ground contact and suspension. This is the first serious vehicle-dynamics step.

Use the M2 audit and original mechanics data for wheel locations, suspension rest/travel, spring/damping semantics, wheel radius/contact assumptions, gravity, and coordinate conventions.

## Requirements

Implement:
- per-wheel world-space contact query;
- contact point and normal;
- suspension compression/travel state;
- spring force;
- damping force;
- gravity;
- force/torque application into `VehicleState`;
- stable resting behavior.

Wheel state must be explicit and inspectable. Use fixed-timestep simulation only.

## Acceptance

With zero driver input the car falls/settles onto Maim Street, does not fall through, levitate, explode into NaN/Inf, or bounce forever under normal initial conditions. Presentation follows simulation state correctly.

Provide switchable wheel/contact/compression debug visualization and concise telemetry for grounded wheels, compression, normals, and velocities.

## Tests

Use synthetic flat-ground tests for equilibrium, partial/no contact, spring/damping response, and finite-state stability over many steps.

## Commit

Suggested: `feat(game): add wheel contact and suspension`
