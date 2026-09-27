# Codex execution order

Run the prompts in order. Each prompt assumes the previous prompt has been completed and committed.

```text
0-setup.md
1-binary-reader.md
2-flic-container.md
3-indexed-image-palette.md
4-flic-decoder-baseline.md
5-flic-chunks-a.md
6-flic-chunks-b.md
7-original-asset-tests.md
8-tool-inspect.md
9-tool-extract.md
10-m0a-validation.md

11-bevy-assets.md
12-main-menu-static.md
13-main-menu-animation.md
14-main-menu-input.md
15-m0-validation.md
```

The split is intentional:

- prompts 0–10 are **M0a**, format tooling with no Bevy dependency in `dethrace-formats` or `dethrace-tools`;
- prompts 11–15 are **M0b**, the original fresh-boot main menu.

Before the first run:

```bash
chmod +x scripts/*.sh
./scripts/bootstrap-reference.sh
cp .env.example .env
# edit .env or export CARMAGEDDON_DIR manually
```

`update-reference.sh` is deliberately separate. Do not silently update the C reference between implementation runs, because that makes behavioral comparisons harder to reproduce.
