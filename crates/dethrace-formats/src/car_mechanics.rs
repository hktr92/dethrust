//! M2 mechanics subset of resolution-independent CARS/*.TXT definitions.
//!
//! Geometry values remain in source units. The original loader applies its
//! 6.9 world scale and 47.61 inertia scale after reading this section.

use crate::game_text::{DataLine, GameText, TextError};

const MECHANICS_HEADER: &str = "START OF MECHANICS STUFF";
const WORLD_SCALE: f32 = 6.9;

#[derive(Debug, Clone, PartialEq)]
pub struct CarMechanicsSpec {
    pub version: u8,
    /// Four loader-order wheel positions in original source units.
    pub wheel_positions: [[f32; 3]; 4],
    pub center_of_mass: [f32; 3],
    /// Source bounds as [minimum, maximum], before the principal ACT offset.
    pub bounds: [[f32; 3]; 2],
    pub extra_points: Vec<[f32; 3]>,
    /// Raw mechanics-section value; the loader inverts it before scaling.
    pub maximum_curve_radius: f32,
    /// Original loader result: 1 / maximum_curve_radius / WORLD_SCALE.
    pub maximum_curvature: f32,
    /// Runtime slot order [rear, front]; the source pair is read into [front, rear].
    pub suspension_give: [f32; 2],
    /// Read from the file, although the original loader replaces it with bounds.min.y + 0.01.
    pub source_ride_height: f32,
    pub damping: f32,
    pub mass: f32,
    pub force_reduction: f32,
    /// Source order: front, rear, compression.
    pub grip_angles_degrees: [f32; 3],
    /// Source order: width, height, length.
    pub body_dimensions: [f32; 3],
    pub friction_ellipticity: f32,
    pub downforce_speed: f32,
    /// Values after the original versioned scale and mass multipliers.
    pub initial_brake: f32,
    pub brake_increase: f32,
    /// Source order: front, rear. These are rolling resistance values.
    pub rolling_resistance: [f32; 2],
    pub max_gears: i32,
    /// Raw speed input from the mechanics section.
    pub engine_speed: f32,
    /// Raw force input from the mechanics section.
    pub engine_force: f32,
    /// Original loader derived drivetrain ratios.
    pub speed_revs_ratio: f32,
    pub force_torque_ratio: f32,
    /// Principal inertia calculated from source dimensions, before the loader's 47.61 scale.
    pub principal_inertia: [f32; 3],
}

fn field_line(text: &mut GameText, field: &str) -> Result<DataLine, TextError> {
    text.next_line().map_err(|error| {
        TextError::new(
            error.line,
            format!("truncated mechanics section while reading {field}"),
        )
    })
}

fn values<const N: usize>(
    text: &mut GameText,
    field: &str,
) -> Result<(usize, [f32; N]), TextError> {
    let line = field_line(text, field)?;
    let mut tokens = line
        .text
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|token| !token.is_empty());
    let mut result = [0.0; N];
    for (index, value) in result.iter_mut().enumerate() {
        let token = tokens.next().ok_or_else(|| {
            TextError::new(
                line.number,
                format!("expected {N} {field} values, found {index}"),
            )
        })?;
        *value = token
            .parse::<f32>()
            .map_err(|_| TextError::new(line.number, format!("invalid {field} value '{token}'")))?;
        if !value.is_finite() {
            return Err(TextError::new(line.number, format!("non-finite {field}")));
        }
    }
    if tokens.next().is_some() {
        return Err(TextError::new(
            line.number,
            format!("too many {field} values"),
        ));
    }
    Ok((line.number, result))
}

fn integer(text: &mut GameText, field: &str) -> Result<(usize, i32), TextError> {
    let line = field_line(text, field)?;
    let mut tokens = line
        .text
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|token| !token.is_empty());
    let token = tokens
        .next()
        .ok_or_else(|| TextError::new(line.number, format!("missing {field}")))?;
    let value = token
        .parse::<i32>()
        .map_err(|_| TextError::new(line.number, format!("invalid {field} '{token}'")))?;
    if tokens.next().is_some() {
        return Err(TextError::new(
            line.number,
            format!("too many {field} values"),
        ));
    }
    Ok((line.number, value))
}

fn positive(line: usize, value: f32, field: &str) -> Result<(), TextError> {
    if value <= 0.0 {
        return Err(TextError::new(line, format!("{field} must be positive")));
    }
    Ok(())
}

fn nonnegative(line: usize, value: f32, field: &str) -> Result<(), TextError> {
    if value < 0.0 {
        return Err(TextError::new(line, format!("{field} must be nonnegative")));
    }
    Ok(())
}

fn section_version(text: &mut GameText) -> Result<u8, TextError> {
    loop {
        let line = text
            .next_line()
            .map_err(|error| TextError::new(error.line, "mechanics section not found"))?;
        if !line
            .text
            .get(..MECHANICS_HEADER.len())
            .is_some_and(|header| header.eq_ignore_ascii_case(MECHANICS_HEADER))
        {
            continue;
        }
        let version = line
            .text
            .split_whitespace()
            .last()
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or_else(|| TextError::new(line.number, "invalid mechanics version"))?;
        if !(1..=4).contains(&version) {
            return Err(TextError::new(
                line.number,
                format!("unsupported mechanics version {version}"),
            ));
        }
        return Ok(version);
    }
}

impl CarMechanicsSpec {
    /// Finds and parses the counted mechanics block in a complete car definition.
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut text = GameText::parse(bytes)?;
        let version = section_version(&mut text)?;

        let mut wheel_positions = [[0.0; 3]; 4];
        for (index, position) in wheel_positions.iter_mut().enumerate() {
            *position = values(&mut text, &format!("wheel position {index}"))?.1;
        }
        let center_of_mass = values(&mut text, "centre of mass")?.1;

        if version < 3 {
            let _legacy_point_count = integer(&mut text, "legacy extra point count")?;
        }
        let (min_line, min_bounds) = values(&mut text, "minimum bounds")?;
        let (max_line, max_bounds) = values(&mut text, "maximum bounds")?;
        for axis in 0..3 {
            if min_bounds[axis] > max_bounds[axis] {
                return Err(TextError::new(
                    max_line,
                    format!(
                        "maximum bound is below minimum on axis {axis} (minimum line {min_line})"
                    ),
                ));
            }
        }

        let mut extra_points = Vec::new();
        if version >= 3 {
            let (count_line, count) = integer(&mut text, "extra point count")?;
            if !(0..=6).contains(&count) {
                return Err(TextError::new(
                    count_line,
                    format!("extra point count {count} is outside 0..=6"),
                ));
            }
            extra_points.reserve(count as usize);
            for index in 0..count {
                extra_points.push(values(&mut text, &format!("extra point {index}"))?.1);
            }
        }

        let (curve_line, [maximum_curve_radius]) = values(&mut text, "maximum curve radius")?;
        positive(curve_line, maximum_curve_radius, "maximum curve radius")?;
        let maximum_curvature = 1.0 / maximum_curve_radius / WORLD_SCALE;
        if !maximum_curvature.is_finite() {
            return Err(TextError::new(curve_line, "invalid maximum curvature"));
        }

        let (give_line, source_give) = values::<2>(&mut text, "suspension give")?;
        positive(give_line, source_give[0], "front suspension give")?;
        positive(give_line, source_give[1], "rear suspension give")?;
        let suspension_give = [source_give[1], source_give[0]];
        let source_ride_height = values::<1>(&mut text, "ride height")?.1[0];
        let (damping_line, [damping]) = values(&mut text, "suspension damping")?;
        nonnegative(damping_line, damping, "suspension damping")?;
        let (mass_line, [mass]) = values(&mut text, "mass")?;
        positive(mass_line, mass, "mass")?;
        let (reduction_line, [force_reduction]) = values(&mut text, "force reduction")?;
        nonnegative(reduction_line, force_reduction, "force reduction")?;

        let (grip_line, grip_angles_degrees) = if version < 4 {
            let (line, [front, rear]) = values(&mut text, "front and rear grip angles")?;
            (line, [front, rear, rear])
        } else {
            values(&mut text, "front, rear, and compression grip angles")?
        };
        if grip_angles_degrees
            .iter()
            .any(|angle| *angle <= 0.0 || *angle >= 90.0)
        {
            return Err(TextError::new(
                grip_line,
                "grip angles must be between 0 and 90 degrees",
            ));
        }

        let (dimensions_line, body_dimensions) = values(&mut text, "body dimensions")?;
        for dimension in body_dimensions {
            positive(dimensions_line, dimension, "body dimension")?;
        }

        let mut friction_ellipticity = 1.0;
        let mut downforce_speed = 2000.0;
        let mut initial_brake = mass * 12.0;
        let mut brake_increase = mass * 12.0;
        if version >= 2 {
            let (line, [value]) = values(&mut text, "friction ellipticity")?;
            positive(line, value, "friction ellipticity")?;
            friction_ellipticity = value;
            let (line, [value]) = values(&mut text, "downforce speed")?;
            positive(line, value, "downforce speed")?;
            downforce_speed = value;
            let (line, [value]) = values(&mut text, "initial brake multiplier")?;
            nonnegative(line, value, "initial brake multiplier")?;
            initial_brake = value * mass * 12.0;
            let (line, [value]) = values(&mut text, "brake increase multiplier")?;
            nonnegative(line, value, "brake increase multiplier")?;
            brake_increase = value * mass * 12.0;
        }

        let (rolling_line, rolling_resistance) =
            values(&mut text, "front and rear rolling resistance")?;
        for value in rolling_resistance {
            nonnegative(rolling_line, value, "rolling resistance")?;
        }
        let (gear_line, max_gears) = integer(&mut text, "maximum gear")?;
        if max_gears <= 0 {
            return Err(TextError::new(gear_line, "maximum gear must be positive"));
        }
        let (speed_line, [engine_speed]) = values(&mut text, "engine speed")?;
        positive(speed_line, engine_speed, "engine speed")?;
        let (force_line, [engine_force]) = values(&mut text, "engine force")?;
        positive(force_line, engine_force, "engine force")?;

        let end = text.next_line().map_err(|error| {
            TextError::new(
                error.line,
                "truncated mechanics section before its end marker",
            )
        })?;
        if end.text != "END OF MECHANICS STUFF" {
            return Err(TextError::new(
                end.number,
                format!("expected END OF MECHANICS STUFF, found '{}'", end.text),
            ));
        }

        let [width, height, length] = body_dimensions;
        let principal_inertia = [
            mass * (length * length + height * height) / 12.0,
            mass * (length * length + width * width) / 12.0,
            mass * (height * height + width * width) / 12.0,
        ];
        let speed_revs_ratio = (engine_speed * 4.0 / 9.0) / max_gears as f32 / 6000.0;
        let force_torque_ratio = max_gears as f32 * mass * engine_force;
        if !initial_brake.is_finite()
            || !brake_increase.is_finite()
            || principal_inertia.iter().any(|value| !value.is_finite())
            || !speed_revs_ratio.is_finite()
            || !force_torque_ratio.is_finite()
        {
            return Err(TextError::new(
                end.number,
                "mechanics derived value is non-finite",
            ));
        }

        Ok(Self {
            version,
            wheel_positions,
            center_of_mass,
            bounds: [min_bounds, max_bounds],
            extra_points,
            maximum_curve_radius,
            maximum_curvature,
            suspension_give,
            source_ride_height,
            damping,
            mass,
            force_reduction,
            grip_angles_degrees,
            body_dimensions,
            friction_ellipticity,
            downforce_speed,
            initial_brake,
            brake_increase,
            rolling_resistance,
            max_gears,
            engine_speed,
            engine_force,
            speed_revs_ratio,
            force_torque_ratio,
            principal_inertia,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CarMechanicsSpec;

    const VERSION_4: &str = "prefix\nSTART OF MECHANICS STUFF version 4\n-1,0,2\n1,0,2\n-1,0,-2\n1,0,-2\n0,0,0\n-2,-1,-3\n2,1,3\n1\n0,1,0\n0.5\n0.2,0.3\n0.4\n0.5\n1000\n0.8\n80,75,85\n2,1,4\n1.5\n2000\n1.25\n0.75\n0.05,0.06\n6\n200\n4\nEND OF MECHANICS STUFF\n";

    #[test]
    fn parses_version_four_fields_and_loader_derivations() {
        let spec = CarMechanicsSpec::parse(VERSION_4.as_bytes()).unwrap();
        assert_eq!(spec.version, 4);
        assert_eq!(spec.wheel_positions[0], [-1.0, 0.0, 2.0]);
        assert_eq!(spec.suspension_give, [0.3, 0.2]);
        assert_eq!(spec.grip_angles_degrees, [80.0, 75.0, 85.0]);
        assert_eq!(spec.principal_inertia, [1416.6666, 1666.6666, 416.66666]);
        assert_eq!(spec.initial_brake, 15_000.0);
        assert_eq!(spec.brake_increase, 9_000.0);
        assert_eq!(spec.max_gears, 6);
        assert_eq!(spec.speed_revs_ratio, 200.0 * 4.0 / 9.0 / 6.0 / 6000.0);
        assert_eq!(spec.force_torque_ratio, 24_000.0);
    }

    #[test]
    fn handles_legacy_version_one_defaults_and_field_order() {
        let data = b"START OF MECHANICS STUFF version 1\n-1,0,2\n1,0,2\n-1,0,-2\n1,0,-2\n0,0,0\n0\n-2,-1,-3\n2,1,3\n2\n0.1,0.2\n0.4\n0.5\n500\n1\n70,65\n2,1,4\n0.02,0.03\n4\n180\n3\nEND OF MECHANICS STUFF\n";
        let spec = CarMechanicsSpec::parse(data).unwrap();
        assert_eq!(spec.version, 1);
        assert!(spec.extra_points.is_empty());
        assert_eq!(spec.suspension_give, [0.2, 0.1]);
        assert_eq!(spec.grip_angles_degrees, [70.0, 65.0, 65.0]);
        assert_eq!(spec.friction_ellipticity, 1.0);
        assert_eq!(spec.downforce_speed, 2000.0);
        assert_eq!(spec.initial_brake, 6000.0);
        assert_eq!(spec.brake_increase, 6000.0);
        assert_eq!(spec.max_gears, 4);
    }

    #[test]
    fn rejects_bad_counts_angles_and_unusable_dimensions() {
        let bad_count = VERSION_4.replace("\n1\n0,1,0\n", "\n7\n");
        assert!(
            CarMechanicsSpec::parse(bad_count.as_bytes())
                .unwrap_err()
                .detail
                .contains("extra point count")
        );

        let bad_angle = VERSION_4.replace("80,75,85", "90,75,85");
        assert!(
            CarMechanicsSpec::parse(bad_angle.as_bytes())
                .unwrap_err()
                .detail
                .contains("grip angles")
        );

        let bad_mass = VERSION_4.replace("\n1000\n", "\n0\n");
        assert!(
            CarMechanicsSpec::parse(bad_mass.as_bytes())
                .unwrap_err()
                .detail
                .contains("mass")
        );
    }

    #[test]
    fn reports_missing_and_truncated_mechanics_sections() {
        assert!(
            CarMechanicsSpec::parse(b"no mechanics here\n")
                .unwrap_err()
                .detail
                .contains("section not found")
        );

        let truncated = VERSION_4.replace("END OF MECHANICS STUFF\n", "");
        assert!(
            CarMechanicsSpec::parse(truncated.as_bytes())
                .unwrap_err()
                .detail
                .contains("truncated mechanics section")
        );
    }
}
