# Codex run: 4 — FLIC decoder baseline

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Create the stateful FLIC decoder architecture and support the simplest frame chunk operations first.

Inspect upstream `flicplay.c`, especially:
- `DoBlack`
- `DoUncompressed`
- `DoMini`
- `PlayNextFlicFrame2`

## Scope

Implement a decoder that owns/maintains:
- current indexed image;
- current 256-color palette;
- current frame position.

Decode these chunk types:

- `13` — black/clear frame;
- `16` — uncompressed/copy pixels;
- `18` — mini/pstamp-style chunk as the upstream behavior requires for Carmageddon. If upstream intentionally skips it, mirror that behavior safely.

Do not implement chunk types 4, 7, 11, 12, or 15 yet.

The API should make it possible to decode sequential frames and inspect the resulting indexed image and palette.

Do not convert to RGBA.

## Correctness

Chunk decoders must be bounded by the chunk payload, not merely the whole file slice.

A malformed chunk must not consume bytes from the following chunk/frame.

Do not assume width is divisible by four just because upstream uses wide integer copies.

## Tests

Synthetic fixtures should cover:
- black frame;
- full uncompressed frame;
- sequential frame state;
- truncated COPY payload;
- extra bytes isolated inside a chunk;
- MINI safely ignored/skipped according to upstream behavior.

## Commit

Suggested message:

```text
feat(formats): add baseline FLIC decoder
```
