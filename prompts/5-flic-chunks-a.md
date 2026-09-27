# Codex run: 5 — FLIC palette and byte-run chunks

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Implement the palette and byte-run chunk families needed by Carmageddon menu FLICs.

Inspect upstream implementations in `flicplay.c` before coding.

## Chunk types

Implement:

- `4` — 256-level palette update (`DoColour256`);
- `11` — 64-level palette update (`DoColourMap`);
- `15` — byte-run / run-length encoded scanlines (`DoRunLengthX` semantics).

Keep source-format palette values as RGB 0–255.

For chunk 11, reproduce the source scaling semantics intentionally and safely. Document why it differs from chunk 4.

Do not implement Carmageddon transparent-overlay behavior in the generic decoder in this run. Decode normal FLIC pixel semantics.

## Validation

Validate:
- palette packet count;
- skip/change counts never run beyond 256 palette entries;
- payload lengths;
- each scanline remains within image width;
- RLE runs cannot overrun a row.

Return structured decode errors instead of clipping malformed data silently.

## Tests

Create small synthetic examples for:
- partial palette update;
- `change_count == 0` meaning 256 where appropriate;
- multiple palette packets;
- chunk 11 scaling;
- literal byte-run packet;
- repeated byte-run packet;
- malformed palette overflow;
- malformed scanline overflow;
- truncated packet.

## Commit

Suggested message:

```text
feat(formats): decode FLIC palette and byte-run chunks
```
