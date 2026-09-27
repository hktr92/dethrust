# Codex run: 2 — FLIC container parser

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Parse the structural/container layer of Carmageddon `.FLI` / `.FLC` files without decoding pixels yet.

Before implementing, inspect:

```text
.reference/dethrace/src/DETHRACE/common/flicplay.c
.reference/dethrace/src/DETHRACE/common/flicplay.h
```

Pay particular attention to `StartFlic` and `PlayNextFlicFrame2`.

## Required behavior

Support the file magic values used by upstream:

- `0xAF11`
- `0xAF12`

Parse and validate at minimum:
- declared file size;
- format/magic;
- frame count;
- width;
- height;
- 8-bit depth requirement;
- claimed speed / timing information;
- frame headers;
- frame magic `0xF1FA`;
- chunk count;
- chunk length and raw chunk type;
- even-byte chunk alignment.

Represent frame/chunk payloads without needlessly copying the entire payload for every chunk. Keeping ranges/offsets into owned file bytes is acceptable.

Prefer explicit types such as:
- `Flic`
- `FlicHeader`
- `FlicFrame`
- `FlicChunk`
- `FlicFormat`

Exact naming is up to you.

Do not decode palette or pixel chunks in this run.

Do not reproduce the upstream streaming `FILE*` buffer mechanism. Parse from safe owned bytes/slices.

## Validation

Reject malformed structures cleanly:
- unsupported magic;
- non-8-bit data;
- frame extending past EOF;
- chunk smaller than its header;
- chunk extending past its frame;
- integer overflow while computing boundaries.

Errors should identify offsets and, when known, frame/chunk indexes.

## Tests

Build synthetic FLIC byte fixtures in test code for:
- minimal valid AF11 file;
- minimal valid AF12 file;
- multiple frames;
- multiple chunks;
- odd-sized chunk plus alignment;
- bad file magic;
- bad frame magic;
- truncated frame;
- truncated chunk;
- impossible chunk size.

## Commit

Suggested message:

```text
feat(formats): parse FLIC containers
```
