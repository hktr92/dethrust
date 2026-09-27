# Dethrace Rust Rewrite — Agent Instructions

## Mission

This repository is a clean Rust + Bevy reimplementation of Carmageddon behavior using the original game data.

It is **not** a mechanical C-to-Rust port of Dethrace.

The upstream Dethrace project is an executable behavioral/reference implementation and a source of reverse-engineered knowledge. Use it to understand file formats, game behavior, constants, quirks, and compatibility requirements. Re-design the Rust implementation around explicit ownership, validated data, testable boundaries, and idiomatic Rust.

## Canonical upstream reference

The only canonical Dethrace reference for this project is:

- `https://github.com/dethrace-labs/dethrace`
- local checkout: `.reference/dethrace`

Do **not** use personal forks as the behavioral reference unless a task explicitly says otherwise.

Bootstrap it with:

```bash
./scripts/bootstrap-reference.sh
```

The local reference checkout and its submodules are intentionally gitignored.

Useful upstream files for Milestone 0:

- `.reference/dethrace/src/DETHRACE/common/flicplay.c`
- `.reference/dethrace/src/DETHRACE/common/flicplay.h`
- `.reference/dethrace/src/DETHRACE/common/mainmenu.c`
- `.reference/dethrace/src/DETHRACE/common/mainmenu.h`
- `.reference/dethrace/src/DETHRACE/common/intrface.c`
- `.reference/dethrace/src/DETHRACE/common/intrface.h`
- `.reference/dethrace/src/DETHRACE/common/graphics.c`
- `.reference/dethrace/src/DETHRACE/common/loading.c`
- `.reference/dethrace/docs/CODE_LAYOUT.md`
- `.reference/dethrace/docs/RENDERING_PIPELINE.md`
- `.reference/dethrace/lib/BRender-v1.3.2`

When behavior is unclear, trace it in upstream before guessing.

## Original Carmageddon data

Original game assets are user-supplied and must never be committed.

Use:

```bash
export CARMAGEDDON_DIR=/absolute/path/to/CARMA
```

The full GOG installation is a valid local reference. Tests that require copyrighted/original assets must be opt-in and must not make CI depend on those files.

Never copy original Carmageddon assets into tracked fixtures.

Synthetic fixtures created specifically for tests are allowed.

## Workspace architecture

The intended workspace is multi-crate from the beginning:

```text
crates/
  dethrace-app/
  dethrace-core/
  dethrace-formats/
  dethrace-assets/
  dethrace-game/
  dethrace-ui/
  dethrace-audio/
  dethrace-tools/
```

Dependency direction:

```text
dethrace-core
    ↑
dethrace-formats
    ↑
dethrace-assets
    ↑
dethrace-ui / dethrace-game / dethrace-audio
    ↑
dethrace-app

dethrace-tools -> dethrace-core + dethrace-formats
```

More precisely:

- `dethrace-core`: domain primitives and generic shared types. Prefer no Bevy dependency.
- `dethrace-formats`: parsers/decoders for original file formats. **No Bevy dependency.**
- `dethrace-assets`: bridge from parsed formats into Bevy assets/resources.
- `dethrace-game`: game/simulation systems.
- `dethrace-ui`: menus, HUD, UI presentation and FLIC composition.
- `dethrace-audio`: original audio conventions and Bevy-facing playback.
- `dethrace-app`: executable composition root.
- `dethrace-tools`: CLI inspection/extraction tools. No Bevy dependency for Milestone 0a.

Do not create extra crates without a concrete boundary that already exists.

## Engineering rules

### 1. Parse safely

All binary parsing must be bounds checked.

Malformed input must return structured errors with enough context to diagnose the file, offset, frame, chunk, or field involved.

Do not reproduce unchecked pointer arithmetic from C.

`dethrace-core` and `dethrace-formats` should use:

```rust
#![forbid(unsafe_code)]
```

unless a future task explicitly approves an exception.

### 2. Preserve the source format

Do not prematurely turn source-format concepts into Bevy concepts.

For example, FLIC decoding should operate on indexed pixels and a 256-entry palette. RGBA/GPU conversion belongs outside `dethrace-formats`.

### 3. Separate parsing from presentation

Carmageddon-specific presentation behavior, such as treating palette index 0 as transparent for an overlay, belongs in the asset/UI layer unless the underlying format itself requires it.

Do not contaminate generic format parsers with menu behavior.

### 4. Prefer explicit code over parser-framework cleverness

For Milestone 0, prefer a small checked binary reader and direct parsing code.

Do not add `nom`, `binrw`, or a large parser framework unless the task demonstrates a concrete benefit.

### 5. Do not mechanically port C

Do not preserve:
- global-variable architecture,
- raw-memory ownership,
- function boundaries merely because they exist in C,
- BRender pixelmap abstractions,
- SDL abstractions,
- legacy framebuffer architecture.

Preserve:
- file-format semantics,
- visible behavior,
- timing where compatibility requires it,
- coordinates/constants where they define original behavior,
- gameplay semantics later.

### 6. Keep runs narrow

Each prompt under `prompts/` is one implementation run.

Implement the requested scope only. Do not proactively implement later prompt files.

Small supporting refactors are allowed when they are necessary for the current task.

### 7. Tests are part of the implementation

For parser/decoder work:
- add unit tests for valid input;
- add malformed/truncated input tests;
- avoid panics on untrusted bytes;
- add original-asset compatibility tests only as opt-in local tests.

Run relevant tests before committing.

### 8. Formatting and linting

Before each commit, run the checks relevant to changed crates. At minimum:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If the workspace is at a stage where a command cannot reasonably pass, document the exact limitation in the final response. Do not silently skip failures.

### 9. Commit at the end of every prompt

Every prompt file is expected to end in one focused Git commit.

Use a clear conventional-style message, for example:

```text
feat(formats): add checked binary reader
```

Do not squash prior work or rewrite unrelated history.

Do not commit `.reference`, original game assets, generated extraction output, or user-local configuration.

## Milestone 0

### M0a — formats foundation

Success means we can parse and decode the original menu FLIC files independently of Bevy and inspect/extract them using `dethrace-tools`.

A representative final command should look like:

```bash
cargo run -p dethrace-tools -- \
  flic extract "$CARMAGEDDON_DIR/ANIM/MAINSTIL.FLI" \
  --frame last \
  --output out/main-menu.png
```

### M0b — original main menu

Success means an untouched original Carmageddon installation can be passed to the Rust executable and the fresh-boot main menu is visible and interactive using original assets.

For M0b, trace the **fresh boot / continue-not-allowed** path in upstream `mainmenu.c`. Do not guess which FLICs, menu layout, hover animations, or coordinates are used.

No race loading or physics is part of Milestone 0.
