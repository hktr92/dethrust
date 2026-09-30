use std::time::Duration;

use crate::collision::StaticCollisionWorld;

const MAX_STEPS_PER_UPDATE: u32 = 5;
/// Upstream initializes each wheel ray to a two-mechanics-unit reach.
const WHEEL_QUERY_DISTANCE: f32 = 2.0;
const WHEEL_SWEEP_MARGIN: f32 = 0.5;
const CHASSIS_SKIN: f32 = 0.002;
const POSE_CLEARANCE_TOLERANCE: f32 = 0.01;
const CHASSIS_FRICTION: f32 = 0.55;
const CHASSIS_TOI_WINDOW: f32 = 0.01;
// ponytail: four impacts cap per-step work; raise this if stacked track faces stall motion.
const MAX_CHASSIS_CONTACT_ITERATIONS: usize = 4;
const MAX_ANGULAR_SWEEP_SEGMENTS: usize = 16;
const CHASSIS_SUPPORT_POINT_COUNT: usize = 14;
const MAX_ANGULAR_SWEEP_STEP: f32 = 0.08726646;

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
    /// Last chassis hit's index in the active static collision world, for debug lookup.
    pub last_collision_triangle: Option<usize>,
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
            last_collision_triangle: None,
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

    /// Checks the source-derived chassis support points for world penetration.
    pub fn is_chassis_clear(&self, config: &VehicleConfig, world: &StaticCollisionWorld) -> bool {
        if !self.is_finite() || !config.is_valid() {
            return false;
        }
        let points = chassis_support_points(self.position, self.orientation_xyzw, config);
        points.iter().all(|point| {
            let source_point = point.map(|value| value / config.collision_world_scale);
            world
                .raycast_chassis(source_point, [0.0; 3], 0.0)
                .is_none_or(|hit| {
                    !support_points_span_face(
                        &points,
                        &points,
                        world,
                        hit.triangle_index,
                        config.collision_world_scale,
                    ) || hit.penetration * config.collision_world_scale <= POSE_CLEARANCE_TOLERANCE
                })
        })
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
        resolve_chassis_motion(state, &mut next, config, collision, dt)?;
        if !next.is_finite() {
            return Err(VehicleSimulationError::NonFiniteResult);
        }
        *state = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct ChassisContact {
    fraction: f32,
    point: [f32; 3],
    normal: [f32; 3],
    penetration: f32,
    triangle_index: usize,
}

fn resolve_chassis_motion(
    previous: &VehicleState,
    next: &mut VehicleState,
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
    dt: f32,
) -> Result<(), VehicleSimulationError> {
    let mut position = previous.position;
    let mut orientation = previous.orientation_xyzw;
    let mut remaining = dt;
    let mut first_sweep = true;

    for _ in 0..MAX_CHASSIS_CONTACT_ITERATIONS {
        let target_position = if first_sweep {
            next.position
        } else {
            add(position, scale(next.linear_velocity, remaining))
        };
        let target_orientation = if first_sweep {
            next.orientation_xyzw
        } else {
            let mut target = orientation;
            integrate_orientation(&mut target, next.angular_velocity, remaining)?;
            target
        };
        let Some((fraction, contacts)) = sweep_chassis(
            position,
            orientation,
            target_position,
            target_orientation,
            config,
            collision,
        ) else {
            next.position = target_position;
            next.orientation_xyzw = target_orientation;
            return Ok(());
        };
        position = lerp3(position, target_position, fraction);
        orientation = interpolate_orientation(orientation, target_orientation, fraction);
        next.position = position;
        next.orientation_xyzw = orientation;

        let mut corrections: Vec<([f32; 3], f32)> = Vec::new();
        for contact in &contacts {
            if let Some((_, depth)) = corrections
                .iter_mut()
                .find(|(normal, _)| dot(*normal, contact.normal) > 0.99)
            {
                *depth = depth.max(contact.penetration);
            } else {
                corrections.push((contact.normal, contact.penetration));
            }
        }
        for (normal, penetration) in corrections {
            position = add(position, scale(normal, penetration + CHASSIS_SKIN));
        }
        next.position = position;

        for _ in 0..2 {
            for contact in &contacts {
                apply_chassis_impulse(next, config, contact.point, contact.normal);
                next.last_collision_triangle = Some(contact.triangle_index);
            }
        }

        remaining *= 1.0 - fraction;
        first_sweep = false;
        if remaining <= f32::EPSILON {
            next.position = position;
            next.orientation_xyzw = orientation;
            return Ok(());
        }
    }

    // Stop this fixed step at the last safe contact if several surfaces consume the solver budget.
    next.position = position;
    next.orientation_xyzw = orientation;
    Ok(())
}

// A one-sided overlap is real only while the hull still reaches the face's front side.
// Otherwise this point is behind an unrelated backface, not embedded in the triangle.
fn support_points_span_face(
    first: &[[f32; 3]],
    second: &[[f32; 3]],
    collision: &StaticCollisionWorld,
    triangle_index: usize,
    scale: f32,
) -> bool {
    let Some(triangle) = collision.triangles().get(triangle_index) else {
        return false;
    };
    triangle.two_sided
        || first.iter().chain(second).any(|point| {
            let source_point = point.map(|value| value / scale);
            dot(
                subtract(source_point, triangle.vertices[0]),
                triangle.normal,
            ) >= -1.0e-4
        })
}

fn sweep_chassis(
    from_position: [f32; 3],
    from_orientation: [f32; 4],
    to_position: [f32; 3],
    to_orientation: [f32; 4],
    config: &VehicleConfig,
    collision: &StaticCollisionWorld,
) -> Option<(f32, Vec<ChassisContact>)> {
    let rotation_dot = from_orientation
        .iter()
        .zip(to_orientation)
        .map(|(from, to)| from * to)
        .sum::<f32>()
        .abs()
        .clamp(-1.0, 1.0);
    let angle = 2.0 * rotation_dot.acos();
    let segments = (angle / MAX_ANGULAR_SWEEP_STEP)
        .ceil()
        .clamp(1.0, MAX_ANGULAR_SWEEP_SEGMENTS as f32) as usize;
    let mut earliest = f32::INFINITY;
    let mut contacts = Vec::new();

    for segment in 0..segments {
        let segment_start = segment as f32 / segments as f32;
        let segment_end = (segment + 1) as f32 / segments as f32;
        let start_position = lerp3(from_position, to_position, segment_start);
        let end_position = lerp3(from_position, to_position, segment_end);
        let start_orientation =
            interpolate_orientation(from_orientation, to_orientation, segment_start);
        let end_orientation =
            interpolate_orientation(from_orientation, to_orientation, segment_end);
        let start_points = chassis_support_points(start_position, start_orientation, config);
        let end_points = chassis_support_points(end_position, end_orientation, config);

        for (start, end) in start_points.into_iter().zip(end_points) {
            let source_start = start.map(|value| value / config.collision_world_scale);
            let source_delta =
                subtract(end, start).map(|value| value / config.collision_world_scale);
            let source_distance = dot(source_delta, source_delta).sqrt();
            let Some(hit) = collision.raycast_chassis(source_start, source_delta, source_distance)
            else {
                continue;
            };
            if hit.penetration > 0.0
                && !support_points_span_face(
                    &start_points,
                    &end_points,
                    collision,
                    hit.triangle_index,
                    config.collision_world_scale,
                )
            {
                continue;
            }
            if source_distance <= f32::EPSILON && hit.penetration <= f32::EPSILON {
                continue;
            }
            let local_fraction = if source_distance <= f32::EPSILON {
                0.0
            } else {
                (hit.distance / source_distance).clamp(0.0, 1.0)
            };
            let fraction = (segment as f32 + local_fraction) / segments as f32;
            if fraction < earliest - CHASSIS_TOI_WINDOW {
                contacts.clear();
                earliest = fraction;
            } else if fraction < earliest {
                earliest = fraction;
                contacts.retain(|contact: &ChassisContact| {
                    contact.fraction <= earliest + CHASSIS_TOI_WINDOW
                });
            }
            let point = hit.point.map(|value| value * config.collision_world_scale);
            if fraction <= earliest + CHASSIS_TOI_WINDOW
                && !contacts.iter().any(|contact| {
                    let separation = subtract(contact.point, point);
                    dot(contact.normal, hit.normal) > 0.99 && dot(separation, separation) < 1.0e-4
                })
            {
                contacts.push(ChassisContact {
                    fraction,
                    point,
                    normal: hit.normal,
                    penetration: hit.penetration * config.collision_world_scale,
                    triangle_index: hit.triangle_index,
                });
            }
        }
    }
    (!contacts.is_empty()).then_some((earliest.clamp(0.0, 1.0), contacts))
}

fn chassis_corners(
    position: [f32; 3],
    orientation: [f32; 4],
    config: &VehicleConfig,
) -> [[f32; 3]; 8] {
    std::array::from_fn(|index| {
        let local = std::array::from_fn(|axis| {
            let bound = if index & (1 << axis) == 0 { 0 } else { 1 };
            config.bounds[bound][axis] - config.center_of_mass[axis]
        });
        add(position, rotate(orientation, local))
    })
}

fn chassis_support_points(
    position: [f32; 3],
    orientation: [f32; 4],
    config: &VehicleConfig,
) -> [[f32; 3]; CHASSIS_SUPPORT_POINT_COUNT] {
    // ponytail: corners and face centers can miss small edge-only obstacles; add edge centers if track tests expose them.
    let corners = chassis_corners(position, orientation, config);
    let center: [f32; 3] =
        std::array::from_fn(|axis| (config.bounds[0][axis] + config.bounds[1][axis]) * 0.5);
    std::array::from_fn(|index| {
        if index < corners.len() {
            return corners[index];
        }
        let face = index - corners.len();
        let axis = face / 2;
        let bound = face % 2;
        let local = std::array::from_fn(|component| {
            let value = if component == axis {
                config.bounds[bound][component]
            } else {
                center[component]
            };
            value - config.center_of_mass[component]
        });
        add(position, rotate(orientation, local))
    })
}

fn apply_chassis_impulse(
    state: &mut VehicleState,
    config: &VehicleConfig,
    point: [f32; 3],
    normal: [f32; 3],
) {
    let lever = subtract(point, state.position);
    let point_velocity = add(state.linear_velocity, cross(state.angular_velocity, lever));
    let inward_speed = dot(point_velocity, normal);
    if inward_speed >= 0.0 {
        return;
    }
    let normal_moment = cross(lever, normal);
    let normal_denominator = 1.0 / config.mass
        + dot(
            normal_moment,
            inverse_inertia_world(state.orientation_xyzw, config, normal_moment),
        );
    if normal_denominator <= f32::EPSILON || !normal_denominator.is_finite() {
        return;
    }
    let normal_impulse = -inward_speed / normal_denominator;
    apply_impulse(state, config, lever, scale(normal, normal_impulse));

    let point_velocity = add(state.linear_velocity, cross(state.angular_velocity, lever));
    let tangent = subtract(point_velocity, scale(normal, dot(point_velocity, normal)));
    let tangent_speed = dot(tangent, tangent).sqrt();
    if tangent_speed <= f32::EPSILON {
        return;
    }
    let tangent_direction = scale(tangent, tangent_speed.recip());
    let tangent_moment = cross(lever, tangent_direction);
    let tangent_denominator = 1.0 / config.mass
        + dot(
            tangent_moment,
            inverse_inertia_world(state.orientation_xyzw, config, tangent_moment),
        );
    if tangent_denominator <= f32::EPSILON || !tangent_denominator.is_finite() {
        return;
    }
    let friction_impulse =
        (tangent_speed / tangent_denominator).min(CHASSIS_FRICTION * normal_impulse);
    apply_impulse(
        state,
        config,
        lever,
        scale(tangent_direction, -friction_impulse),
    );
}

fn apply_impulse(
    state: &mut VehicleState,
    config: &VehicleConfig,
    lever: [f32; 3],
    impulse: [f32; 3],
) {
    state.linear_velocity = add(state.linear_velocity, scale(impulse, 1.0 / config.mass));
    let angular_impulse = cross(lever, impulse);
    state.angular_velocity = add(
        state.angular_velocity,
        inverse_inertia_world(state.orientation_xyzw, config, angular_impulse),
    );
}

fn inverse_inertia_world(
    orientation: [f32; 4],
    config: &VehicleConfig,
    vector: [f32; 3],
) -> [f32; 3] {
    let local = inverse_rotate(orientation, vector);
    rotate(
        orientation,
        std::array::from_fn(|axis| local[axis] / config.principal_inertia[axis]),
    )
}

fn interpolate_orientation(from: [f32; 4], mut to: [f32; 4], fraction: f32) -> [f32; 4] {
    let dot = from.iter().zip(to).map(|(a, b)| a * b).sum::<f32>();
    if dot < 0.0 {
        to = to.map(|value| -value);
    }
    let mut result =
        std::array::from_fn(|index| from[index] + (to[index] - from[index]) * fraction);
    let length_squared = result.iter().map(|value| value * value).sum::<f32>();
    if length_squared > f32::MIN_POSITIVE {
        let inverse_length = length_squared.sqrt().recip();
        result.iter_mut().for_each(|value| *value *= inverse_length);
    }
    result
}

fn lerp3(from: [f32; 3], to: [f32; 3], fraction: f32) -> [f32; 3] {
    add(from, scale(subtract(to, from), fraction))
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
        VehicleSimulationSettings, VehicleState, chassis_corners,
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

    fn source(actor: &'static str, model: &'static str, face_index: usize) -> SurfaceIdentity {
        SurfaceIdentity {
            actor_path: Arc::from(actor),
            model: Arc::from(model),
            face_index,
            material: Some(Arc::from("WALL")),
        }
    }

    fn add_wall_x(
        world: &mut StaticCollisionWorld,
        x: f32,
        min_y: f32,
        max_y: f32,
        min_z: f32,
        max_z: f32,
    ) {
        let a = [x, min_y, min_z];
        let b = [x, max_y, min_z];
        let c = [x, max_y, max_z];
        let d = [x, min_y, max_z];
        world.add_triangle([a, c, b], 0, true, source("TEST/WALL_X", "WALL_X", 0));
        world.add_triangle([a, d, c], 0, true, source("TEST/WALL_X", "WALL_X", 1));
    }

    fn add_wall_z(
        world: &mut StaticCollisionWorld,
        z: f32,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) {
        let a = [min_x, min_y, z];
        let b = [max_x, min_y, z];
        let c = [max_x, max_y, z];
        let d = [min_x, max_y, z];
        world.add_triangle([a, b, c], 0, true, source("TEST/WALL_Z", "WALL_Z", 0));
        world.add_triangle([a, c, d], 0, true, source("TEST/WALL_Z", "WALL_Z", 1));
    }

    fn no_gravity_simulation(step_ms: u64) -> VehicleSimulation {
        VehicleSimulation::new(VehicleSimulationSettings {
            fixed_step: Duration::from_millis(step_ms),
            gravity: [0.0; 3],
        })
        .unwrap()
    }

    #[test]
    fn chassis_floor_impact_stops_before_crossing_and_keeps_surface_identity() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut state = VehicleState {
            position: [0.0, 5.0, 0.0],
            linear_velocity: [0.0, -60.0, 0.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &flat_ground())
            .unwrap();
        let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
        assert!(corners.iter().all(|corner| corner[1] >= -0.01), "{state:?}");
        assert!(state.linear_velocity[1] > -60.0, "{state:?}");
        assert!(state.last_collision_triangle.is_some());
    }

    #[test]
    fn chassis_clearance_detects_penetration_using_source_bounds() {
        let mut world = StaticCollisionWorld::default();
        let source = SurfaceIdentity {
            actor_path: Arc::from("TEST/WALL"),
            model: Arc::from("WALL"),
            face_index: 0,
            material: Some(Arc::from("WALL")),
        };
        let a = [0.0, -10.0, -10.0];
        let b = [0.0, 10.0, -10.0];
        let c = [0.0, 10.0, 10.0];
        let d = [0.0, -10.0, 10.0];
        world.add_triangle([a, b, c], 0, false, source.clone());
        world.add_triangle([a, c, d], 0, false, source);
        let config = vehicle_config();
        let embedded = VehicleState {
            position: [-1.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        let clear = VehicleState {
            position: [3.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        assert!(!embedded.is_chassis_clear(&config, &world));
        assert!(clear.is_chassis_clear(&config, &world));
    }

    #[test]
    fn ignores_one_sided_faces_wholly_behind_the_chassis() {
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        let wall = source("TEST/WALL", "WALL", 0);
        let a = [0.0, -200.0, -200.0];
        let b = [0.0, 200.0, -200.0];
        let c = [0.0, 200.0, 200.0];
        let d = [0.0, -200.0, 200.0];
        world.add_triangle([a, c, b], 0, false, wall.clone());
        world.add_triangle([a, d, c], 0, false, wall);

        let behind = VehicleState {
            position: [10.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        assert!(behind.is_chassis_clear(&config, &world));
        let mut moving = VehicleState {
            linear_velocity: [1.0, 0.0, 0.0],
            ..behind
        };
        no_gravity_simulation(40)
            .step_fixed(&mut moving, &config, DriverInput::default(), &world)
            .unwrap();
        assert!(moving.position[0] > behind.position[0]);
        assert_eq!(moving.last_collision_triangle, None);

        let straddling = VehicleState::default();
        assert!(!straddling.is_chassis_clear(&config, &world));
    }

    #[test]
    fn chassis_face_center_hits_a_narrow_floor_patch() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut state = VehicleState {
            position: [0.0, 5.0, 0.0],
            linear_velocity: [0.0, -60.0, 0.0],
            ..VehicleState::default()
        };
        let patch = ground(-1.0, 1.0, -1.0, 1.0);
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &patch)
            .unwrap();
        assert!(state.last_collision_triangle.is_some());
        assert!(state.position[1] >= -0.01, "{state:?}");
        assert!(state.is_finite());
    }

    #[test]
    fn chassis_wall_impact_has_angular_response_and_surface_identity() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        let mut state = VehicleState {
            position: [-8.0, 10.0, 0.0],
            linear_velocity: [100.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &world)
            .unwrap();
        let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
        assert!(corners.iter().all(|corner| corner[0] <= 0.01), "{state:?}");
        assert!(state.linear_velocity[0] < 20.0, "{state:?}");
        assert!(
            state
                .angular_velocity
                .iter()
                .any(|value| value.abs() > 0.01)
        );
        let triangle = &world.triangles()[state.last_collision_triangle.unwrap()];
        assert_eq!(triangle.source.model.as_ref(), "WALL_X");
    }

    #[test]
    fn glancing_chassis_contact_keeps_tangent_motion() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        let mut state = VehicleState {
            position: [-8.0, 10.0, 0.0],
            linear_velocity: [100.0, 0.0, -10.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &world)
            .unwrap();
        assert!(state.linear_velocity[0] < 20.0, "{state:?}");
        assert!(state.linear_velocity[2] < -1.0, "{state:?}");
        assert!(state.is_finite());
    }

    #[test]
    fn simultaneous_corner_contacts_stop_motion_into_both_walls() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        add_wall_z(&mut world, 0.0, -20.0, 20.0, 0.0, 30.0);
        let mut state = VehicleState {
            position: [-7.0, 10.0, -7.0],
            linear_velocity: [50.0, 0.0, 40.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &world)
            .unwrap();
        let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
        assert!(corners.iter().all(|corner| corner[0] <= 0.01), "{state:?}");
        assert!(corners.iter().all(|corner| corner[2] <= 0.01), "{state:?}");
        assert!(state.is_finite());
    }

    #[test]
    fn chassis_can_rest_next_to_a_wall_after_impact() {
        let simulation = no_gravity_simulation(40);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        let mut state = VehicleState {
            position: [-2.1, 10.0, 0.0],
            linear_velocity: [5.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        for _ in 0..100 {
            simulation
                .step_fixed(&mut state, &config, DriverInput::default(), &world)
                .unwrap();
            assert!(state.is_finite());
        }
        let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
        assert!(corners.iter().all(|corner| corner[0] <= 0.02), "{state:?}");
        assert!(state.linear_velocity[0].abs() < 0.1, "{state:?}");
    }

    #[test]
    fn high_speed_chassis_sweep_does_not_tunnel_through_a_wall() {
        let simulation = no_gravity_simulation(100);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        let mut state = VehicleState {
            position: [-10.0, 10.0, 0.0],
            linear_velocity: [250.0, 0.0, 0.0],
            ..VehicleState::default()
        };
        simulation
            .step_fixed(&mut state, &config, DriverInput::default(), &world)
            .unwrap();
        let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
        assert!(corners.iter().all(|corner| corner[0] <= 0.02), "{state:?}");
        assert!(state.is_finite());
    }

    #[test]
    fn repeated_wall_contacts_remain_finite_and_nonpenetrating() {
        let simulation = no_gravity_simulation(40);
        let config = vehicle_config();
        let mut world = StaticCollisionWorld::default();
        add_wall_x(&mut world, 0.0, 0.0, 30.0, -20.0, 20.0);
        let mut state = VehicleState {
            position: [-2.1, 10.0, 0.0],
            ..VehicleState::default()
        };
        for _ in 0..500 {
            state.linear_velocity[0] = 1.0;
            simulation
                .step_fixed(&mut state, &config, DriverInput::default(), &world)
                .unwrap();
            assert!(state.is_finite(), "{state:?}");
            let corners = chassis_corners(state.position, state.orientation_xyzw, &config);
            assert!(corners.iter().all(|corner| corner[0] <= 0.03), "{state:?}");
        }
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
            position: [0.0, 0.2, 0.0],
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
