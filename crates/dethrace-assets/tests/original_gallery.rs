use std::path::PathBuf;

use bevy::prelude::*;
use dethrace_assets::{GameDir, brender::GallerySources};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn prepares_six_data_resolved_maim_street_gallery_cars() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let dir = GameDir::new(root).unwrap();
    let gallery = GallerySources::load(&dir, "Maim Street", 1).unwrap();
    assert_eq!(gallery.entries.len(), 6);
    assert_eq!(gallery.entries[0].name, "Max Damage");
    for entry in gallery.entries {
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        entry
            .scene
            .prepare(&mut images, &mut materials, &mut meshes)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.name));
        assert!(entry.radius > 0.0 && !meshes.is_empty());
    }
}
