# Codex run: 6 — FLIC delta chunks

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Implement the remaining delta/difference chunk types used by the upstream Carmageddon FLIC decoder.

Inspect upstream `flicplay.c` implementations before coding:

- `DoDeltaX`
- `DoDifferenceX`

The transparent variants in upstream are presentation behavior and are not the generic format semantics for this run.

## Chunk types

Implement:

- `7` — FLC delta / word-oriented delta;
- `12` — FLI line-compressed difference.

Preserve prior-frame state correctly.

Avoid unaligned integer loads. Decode bytes explicitly or use safe little-endian conversion.

## Validation

Reject:
- line skips beyond image height;
- packet skips beyond row width;
- literal/repeated runs beyond row width;
- truncated words/bytes;
- malformed negative counts;
- arithmetic overflow.

Be careful with the exact signed-count semantics. Verify them against the upstream implementation instead of relying on memory of the Autodesk FLIC spec.

## Tests

Add synthetic coverage for:
- line skips;
- literal delta runs;
- repeated delta runs;
- multiple packets;
- unchanged pixels preserved from prior frame;
- first-line offset behavior for chunk 12;
- malformed row/height overflow;
- truncated word payload.

## Commit

Suggested message:

```text
feat(formats): decode FLIC delta chunks
```
