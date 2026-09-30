use std::path::PathBuf;

use dethrace_formats::{car_mechanics::CarMechanicsSpec, race::initial_player};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn parses_original_starting_car_mechanics() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/CARS").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    let general = std::fs::read(data.join("GENERAL.TXT")).unwrap();
    let (_, car_file) = initial_player(&general).unwrap();
    assert!(car_file.eq_ignore_ascii_case("BLKEAGLE.TXT"));

    let path = data.join("CARS").join(&car_file);
    let bytes = std::fs::read(&path).unwrap();
    let mechanics = CarMechanicsSpec::parse(&bytes)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

    assert_eq!(mechanics.version, 4);
    assert_eq!(mechanics.extra_points.len(), 2);
    assert!(mechanics.mass.is_finite() && mechanics.mass > 0.0);
    assert!(
        mechanics
            .wheel_positions
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    assert!(mechanics.body_dimensions.iter().all(|value| *value > 0.0));
    assert_eq!(
        mechanics.principal_inertia[0],
        mechanics.mass
            * (mechanics.body_dimensions[2].powi(2) + mechanics.body_dimensions[1].powi(2))
            / 12.0
    );
    assert_eq!(
        mechanics.speed_revs_ratio,
        mechanics.engine_speed * 4.0 / 9.0 / mechanics.max_gears as f32 / 6000.0
    );
    assert_eq!(
        mechanics.force_torque_ratio,
        mechanics.max_gears as f32 * mechanics.mass * mechanics.engine_force
    );
}
