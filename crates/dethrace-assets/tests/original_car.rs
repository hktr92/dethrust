use std::path::PathBuf;

use bevy::prelude::*;
use dethrace_assets::{GameDir, brender::VisualScene};
use dethrace_formats::car_visual::{CarVisualSpec, VisualVariant};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn prepares_original_starting_car_geometry_and_materials() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let dir = GameDir::new(root).unwrap();
    let path = dir.data_path("CARS/BLKEAGLE.TXT").unwrap();
    let spec = CarVisualSpec::parse(&std::fs::read(&path).unwrap()).unwrap();
    let scene = VisualScene::car(&dir, &spec, VisualVariant::Low).unwrap();
    assert!(!scene.actor.roots.is_empty());
    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut meshes = Assets::<Mesh>::default();
    scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .unwrap();
    assert!(!images.is_empty() && !materials.is_empty() && !meshes.is_empty());
}
