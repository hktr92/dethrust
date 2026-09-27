# Codex run: 3 — indexed image and palette primitives

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Add source-format image primitives needed by the FLIC decoder.

These types belong in `dethrace-formats` and must not know about Bevy, GPU textures, or RGBA presentation.

## Requirements

Introduce an indexed 8-bit image/canvas abstraction with:
- width;
- height;
- contiguous `u8` palette indexes;
- checked construction/allocation;
- checked row/pixel access helpers where useful.

Introduce a fixed 256-entry RGB palette type.

Prefer simple owned representations such as conceptually:

```rust
IndexedImage {
    width,
    height,
    pixels: Vec<u8>,
}

Palette256 {
    entries: [[u8; 3]; 256],
}
```

Exact field visibility and APIs should preserve invariants.

Avoid storing row pointers or BRender-style pixelmaps.

Add helpers required by the upcoming FLIC decoder, but do not add Bevy conversion.

## Tests

Cover:
- correct buffer size;
- invalid/overflowing dimensions;
- row boundaries;
- palette indexing/update operations;
- zero-initialized image behavior.

## Commit

Suggested message:

```text
feat(formats): add indexed image primitives
```
