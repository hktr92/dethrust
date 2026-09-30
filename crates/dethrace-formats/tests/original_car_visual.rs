use dethrace_formats::car_visual::{CarVisualSpec, VisualVariant};
use std::path::{Path, PathBuf};

fn exists_ignore_case(dir: &Path, name: &str) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        })
    })
}

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn resolves_starting_car_and_opponent_visual_files() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/CARS").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    for car in [
        "BLKEAGLE.TXT",
        "AGENTO.TXT",
        "ANNIECAR.TXT",
        "ED.TXT",
        "GRIMM.TXT",
    ] {
        let path = data.join("CARS").join(car);
        let spec = CarVisualSpec::parse(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(spec.definition.eq_ignore_ascii_case(car));
        assert!(!spec.models.is_empty() && !spec.actors.is_empty());
        for name in spec.pixelmaps_for(VisualVariant::Low) {
            assert!(
                exists_ignore_case(&data.join("REG/PIXELMAP"), name)
                    || exists_ignore_case(&data.join("PIXELMAP"), name),
                "{car}: missing PIX {name}"
            );
        }
        for name in spec.materials_for(VisualVariant::Low) {
            assert!(
                exists_ignore_case(&data.join("MATERIAL"), name),
                "{car}: missing MAT {name}"
            );
        }
        for name in &spec.models {
            assert!(
                exists_ignore_case(&data.join("MODELS"), name),
                "{car}: missing DAT {name}"
            );
        }
        for actor in &spec.actors {
            assert!(
                exists_ignore_case(&data.join("ACTORS"), &actor.file),
                "{car}: missing ACT {}",
                actor.file
            );
        }
    }
}
