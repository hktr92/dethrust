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
    assert_eq!(config.maximum_curvature, player.mechanics.maximum_curvature);
    assert_eq!(
        config.force_torque_ratio,
        player.mechanics.force_torque_ratio
    );
    assert_eq!(config.speed_revs_ratio, player.mechanics.speed_revs_ratio);
    assert_eq!(config.max_gears, player.mechanics.max_gears);
    let rear_arm =
        (player.mechanics.wheel_positions[0][2] - player.mechanics.center_of_mass[2]).abs();
    let front_arm =
        (player.mechanics.wheel_positions[2][2] - player.mechanics.center_of_mass[2]).abs();
    let rear_share = front_arm / (front_arm + rear_arm);
    let front_share = rear_arm / (front_arm + rear_arm);
    let [front_grip, rear_grip, compression_grip] = player.mechanics.grip_angles_degrees;
    let expected_grip = [
        rear_grip.to_radians().tan() * 0.25 * (config.mass * rear_share * 5.0).sqrt(),
        front_grip.to_radians().tan() * 0.25 * (config.mass * front_share * 5.0).sqrt(),
        compression_grip.to_radians().tan() * 0.25 * (config.mass * rear_share * 5.0).sqrt(),
    ];
    assert_eq!(config.tyre_grip, expected_grip);
    assert_eq!(config.initial_brake, player.mechanics.initial_brake);
    assert_eq!(config.brake_increase, player.mechanics.brake_increase);
    assert_eq!(
        config.rolling_resistance,
        player.mechanics.rolling_resistance
    );
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

    let state = state_at_start(start_position, track.spec.start_yaw_degrees, &config);
    let presentation = presentation_transform(&state, &config);
    assert_eq!(presentation.translation, Vec3::from_array(start_position));
    assert_eq!(presentation.scale, Vec3::ONE);
    let expected_forward =
        Quat::from_rotation_y(track.spec.start_yaw_degrees.to_radians()) * -Vec3::Z;
    assert!((presentation.rotation * -Vec3::Z - expected_forward).length() < 1e-5);

    let mut settled = state;
    let simulation = dethrace_core::vehicle::VehicleSimulation::new(
        dethrace_core::vehicle::VehicleSimulationSettings::default(),
    )
    .unwrap();
    for _ in 0..250 {
        simulation
            .step_fixed(
                &mut settled,
                &config,
                dethrace_core::vehicle::DriverInput::default(),
                &collision,
            )
            .unwrap();
    }
    assert!(settled.is_finite());
    let source_drift = (Vec3::from_array(settled.position) - Vec3::from_array(state.position))
        .length()
        / config.collision_world_scale;
    assert!(
        source_drift < 1.0,
        "neutral settling moved the car {source_drift:.3} source units from its grid"
    );
    assert!(settled.wheels.iter().all(|wheel| wheel.grounded));
    assert!(
        settled
            .linear_velocity
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt()
            < 0.05
    );
    assert!(
        settled
            .angular_velocity
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt()
            < 0.01
    );

    let mut driven = settled;
    for _ in 0..40 {
        simulation
            .step_fixed(
                &mut driven,
                &config,
                dethrace_core::vehicle::DriverInput {
                    throttle: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    let start_forward = Quat::from_rotation_y(track.spec.start_yaw_degrees.to_radians()) * -Vec3::Z;
    let forward_speed = start_forward.dot(Vec3::from_array(driven.linear_velocity));
    assert!(driven.is_finite());
    assert!(
        forward_speed > 0.5,
        "player car did not pull away: {driven:?}"
    );

    let source_speed = |state: &dethrace_core::vehicle::VehicleState| {
        Vec3::from_array(state.linear_velocity).length() / config.collision_world_scale
    };
    let yaw = |state: &dethrace_core::vehicle::VehicleState| {
        Quat::from_array(state.orientation_xyzw)
            .to_euler(EulerRot::YXZ)
            .0
            .to_degrees()
    };
    let yaw_delta = |from: f32, to: f32| (to - from + 180.0).rem_euclid(360.0) - 180.0;
    let pitch_roll = |state: &dethrace_core::vehicle::VehicleState| {
        let (_, pitch, roll) = Quat::from_array(state.orientation_xyzw).to_euler(EulerRot::YXZ);
        (pitch.to_degrees(), roll.to_degrees())
    };

    let mut fast = settled;
    for _ in 0..120 {
        simulation
            .step_fixed(
                &mut fast,
                &config,
                dethrace_core::vehicle::DriverInput {
                    throttle: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    println!(
        "Maim BLKEAGLE standing acceleration: 1.6s={:.3}, 4.8s={:.3} source units/s",
        source_speed(&driven),
        source_speed(&fast),
    );

    let mut low_turn = settled;
    let low_yaw_start = yaw(&low_turn);
    for _ in 0..40 {
        simulation
            .step_fixed(
                &mut low_turn,
                &config,
                dethrace_core::vehicle::DriverInput {
                    throttle: 1.0,
                    steering: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    let low_yaw = yaw_delta(low_yaw_start, yaw(&low_turn));
    let (low_pitch, low_roll) = pitch_roll(&low_turn);
    println!(
        "Maim BLKEAGLE low-speed turn: 1.6s speed={:.3}, yaw={low_yaw:.2}deg, pitch={low_pitch:.2}deg, roll={low_roll:.2}deg, slipping wheels={}, compression={:?}",
        source_speed(&low_turn),
        low_turn
            .wheels
            .iter()
            .filter(|wheel| wheel.slipping)
            .count(),
        low_turn.wheels.map(|wheel| wheel.compression),
    );
    assert!(low_yaw.abs() > 10.0);

    let mut high_turn = fast;
    let high_yaw_start = yaw(&high_turn);
    for _ in 0..40 {
        simulation
            .step_fixed(
                &mut high_turn,
                &config,
                dethrace_core::vehicle::DriverInput {
                    throttle: 1.0,
                    steering: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    let high_yaw = yaw_delta(high_yaw_start, yaw(&high_turn));
    let (high_pitch, high_roll) = pitch_roll(&high_turn);
    println!(
        "Maim BLKEAGLE high-speed turn: start={:.3}, after 1.6s={:.3} source units/s, yaw={high_yaw:.2}deg, pitch={high_pitch:.2}deg, roll={high_roll:.2}deg, slipping wheels={}, compression={:?}",
        source_speed(&fast),
        source_speed(&high_turn),
        high_turn
            .wheels
            .iter()
            .filter(|wheel| wheel.slipping)
            .count(),
        high_turn.wheels.map(|wheel| wheel.compression),
    );

    let mut handbrake_turn = fast;
    let handbrake_yaw_start = yaw(&handbrake_turn);
    for _ in 0..40 {
        simulation
            .step_fixed(
                &mut handbrake_turn,
                &config,
                dethrace_core::vehicle::DriverInput {
                    steering: 1.0,
                    handbrake: true,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    println!(
        "Maim BLKEAGLE handbrake turn: start={:.3}, after 1.6s={:.3} source units/s, yaw={:.2}deg, slipping wheels={}",
        source_speed(&fast),
        source_speed(&handbrake_turn),
        yaw_delta(handbrake_yaw_start, yaw(&handbrake_turn)),
        handbrake_turn
            .wheels
            .iter()
            .filter(|wheel| wheel.slipping)
            .count(),
    );
    assert!(source_speed(&handbrake_turn) < source_speed(&fast));

    let mut braking = fast;
    let brake_start_position = Vec3::from_array(braking.position);
    let brake_start_speed = source_speed(&braking);
    let mut braking_stop = None;
    for frame in 0..150 {
        simulation
            .step_fixed(
                &mut braking,
                &config,
                dethrace_core::vehicle::DriverInput {
                    brake: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
        if source_speed(&braking) < 0.1 {
            let distance = (Vec3::from_array(braking.position) - brake_start_position).length()
                / config.collision_world_scale;
            braking_stop = Some((frame + 1, distance));
            break;
        }
    }
    let (brake_pitch, brake_roll) = pitch_roll(&braking);
    println!(
        "Maim BLKEAGLE full braking: start={brake_start_speed:.3} source units/s, stop={braking_stop:?} (steps, source units), pitch={brake_pitch:.2}deg, roll={brake_roll:.2}deg",
    );
    assert!(braking_stop.is_some());

    let mut reverse = settled;
    for _ in 0..40 {
        simulation
            .step_fixed(
                &mut reverse,
                &config,
                dethrace_core::vehicle::DriverInput {
                    brake: 1.0,
                    ..Default::default()
                },
                &collision,
            )
            .unwrap();
    }
    let reverse_speed =
        start_forward.dot(Vec3::from_array(reverse.linear_velocity)) / config.collision_world_scale;
    println!("Maim BLKEAGLE reverse: 1.6s speed={reverse_speed:.3} source units/s");
    assert_eq!(reverse.gear, -1);
    assert!(reverse_speed < -1.0);

    let wall = collision
        .triangles()
        .iter()
        .find(|triangle| {
            triangle.source.material.as_deref() == Some("BRNWINDW.MAT")
                && !triangle.two_sided
                && triangle.normal[0] < -0.95
                && triangle
                    .vertices
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::NEG_INFINITY, f32::max)
                    - triangle
                        .vertices
                        .iter()
                        .map(|point| point[1])
                        .fold(f32::INFINITY, f32::min)
                    > 0.1
                && triangle
                    .vertices
                    .iter()
                    .map(|point| point[2])
                    .fold(f32::NEG_INFINITY, f32::max)
                    - triangle
                        .vertices
                        .iter()
                        .map(|point| point[2])
                        .fold(f32::INFINITY, f32::min)
                    > 0.5
        })
        .expect("Maim Street includes a one-sided original window face");
    let wall_center = wall
        .vertices
        .iter()
        .fold(Vec3::ZERO, |sum, point| sum + Vec3::from_array(*point))
        / 3.0;
    let wall_normal = Vec3::from_array(wall.normal);
    let yaw_rotation = Quat::from_rotation_y(track.spec.start_yaw_degrees.to_radians());
    let bounds_min = Vec3::from_array(config.bounds[0]) / config.collision_world_scale;
    let bounds_max = Vec3::from_array(config.bounds[1]) / config.collision_world_scale;
    let bounds_center = (bounds_min + bounds_max) * 0.5;
    let support_radius = (0..8)
        .map(|index| {
            let corner = Vec3::new(
                if index & 1 == 0 {
                    bounds_min.x
                } else {
                    bounds_max.x
                },
                if index & 2 == 0 {
                    bounds_min.y
                } else {
                    bounds_max.y
                },
                if index & 4 == 0 {
                    bounds_min.z
                } else {
                    bounds_max.z
                },
            );
            (yaw_rotation * (corner - bounds_center)).dot(wall_normal)
        })
        .fold(0.0, f32::max);
    let wall_car_root =
        wall_center - yaw_rotation * bounds_center + wall_normal * (support_radius + 0.05);
    let mut impact = state_at_start(
        wall_car_root.to_array(),
        track.spec.start_yaw_degrees,
        &config,
    );
    impact.linear_velocity = (-wall_normal * 10.0 * config.collision_world_scale).to_array();
    let impact_speed = source_speed(&impact);
    simulation
        .step_fixed(
            &mut impact,
            &config,
            dethrace_core::vehicle::DriverInput::default(),
            &collision,
        )
        .unwrap();
    println!(
        "Maim BLKEAGLE window impact: start={impact_speed:.3} source units/s, after one step={:.3}, face={:?}",
        source_speed(&impact),
        collision
            .triangles()
            .get(impact.last_collision_triangle.unwrap_or(usize::MAX))
            .and_then(|triangle| triangle.source.material.as_deref()),
    );
    assert!(impact.last_collision_triangle.is_some());
    assert!(source_speed(&impact) < impact_speed * 0.5);

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
