# Dethrust

Dethrust is a Rust and Bevy reimplementation of Carmageddon using game data
supplied by the user. Milestone 0 is complete: the original fresh-boot menu
renders and its five top-level choices respond to keyboard, mouse, and gamepad
input. Destinations other than Quit currently show a temporary screen; race
loading and physics are outside Milestone 0.

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
CARMAGEDDON_DIR=/absolute/path/to/CARMA cargo test -p dethrace-formats --test original_flics -- --ignored
```

Normal workspace tests use synthetic data and do not require game assets. The
last command decodes the original fresh-boot menu FLICs locally.
