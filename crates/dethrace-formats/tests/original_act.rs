use dethrace_formats::{act::ActFile, dat::DatFile, mat::MatFile};
use std::path::PathBuf;

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn parses_and_links_original_car_and_maim_street_actors() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/ACTORS").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    for (act_name, dat_name) in [
        ("EAGBLAK.ACT", "EAGBLAK.DAT"),
        ("AGENTO.ACT", "AGENTO.DAT"),
        ("CITYANW1.ACT", "CITYANW1.DAT"),
    ] {
        let act_path = data.join("ACTORS").join(act_name);
        let act = ActFile::parse(&std::fs::read(&act_path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", act_path.display()));
        assert_eq!(act.roots.len(), 1);
        let dat_path = data.join("MODELS").join(dat_name);
        let dat = DatFile::parse(&std::fs::read(&dat_path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", dat_path.display()));
        let mat_path = data.join("MATERIAL").join(dat_name.replace(".DAT", ".MAT"));
        let mat = mat_path
            .exists()
            .then(|| MatFile::parse(&std::fs::read(&mat_path).unwrap()).unwrap());
        let mats: Vec<&MatFile> = mat.iter().collect();
        act.link(&[&dat], &mats)
            .unwrap_or_else(|e| panic!("{}: {e}", act_path.display()));
    }
}
