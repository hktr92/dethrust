# Dethrust

Dethrust is a Rust and Bevy reimplementation of Carmageddon using game data
supplied by the user. Milestone 0 provides the original fresh-boot menu and
FLIC tooling. Milestone 1 adds an original-data Maim Street Damage Gallery and
a static Maim Street viewer. Milestone 2 adds a fixed-step, drivable player car
on Maim Street. Menu destinations other than Quit still show a temporary screen;
full race gameplay is not implemented.

## Setup

Install Rust 1.95 or newer. Use an untouched original Carmageddon installation;
its assets are never copied into this repository. Set its directory for the
opt-in asset test and the commands below:

```bash
export CARMAGEDDON_DIR=/absolute/path/to/CARMA
```

The canonical [Dethrace](https://github.com/dethrace-labs/dethrace) source is
the behavioral reference. Bootstrap its ignored local checkout with:

```bash
./scripts/bootstrap-reference.sh
```

The reference checkout is for development; the Rust app does not need it at
runtime.

## Run the menu

```bash
cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR"
```

`--game-dir` accepts either the installation root containing `DATA/ANIM` or
the `DATA` directory containing `ANIM`. Arrow keys or the gamepad D-pad move
between choices; Enter, Space, a mouse click, or the gamepad South button
selects. Escape or the gamepad East button opens Quit confirmation.

## Inspect the Maim Street Damage Gallery

```bash
cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR" --debug-scene damage-gallery-maim-street
```

The debug gallery loads the starting player car and five opponents resolved from
Maim Street's original race and opponent data. Arrow keys or the gamepad D-pad
select a car; Enter, Space, the gamepad South button, or a click zooms in.
Drag with the mouse, or use WASD, to rotate the zoomed car. Escape, the gamepad
East button, or Back zooms out and then exits; Done exits directly. Cars are
visually intact: damage and crush simulation are outside Milestone 1.

## Inspect Maim Street

```bash
cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR" --debug-scene maim-street
```

The viewer starts above the original Maim Street start position. Use WASD to
move, Space/Q to rise or descend, hold Ctrl to move faster, and drag with the
right mouse button to look around. Escape exits.

## Milestone 1 scope and limits

Both scenes load original PIX, MAT, DAT, and ACT data through the same checked
parsers and Bevy asset bridge. The gallery resolves its five opponents from
`RACES.TXT` and `OPPONENT.TXT` with a fixed inspection seed; the track viewer
loads the normal-resolution Maim Street visual group and its referenced files.
No original assets are stored in Git or needed from `.reference` at runtime.

Rendering uses unlit source colors and textures. Exact palette shade tables,
environment mapping, original sky/fog, animated materials, gallery button
animations, and original font styling are deferred. Cars are undamaged. The
static track viewer has no collision, peds, AI, checkpoints as gameplay, or
driving.

## Drive Maim Street (Milestone 2)

```bash
cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR" --debug-scene maim-street-drive
```

The drive scene loads Maim Street, the original starting BLKEAGLE, and the
original start transform. It settles the car on four wheel contacts, simulates
at a fixed 25 Hz, collides with the parsed static track, and follows it with a
chase camera.

| Action | Keyboard | Xbox-style gamepad |
| --- | --- | --- |
| Accelerate | W / Up | A button or RT |
| Steer | A / D or Left / Right | Left stick or D-pad |
| Brake / reverse | S / Down | X button or LT |
| Handbrake | Space | B button |
| Recover / reset | R | Start |

At very low speed, holding brake selects reverse. F1 logs `DriverInput` changes
to the terminal. F2 enables once-per-second vehicle and recovery telemetry and
wheel contact markers; the log includes pose, speed, velocities, gear, wheel
contacts and slip, fixed-step rate, last collision surface, and recovery state.

The vehicle simulation is Dethrust-owned and separate from Bevy transforms. The
current model uses parsed mechanics with four suspension contacts and a sampled
14-point chassis sweep. Tyre response is simplified: there is no
material-specific grip or speed downforce, no engine-inertia model, and no
dynamic car-to-car collision. See
[docs/m2-vehicle-simulation.md](docs/m2-vehicle-simulation.md) for handling
traces and the differences from Dethrace. Opponents, race progression, damage,
HUD, and audio remain out of scope.

## Next milestone

The next milestone is **track semantics**, not opponents.

## Inspect original FLICs

The CLI tools use `dethrace-formats` directly and do not depend on Bevy. For a
GOG installation with `DATA/ANIM`:

```bash
cargo run -p dethrace-tools -- flic inspect "$CARMAGEDDON_DIR/DATA/ANIM/MAI2COME.FLI"
cargo run -p dethrace-tools -- flic extract "$CARMAGEDDON_DIR/DATA/ANIM/MAI2STIL.FLI" --frame last --output out/main-menu.png
```

If the installation has `ANIM` at its root, remove `DATA/` from these paths.
`out/` is ignored by Git.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
CARMAGEDDON_DIR=/absolute/path/to/CARMA cargo test --workspace -- --ignored
```

Normal workspace tests use synthetic data and do not require game assets. The
last command validates original FLICs, BRender assets, the Maim Street car
roster, and the track visual graph locally.
