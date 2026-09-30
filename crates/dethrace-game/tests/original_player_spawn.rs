use std::path::PathBuf;

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use dethrace_assets::{
    GameDir,
    brender::{PlayerCarSources, TrackSources, build_collision_world, source_transform},
};
use dethrace_game::drive::{
    SimulationVehicleRoot, presentation_transform, resolve_start_position, state_at_start,
};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn resolves_and_spawns_the_original_player_car_at_maim_street_start() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let dir = GameDir::new(root).unwrap();
    let track = TrackSources::load(&dir, "Maim Street").unwrap();
    let collision = build_collision_world(&track).unwrap();
    let grid_origin = [
        track.spec.start_position[0],
        track.spec.start_position[1] + 10.0,
        track.spec.start_position[2],
    ];
    let snap_distance = collision
        .bounds()
        .map_or(0.0, |bounds| (grid_origin[1] - bounds[0][1]).max(0.0));
    assert!(
        collision
            .raycast_ground(grid_origin, [0.0, -1.0, 0.0], snap_distance)
            .is_some()
    );
    let start_position = resolve_start_position(&collision, track.spec.start_position);
    let player = PlayerCarSources::initial(&dir).unwrap();
    assert!(player.file.eq_ignore_ascii_case("BLKEAGLE.TXT"));
    let config = player.vehicle_config().unwrap();
    assert!(config.is_valid());
    let principal = player.scene.actor.roots.first().unwrap();
    let actor_offset = source_transform(&principal.transform)
        .translation
        .to_array();
    assert_eq!(
        config.center_of_mass,
        player
            .mechanics
            .center_of_mass
            .map(|value| value * dethrace_assets::brender::MECHANICS_WORLD_SCALE)
    );
    assert_eq!(
        config.principal_inertia,
        player
            .mechanics
            .principal_inertia
            .map(|value| value * dethrace_assets::brender::MECHANICS_INERTIA_SCALE)
    );
    let expected_ride_height = (player.mechanics.bounds[0][1] + actor_offset[1] + 0.01)
        * dethrace_assets::brender::MECHANICS_WORLD_SCALE;
    assert!(
        config
            .wheel_positions
            .iter()
            .all(|wheel| (wheel[1] - expected_ride_height).abs() < 1e-5)
    );
    for (converted_bound, source_bound) in config.bounds.iter().zip(&player.mechanics.bounds) {
        for ((converted, source), offset) in
            converted_bound.iter().zip(source_bound).zip(actor_offset)
        {
            let expected = (source + offset) * dethrace_assets::brender::MECHANICS_WORLD_SCALE;
            assert!((converted - expected).abs() < 1e-5);
        }
    }

    let state = state_at_start(start_position, track.spec.start_yaw_degrees);
    let presentation = presentation_transform(&state);
    assert_eq!(presentation.translation, Vec3::from_array(start_position));
    assert_eq!(presentation.scale, Vec3::ONE);
    let expected_forward =
        Quat::from_rotation_y(track.spec.start_yaw_degrees.to_radians()) * -Vec3::Z;
    assert!((presentation.rotation * -Vec3::Z - expected_forward).length() < 1e-5);

    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut meshes = Assets::<Mesh>::default();
    let source_transforms = player
        .scene
        .actor
        .roots
        .iter()
        .map(|actor| source_transform(&actor.transform))
        .collect::<Vec<_>>();
    let source_root_count = source_transforms.len();
    let prepared = player
        .scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .unwrap();
    let mut world = World::new();
    let mut queue = CommandQueue::default();
    let (vehicle_root, actor_roots) = {
        let mut commands = Commands::new(&mut queue, &world);
        let vehicle_root = commands.spawn((SimulationVehicleRoot, presentation)).id();
        let actor_roots = prepared.spawn(&mut commands).unwrap();
        for actor in &actor_roots {
            commands.entity(vehicle_root).add_child(*actor);
        }
        (vehicle_root, actor_roots)
    };
    queue.apply(&mut world);
    let children = world.get::<Children>(vehicle_root).unwrap();
    assert_eq!(children.len(), source_root_count);
    assert_eq!(children.len(), actor_roots.len());
    for (child, expected_transform) in children.iter().zip(source_transforms) {
        assert_eq!(world.get::<Transform>(child).unwrap(), &expected_transform);
    }
}
