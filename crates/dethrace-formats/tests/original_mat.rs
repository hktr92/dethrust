use std::path::PathBuf;

use dethrace_formats::mat::MatFile;

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn parses_original_car_and_maim_street_materials() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/MATERIAL").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    for name in [
        "EAGBLAK.MAT",
        "CITYA2.MAT",
        "GRIDDY.MAT",
        "STADY.MAT",
        "WATTY.MAT",
    ] {
        let path = data.join("MATERIAL").join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mats = MatFile::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!mats.materials.is_empty(), "{}", path.display());
        assert!(mats.materials.iter().all(|mat| !mat.identifier.is_empty()));
        if name == "CITYA2.MAT" || name == "EAGBLAK.MAT" {
            assert!(mats.materials.iter().any(|mat| mat.colour_map.is_some()));
        }
    }
}
