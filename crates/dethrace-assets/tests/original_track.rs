use std::path::PathBuf;

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use dethrace_assets::{GameDir, brender::TrackSources};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn prepares_maim_street_visual_graph_with_resolved_links() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let dir = GameDir::new(root).unwrap();
    let track = TrackSources::load(&dir, "Maim Street").unwrap();
    assert_eq!(
        track.scene.pixelmaps.len(),
        6,
        "five track PIX files plus SKIDMARK.PIX"
    );
    assert_eq!(
        track.scene.materials.len(),
        9,
        "eight track MAT files plus DRKCURB.MAT"
    );
    assert!(
        !track.scene.actor.roots.is_empty(),
        "CITYANW1.ACT has no root actors"
    );
    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut meshes = Assets::<Mesh>::default();
    let prepared = track
        .scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .unwrap_or_else(|e| panic!("Maim Street {}: {e}", track.spec.actor_file));
    let mut world = World::new();
    let mut queue = CommandQueue::default();
    let roots = {
        let mut commands = Commands::new(&mut queue, &world);
        prepared
            .spawn(&mut commands)
            .unwrap_or_else(|e| panic!("Maim Street actor spawn: {e}"))
    };
    queue.apply(&mut world);
    assert!(!roots.is_empty(), "track has no spawned actor roots");
    assert!(!images.is_empty(), "track has no pixelmaps");
    assert!(!materials.is_empty(), "track has no materials");
    assert!(!meshes.is_empty(), "track has no geometry");
}
