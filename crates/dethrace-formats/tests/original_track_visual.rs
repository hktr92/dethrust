use std::{env, fs, path::Path};

use dethrace_formats::{race::RaceCatalog, track_visual::TrackVisualSpec};

fn has_file(data: &Path, folders: &[&str], name: &str) -> bool {
    folders.iter().any(|folder| {
        fs::read_dir(data.join(folder)).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(name)
            })
        })
    })
}

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn resolves_maim_street_normal_visual_dependencies() {
    let root = env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR");
    let root = Path::new(&root);
    let data = if root.join("DATA").is_dir() {
        root.join("DATA")
    } else {
        root.to_path_buf()
    };
    let races = RaceCatalog::parse(&fs::read(data.join("RACES.TXT")).unwrap()).unwrap();
    let (_, race) = races.find("Maim Street").expect("Maim Street race");
    let path = data.join("RACES").join(&race.track_file);
    let spec = TrackVisualSpec::parse(&fs::read(&path).unwrap(), &race.track_file).unwrap();
    assert_eq!(spec.track_file.to_ascii_uppercase(), "CITYA1.TXT");
    assert_eq!(spec.start_position, [159.94, -8.5, -225.67]);
    assert_eq!(spec.pixelmaps.len(), 5);
    assert_eq!(spec.materials.len(), 8);
    assert_eq!(spec.models, ["CITYANW1.DAT"]);
    assert_eq!(spec.actor_file, "CITYANW1.ACT");
    for file in &spec.pixelmaps {
        assert!(
            has_file(&data, &["REG/PIXELMAP", "PIXELMAP"], file),
            "missing PIX {file}"
        );
    }
    for file in &spec.materials {
        assert!(has_file(&data, &["MATERIAL"], file), "missing MAT {file}");
    }
    for file in &spec.models {
        assert!(has_file(&data, &["MODELS"], file), "missing DAT {file}");
    }
    assert!(
        has_file(&data, &["ACTORS"], &spec.actor_file),
        "missing ACT {}",
        spec.actor_file
    );
}
