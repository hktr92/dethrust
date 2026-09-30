# Dethrust M2 end-to-end Codex driver

Read `AGENTS.md` completely first.

Then read every Milestone 2 prompt below in numerical order:

```text
prompts/32-m2-reference-audit.md
prompts/33-car-mechanics-format.md
prompts/34-vehicle-simulation-core.md
prompts/35-track-collision-world.md
prompts/36-player-spawn.md
prompts/37-player-input.md
prompts/38-wheel-contact-suspension.md
prompts/39-propulsion-steering-brakes.md
prompts/40-chassis-world-collision.md
prompts/41-chase-camera-recovery.md
prompts/42-m2-handling-pass.md
prompts/43-m2-validation.md
```

Drive Milestone 2 to completion end-to-end.

## First: establish actual repository state

Do not assume Milestone 1 is exactly as described by commit messages.

Before modifying anything:

1. inspect `git status`;
2. inspect recent history;
3. inspect the current crates and M1 implementation;
4. verify the M0 main menu and FLIC tooling are still healthy from code/tests;
5. verify the M1 Damage Gallery and static Maim Street paths are present;
6. read prompts 32–43 and all acceptance criteria;
7. build an internal `32..43 complete/incomplete` checklist from actual code.

Start at the first incomplete prompt.

## Canonical reference

Use only `.reference/dethrace` as the Dethrace/BRender behavioral and reverse-engineering reference described by `AGENTS.md`.

Bootstrap it with the repository script if missing.

Do not use personal forks as behavior specifications. Do not mechanically translate C into Rust.

Use the reference to understand mechanics data, vehicle update semantics, controls, wheel/suspension behavior, grip/propulsion, collision, recovery, and timestep behavior.

## Exact Milestone 2 target

Milestone 2 has one target:

> **Place the original player car on Maim Street and make it genuinely drivable.**

The final development scene should be equivalent to:

```bash
cargo run -p dethrace-app -- \
  --game-dir "$CARMAGEDDON_DIR" \
  --debug-scene maim-street-drive
```

By the end, the player can:

- spawn at the original start position;
- settle onto the road;
- accelerate;
- steer;
- brake;
- reverse;
- handbrake;
- collide with the static world;
- drive around Maim Street;
- recover/reset after a bad pose;
- use a chase camera;
- drive with keyboard and physical Xbox-style gamepad input.

## Explicit non-goals

Do not implement:

- opponent cars or AI;
- opponent car-vs-car collision;
- pedestrians;
- checkpoint/lap/race progression;
- damage/crushing;
- powerups;
- scoring/credits;
- Action Replay;
- save/load;
- final HUD/audio.

If an apparent requirement depends on one of these, implement only the smallest M2-safe boundary/stub and defer the rest.

## Core architecture rule

Preserve this design:

```text
Human input ─┐
AI input ────┤
Replay ──────┼──> DriverInput ───> ONE vehicle simulation
Network ─────┘
```

M2 only implements the human source.

Input systems must not directly move the car. The vehicle simulation must use a fixed timestep and remain testable independently of visual presentation.

Simulation and presentation remain separate.

## Physics/collision rule

Do **not** decide in advance that Avian, Rapier, or any other physics crate must own the vehicle.

Prompt 32 must inspect the reference and existing architecture first.

If a physics crate is useful, integrate it only as broadly as evidence justifies. Prefer preserving Carmageddon-like handling and explicit/testable vehicle mechanics over adopting a generic vehicle controller.

Do not run two competing full vehicle simulations.

## Execution strategy

For every incomplete prompt, sequentially through 43:

1. reread the prompt;
2. inspect current Rust code;
3. inspect relevant Dethrace/BRender code;
4. inspect original assets when required;
5. implement only that prompt's scope;
6. add/update tests;
7. run relevant checks;
8. manually validate when required;
9. inspect diff/status;
10. make the focused commit requested by the prompt;
11. continue without asking for confirmation.

Do not collapse M2 into one giant commit. Do not create empty commits for validation-only prompts.

## Original assets

Use `$CARMAGEDDON_DIR`.

Never copy original assets into tracked fixtures. Do not guess mechanics values or track data when original data/reference can answer the question.

## Preserve M0/M1

Periodically run regression checks. Do not wait until prompt 43 to discover that vehicle work broke:

- FLIC decoding;
- menu input;
- Damage Gallery;
- static track viewer;
- BRender asset conversion.

## Debuggability

Treat diagnostics as part of M2 engineering.

By the end, it should be possible to inspect useful vehicle state such as:

- pose;
- speed;
- linear/angular velocity;
- current `DriverInput`;
- grounded wheel count;
- suspension compression;
- contact normals;
- fixed-step rate;
- recovery state.

Do not spam logs every frame by default.

## Stop conditions

Continue through prompt 43 unless a real blocker occurs:

- canonical reference unavailable;
- required original assets unavailable;
- mechanics/source ambiguity cannot be resolved from reference + original data;
- a physics architecture decision genuinely requires user input;
- a pre-existing issue makes safe progress impossible.

If blocked, report the exact prompt, commits completed, evidence inspected, precise blocker, and smallest required user decision/input.

Do not guess through mechanics or binary-data ambiguity.

## Final verification

M2 is complete only after:

- full fmt/tests/clippy pass;
- all opt-in original-asset tests pass;
- M0/M1 regression paths remain healthy;
- a manual Maim Street driving session is stable;
- keyboard is verified;
- physical gamepad is verified when available;
- collision, recovery, and chase camera are verified.

## Final response

Report:

- prompts already complete at start;
- prompts implemented/repaired;
- one commit hash per commit created;
- selected physics/collision architecture;
- test/lint status;
- original-asset validation;
- manual driving validation;
- gamepad status;
- known handling/collision limitations;
- whether M2 is complete.

Do not merely give me a plan. Inspect the repository and execute the remaining work sequentially.

Good luck. Try not to put the Eagle through the floor. 😎
