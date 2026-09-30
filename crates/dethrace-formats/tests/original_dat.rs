use dethrace_formats::dat::DatFile;
use std::path::PathBuf;

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn parses_original_car_and_maim_street_models() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/MODELS").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    for name in ["EAGBLAK.DAT", "AGENTO.DAT", "CITYANW1.DAT", "CITYA2.DAT"] {
        let path = data.join("MODELS").join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let dat = DatFile::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!dat.models.is_empty(), "{}", path.display());
        assert!(
            dat.models
                .iter()
                .all(|m| !m.vertices.is_empty() && !m.faces.is_empty())
        );
    }
}
