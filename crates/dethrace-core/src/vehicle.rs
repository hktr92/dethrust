use std::time::Duration;

use crate::collision::StaticCollisionWorld;

const MAX_STEPS_PER_UPDATE: u32 = 5;
/// Upstream initializes each wheel ray to a two-mechanics-unit reach.
const WHEEL_QUERY_DISTANCE: f32 = 2.0;
const WHEEL_SWEEP_MARGIN: f32 = 0.5;

/// Controller-neutral normalized controls; mapping physical devices lives above core.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DriverInput {
    pub steering: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
}

impl DriverInput {
    pub fn is_valid(&self) -> bool {
        self.steering.is_finite()
            && (-1.0..=1.0).contains(&self.steering)
            && self.throttle.is_finite()
            && (0.0..=1.0).contains(&self.throttle)
            && self.brake.is_finite()
            && (0.0..=1.0).contains(&self.brake)
    }
}

/// Mechanics converted to simulation units by the game/asset boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleConfig {
    pub mass: f32,
    pub center_of_mass: [f32; 3],
    pub principal_inertia: [f32; 3],
    pub wheel_positions: [[f32; 3]; 4],
    /// Minimum and maximum body bounds after mechanics conversion.
    pub bounds: [[f32; 3]; 2],
    /// Loader-adjusted wheel-point height in vehicle-local mechanics units.
    pub ride_height: f32,
    /// Front and rear suspension travel in mechanics units.
    pub suspension_travel: [f32; 2],
    pub suspension_damping: f32,
    /// Mechanics units per source track unit.
    pub collision_world_scale: f32,
}

impl VehicleConfig {
    pub fn is_valid(&self) -> bool {
        let front_arm = (self.wheel_positions[0][2] - self.center_of_mass[2]).abs();
        let rear_arm = (self.wheel_positions[2][2] - self.center_of_mass[2]).abs();
        let valid = self.mass.is_finite()
            && self.mass > 0.0
            && self
                .principal_inertia
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            && self.center_of_mass.iter().all(|value| value.is_finite())
            && self
                .wheel_positions
                .iter()
                .flatten()
                .all(|value| value.is_finite())
            && self.bounds.iter().flatten().all(|value| value.is_finite())
            && (0..3).all(|axis| self.bounds[0][axis] <= self.bounds[1][axis])
            && self.ride_height.is_finite()
            && self
                .suspension_travel
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            && self.suspension_damping.is_finite()
            && self.suspension_damping >= 0.0
            && self.collision_world_scale.is_finite()
            && self.collision_world_scale > 0.0
            && front_arm.is_finite()
            && rear_arm.is_finite()
            && (front_arm + rear_arm).is_finite()
            && front_arm + rear_arm > f32::EPSILON;
        if !valid {
            return false;
        }
        let (spring, damping) = suspension_rates(self);
        spring.iter().chain(&damping).all(|rate| rate.is_finite())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelState {
    pub grounded: bool,
    /// Current ray origin, in mechanics world units.
    pub suspension_anchor: [f32; 3],
    /// Surface contact point, in mechanics world units; zero while airborne.
    pub contact_point: [f32; 3],
    /// World-space unit normal; zero while airborne.
    pub contact_normal: [f32; 3],
    /// Compression and nominal travel, in mechanics units. Compression may exceed travel
    /// within the sweep margin to recover small fixed-step penetrations.
    pub compression: f32,
    pub travel: f32,
}

impl WheelState {
    fn is_finite(&self) -> bool {
        self.suspension_anchor.iter().all(|value| value.is_finite())
            && self.contact_point.iter().all(|value| value.is_finite())
            && self.contact_normal.iter().all(|value| value.is_finite())
            && self.compression.is_finite()
            && self.travel.is_finite()
    }
}

/// Simulation pose at the vehicle's center of mass. Orientation is a unit quaternion in [x, y, z, w] order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleState {
    pub position: [f32; 3],
    pub orientation_xyzw: [f32; 4],
    pub linear_velocity: [f32; 3],
    /// World-space angular velocity in radians per second.
    pub angular_velocity: [f32; 3],
    pub wheels: [WheelState; 4],
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            wheels: [WheelState::default(); 4],
        }
    }
}

impl VehicleState {
    pub fn is_finite(&self) -> bool {
        self.position.iter().all(|value| value.is_finite())
            && self.orientation_xyzw.iter().all(|value| value.is_finite())
            && self.linear_velocity.iter().all(|value| value.is_finite())
            && self.angular_velocity.iter().all(|value| value.is_finite())
            && self.wheels.iter().all(WheelState::is_finite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSimulationSettings {
    pub fixed_step: Duration,
    /// The reference applies approximately 10 mechanics units/s² downward.
    pub gravity: [f32; 3],
}

impl Default for VehicleSimulationSettings {
    fn default() -> Self {
        Self {
            fixed_step: Duration::from_millis(40),
            gravity: [0.0, -10.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleSimulationError {
    InvalidSettings,
    InvalidConfig,
    InvalidInput,
    NonFiniteState,
    InvalidOrientation,
    NonFiniteResult,
}

#[derive(Debug)]
pub struct VehicleSimulation {
    settings: VehicleSimulationSettings,
    accumulator: Duration,
}

impl VehicleSimulation {
    pub fn new(settings: VehicleSimulationSettings) -> Result<Self, VehicleSimulationError> {
        if settings.fixed_step.is_zero()
            || !settings.fixed_step.as_secs_f32().is_finite()
            || settings.gravity.iter().any(|value| !value.is_finite())
        {
            return Err(VehicleSimulationError::InvalidSettings);
        }
        Ok(Self {
            settings,
            accumulator: Duration::ZERO,
        })
    }

    pub fn settings(&self) -> VehicleSimulationSettings {
        self.settings
    }

    /// Accumulates outer-frame time and integrates only fixed-size mechanics steps.
    /// Time beyond the per-update cap remains queued for later updates.
    pub fn advance_frame(
        &mut self,
        elapsed: Duration,
        state: &mut VehicleState,
        config: &VehicleConfig,
        input: DriverInput,
        collision: &StaticCollisionWorld,
    ) -> Result<u32, VehicleSimulationError> {
        if !config.is_valid() {
            return Err(VehicleSimulationError::InvalidConfig);
        }
        if !input.is_valid() {
            return Err(VehicleSimulationError::InvalidInput);
        }
        self.accumulator = self.accumulator.saturating_add(elapsed);
        let mut steps = 0;
        while self.accumulator >= self.settings.fixed_step && steps < MAX_STEPS_PER_UPDATE {
            self.step_fixed(state, config, input, collision)?;
            self.accumulator -= self.settings.fixed_step;
            steps += 1;
        }
        Ok(steps)
    }

    /// Queries four wheel contacts and integrates one mechanics step.
    pub fn step_fixed(
        &self,
        state: &mut VehicleState,
        config: &VehicleConfig,
        input: DriverInput,
        collision: &StaticCollisionWorld,
    ) -> Result<(), VehicleSimulationError> {
        if !config.is_valid() {
            return Err(VehicleSimulationError::InvalidConfig);
        }
        if !input.is_valid() {
            return Err(VehicleSimulationError::InvalidInput);
        }
        if !state.is_finite() {
            return Err(VehicleSimulationError::NonFiniteState);
        }

        let dt = self.settings.fixed_step.as_secs_f32();
        let orientation = state.orientation_xyzw;
        let down = rotate(orientation, [0.0, -1.0, 0.0]);
        let (spring_rates, damping_rates) = suspension_rates(config);
        let mut next = *state;
        let mut wheels = [WheelState::default(); 4];
        let mut force = self.settings.gravity.map(|gravity| gravity * config.mass);
        let mut torque = [0.0; 3];

        for (index, wheel) in wheels.iter_mut().enumerate() {
            let axle = usize::from(index >= 2);
            let lever = subtract(config.wheel_positions[index], config.center_of_mass);
            let lever_world = rotate(orientation, lever);
            let anchor = add(state.position, lever_world);
            // Match the source wheel points and ray direction, with a small integration margin.
            let query_origin = subtract(anchor, scale(down, WHEEL_SWEEP_MARGIN));
            let source_origin = query_origin.map(|value| value / config.collision_world_scale);
            let source_reach =
                (WHEEL_QUERY_DISTANCE + WHEEL_SWEEP_MARGIN) / config.collision_world_scale;
            let Some((hit, _)) = collision.raycast_ground(source_origin, down, source_reach) else {
                wheel.suspension_anchor = anchor;
                wheel.travel = config.suspension_travel[axle];
                continue;
            };

            let travel = config.suspension_travel[axle];
            let distance = hit.distance * config.collision_world_scale - WHEEL_SWEEP_MARGIN;
            let compression =
                (config.ride_height + travel - distance).clamp(0.0, travel + WHEEL_SWEEP_MARGIN);
            let contact_point = hit.point.map(|value| value * config.collision_world_scale);
            let contact_normal = hit.normal;
            *wheel = WheelState {
                grounded: true,
                suspension_anchor: anchor,
                contact_point,
                contact_normal,
                compression,
                travel,
            };

            let point_velocity = add(
                state.linear_velocity,
                cross(state.angular_velocity, lever_world),
            );
            let compression_speed = -dot(point_velocity, contact_normal);
            let magnitude = (spring_rates[axle] * compression
                + damping_rates[axle] * compression_speed)
                .max(0.0);
            let contact_force = contact_normal.map(|component| component * magnitude);
            force = add(force, contact_force);
            torque = add(torque, cross(lever_world, contact_force));
        }

        next.wheels = wheels;
        for (axis, force_component) in force.iter().enumerate() {
            next.linear_velocity[axis] += force_component / config.mass * dt;
            next.position[axis] += next.linear_velocity[axis] * dt;
        }

        let torque_local = inverse_rotate(orientation, torque);
        let angular_acceleration_local =
            std::array::from_fn(|axis| torque_local[axis] / config.principal_inertia[axis]);
        let angular_acceleration_world = rotate(orientation, angular_acceleration_local);
        for (velocity, acceleration) in next
            .angular_velocity
            .iter_mut()
            .zip(angular_acceleration_world)
        {
            *velocity += acceleration * dt;
        }
        integrate_orientation(&mut next.orientation_xyzw, next.angular_velocity, dt)?;
        if !next.is_finite() {
            return Err(VehicleSimulationError::NonFiniteResult);
        }
        *state = next;
        Ok(())
    }
}

fn suspension_rates(config: &VehicleConfig) -> ([f32; 2], [f32; 2]) {
    // SetCarSuspGiveAndHeight uses a factor of five for both original axle rates.
    let front_arm = (config.wheel_positions[0][2] - config.center_of_mass[2]).abs();
    let rear_arm = (config.wheel_positions[2][2] - config.center_of_mass[2]).abs();
    let front_share = front_arm / (front_arm + rear_arm);
    let rear_share = 1.0 - front_share;
    let shares = [front_share, rear_share];
    let spring = std::array::from_fn(|axle| {
        config.mass * shares[axle] * 5.0 / config.suspension_travel[axle]
    });
    let damping = std::array::from_fn(|axle| {
        config.mass * shares[axle] * 5.0_f32.sqrt() / config.suspension_travel[axle].sqrt()
            * config.suspension_damping
    });
    (spring, damping)
}

fn integrate_orientation(
    orientation: &mut [f32; 4],
    angular_velocity: [f32; 3],
    dt: f32,
) -> Result<(), VehicleSimulationError> {
    let [x, y, z, w] = *orientation;
    let [wx, wy, wz] = angular_velocity;
    let half_dt = dt * 0.5;
    let derivative = [
        wx * w + wy * z - wz * y,
        -wx * z + wy * w + wz * x,
        wx * y - wy * x + wz * w,
        -wx * x - wy * y - wz * z,
    ];
    for (component, rate) in orientation.iter_mut().zip(derivative) {
        *component += rate * half_dt;
    }
    let length_squared = orientation.iter().map(|value| value * value).sum::<f32>();
    if !length_squared.is_finite() || length_squared <= f32::MIN_POSITIVE {
        return Err(VehicleSimulationError::InvalidOrientation);
    }
    let inverse_length = length_squared.sqrt().recip();
    for component in orientation {
        *component *= inverse_length;
    }
    Ok(())
}

fn rotate(orientation: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let vector_part = [orientation[0], orientation[1], orientation[2]];
    let twice_cross = scale(cross(vector_part, vector), 2.0);
    add(
        vector,
        add(
            scale(twice_cross, orientation[3]),
            cross(vector_part, twice_cross),
        ),
    )
}

fn inverse_rotate(orientation: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    rotate(
        [
            -orientation[0],
            -orientation[1],
            -orientation[2],
            orientation[3],
        ],
        vector,
    )
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f32; 3], factor: f32) -> [f32; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        DriverInput, VehicleConfig, VehicleSimulation, VehicleSimulationError,
        VehicleSimulationSettings, VehicleState,
    };
    use crate::collision::{StaticCollisionWorld, SurfaceIdentity};
    use std::{sync::Arc, time::Duration};

    fn vehicle_config() -> VehicleConfig {
        VehicleConfig {
            mass: 1000.0,
            center_of_mass: [0.0; 3],
            principal_inertia: [1000.0; 3],
            wheel_positions: [
                [-1.0, 1.0, -2.0],
                [1.0, 1.0, -2.0],
                [-1.0, 1.0, 2.0],
                [1.0, 1.0, 2.0],
            ],
            bounds: [[-2.0, 0.0, -3.0], [2.0, 2.0, 3.0]],
            ride_height: 1.0,
            suspension_travel: [2.0; 2],
            suspension_damping: 0.5,
            collision_world_scale: 1.0,
        }
    }

    fn ground(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> StaticCollisionWorld {
        let mut world = StaticCollisionWorld::default();
        let source = SurfaceIdentity {
            actor_path: Arc::from("TEST/GROUND"),
            model: Arc::from("GROUND"),
            face_index: 0,
            material: Some(Arc::from("ROAD")),
        };
        world.add_triangle(
            [
                [min_x, 0.0, min_z],
                [min_x, 0.0, max_z],
                [max_x, 0.0, min_z],
            ],
            0,
            false,
            source.clone(),
        );
        world.add_triangle(
            [
                [max_x, 0.0, max_z],
                [max_x, 0.0, min_z],
                [min_x, 0.0, max_z],
            ],
            0,
            false,
            source,
        );
        world
    }

    fn flat_ground() -> StaticCollisionWorld {
        ground(-10.0, 10.0, -10.0, 10.0)
    }

    #[test]
    fn default_driver_input_is_neutral() {
        assert_eq!(
            DriverInput::default(),
            DriverInput {
                steering: 0.0,
                throttle: 0.0,
                brake: 0.0,
                handbrake: false,
            }
        );
    }

    #[test]
    fn rejects_out_of_range_driver_input() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState::default();
        let input = DriverInput {
            throttle: 1.1,
            ..DriverInput::default()
        };
        assert_eq!(
            simulation
                .step_fixed(
                    &mut state,
                    &vehicle_config(),
                    input,
                    &StaticCollisionWorld::default()
                )
                .unwrap_err(),
            VehicleSimulationError::InvalidInput
        );
        assert_eq!(state, VehicleState::default());
    }

    #[test]
    fn vehicle_config_rejects_invalid_mass_inertia_bounds_and_suspension() {
        let config = vehicle_config();
        assert!(config.is_valid());
        assert!(
            !VehicleConfig {
                mass: 0.0,
                ..config
            }
            .is_valid()
        );
        assert!(
            !VehicleConfig {
                principal_inertia: [1.0, f32::NAN, 1.0],
                ..config
            }
            .is_valid()
        );
        assert!(
            !VehicleConfig {
                bounds: [[1.0; 3], [-1.0; 3]],
                ..config
            }
            .is_valid()
        );
        assert!(
            !VehicleConfig {
                suspension_travel: [0.0, 1.0],
                ..config
            }
            .is_valid()
        );
    }

    #[test]
    fn flat_ground_suspension_equilibrates_on_all_four_wheels() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let mut state = VehicleState::default();
        let ground = flat_ground();
        for _ in 0..100 {
            simulation
                .step_fixed(&mut state, &config, DriverInput::default(), &ground)
                .unwrap();
        }
        assert_eq!(state.position, [0.0; 3]);
        assert_eq!(state.linear_velocity, [0.0; 3]);
        assert!(state.wheels.iter().all(|wheel| wheel.grounded));
        assert!(
            state
                .wheels
                .iter()
                .all(|wheel| (wheel.compression - 2.0).abs() < 1e-5)
        );
        assert!(
            state
                .wheels
                .iter()
                .all(|wheel| wheel.contact_normal == [0.0, 1.0, 0.0])
        );
    }

    #[test]
    fn supports_partial_and_no_wheel_contact() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let mut state = VehicleState::default();
        let one_wheel_patch = ground(-2.0, 0.0, -3.0, -1.0);
        simulation
            .step_fixed(
                &mut state,
                &config,
                DriverInput::default(),
                &one_wheel_patch,
            )
            .unwrap();
        assert_eq!(
            state.wheels.iter().filter(|wheel| wheel.grounded).count(),
            1
        );
        assert!(
            state
                .angular_velocity
                .iter()
                .any(|velocity| velocity.abs() > 0.0)
        );

        let mut airborne = VehicleState::default();
        simulation
            .step_fixed(
                &mut airborne,
                &config,
                DriverInput::default(),
                &StaticCollisionWorld::default(),
            )
            .unwrap();
        assert!(airborne.wheels.iter().all(|wheel| !wheel.grounded));
        assert!((airborne.linear_velocity[1] + 0.4).abs() < 1e-6);
    }

    #[test]
    fn suspension_spring_and_damper_oppose_vehicle_drop() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let ground = flat_ground();

        let mut spring_state = VehicleState {
            position: [0.0, 0.2, 0.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut spring_state, &config, DriverInput::default(), &ground)
            .unwrap();
        assert!(spring_state.linear_velocity[1] < 0.0);
        assert!(spring_state.linear_velocity[1] > -0.4);

        let mut damped_state = VehicleState {
            linear_velocity: [0.0, -1.0, 0.0],
            ..VehicleState::default()
        };
        let mut no_damping = config;
        no_damping.suspension_damping = 0.0;
        let mut undamped_state = damped_state;
        simulation
            .step_fixed(&mut damped_state, &config, DriverInput::default(), &ground)
            .unwrap();
        simulation
            .step_fixed(
                &mut undamped_state,
                &no_damping,
                DriverInput::default(),
                &ground,
            )
            .unwrap();
        assert!(damped_state.linear_velocity[1] > undamped_state.linear_velocity[1]);
    }

    #[test]
    fn remains_finite_while_settling_for_many_fixed_steps() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let mut state = VehicleState {
            position: [0.0, 0.5, 0.0],
            linear_velocity: [0.0, -1.0, 0.0],
            ..VehicleState::default()
        };
        let ground = flat_ground();
        for _ in 0..1000 {
            simulation
                .step_fixed(&mut state, &config, DriverInput::default(), &ground)
                .unwrap();
            assert!(state.is_finite());
        }
        assert!(state.wheels.iter().all(|wheel| wheel.grounded));
        assert!(state.position[1].abs() < 0.1, "{state:?}");
        assert!(state.linear_velocity[1].abs() < 0.1, "{state:?}");
    }

    #[test]
    fn fixed_step_integrates_gravity_and_angular_velocity_in_air() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState {
            angular_velocity: [0.0, 1.0, 0.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(
                &mut state,
                &vehicle_config(),
                DriverInput::default(),
                &StaticCollisionWorld::default(),
            )
            .unwrap();
        assert!((state.linear_velocity[1] + 0.4).abs() < 1e-6);
        assert!((state.position[1] + 0.016).abs() < 1e-6);
        assert!((state.orientation_xyzw[1] - 0.019996).abs() < 1e-5);
        assert!((state.orientation_xyzw[3] - 0.9998).abs() < 1e-5);
    }

    #[test]
    fn caps_catch_up_steps_and_keeps_the_remaining_time() {
        let mut simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState::default();
        let ground = StaticCollisionWorld::default();
        assert_eq!(
            simulation
                .advance_frame(
                    Duration::from_millis(240),
                    &mut state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &ground,
                )
                .unwrap(),
            5
        );
        assert_eq!(
            simulation
                .advance_frame(
                    Duration::ZERO,
                    &mut state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &ground,
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn rejects_invalid_settings_and_preserves_invalid_state() {
        let invalid_settings = VehicleSimulationSettings {
            gravity: [f32::NAN, -10.0, 0.0],
            ..VehicleSimulationSettings::default()
        };
        assert_eq!(
            VehicleSimulation::new(invalid_settings).unwrap_err(),
            VehicleSimulationError::InvalidSettings
        );

        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState {
            position: [f32::INFINITY, 0.0, 0.0],
            ..VehicleState::default()
        };
        assert_eq!(
            simulation
                .step_fixed(
                    &mut state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &StaticCollisionWorld::default(),
                )
                .unwrap_err(),
            VehicleSimulationError::NonFiniteState
        );
        assert!(state.position[0].is_infinite());
        assert_eq!(state.linear_velocity, [0.0; 3]);
    }

    #[test]
    fn rejects_a_degenerate_orientation_without_mutating_state() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState {
            orientation_xyzw: [0.0; 4],
            ..VehicleState::default()
        };
        assert_eq!(
            simulation
                .step_fixed(
                    &mut state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &StaticCollisionWorld::default(),
                )
                .unwrap_err(),
            VehicleSimulationError::InvalidOrientation
        );
        assert_eq!(state.position, [0.0; 3]);
    }

    #[test]
    fn repeated_fixed_steps_are_deterministic() {
        let mut first = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut second = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut first_state = VehicleState::default();
        let mut second_state = VehicleState::default();
        let ground = StaticCollisionWorld::default();
        for frame in [20, 60, 160, 240] {
            let elapsed = Duration::from_millis(frame);
            first
                .advance_frame(
                    elapsed,
                    &mut first_state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &ground,
                )
                .unwrap();
            second
                .advance_frame(
                    elapsed,
                    &mut second_state,
                    &vehicle_config(),
                    DriverInput::default(),
                    &ground,
                )
                .unwrap();
        }
        assert_eq!(first_state, second_state);
    }
}
