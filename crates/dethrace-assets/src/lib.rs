//! Converts original FLIC data into Bevy images for presentation.

use std::error::Error;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use dethrace_formats::flic::{Flic, FlicDecoder};
use dethrace_formats::image::{IndexedImage, Palette256};

#[derive(Debug, Clone)]
pub struct GameDir {
    data: PathBuf,
}

impl GameDir {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();
        let data = if root.join("DATA/ANIM").is_dir() {
            root.join("DATA")
        } else if root.join("ANIM").is_dir() {
            root.to_path_buf()
        } else {
            return Err(format!(
                "{} has no DATA/ANIM or ANIM directory",
                root.display()
            ));
        };
        if !data.join("ANIM/MAI2STIL.FLI").is_file() {
            return Err(format!("{} is missing ANIM/MAI2STIL.FLI", data.display()));
        }
        Ok(Self { data })
    }

    pub fn anim_path(&self, name: &str) -> Result<PathBuf, String> {
        let file = Path::new(name);
        if file.file_name() != Some(file.as_os_str()) {
            return Err(format!("invalid animation filename: {name}"));
        }
        Ok(self.data.join("ANIM").join(file))
    }
}

pub struct FlicClip {
    pub frames: Vec<Image>,
    pub frame_delay_ms: u32,
}

impl FlicClip {
    pub fn load(path: &Path, transparent_zero: bool) -> Result<Self, Box<dyn Error>> {
        let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let flic = Flic::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        let frame_delay_ms = flic.header.frame_delay_ms();
        if frame_delay_ms == 0 {
            return Err(format!("{} has zero frame delay", path.display()).into());
        }
        let mut decoder = FlicDecoder::new(&flic)?;
        let mut frames = Vec::with_capacity(flic.frames.len());
        for index in 0..flic.frames.len() {
            decoder
                .decode_next_frame()
                .map_err(|error| format!("{} frame {index}: {error}", path.display()))?;
            frames.push(to_image(
                decoder.image(),
                decoder.palette(),
                transparent_zero,
            )?);
        }
        Ok(Self {
            frames,
            frame_delay_ms,
        })
    }
}

pub fn to_image(
    image: &IndexedImage,
    palette: &Palette256,
    transparent_zero: bool,
) -> Result<Image, String> {
    let width = u32::try_from(image.width()).map_err(|error| error.to_string())?;
    let height = u32::try_from(image.height()).map_err(|error| error.to_string())?;
    let capacity = image
        .pixels()
        .len()
        .checked_mul(4)
        .ok_or("RGBA image size overflow")?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(capacity)
        .map_err(|error| error.to_string())?;
    for &index in image.pixels() {
        rgba.extend_from_slice(palette.color(index));
        rgba.push(if transparent_zero && index == 0 {
            0
        } else {
            255
        });
    }
    let mut output = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    output.sampler = ImageSampler::nearest();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{GameDir, to_image};
    use dethrace_formats::image::{IndexedImage, Palette256};

    #[test]
    fn maps_game_paths_and_converts_alpha() {
        let root = std::env::temp_dir().join(format!("dethrust-assets-{}", std::process::id()));
        let anim = root.join("DATA/ANIM");
        std::fs::create_dir_all(&anim).unwrap();
        std::fs::write(anim.join("MAI2STIL.FLI"), []).unwrap();
        let dir = GameDir::new(&root).unwrap();
        assert_eq!(
            dir.anim_path("MAI2COME.FLI").unwrap(),
            anim.join("MAI2COME.FLI")
        );
        assert!(dir.anim_path("../bad.FLI").is_err());
        let mut image = IndexedImage::new(2, 1).unwrap();
        image.pixels_mut().copy_from_slice(&[0, 1]);
        let mut palette = Palette256::default();
        palette.set_color(0, [1, 2, 3]);
        palette.set_color(1, [4, 5, 6]);
        assert_eq!(
            to_image(&image, &palette, false).unwrap().data.unwrap(),
            [1, 2, 3, 255, 4, 5, 6, 255]
        );
        assert_eq!(
            to_image(&image, &palette, true).unwrap().data.unwrap(),
            [1, 2, 3, 0, 4, 5, 6, 255]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
