# Codex run: 34 — vehicle simulation core

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Create the vehicle-simulation domain boundary before wiring it to actual player controls or final Bevy presentation.

## Architecture

Implement neutral simulation types in the appropriate core/game boundary, with equivalents of:

```rust
pub struct DriverInput {
    pub steering: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
}

pub struct VehicleState { /* pose, velocities, wheels, drivetrain... */ }
pub struct VehicleConfig { /* converted mechanics config */ }
```

Exact fields should follow M2 needs.

Create an explicit simulation-step API. It may depend on a testable collision-query abstraction such as a `CollisionWorld` trait.

The core update must be exercisable in tests without spawning a Bevy scene.

## Fixed timestep

Establish the fixed-timestep execution model decided in prompt 32. Do not use render delta time for core integration.

If Bevy `FixedUpdate` is used, keep the simulation API testable directly.

## Scope

Implement only foundational state/integration behavior that is safe before real contacts and propulsion land: initialization, pose/velocity representation, gravity/integration hooks, fixed-step orchestration, and presentation-pose conversion where appropriate.

Do not fake a drivable car by directly moving transforms.

## Tests

Cover zero input, fixed-step behavior, free/gravity state if applicable, finite-state invariants, no ordinary NaN/Inf propagation, and same-platform repeatability where practical.

## Commit

Suggested: `feat(game): add vehicle simulation core`
