# Dethrust M1 end-to-end driver

Read `AGENTS.md` completely first.

Then read every Milestone 1 prompt below, in order:

```text
prompts/16-m1-reference-audit.md
prompts/17-pix-format.md
prompts/18-brender-materials.md
prompts/19-brender-models.md
prompts/20-brender-actors.md
prompts/21-car-visual-spec.md
prompts/22-races-opponents.md
prompts/23-bevy-car-assets.md
prompts/24-damage-gallery-static.md
prompts/25-damage-gallery-spin.md
prompts/26-damage-gallery-interaction.md
prompts/27-m1a-validation.md
prompts/28-track-visual-spec.md
prompts/29-maim-street-assets.md
prompts/30-maim-street-render.md
prompts/31-m1-validation.md
```

Drive Milestone 1 to completion end-to-end.

Before changing anything:
1. inspect `git status`;
2. inspect recent history;
3. inspect the current workspace/crates;
4. verify M0 main-menu/tooling health;
5. read all M1 acceptance criteria;
6. build an internal `16..31 complete/incomplete` checklist from actual code, not commit-message guesses.

Start at the first incomplete prompt and execute all remaining prompts sequentially through 31.

For each prompt:
- reread it;
- inspect relevant Rust code;
- inspect `.reference/dethrace` and BRender as needed;
- implement only that scope;
- add/update tests;
- run required checks;
- inspect diff/status;
- make the focused commit requested by the prompt;
- continue immediately.

Do not ask for confirmation between prompts. Do not combine the milestone into one giant commit. Validation-only prompts should not create empty commits.

## Canonical reference

Use only `.reference/dethrace` as the Dethrace/BRender behavioral and format reference. Bootstrap it using the repo script if missing.

Do not use personal forks as the specification.
Do not mechanically translate C to Rust.
Do not guess binary formats.

## Exact M1 target

### M1a
Recreate the end-race Damage/Wrecks Gallery, populated with the **Maim Street** car set resolved from original game data.

This must validate PIX/MAT/DAT/ACT, car visual definitions, race/opponent resolution, hierarchy/transforms, textures/materials, scale, auto-spin, selection and zoom/manual rotation.

Cars may remain physically undamaged. Damage/crush simulation is out of scope.

### M1b
Load and render the static **Maim Street** environment using the same BRender visual asset pipeline.

No race gameplay, physics, peds, AI, checkpoints or driving.

## Architecture constraints

Preserve `AGENTS.md`, especially:
- `dethrace-formats` is Bevy-free;
- parse source formats once into neutral Rust types;
- Bevy conversion lives outside formats;
- safe/bounds-checked parsing;
- no original assets in Git;
- `.reference` is never a runtime dependency;
- no BRender global-registry architecture;
- centralized BRender -> Bevy coordinate conversion;
- no hardcoded Maim Street opponent filenames;
- do not port all of `LoadCar` when only visuals are needed;
- car and track rendering must share PIX/MAT/DAT/ACT infrastructure.

Use `$CARMAGEDDON_DIR` for opt-in tests and manual validation.

Preserve the existing M0 menu and FLIC tooling throughout.

Prefer direct development entry points equivalent to:

```bash
cargo run -p dethrace-app --   --game-dir "$CARMAGEDDON_DIR"   --debug-scene damage-gallery-maim-street

cargo run -p dethrace-app --   --game-dir "$CARMAGEDDON_DIR"   --debug-scene maim-street
```

Integrate with existing CLI conventions if they already provide a cleaner equivalent.

## Stop only for a real blocker

Stop only if original assets/reference are unavailable when genuinely required, a source-format ambiguity cannot be resolved from BRender/Dethrace/assets, or a product/architecture decision outside the prompts is required.

If blocked, report the exact prompt, completed commits, evidence inspected, blocker, and smallest user decision/input needed. Do not guess around it.

## Final verification

M1 is complete only after both scenes are manually validated and full fmt/tests/clippy + opt-in original-asset tests pass.

Final report must include:
- prompts already complete at start;
- prompts implemented/repaired;
- every commit hash created;
- checks;
- original-asset validation;
- Damage Gallery status;
- Maim Street viewer status;
- known rendering limitations;
- whether M1 is complete.

Do not merely give me a plan. Inspect the repository and execute the remaining work.

Good luck. 😎
