use std::path::PathBuf;

use dethrace_assets::{
    GameDir,
    brender::{TrackSources, build_collision_world},
};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn builds_maim_street_collision_graph_and_hits_the_start_grid() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let dir = GameDir::new(root).unwrap();
    let track = TrackSources::load(&dir, "Maim Street").unwrap();
    assert_eq!(track.spec.actor_file.to_ascii_uppercase(), "CITYANW1.ACT");

    let world = build_collision_world(&track).unwrap();
    assert!(!world.triangles().is_empty());
    assert!(world.bounds().is_some());

    let start = track.spec.start_position;
    let origin = [start[0], start[1] + 100.0, start[2]];
    let (hit, _) = world
        .raycast_ground(origin, [0.0, -1.0, 0.0], 200.0)
        .expect("Maim Street start grid should have a ground face");
    assert!(hit.distance >= 0.0 && hit.distance <= 200.0);
}
