# Codex run: 9 — dethrace-tools flic extract

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Add FLIC frame extraction to `dethrace-tools`.

Target UX:

```bash
cargo run -p dethrace-tools -- \
  flic extract path/to/file.FLI \
  --frame 0 \
  --output out/frame.png
```

Also support:

```text
--frame last
```

## Requirements

- Decode using `dethrace-formats`.
- Convert indexed pixels + palette to RGBA only in the tool/output layer.
- Write PNG output using a small appropriate dependency.
- For normal standalone FLIC extraction, palette index 0 is not automatically transparent.
- Validate requested frame indexes.
- Create the parent output directory if appropriate.
- Do not add Bevy.

Optionally support extracting all frames only if it is small and does not complicate the core task.

## Validation

Use original local files to visually validate at least:
- `MAINSTIL.FLI` if present;
- the fresh-boot main-menu opening/still FLIC determined from upstream;
- one button/highlight FLIC.

Do not commit extracted PNG files.

Add automated tests around indexed-to-RGBA conversion and frame selection.

## Commit

Suggested message:

```text
feat(tools): extract FLIC frames to PNG
```
