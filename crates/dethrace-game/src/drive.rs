use bevy::prelude::*;
use dethrace_assets::brender::{
    MECHANICS_WORLD_SCALE, PlayerCarSources, TrackSources, build_collision_world,
};
use dethrace_core::{
    collision::StaticCollisionWorld,
    vehicle::{VehicleConfig, VehicleState},
};

use crate::collision::TrackCollisionWorld;

#[derive(Resource)]
pub struct MaimStreetDriveSource(pub Option<(TrackSources, PlayerCarSources)>);

#[derive(Resource)]
pub struct PlayerVehicle {
    pub car_file: String,
    pub state: VehicleState,
    pub config: VehicleConfig,
}

#[derive(Component)]
pub struct SimulationVehicleRoot;

pub struct MaimStreetDrivePlugin;

impl Plugin for MaimStreetDrivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_drive_scene)
            .add_systems(Update, sync_vehicle_presentation);
    }
}

/// Applies the original grid spawn surface snap in source units.
pub fn resolve_start_position(world: &StaticCollisionWorld, position: [f32; 3]) -> [f32; 3] {
    let mut resolved = position;
    let origin = [position[0], position[1] + 10.0, position[2]];
    let max_distance = world
        .bounds()
        .map_or(0.0, |bounds| (origin[1] - bounds[0][1]).max(0.0));
    resolved[1] = world
        .raycast_ground(origin, [0.0, -1.0, 0.0], max_distance)
        .map_or(0.0, |(hit, _)| hit.point[1]);
    resolved
}

pub fn state_at_start(position: [f32; 3], yaw_degrees: f32) -> VehicleState {
    VehicleState {
        position: position.map(|value| value * MECHANICS_WORLD_SCALE),
        orientation_xyzw: Quat::from_rotation_y(yaw_degrees.to_radians()).to_array(),
        ..default()
    }
}

pub fn presentation_transform(state: &VehicleState) -> Transform {
    Transform {
        translation: Vec3::from_array(state.position) / MECHANICS_WORLD_SCALE,
        rotation: Quat::from_array(state.orientation_xyzw),
        scale: Vec3::ONE,
    }
}

fn spawn_drive_scene(
    mut commands: Commands,
    mut source: ResMut<MaimStreetDriveSource>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (track, player) = source
        .0
        .take()
        .expect("Maim Street drive source already used");
    let yaw_degrees = track.spec.start_yaw_degrees;
    let yaw = yaw_degrees.to_radians();
    let config = player
        .vehicle_config()
        .unwrap_or_else(|error| panic!("could not convert player car mechanics: {error}"));
    let car_file = player.file.clone();
    let collision = build_collision_world(&track)
        .unwrap_or_else(|error| panic!("could not build Maim Street collision world: {error}"));
    // The standalone scene uses grid slot zero, which has no row offset upstream.
    let start_position = resolve_start_position(&collision, track.spec.start_position);
    let state = state_at_start(start_position, yaw_degrees);
    commands.insert_resource(TrackCollisionWorld(collision));
    commands.insert_resource(PlayerVehicle {
        car_file,
        state,
        config,
    });

    track
        .scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .expect("could not prepare Maim Street visuals")
        .spawn(&mut commands)
        .expect("could not spawn Maim Street actors");

    let car_roots = player
        .scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .expect("could not prepare starting player car visuals")
        .spawn(&mut commands)
        .expect("could not spawn starting player car actors");
    let vehicle_root = commands
        .spawn((
            Name::new("Player vehicle simulation root"),
            SimulationVehicleRoot,
            presentation_transform(&state),
        ))
        .id();
    for actor in car_roots {
        commands.entity(vehicle_root).add_child(actor);
    }

    let start = Vec3::from_array(start_position);
    let rotation = Quat::from_rotation_y(yaw);
    let camera_position = start + rotation * Vec3::new(0.0, 3.0, -12.0);
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.1,
            far: 2000.0,
            ..default()
        }),
        Transform::from_translation(camera_position).looking_at(start + Vec3::Y, Vec3::Y),
    ));
}

fn sync_vehicle_presentation(
    vehicle: Res<PlayerVehicle>,
    mut roots: Query<&mut Transform, With<SimulationVehicleRoot>>,
) {
    if !vehicle.is_changed() {
        return;
    }
    for mut transform in &mut roots {
        *transform = presentation_transform(&vehicle.state);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Quat, Vec3};
    use dethrace_assets::brender::MECHANICS_WORLD_SCALE;

    use super::{presentation_transform, resolve_start_position, state_at_start};

    #[test]
    fn grid_spawn_snaps_to_the_nearest_track_surface() {
        use dethrace_core::collision::{StaticCollisionWorld, SurfaceIdentity};
        use std::sync::Arc;

        let mut world = StaticCollisionWorld::default();
        world.add_triangle(
            [[-1.0, 2.0, -1.0], [0.0, 2.0, 1.0], [1.0, 2.0, -1.0]],
            0,
            false,
            SurfaceIdentity {
                actor_path: Arc::from("TRACK/ROAD"),
                model: Arc::from("ROAD"),
                face_index: 0,
                material: Some(Arc::from("ROAD")),
            },
        );
        assert_eq!(
            resolve_start_position(&world, [0.0, 0.0, 0.0]),
            [0.0, 2.0, 0.0]
        );
    }

    #[test]
    fn start_pose_uses_original_position_yaw_and_simulation_scale() {
        let state = state_at_start([10.0, 20.0, 30.0], 180.0);
        assert_eq!(
            state.position,
            [
                10.0 * MECHANICS_WORLD_SCALE,
                20.0 * MECHANICS_WORLD_SCALE,
                30.0 * MECHANICS_WORLD_SCALE,
            ]
        );
        let forward = Quat::from_array(state.orientation_xyzw) * -Vec3::Z;
        assert!((forward - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn presentation_converts_to_track_units_without_scaling_visual_hierarchy() {
        let state = state_at_start([10.0, 20.0, 30.0], 180.0);
        let transform = presentation_transform(&state);
        assert_eq!(transform.translation, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(transform.scale, Vec3::ONE);
        let forward = transform.rotation * -Vec3::Z;
        assert!((forward - Vec3::Z).length() < 1e-6);
    }
}
