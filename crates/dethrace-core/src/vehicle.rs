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
    pub maximum_curvature: f32,
    /// Loader-scaled tyre limits in runtime order: rear, front, compression.
    pub tyre_grip: [f32; 3],
    pub force_reduction: f32,
    pub friction_ellipticity: f32,
    pub force_torque_ratio: f32,
    pub speed_revs_ratio: f32,
    pub initial_brake: f32,
    pub brake_increase: f32,
    /// Front and rear rolling resistance.
    pub rolling_resistance: [f32; 2],
    pub max_gears: i32,
}

impl VehicleConfig {
    pub fn is_valid(&self) -> bool {
        let rear_arm = (self.wheel_positions[0][2] - self.center_of_mass[2]).abs();
        let front_arm = (self.wheel_positions[2][2] - self.center_of_mass[2]).abs();
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
            && self.maximum_curvature.is_finite()
            && self.maximum_curvature > 0.0
            && self
                .tyre_grip
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            && self.force_reduction.is_finite()
            && self.force_reduction >= 0.0
            && self.friction_ellipticity.is_finite()
            && self.friction_ellipticity > 0.0
            && self.force_torque_ratio.is_finite()
            && self.force_torque_ratio > 0.0
            && self.speed_revs_ratio.is_finite()
            && self.speed_revs_ratio > 0.0
            && self.initial_brake.is_finite()
            && self.initial_brake >= 0.0
            && self.brake_increase.is_finite()
            && self.brake_increase >= 0.0
            && self
                .rolling_resistance
                .iter()
                .all(|value| value.is_finite() && *value >= 0.0)
            && self.max_gears > 0
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
    pub normal_load: f32,
    pub slipping: bool,
}

impl WheelState {
    fn is_finite(&self) -> bool {
        self.suspension_anchor.iter().all(|value| value.is_finite())
            && self.contact_point.iter().all(|value| value.is_finite())
            && self.contact_normal.iter().all(|value| value.is_finite())
            && self.compression.is_finite()
            && self.travel.is_finite()
            && self.normal_load.is_finite()
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
    /// -1 is reverse, 0 is neutral, positive values are automatic forward gears.
    pub gear: i32,
    pub engine_revs: f32,
    pub wheels: [WheelState; 4],
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            gear: 0,
            engine_revs: 0.0,
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
            && self.engine_revs.is_finite()
            && self.engine_revs >= 0.0
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
        let forward = rotate(orientation, [0.0, 0.0, -1.0]);
        let forward_speed = dot(state.linear_velocity, forward);
        let (gear, revs, engine_force, brake) = drivetrain(config, state, input, forward_speed, dt);
        let (spring_rates, damping_rates) = suspension_rates(config);
        let mut next = *state;
        next.gear = gear;
        next.engine_revs = revs;
        let mut wheels = [WheelState::default(); 4];
        let mut force = self.settings.gravity.map(|gravity| gravity * config.mass);
        let mut torque = [0.0; 3];

        for (index, wheel) in wheels.iter_mut().enumerate() {
            // Source wheel indices 0-1 are rear, 2-3 are front; config travel is [front, rear].
            let axle = if index < 2 { 1 } else { 0 };
            let lever = subtract(config.wheel_positions[index], config.center_of_mass);
            let lever_world = rotate(orientation, lever);
            let anchor = add(state.position, lever_world);
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
                normal_load: 0.0,
                slipping: false,
            };

            let point_velocity = add(
                state.linear_velocity,
                cross(state.angular_velocity, lever_world),
            );
            let compression_speed = -dot(point_velocity, contact_normal);
            let normal_load = (spring_rates[axle] * compression
                + damping_rates[axle] * compression_speed)
                .max(0.0);
            let contact_force = contact_normal.map(|component| component * normal_load);
            wheel.normal_load = normal_load;
            force = add(force, contact_force);
            torque = add(torque, cross(lever_world, contact_force));
        }

        next.wheels = wheels;
        let grounded_count = next.wheels.iter().filter(|wheel| wheel.grounded).count();
        if grounded_count > 0 {
            let brake_force = (config.initial_brake + config.brake_increase) * brake;
            let handbrake_force = config.initial_brake + config.brake_increase;
            let curvature = -input.steering * config.maximum_curvature;
            let wheelbase = (config.wheel_positions[0][2] - config.wheel_positions[2][2]).abs();
            let steering_angle = (wheelbase * curvature).atan();

            for (index, wheel) in next.wheels.iter_mut().enumerate() {
                if !wheel.grounded || wheel.normal_load <= 0.0 {
                    continue;
                }
                let rear = index < 2;
                let lever = subtract(config.wheel_positions[index], config.center_of_mass);
                let lever_world = rotate(orientation, lever);
                let point_velocity = add(
                    state.linear_velocity,
                    cross(state.angular_velocity, lever_world),
                );
                let local_forward = if rear {
                    [0.0, 0.0, -1.0]
                } else {
                    [-steering_angle.sin(), 0.0, -steering_angle.cos()]
                };
                let mut tire_forward = rotate(orientation, local_forward);
                tire_forward = subtract(
                    tire_forward,
                    scale(
                        wheel.contact_normal,
                        dot(tire_forward, wheel.contact_normal),
                    ),
                );
                let forward_length = dot(tire_forward, tire_forward).sqrt();
                if forward_length <= f32::EPSILON {
                    continue;
                }
                tire_forward = scale(tire_forward, forward_length.recip());
                let mut tire_right = cross(tire_forward, wheel.contact_normal);
                let right_length = dot(tire_right, tire_right).sqrt();
                if right_length <= f32::EPSILON {
                    continue;
                }
                tire_right = scale(tire_right, right_length.recip());

                let longitudinal_speed = dot(point_velocity, tire_forward);
                let lateral_speed = dot(point_velocity, tire_right);
                let axle_brake = brake_force / 4.0
                    + if rear && input.handbrake {
                        handbrake_force / 2.0
                    } else {
                        0.0
                    };
                let stopping_force =
                    config.mass * longitudinal_speed.abs() / (dt * grounded_count as f32);
                let brakes = axle_brake.min(stopping_force) * -longitudinal_speed.signum();
                let resistance = config.rolling_resistance[usize::from(rear)] * wheel.normal_load;
                let rolling = resistance.min(stopping_force) * -longitudinal_speed.signum();
                let drive = if rear { engine_force * 0.5 } else { 0.0 };
                let longitudinal_force = drive + brakes + rolling;
                let lateral_force = -config.mass * lateral_speed / (dt * grounded_count as f32);
                let grip = if rear {
                    let slip = (lateral_speed.abs() / 10.0).clamp(0.0, 1.0);
                    config.tyre_grip[2] + (config.tyre_grip[0] - config.tyre_grip[2]) * slip
                } else {
                    config.tyre_grip[1]
                };
                let force_limit = wheel.normal_load.sqrt() * grip;
                let demand = ((longitudinal_force / config.friction_ellipticity).powi(2)
                    + lateral_force.powi(2))
                .sqrt();
                let scale_factor = if demand > force_limit && demand > f32::EPSILON {
                    // ponytail: this independent-wheel ellipse is the ceiling; add axle force sharing if handling tests need it.
                    force_limit / demand * config.force_reduction.min(1.0)
                } else {
                    1.0
                };
                wheel.slipping = demand > force_limit;
                let contact_force = add(
                    scale(tire_forward, longitudinal_force * scale_factor),
                    scale(tire_right, lateral_force * scale_factor),
                );
                force = add(force, contact_force);
                torque = add(torque, cross(lever_world, contact_force));
            }
        }

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

fn drivetrain(
    config: &VehicleConfig,
    state: &VehicleState,
    input: DriverInput,
    forward_speed: f32,
    dt: f32,
) -> (i32, f32, f32, f32) {
    let mut gear = state.gear.clamp(-1, config.max_gears);
    if forward_speed.abs() < 1.0 {
        if gear == 0 && input.throttle > 0.0 && input.brake == 0.0 {
            gear = 1;
        } else if gear >= 0 && input.brake > 0.0 && input.throttle == 0.0 {
            gear = -1;
        } else if gear < 0 && input.throttle > 0.0 && input.brake == 0.0 {
            gear = 1;
        }
    }
    if gear > 0 {
        let target_revs = forward_speed.abs() / config.speed_revs_ratio / gear as f32;
        if target_revs > 6000.0 && gear < config.max_gears {
            gear += 1;
        } else if target_revs < 3000.0 && gear > 1 {
            gear -= 1;
        }
    }

    // ponytail: wheel-speed RPM omits engine inertia; add it if original-data handling tests need rev transients.
    let revs = if gear == 0 {
        0.0
    } else {
        (forward_speed.abs() / config.speed_revs_ratio / gear.abs() as f32).min(8000.0)
    };
    let throttle = if gear < 0 {
        input.brake
    } else {
        input.throttle
    };
    let brake = if gear < 0 {
        input.throttle
    } else {
        input.brake
    };
    let engine_force = if gear == 0 || brake > 0.0 {
        0.0
    } else if throttle > 0.0 {
        config.force_torque_ratio * (1.2 * throttle - revs * revs / 100_000_000.0 - 0.2)
            / gear as f32
    } else if forward_speed.abs() > 0.05 {
        let drag =
            config.force_torque_ratio * (revs * revs / 100_000_000.0 + 0.2) / gear.abs() as f32;
        -forward_speed.signum() * drag.min(config.mass * forward_speed.abs() / dt)
    } else {
        0.0
    };
    (gear, revs, engine_force, brake)
}

fn suspension_rates(config: &VehicleConfig) -> ([f32; 2], [f32; 2]) {
    // SetCarSuspGiveAndHeight uses a factor of five for both original axle rates.
    let rear_arm = (config.wheel_positions[0][2] - config.center_of_mass[2]).abs();
    let front_arm = (config.wheel_positions[2][2] - config.center_of_mass[2]).abs();
    let front_share = rear_arm / (front_arm + rear_arm);
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
                [-1.0, 1.0, 2.0],
                [1.0, 1.0, 2.0],
                [-1.0, 1.0, -2.0],
                [1.0, 1.0, -2.0],
            ],
            bounds: [[-2.0, 0.0, -3.0], [2.0, 2.0, 3.0]],
            ride_height: 1.0,
            suspension_travel: [2.0; 2],
            suspension_damping: 0.5,
            collision_world_scale: 1.0,
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
        assert!(
            !VehicleConfig {
                speed_revs_ratio: 0.0,
                ..config
            }
            .is_valid()
        );
    }

    #[test]
    fn throttle_coast_brake_and_reverse_follow_the_drivetrain() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let ground = flat_ground();
        let mut neutral = VehicleState {
            linear_velocity: [0.0, 0.0, -5.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut neutral, &config, DriverInput::default(), &ground)
            .unwrap();
        assert!(neutral.linear_velocity[2] > -5.0 && neutral.linear_velocity[2] < 0.0);

        let mut forward = VehicleState::default();
        for _ in 0..40 {
            simulation
                .step_fixed(
                    &mut forward,
                    &config,
                    DriverInput {
                        throttle: 1.0,
                        ..DriverInput::default()
                    },
                    &ground,
                )
                .unwrap();
        }
        assert!(forward.linear_velocity[2] < -1.0);
        assert!(forward.gear > 0);

        let mut reverse = VehicleState::default();
        for _ in 0..40 {
            simulation
                .step_fixed(
                    &mut reverse,
                    &config,
                    DriverInput {
                        brake: 1.0,
                        ..DriverInput::default()
                    },
                    &ground,
                )
                .unwrap();
        }
        assert!(reverse.linear_velocity[2] > 1.0);
        assert_eq!(reverse.gear, -1);

        let mut braking = VehicleState {
            linear_velocity: [0.0, 0.0, -5.0],
            gear: 1,
            ..VehicleState::default()
        };
        simulation
            .step_fixed(
                &mut braking,
                &config,
                DriverInput {
                    brake: 1.0,
                    ..DriverInput::default()
                },
                &ground,
            )
            .unwrap();
        assert!(braking.linear_velocity[2] > -5.0);
    }

    #[test]
    fn steering_turns_symmetrically_in_both_directions() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let ground = flat_ground();
        let mut right = VehicleState {
            linear_velocity: [0.0, 0.0, -5.0],
            gear: 1,
            ..VehicleState::default()
        };
        let mut left = right;
        for _ in 0..30 {
            simulation
                .step_fixed(
                    &mut right,
                    &config,
                    DriverInput {
                        steering: 1.0,
                        ..DriverInput::default()
                    },
                    &ground,
                )
                .unwrap();
            simulation
                .step_fixed(
                    &mut left,
                    &config,
                    DriverInput {
                        steering: -1.0,
                        ..DriverInput::default()
                    },
                    &ground,
                )
                .unwrap();
        }
        assert!(right.position[0] > 0.0, "{right:?}");
        assert!(left.position[0] < 0.0, "{left:?}");
        assert!(
            (right.position[0] + left.position[0]).abs() < 0.05,
            "right={right:?} left={left:?}"
        );
    }

    #[test]
    fn handbrake_limits_rear_tyre_grip() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let ground = flat_ground();
        let mut coasting = VehicleState {
            linear_velocity: [0.0, 0.0, -5.0],
            gear: 1,
            ..VehicleState::default()
        };
        let mut handbraking = coasting;
        simulation
            .step_fixed(&mut coasting, &config, DriverInput::default(), &ground)
            .unwrap();
        simulation
            .step_fixed(
                &mut handbraking,
                &config,
                DriverInput {
                    handbrake: true,
                    ..DriverInput::default()
                },
                &ground,
            )
            .unwrap();
        assert!(handbraking.linear_velocity[2] > coasting.linear_velocity[2]);
        assert!(handbraking.wheels[..2].iter().any(|wheel| wheel.slipping));
    }

    #[test]
    fn aggressive_steering_and_braking_remain_finite() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let config = vehicle_config();
        let ground = flat_ground();
        let mut state = VehicleState::default();
        for step in 0..1000 {
            let input = if step % 2 == 0 {
                DriverInput {
                    steering: 1.0,
                    throttle: 1.0,
                    handbrake: true,
                    ..DriverInput::default()
                }
            } else {
                DriverInput {
                    steering: -1.0,
                    brake: 1.0,
                    ..DriverInput::default()
                }
            };
            simulation
                .step_fixed(&mut state, &config, input, &ground)
                .unwrap();
            assert!(state.is_finite());
        }
    }

    #[test]
    fn asymmetric_suspension_uses_source_rear_and_front_wheels() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut config = vehicle_config();
        config.suspension_travel = [1.0, 3.0];
        let mut state = VehicleState::default();
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &flat_ground())
            .unwrap();
        assert!(state.wheels[..2].iter().all(|wheel| wheel.travel == 3.0));
        assert!(state.wheels[2..].iter().all(|wheel| wheel.travel == 1.0));
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
