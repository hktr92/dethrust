# Codex run: 1 — checked binary reader

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Implement a small, explicit, bounds-checked little-endian binary reader in `dethrace-formats`.

This will be the foundation for FLIC and later Carmageddon/BRender parsers.

## Requirements

Add a reader type over `&[u8]` with at least:

```rust
u8
i8
u16_le
i16_le
u32_le
skip
take
position
remaining
align_to_2
```

Use names that are idiomatic Rust; the exact method names above are not mandatory.

Errors must be structured and include enough information to identify:
- attempted operation;
- current offset;
- requested byte count when relevant;
- remaining byte count.

Do not panic on truncated input.

Do not add a parser framework such as `nom` or `binrw`.

The reader should be usable by future nested/chunk parsers without unchecked indexing.

## Tests

Cover:
- all integer reads;
- signed values;
- `take`;
- `skip`;
- even alignment from both odd/even offsets;
- exact-end reads;
- one-byte-short truncation;
- oversized `take`/`skip`;
- offset reporting.

No original Carmageddon assets are required for this run.

## Acceptance

The workspace passes formatting, tests, and clippy.

## Commit

Suggested message:

```text
feat(formats): add checked binary reader
```
