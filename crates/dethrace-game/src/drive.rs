use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::prelude::*;
use dethrace_assets::brender::{
    MECHANICS_WORLD_SCALE, PlayerCarSources, TrackSources, build_collision_world,
};
use dethrace_core::{
    collision::StaticCollisionWorld,
    vehicle::{DriverInput, VehicleConfig, VehicleState},
};

use crate::collision::TrackCollisionWorld;

#[derive(Resource)]
pub struct MaimStreetDriveSource(pub Option<(TrackSources, PlayerCarSources)>);

#[derive(Resource)]
pub struct PlayerVehicle {
    pub car_file: String,
    pub state: VehicleState,
    pub config: VehicleConfig,
    pub driver_input: DriverInput,
}

#[derive(Component)]
pub struct SimulationVehicleRoot;

#[derive(Resource, Default)]
pub struct DriverInputTelemetry {
    pub enabled: bool,
}

#[derive(Default)]
struct PlayerControls {
    keyboard_left: bool,
    keyboard_right: bool,
    keyboard_throttle: bool,
    keyboard_brake: bool,
    keyboard_handbrake: bool,
    gamepad_left: bool,
    gamepad_right: bool,
    gamepad_steering: f32,
    gamepad_throttle: f32,
    gamepad_brake: f32,
    gamepad_throttle_button: bool,
    gamepad_brake_button: bool,
    gamepad_handbrake: bool,
}

pub struct MaimStreetDrivePlugin;

impl Plugin for MaimStreetDrivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DriverInputTelemetry>()
            .add_systems(Startup, spawn_drive_scene)
            .add_systems(
                Update,
                (
                    map_player_controls,
                    toggle_input_telemetry,
                    log_driver_input,
                    sync_vehicle_presentation,
                )
                    .chain(),
            );
    }
}

const STEERING_DEADZONE: f32 = 0.15;
const PEDAL_DEADZONE: f32 = 0.03;

fn digital_axis(negative: bool, positive: bool) -> f32 {
    positive as i8 as f32 - negative as i8 as f32
}

fn normalize_steering(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let value = value.clamp(-1.0, 1.0);
    let magnitude = value.abs();
    if magnitude <= STEERING_DEADZONE {
        0.0
    } else {
        value.signum() * (magnitude - STEERING_DEADZONE) / (1.0 - STEERING_DEADZONE)
    }
}

fn normalize_pedal(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let value = value.clamp(0.0, 1.0);
    if value <= PEDAL_DEADZONE {
        0.0
    } else {
        (value - PEDAL_DEADZONE) / (1.0 - PEDAL_DEADZONE)
    }
}

fn normalize_controls(controls: PlayerControls) -> DriverInput {
    let steering = (digital_axis(
        controls.keyboard_left || controls.gamepad_left,
        controls.keyboard_right || controls.gamepad_right,
    ) + normalize_steering(controls.gamepad_steering))
    .clamp(-1.0, 1.0);
    let digital_throttle = controls.keyboard_throttle || controls.gamepad_throttle_button;
    let digital_brake = controls.keyboard_brake || controls.gamepad_brake_button;
    let (digital_throttle, digital_brake) = if digital_throttle && digital_brake {
        (0.0, 0.0)
    } else {
        (digital_throttle as u8 as f32, digital_brake as u8 as f32)
    };
    DriverInput {
        steering,
        throttle: digital_throttle.max(normalize_pedal(controls.gamepad_throttle)),
        brake: digital_brake.max(normalize_pedal(controls.gamepad_brake)),
        handbrake: controls.keyboard_handbrake || controls.gamepad_handbrake,
    }
}

fn map_player_controls(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut vehicle: ResMut<PlayerVehicle>,
) {
    let gamepad = gamepads.iter().next();
    let controls = PlayerControls {
        keyboard_left: keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keyboard_right: keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
        keyboard_throttle: keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        keyboard_brake: keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keyboard_handbrake: keys.pressed(KeyCode::Space),
        gamepad_left: gamepad.is_some_and(|pad| pad.pressed(GamepadButton::DPadLeft)),
        gamepad_right: gamepad.is_some_and(|pad| pad.pressed(GamepadButton::DPadRight)),
        gamepad_steering: gamepad
            .and_then(|pad| pad.get(GamepadAxis::LeftStickX))
            .unwrap_or(0.0),
        gamepad_throttle: gamepad
            .and_then(|pad| pad.get(GamepadButton::RightTrigger2))
            .unwrap_or(0.0),
        gamepad_brake: gamepad
            .and_then(|pad| pad.get(GamepadButton::LeftTrigger2))
            .unwrap_or(0.0),
        gamepad_throttle_button: gamepad.is_some_and(|pad| {
            pad.pressed(GamepadButton::South) || pad.pressed(GamepadButton::RightTrigger)
        }),
        gamepad_brake_button: gamepad.is_some_and(|pad| {
            pad.pressed(GamepadButton::West) || pad.pressed(GamepadButton::LeftTrigger)
        }),
        gamepad_handbrake: gamepad.is_some_and(|pad| pad.pressed(GamepadButton::East)),
    };
    let input = normalize_controls(controls);
    if vehicle.driver_input != input {
        vehicle.driver_input = input;
    }
}

fn toggle_input_telemetry(
    keys: Res<ButtonInput<KeyCode>>,
    mut telemetry: ResMut<DriverInputTelemetry>,
) {
    if keys.just_pressed(KeyCode::F1) {
        telemetry.enabled = !telemetry.enabled;
    }
}

fn log_driver_input(
    vehicle: Res<PlayerVehicle>,
    telemetry: Res<DriverInputTelemetry>,
    mut previous: Local<Option<DriverInput>>,
    mut was_enabled: Local<bool>,
) {
    if telemetry.enabled {
        if !*was_enabled || *previous != Some(vehicle.driver_input) {
            bevy::log::info!("Player DriverInput: {:?}", vehicle.driver_input);
        }
        *was_enabled = true;
        *previous = Some(vehicle.driver_input);
    } else {
        *was_enabled = false;
        *previous = Some(vehicle.driver_input);
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
        driver_input: DriverInput::default(),
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
    use dethrace_core::vehicle::DriverInput;

    use super::{
        PlayerControls, normalize_controls, normalize_pedal, normalize_steering,
        presentation_transform, resolve_start_position, state_at_start,
    };

    #[test]
    fn opposing_digital_inputs_cancel() {
        let input = normalize_controls(PlayerControls {
            keyboard_left: true,
            keyboard_right: true,
            keyboard_throttle: true,
            gamepad_brake_button: true,
            ..PlayerControls::default()
        });
        assert_eq!(input, DriverInput::default());
    }

    #[test]
    fn analog_controls_apply_deadzone_and_clamp() {
        assert_eq!(normalize_steering(0.1), 0.0);
        assert!((normalize_steering(0.575) - 0.5).abs() < 1e-6);
        assert_eq!(normalize_steering(5.0), 1.0);
        assert_eq!(normalize_steering(f32::NAN), 0.0);
        assert_eq!(normalize_pedal(-1.0), 0.0);
        assert_eq!(normalize_pedal(5.0), 1.0);
    }

    #[test]
    fn releasing_controls_returns_to_neutral() {
        let pressed = normalize_controls(PlayerControls {
            keyboard_right: true,
            keyboard_throttle: true,
            keyboard_handbrake: true,
            ..PlayerControls::default()
        });
        assert_eq!(pressed.steering, 1.0);
        assert_eq!(pressed.throttle, 1.0);
        assert!(pressed.handbrake);
        assert_eq!(
            normalize_controls(PlayerControls::default()),
            DriverInput::default()
        );
    }

    #[test]
    fn maps_gamepad_triggers_and_handbrake() {
        let input = normalize_controls(PlayerControls {
            gamepad_throttle: 0.515,
            gamepad_brake_button: true,
            gamepad_handbrake: true,
            ..PlayerControls::default()
        });
        assert!((input.throttle - 0.5).abs() < 1e-6);
        assert_eq!(input.brake, 1.0);
        assert!(input.handbrake);
        assert!(input.is_valid());
    }

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
