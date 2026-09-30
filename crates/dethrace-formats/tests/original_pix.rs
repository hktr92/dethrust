use std::path::PathBuf;

use dethrace_formats::pix::{PixFile, PixelType};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn parses_original_car_track_and_palette_pix() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/PIXELMAP").is_dir() {
        root.join("DATA")
    } else {
        root
    };
    for name in ["EAGBLAK.PIX", "CITYA2.PIX", "NYSKY1.PIX"] {
        let path = data.join("PIXELMAP").join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let file = PixFile::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!file.pixelmaps.is_empty(), "{}", path.display());
        assert!(
            file.pixelmaps
                .iter()
                .all(|map| map.pixel_type == PixelType::Index8)
        );
    }
    let path = data.join("REG/PALETTES/DRRENDER.PAL");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let file = PixFile::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(file.pixelmaps[0].pixel_type, PixelType::Rgbx888);
    assert_eq!(file.pixelmaps[0].pixels.len(), 1024);
}
