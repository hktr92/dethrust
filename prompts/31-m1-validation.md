# Codex run: 31 m1 validation

Read `AGENTS.md` first. Use `.reference/dethrace` as the canonical upstream reference.

Stay strictly inside this prompt. Do not implement later prompts early. Preserve M0 behavior and these invariants:
- `dethrace-formats` stays Bevy-free;
- parsing is bounds-checked and safe;
- original Carmageddon assets and `.reference/` are never committed;
- source formats parse into neutral Rust types;
- Bevy conversion stays outside format parsing;
- do not mechanically port Dethrace C architecture.

At the end run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus checks.

## Goal
Complete M1 with two validated outcomes:
1. Maim Street Damage Gallery;
2. static Maim Street environment.

Do not start driving or physics.

Final proof:
- gallery demonstrates original car asset loading, hierarchy/transforms, materials/textures, normalization, spin, selection, zoom/manual rotation, clean exit;
- Maim Street viewer demonstrates original track dependency resolution, hierarchy/transforms, recognizable geometry/textures, free-fly inspection.

Audit architecture:
- formats remains Bevy-free;
- formats parse once into neutral types;
- car and track share the same BRender asset infrastructure;
- coordinate conversion is centralized;
- no proprietary assets or `.reference` runtime dependency;
- no gameplay/physics scope creep.

Run:
`cargo fmt --all -- --check`
`cargo test --workspace`
`cargo clippy --workspace --all-targets -- -D warnings`
plus all opt-in original-asset tests and both manual debug scenes.

Update README with both commands, M1 guarantees, known fidelity limits, and note that the next milestone starts by placing a player car on Maim Street.

If changes are needed:
`fix: complete milestone one visual validation`
Do not create an empty commit.

Recommend (but do not automatically create/push) tag `m1-visual-assets` if complete.
