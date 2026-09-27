# Dethrust

Dethrust is a Rust and Bevy reimplementation of Carmageddon. It uses original
game data supplied by the user; do not commit game assets to this repository.

The canonical Dethrace upstream project is the behavioral and reverse-
engineering reference: <https://github.com/dethrace-labs/dethrace>. Bootstrap
its local checkout with:

```bash
./scripts/bootstrap-reference.sh
```

Set `CARMAGEDDON_DIR` to the absolute path of a local Carmageddon installation,
for example:

```bash
export CARMAGEDDON_DIR=/absolute/path/to/CARMA
```

The current milestone is **M0a, formats foundation**. FLIC parsing and tools
will be added in later runs.
