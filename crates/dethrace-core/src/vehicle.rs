use std::time::Duration;

const MAX_STEPS_PER_UPDATE: u32 = 5;

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
}

impl VehicleConfig {
    pub fn is_valid(&self) -> bool {
        self.mass.is_finite()
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
    }
}

/// Simulation pose and velocities. Orientation is a unit quaternion in [x, y, z, w] order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleState {
    pub position: [f32; 3],
    pub orientation_xyzw: [f32; 4],
    pub linear_velocity: [f32; 3],
    /// World-space angular velocity in radians per second.
    pub angular_velocity: [f32; 3],
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
        }
    }
}

impl VehicleState {
    pub fn is_finite(&self) -> bool {
        self.position.iter().all(|value| value.is_finite())
            && self.orientation_xyzw.iter().all(|value| value.is_finite())
            && self.linear_velocity.iter().all(|value| value.is_finite())
            && self.angular_velocity.iter().all(|value| value.is_finite())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSimulationSettings {
    pub fixed_step: Duration,
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
            self.step_fixed(state, config, input)?;
            self.accumulator -= self.settings.fixed_step;
            steps += 1;
        }
        Ok(steps)
    }

    /// Runs one mechanics update without reading render-frame time.
    pub fn step_fixed(
        &self,
        state: &mut VehicleState,
        config: &VehicleConfig,
        input: DriverInput,
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
        let mut next = *state;
        for axis in 0..3 {
            next.linear_velocity[axis] += self.settings.gravity[axis] * dt;
            next.position[axis] += next.linear_velocity[axis] * dt;
        }
        integrate_orientation(&mut next.orientation_xyzw, next.angular_velocity, dt)?;
        if !next.is_finite() {
            return Err(VehicleSimulationError::NonFiniteResult);
        }
        *state = next;
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::{
        DriverInput, VehicleConfig, VehicleSimulation, VehicleSimulationError,
        VehicleSimulationSettings, VehicleState,
    };
    use std::time::Duration;

    fn vehicle_config() -> VehicleConfig {
        VehicleConfig {
            mass: 1000.0,
            center_of_mass: [0.0; 3],
            principal_inertia: [1.0; 3],
            wheel_positions: [[0.0; 3]; 4],
            bounds: [[-1.0; 3], [1.0; 3]],
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
                .step_fixed(&mut state, &vehicle_config(), input)
                .unwrap_err(),
            VehicleSimulationError::InvalidInput
        );
        assert_eq!(state, VehicleState::default());
    }

    #[test]
    fn vehicle_config_rejects_invalid_mass_inertia_and_bounds() {
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
    }

    #[test]
    fn fixed_step_integrates_gravity_and_angular_velocity() {
        let simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState {
            angular_velocity: [0.0, 1.0, 0.0],
            ..VehicleState::default()
        };

        simulation
            .step_fixed(&mut state, &vehicle_config(), DriverInput::default())
            .unwrap();

        assert!((state.linear_velocity[1] + 0.4).abs() < 0.000001);
        assert!((state.position[1] + 0.016).abs() < 0.000001);
        assert!((state.orientation_xyzw[1] - 0.019996).abs() < 0.00001);
        assert!((state.orientation_xyzw[3] - 0.9998).abs() < 0.00001);
    }

    #[test]
    fn caps_catch_up_steps_and_keeps_the_remaining_time() {
        let mut simulation = VehicleSimulation::new(VehicleSimulationSettings::default()).unwrap();
        let mut state = VehicleState::default();

        assert_eq!(
            simulation
                .advance_frame(
                    Duration::from_millis(240),
                    &mut state,
                    &vehicle_config(),
                    DriverInput::default(),
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
                .step_fixed(&mut state, &vehicle_config(), DriverInput::default())
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
                .step_fixed(&mut state, &vehicle_config(), DriverInput::default())
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
        for frame in [20, 60, 160, 240] {
            let elapsed = Duration::from_millis(frame);
            first
                .advance_frame(
                    elapsed,
                    &mut first_state,
                    &vehicle_config(),
                    DriverInput::default(),
                )
                .unwrap();
            second
                .advance_frame(
                    elapsed,
                    &mut second_state,
                    &vehicle_config(),
                    DriverInput::default(),
                )
                .unwrap();
        }
        assert_eq!(first_state, second_state);
    }
}
