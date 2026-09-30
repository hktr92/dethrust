# Codex run: 32 — M2 mechanics and collision reference audit

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Map the original mechanics and collision behavior required to make one player car genuinely drivable on Maim Street. This is an audit/design run. Do not build the final vehicle simulation yet.

## Required upstream investigation

Inspect the relevant portions of `car.c/.h`, `controls.c/.h`, `loading.c`, `world.c`, `raycast.c`, `finteray.c`, `dr_types.h`, and any actual physics/mechanics files present in the current upstream checkout. If a file named by this prompt does not exist, trace the equivalent behavior where it actually lives instead of inventing it.

Trace at least:

- per-frame/per-tick car update path;
- `ControlCar*`, `CalcForce`, `DoRevs`, torque/angular integration;
- wheel contact and ground queries;
- suspension;
- steering;
- throttle/brake/reverse/handbrake;
- gravity;
- track collision and response;
- safe-position/recovery behavior;
- fields loaded by `ReadMechanicsData`;
- timestep units, caps, substeps, and frame-rate assumptions;
- coordinate conventions relevant to mechanics.

## Physics integration decision

Do not assume a third-party physics engine is the solution. Inspect existing Dethrust dependencies and architecture and explicitly evaluate:

1. Dethrust-owned vehicle integration + Dethrust-owned collision queries;
2. Dethrust-owned vehicle model using a physics crate only for static collision/query infrastructure;
3. broader rigid-body integration through a physics crate.

If Avian or another crate is considered, verify compatibility with the Bevy version already used by this repository before adding it.

Prefer the approach that preserves Carmageddon-like handling semantics, keeps `DriverInput -> vehicle simulation` explicit, remains testable, and can later be reused by opponents.

Do not add a physics dependency in this audit unless the choice is both clear and needed for a tiny proof.

## Deliverable

Create `docs/m2-reference-audit.md` documenting:

- update/control/mechanics data flow;
- the original mechanics fields required by M2 and what is deferred;
- wheel/suspension representation;
- static world collision/query behavior and Maim Street collision source;
- original timestep behavior and the proposed fixed-timestep strategy;
- selected M2 physics/collision architecture and rationale;
- explicit deferrals: damage, car-vs-car, opponents, peds, race logic, replay, final HUD/audio.

Do not arbitrarily choose 60 Hz without evidence. If a new fixed frequency is selected, justify it and keep it configurable/testable.

## AGENTS.md

If absent, append a concise `Milestone 2+ simulation rules` section covering:

- one vehicle simulation for human/AI/replay/network;
- neutral `DriverInput`;
- fixed simulation timestep;
- simulation separated from presentation;
- original data authoritative;
- Dethrace as oracle, not architecture template;
- debuggability as a first-class requirement.

## Commit

Suggested: `docs: map milestone two vehicle simulation`
