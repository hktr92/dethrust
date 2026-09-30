use std::path::PathBuf;

use dethrace_formats::flic::{Flic, FlicDecoder};

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn decodes_original_fresh_boot_menu_flics() {
    let game_dir = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let anim = if game_dir.join("DATA/ANIM").is_dir() {
        game_dir.join("DATA/ANIM")
    } else {
        game_dir.join("ANIM")
    };
    for name in [
        "MAINSTIL.FLI",
        "MAI2STIL.FLI",
        "MAI2COME.FLI",
        "MAI2AWAY.FLI",
        "MAI2N1FL.FLI",
        "MAI2N1GL.FLI",
        "MAI2NNFL.FLI",
        "MAI2NNGL.FLI",
        "MAI2OPFL.FLI",
        "MAI2OPGL.FLI",
        "MAI2LDFL.FLI",
        "MAI2LDGL.FLI",
        "MAI2QTFL.FLI",
        "MAI2QTGL.FLI",
    ] {
        let path = anim.join(name);
        let bytes =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let flic =
            Flic::parse(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(
            flic.header.width > 0 && flic.header.height > 0,
            "{}",
            path.display()
        );
        assert!(!flic.frames.is_empty(), "{}", path.display());
        let mut decoder =
            FlicDecoder::new(&flic).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for frame in 0..flic.frames.len() {
            assert!(decoder.decode_next_frame().unwrap_or_else(|error| {
                let chunk_type = error
                    .chunk_index
                    .and_then(|index| flic.frames[frame].chunks.get(index))
                    .map(|chunk| chunk.chunk_type);
                panic!(
                    "{} frame {frame} chunk type {chunk_type:?}: {error}",
                    path.display()
                )
            }));
        }
        assert_eq!(
            decoder.frame_position(),
            flic.frames.len(),
            "{}",
            path.display()
        );
        assert!(!decoder.decode_next_frame().unwrap(), "{}", path.display());
    }
}
