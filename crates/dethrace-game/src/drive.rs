use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::prelude::*;
use dethrace_assets::brender::{PlayerCarSources, TrackSources, build_collision_world};
use dethrace_core::{
    collision::StaticCollisionWorld,
    vehicle::{
        DriverInput, VehicleConfig, VehicleSimulation, VehicleSimulationSettings, VehicleState,
    },
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
    pub simulation: VehicleSimulation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RecoveryPose {
    position: [f32; 3],
    orientation_xyzw: [f32; 4],
}

impl RecoveryPose {
    fn from_state(state: &VehicleState) -> Self {
        Self {
            position: state.position,
            orientation_xyzw: state.orientation_xyzw,
        }
    }

    fn state(self) -> VehicleState {
        VehicleState {
            position: self.position,
            orientation_xyzw: self.orientation_xyzw,
            ..VehicleState::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryCause {
    Manual,
    FellOffWorld,
    InvalidState,
}

#[derive(Resource, Debug)]
struct VehicleRecoveryState {
    spawn_pose: RecoveryPose,
    safe_pose: RecoveryPose,
    safe_pose_age: f32,
    unsafe_elapsed: f32,
    recoveries: u32,
    last_cause: Option<RecoveryCause>,
}

impl VehicleRecoveryState {
    fn at_spawn(state: &VehicleState) -> Self {
        let spawn_pose = RecoveryPose::from_state(state);
        Self {
            spawn_pose,
            safe_pose: spawn_pose,
            safe_pose_age: 0.0,
            unsafe_elapsed: 0.0,
            recoveries: 0,
            last_cause: None,
        }
    }
}

#[derive(Component)]
struct ChaseCamera {
    last_recovery_count: u32,
}

#[derive(Component)]
pub struct SimulationVehicleRoot;

#[derive(Resource, Default)]
pub struct DriverInputTelemetry {
    pub enabled: bool,
}

#[derive(Resource, Default)]
pub struct VehicleDebugSettings {
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
            .init_resource::<VehicleDebugSettings>()
            .add_systems(Startup, spawn_drive_scene)
            .add_systems(
                Update,
                (
                    map_player_controls,
                    toggle_input_telemetry,
                    toggle_vehicle_debug,
                    log_driver_input,
                    recover_player_vehicle,
                    simulate_player_vehicle,
                    log_vehicle_telemetry,
                    sync_vehicle_presentation,
                    update_chase_camera,
                    draw_vehicle_debug,
                )
                    .chain(),
            );
    }
}

const STEERING_DEADZONE: f32 = 0.15;
const PEDAL_DEADZONE: f32 = 0.03;
const SAFE_POSE_SPACING_SQUARED: f32 = 8.401_596;
const RECOVERY_DELAY: f32 = 3.0;
const RECOVERY_CLEARANCE_STEP: f32 = 0.25;

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

fn toggle_vehicle_debug(keys: Res<ButtonInput<KeyCode>>, mut debug: ResMut<VehicleDebugSettings>) {
    if keys.just_pressed(KeyCode::F2) {
        debug.enabled = !debug.enabled;
    }
}

fn orientation_is_valid(state: &VehicleState) -> bool {
    let length_squared = state
        .orientation_xyzw
        .iter()
        .map(|value| value * value)
        .sum::<f32>();
    length_squared.is_finite() && (0.9..=1.1).contains(&length_squared)
}

fn upright_amount(state: &VehicleState) -> f32 {
    if !orientation_is_valid(state) {
        return -1.0;
    }
    (Quat::from_array(state.orientation_xyzw).normalize() * Vec3::Y).dot(Vec3::Y)
}

fn safe_pose_eligible(
    state: &VehicleState,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
) -> bool {
    state.is_finite()
        && upright_amount(state) >= 0.8
        && state.wheels.iter().filter(|wheel| wheel.grounded).count() >= 3
        && state.is_chassis_clear(config, collision)
}

fn remember_safe_position(
    state: &VehicleState,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
    recovery: &mut VehicleRecoveryState,
    elapsed: f32,
) {
    recovery.safe_pose_age += elapsed;
    if state.wheels.iter().filter(|wheel| wheel.grounded).count() < 3
        || !state.is_finite()
        || upright_amount(state) < 0.8
    {
        return;
    }
    let position_delta = (Vec3::from_array(state.position)
        - Vec3::from_array(recovery.safe_pose.position))
        / config.collision_world_scale;
    if position_delta.length_squared() > SAFE_POSE_SPACING_SQUARED
        && safe_pose_eligible(state, config, collision)
    {
        recovery.safe_pose = RecoveryPose::from_state(state);
        recovery.safe_pose_age = 0.0;
    }
}

fn recovery_pose_is_clear(
    pose: RecoveryPose,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
) -> bool {
    pose.state().is_chassis_clear(config, collision)
}

fn find_clear_recovery_pose(
    recovery: &VehicleRecoveryState,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
) -> RecoveryPose {
    for mut pose in [recovery.safe_pose, recovery.spawn_pose] {
        for _ in 0..=40 {
            if recovery_pose_is_clear(pose, config, collision) {
                return pose;
            }
            pose.position[1] += RECOVERY_CLEARANCE_STEP * config.collision_world_scale;
        }
    }

    let mut elevated = recovery.spawn_pose;
    let source_top = collision.bounds().map_or(
        elevated.position[1] / config.collision_world_scale,
        |bounds| bounds[1][1],
    );
    elevated.position[1] = (source_top + 10.0) * config.collision_world_scale;
    for _ in 0..40 {
        if recovery_pose_is_clear(elevated, config, collision) {
            break;
        }
        elevated.position[1] += RECOVERY_CLEARANCE_STEP * config.collision_world_scale;
    }
    elevated
}

fn apply_vehicle_recovery(
    state: &mut VehicleState,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
    recovery: &mut VehicleRecoveryState,
    cause: RecoveryCause,
) {
    let pose = find_clear_recovery_pose(recovery, config, collision);
    state.position = pose.position;
    state.orientation_xyzw = pose.orientation_xyzw;
    state.linear_velocity = [0.0; 3];
    state.angular_velocity = [0.0; 3];
    state.gear = 0;
    state.engine_revs = 0.0;
    state.wheels = Default::default();
    state.last_collision_triangle = None;
    recovery.unsafe_elapsed = 0.0;
    recovery.safe_pose_age = 0.0;
    recovery.recoveries += 1;
    recovery.last_cause = Some(cause);
}

fn recovery_requested(keys: &ButtonInput<KeyCode>, gamepads: &Query<&Gamepad>) -> bool {
    keys.just_pressed(KeyCode::KeyR)
        || gamepads
            .iter()
            .any(|gamepad| gamepad.just_pressed(GamepadButton::Start))
}

fn recover_player_vehicle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    collision: Res<TrackCollisionWorld>,
    mut vehicle: ResMut<PlayerVehicle>,
    mut recovery: ResMut<VehicleRecoveryState>,
) {
    if recovery_requested(&keys, &gamepads) {
        let PlayerVehicle {
            state,
            config,
            driver_input,
            ..
        } = &mut *vehicle;
        apply_vehicle_recovery(
            state,
            config,
            &collision.0,
            &mut recovery,
            RecoveryCause::Manual,
        );
        *driver_input = DriverInput::default();
        return;
    }
    if !vehicle.state.is_finite() || !orientation_is_valid(&vehicle.state) {
        let PlayerVehicle {
            state,
            config,
            driver_input,
            ..
        } = &mut *vehicle;
        apply_vehicle_recovery(
            state,
            config,
            &collision.0,
            &mut recovery,
            RecoveryCause::InvalidState,
        );
        *driver_input = DriverInput::default();
        return;
    }

    let below_world = collision.0.bounds().is_some_and(|bounds| {
        vehicle.state.position[1] / vehicle.config.collision_world_scale < bounds[0][1] - 8.0
    });
    let unsafe_pose = below_world || upright_amount(&vehicle.state) < 0.0;
    if unsafe_pose {
        recovery.unsafe_elapsed += time.delta_secs();
    } else {
        recovery.unsafe_elapsed = 0.0;
    }
    if recovery.unsafe_elapsed >= RECOVERY_DELAY {
        let cause = if below_world {
            RecoveryCause::FellOffWorld
        } else {
            RecoveryCause::InvalidState
        };
        let PlayerVehicle {
            state,
            config,
            driver_input,
            ..
        } = &mut *vehicle;
        apply_vehicle_recovery(state, config, &collision.0, &mut recovery, cause);
        *driver_input = DriverInput::default();
    }
}

fn simulate_player_vehicle(
    time: Res<Time>,
    collision: Res<TrackCollisionWorld>,
    mut vehicle: ResMut<PlayerVehicle>,
    mut recovery: ResMut<VehicleRecoveryState>,
) {
    let result = {
        let PlayerVehicle {
            state,
            config,
            driver_input,
            simulation,
            ..
        } = &mut *vehicle;
        simulation.advance_frame(time.delta(), state, config, *driver_input, &collision.0)
    };
    match result {
        Ok(_) => remember_safe_position(
            &vehicle.state,
            &vehicle.config,
            &collision.0,
            &mut recovery,
            time.delta_secs(),
        ),
        Err(
            dethrace_core::vehicle::VehicleSimulationError::NonFiniteState
            | dethrace_core::vehicle::VehicleSimulationError::InvalidOrientation
            | dethrace_core::vehicle::VehicleSimulationError::NonFiniteResult,
        ) => {
            let PlayerVehicle {
                state,
                config,
                driver_input,
                ..
            } = &mut *vehicle;
            apply_vehicle_recovery(
                state,
                config,
                &collision.0,
                &mut recovery,
                RecoveryCause::InvalidState,
            );
            *driver_input = DriverInput::default();
        }
        Err(error) => panic!("player vehicle simulation failed: {error:?}"),
    }
}

fn log_vehicle_telemetry(
    time: Res<Time>,
    vehicle: Res<PlayerVehicle>,
    collision: Res<TrackCollisionWorld>,
    recovery: Res<VehicleRecoveryState>,
    debug: Res<VehicleDebugSettings>,
    mut elapsed: Local<f32>,
) {
    if !debug.enabled {
        *elapsed = 0.0;
        return;
    }
    *elapsed += time.delta_secs();
    if *elapsed < 1.0 {
        return;
    }
    *elapsed %= 1.0;
    let state = &vehicle.state;
    let grounded = state.wheels.iter().filter(|wheel| wheel.grounded).count();
    let slipping = state.wheels.iter().filter(|wheel| wheel.slipping).count();
    let scale = vehicle.config.collision_world_scale;
    let speed = Vec3::from_array(state.linear_velocity).length() / scale;
    let position = state.position.map(|value| value / scale);
    let safe_position = recovery.safe_pose.position.map(|value| value / scale);
    let velocity = state.linear_velocity.map(|value| value / scale);
    let fixed_hz = vehicle
        .simulation
        .settings()
        .fixed_step
        .as_secs_f32()
        .recip();
    let surface = state
        .last_collision_triangle
        .and_then(|index| collision.0.triangles().get(index))
        .map(|triangle| {
            (
                triangle.source.actor_path.as_ref(),
                triangle.source.model.as_ref(),
                triangle.source.face_index,
                triangle.source.material.as_deref(),
            )
        });
    bevy::log::info!(
        "Vehicle speed {speed:.1}, pose {position:?}, velocity {velocity:?}, angular {:?}, gear {}, revs {:.0}, wheels {grounded}/4 ({slipping} slipping), input {:?}, fixed {fixed_hz:.0}Hz, safe pose {safe_position:?} age {:.1}s, recoveries {} {:?}, collision surface {surface:?}",
        state.angular_velocity,
        state.gear,
        state.engine_revs,
        vehicle.driver_input,
        recovery.safe_pose_age,
        recovery.recoveries,
        recovery.last_cause,
    );
}

fn draw_vehicle_debug(
    vehicle: Res<PlayerVehicle>,
    debug: Res<VehicleDebugSettings>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }
    for wheel in vehicle.state.wheels {
        let anchor =
            Vec3::from_array(wheel.suspension_anchor) / vehicle.config.collision_world_scale;
        if wheel.grounded {
            let point =
                Vec3::from_array(wheel.contact_point) / vehicle.config.collision_world_scale;
            let normal = Vec3::from_array(wheel.contact_normal);
            let color = Color::srgb(0.15, 0.95, 0.35);
            gizmos.line(anchor, point, color);
            gizmos.line(point, point + normal * 0.6, Color::srgb(1.0, 0.75, 0.1));
            gizmos.line(point - Vec3::X * 0.12, point + Vec3::X * 0.12, color);
            gizmos.line(point - Vec3::Z * 0.12, point + Vec3::Z * 0.12, color);
        } else {
            gizmos.line(
                anchor - Vec3::splat(0.12),
                anchor + Vec3::splat(0.12),
                Color::srgb(1.0, 0.2, 0.1),
            );
        }
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

pub fn state_at_start(
    position: [f32; 3],
    yaw_degrees: f32,
    config: &VehicleConfig,
) -> VehicleState {
    let orientation = Quat::from_rotation_y(yaw_degrees.to_radians());
    let center_of_mass = orientation * Vec3::from_array(config.center_of_mass);
    VehicleState {
        position: (Vec3::from_array(position) * config.collision_world_scale + center_of_mass)
            .to_array(),
        orientation_xyzw: orientation.to_array(),
        ..default()
    }
}

pub fn presentation_transform(state: &VehicleState, config: &VehicleConfig) -> Transform {
    let rotation = Quat::from_array(state.orientation_xyzw);
    let root_position =
        Vec3::from_array(state.position) - rotation * Vec3::from_array(config.center_of_mass);
    Transform {
        translation: root_position / config.collision_world_scale,
        rotation,
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
    let config = player
        .vehicle_config()
        .unwrap_or_else(|error| panic!("could not convert player car mechanics: {error}"));
    let car_file = player.file.clone();
    let collision = build_collision_world(&track)
        .unwrap_or_else(|error| panic!("could not build Maim Street collision world: {error}"));
    // The standalone scene uses grid slot zero, which has no row offset upstream.
    let start_position = resolve_start_position(&collision, track.spec.start_position);
    let state = state_at_start(start_position, yaw_degrees, &config);
    commands.insert_resource(VehicleRecoveryState::at_spawn(&state));
    let simulation = VehicleSimulation::new(VehicleSimulationSettings::default())
        .expect("default vehicle simulation settings are valid");
    commands.insert_resource(TrackCollisionWorld(collision));
    commands.insert_resource(PlayerVehicle {
        car_file,
        state,
        config,
        driver_input: DriverInput::default(),
        simulation,
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
            presentation_transform(&state, &config),
        ))
        .id();
    for actor in car_roots {
        commands.entity(vehicle_root).add_child(actor);
    }

    let (camera_position, look_at) = chase_camera_target(&state, &config);
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.1,
            far: 2000.0,
            ..default()
        }),
        Transform::from_translation(camera_position).looking_at(look_at, Vec3::Y),
        ChaseCamera {
            last_recovery_count: 0,
        },
    ));
}

fn chase_camera_target(state: &VehicleState, config: &VehicleConfig) -> (Vec3, Vec3) {
    let root = presentation_transform(state, config);
    let forward = (root.rotation * -Vec3::Z).with_y(0.0).normalize_or_zero();
    let forward = if forward.length_squared() < 0.5 {
        Vec3::Z
    } else {
        forward
    };
    (
        root.translation - forward * 12.0 + Vec3::Y * 4.0,
        root.translation + Vec3::Y + forward * 2.0,
    )
}

fn update_chase_camera(
    time: Res<Time>,
    vehicle: Res<PlayerVehicle>,
    recovery: Res<VehicleRecoveryState>,
    mut cameras: Query<(&mut Transform, &mut ChaseCamera)>,
) {
    if !vehicle.state.is_finite() || !orientation_is_valid(&vehicle.state) {
        return;
    }
    let (position, look_at) = chase_camera_target(&vehicle.state, &vehicle.config);
    let desired = Transform::from_translation(position).looking_at(look_at, Vec3::Y);
    let blend = 1.0 - (-8.0 * time.delta_secs()).exp();
    for (mut transform, mut camera) in &mut cameras {
        if camera.last_recovery_count != recovery.recoveries {
            transform.translation = desired.translation;
            transform.rotation = desired.rotation;
            camera.last_recovery_count = recovery.recoveries;
        } else {
            transform.translation = transform.translation.lerp(desired.translation, blend);
            transform.rotation = transform
                .rotation
                .slerp(desired.rotation, blend)
                .normalize();
        }
    }
}

fn sync_vehicle_presentation(
    vehicle: Res<PlayerVehicle>,
    mut roots: Query<&mut Transform, With<SimulationVehicleRoot>>,
) {
    if !vehicle.is_changed() {
        return;
    }
    for mut transform in &mut roots {
        *transform = presentation_transform(&vehicle.state, &vehicle.config);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Quat, Vec3};
    use dethrace_assets::brender::MECHANICS_WORLD_SCALE;
    use dethrace_core::{
        collision::{StaticCollisionWorld, SurfaceIdentity},
        vehicle::{DriverInput, VehicleConfig, VehicleState, WheelState},
    };
    use std::sync::Arc;

    use super::{
        PlayerControls, RecoveryCause, RecoveryPose, VehicleRecoveryState, apply_vehicle_recovery,
        chase_camera_target, normalize_controls, normalize_pedal, normalize_steering,
        presentation_transform, resolve_start_position, safe_pose_eligible, state_at_start,
    };

    fn pose_config() -> VehicleConfig {
        VehicleConfig {
            mass: 1000.0,
            center_of_mass: [0.5, 0.0, 0.0],
            principal_inertia: [1.0; 3],
            wheel_positions: [
                [-1.0, 1.0, 2.0],
                [1.0, 1.0, 2.0],
                [-1.0, 1.0, -2.0],
                [1.0, 1.0, -2.0],
            ],
            bounds: [[-2.0; 3], [2.0; 3]],
            ride_height: 1.0,
            suspension_travel: [1.0; 2],
            suspension_damping: 0.5,
            collision_world_scale: MECHANICS_WORLD_SCALE,
            maximum_curvature: 0.08,
            tyre_grip: [50.0, 60.0, 70.0],
            force_reduction: 0.5,
            friction_ellipticity: 1.0,
            force_torque_ratio: 3000.0,
            speed_revs_ratio: 0.001,
            initial_brake: 12_000.0,
            brake_increase: 12_000.0,
            rolling_resistance: [0.02, 0.02],
            max_gears: 4,
        }
    }

    #[test]
    fn chase_camera_sits_above_behind_and_looks_ahead_of_car() {
        let config = pose_config();
        let state = state_at_start([10.0, 20.0, 30.0], 180.0, &config);
        let (camera, target) = chase_camera_target(&state, &config);
        let root = presentation_transform(&state, &config).translation;
        let forward = Quat::from_array(state.orientation_xyzw) * -Vec3::Z;
        assert!(camera.y > root.y);
        assert!((camera - root).dot(forward) < 0.0);
        assert!((target - root).dot(forward) > 0.0);
    }

    #[test]
    fn safe_pose_requires_upright_grounded_and_clear_chassis() {
        let config = pose_config();
        let collision = StaticCollisionWorld::default();
        let grounded = VehicleState {
            wheels: [WheelState {
                grounded: true,
                ..WheelState::default()
            }; 4],
            ..VehicleState::default()
        };
        assert!(safe_pose_eligible(&grounded, &config, &collision));

        let flipped = VehicleState {
            orientation_xyzw: Quat::from_rotation_z(std::f32::consts::PI).to_array(),
            ..grounded
        };
        assert!(!safe_pose_eligible(&flipped, &config, &collision));
    }

    #[test]
    fn recovery_restores_safe_pose_and_resets_vehicle_motion() {
        let config = pose_config();
        let collision = StaticCollisionWorld::default();
        let safe = VehicleState {
            position: [12.0, 3.0, -7.0],
            orientation_xyzw: Quat::from_rotation_y(0.4).to_array(),
            ..VehicleState::default()
        };
        let origin = VehicleState::default();
        let mut recovery = VehicleRecoveryState {
            spawn_pose: RecoveryPose::from_state(&origin),
            safe_pose: RecoveryPose::from_state(&safe),
            safe_pose_age: 8.0,
            unsafe_elapsed: 2.0,
            recoveries: 0,
            last_cause: None,
        };
        let mut state = VehicleState {
            position: [100.0, -20.0, 40.0],
            linear_velocity: [20.0, -10.0, 5.0],
            angular_velocity: [1.0, 2.0, 3.0],
            gear: 3,
            engine_revs: 6000.0,
            ..VehicleState::default()
        };
        apply_vehicle_recovery(
            &mut state,
            &config,
            &collision,
            &mut recovery,
            RecoveryCause::Manual,
        );
        assert_eq!(state.position, safe.position);
        assert_eq!(state.orientation_xyzw, safe.orientation_xyzw);
        assert_eq!(state.linear_velocity, [0.0; 3]);
        assert_eq!(state.angular_velocity, [0.0; 3]);
        assert_eq!(state.gear, 0);
        assert_eq!(state.engine_revs, 0.0);
        assert_eq!(state.wheels, [WheelState::default(); 4]);
        assert_eq!(recovery.recoveries, 1);
        assert_eq!(recovery.last_cause, Some(RecoveryCause::Manual));
    }

    #[test]
    fn recovery_skips_a_saved_pose_embedded_in_a_wall() {
        let config = pose_config();
        let mut collision = StaticCollisionWorld::default();
        let source = SurfaceIdentity {
            actor_path: Arc::from("TEST/WALL"),
            model: Arc::from("WALL"),
            face_index: 0,
            material: Some(Arc::from("WALL")),
        };
        let a = [0.0, -30.0, -30.0];
        let b = [0.0, 30.0, -30.0];
        let c = [0.0, 30.0, 30.0];
        let d = [0.0, -30.0, 30.0];
        collision.add_triangle([a, b, c], 0, false, source.clone());
        collision.add_triangle([a, c, d], 0, false, source);
        let origin = RecoveryPose {
            position: [10.0, 0.0, 0.0],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        };
        let embedded = RecoveryPose {
            position: [0.0, 10.0, 0.0],
            ..origin
        };
        let mut recovery = VehicleRecoveryState {
            spawn_pose: origin,
            safe_pose: embedded,
            safe_pose_age: 0.0,
            unsafe_elapsed: 0.0,
            recoveries: 0,
            last_cause: None,
        };
        let mut state = embedded.state();
        apply_vehicle_recovery(
            &mut state,
            &config,
            &collision,
            &mut recovery,
            RecoveryCause::Manual,
        );
        assert_eq!(state.position, origin.position);
        assert!(state.is_chassis_clear(&config, &collision));
    }

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
        let config = pose_config();
        let state = state_at_start([10.0, 20.0, 30.0], 180.0, &config);
        assert_eq!(
            presentation_transform(&state, &config).translation,
            Vec3::new(10.0, 20.0, 30.0)
        );
        let forward = Quat::from_array(state.orientation_xyzw) * -Vec3::Z;
        assert!((forward - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn presentation_converts_to_track_units_without_scaling_visual_hierarchy() {
        let config = pose_config();
        let state = state_at_start([10.0, 20.0, 30.0], 180.0, &config);
        let transform = presentation_transform(&state, &config);
        assert_eq!(transform.translation, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(transform.scale, Vec3::ONE);
        let forward = transform.rotation * -Vec3::Z;
        assert!((forward - Vec3::Z).length() < 1e-6);
    }
}
