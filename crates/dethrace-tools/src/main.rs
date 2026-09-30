use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use dethrace_formats::flic::{Flic, FlicDecoder, FlicFormat};
use dethrace_formats::image::{IndexedImage, Palette256};

fn main() {
    if let Err(error) = run(&env::args_os().skip(1).collect::<Vec<_>>()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    if args.get(1) == Some(&OsString::from("extract")) {
        return extract(args);
    }
    let (path, verbose) = inspect_args(args)?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let flic = Flic::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let header = flic.header;
    let format = match header.format {
        FlicFormat::Fli => "FLI",
        FlicFormat::Flc => "FLC",
    };
    println!("Path: {}", path.display());
    println!("Format: {format}");
    println!("Dimensions: {}x{}", header.width, header.height);
    println!("Frames: {}", flic.frames.len());
    println!(
        "Frame delay: {} ms (claimed speed {})",
        header.frame_delay_ms(),
        header.claimed_speed
    );
    println!(
        "Size: {} declared, {} actual bytes",
        header.declared_file_size,
        bytes.len()
    );
    let mut histogram = BTreeMap::<u16, usize>::new();
    for (index, frame) in flic.frames.iter().enumerate() {
        if verbose {
            println!(
                "Frame {index}: offset {}, {} bytes, {} chunks",
                frame.offset,
                frame.length,
                frame.chunks.len()
            );
        }
        for chunk in &frame.chunks {
            *histogram.entry(chunk.chunk_type).or_default() += 1;
        }
    }
    println!("Chunk types:");
    for (kind, count) in histogram {
        println!("  {kind}: {count}");
    }
    Ok(())
}

fn extract(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    let (path, frame, output) = extract_args(args)?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let flic = Flic::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let frame = select_frame(&frame, flic.frames.len())?;
    let mut decoder = FlicDecoder::new(&flic)?;
    for index in 0..=frame {
        decoder
            .decode_next_frame()
            .map_err(|error| format!("{} frame {index}: {error}", path.display()))?;
    }
    let rgba = rgba(decoder.image(), decoder.palette())?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&output)?;
    let mut encoder = png::Encoder::new(file, flic.header.width.into(), flic.header.height.into());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    println!("Wrote frame {frame} to {}", output.display());
    Ok(())
}

fn extract_args(args: &[OsString]) -> Result<(PathBuf, OsString, PathBuf), &'static str> {
    let usage = "usage: dethrace-tools flic extract PATH --frame INDEX|last --output PNG";
    if args.len() != 7
        || args[0] != OsStr::new("flic")
        || args[1] != OsStr::new("extract")
        || args[3] != OsStr::new("--frame")
        || args[5] != OsStr::new("--output")
        || args[2].is_empty()
        || args[4].is_empty()
        || args[6].is_empty()
    {
        return Err(usage);
    }
    Ok((
        PathBuf::from(&args[2]),
        args[4].clone(),
        PathBuf::from(&args[6]),
    ))
}

fn select_frame(requested: &OsStr, count: usize) -> Result<usize, String> {
    let index = if requested == OsStr::new("last") {
        count.checked_sub(1).ok_or("FLIC has no frames")?
    } else {
        requested
            .to_str()
            .ok_or("frame index must be UTF-8")?
            .parse::<usize>()
            .map_err(|_| "frame index must be a nonnegative integer or last")?
    };
    if index >= count {
        return Err(format!("frame {index} is outside 0..{count}"));
    }
    Ok(index)
}

fn rgba(image: &IndexedImage, palette: &Palette256) -> Result<Vec<u8>, String> {
    let capacity = image
        .pixels()
        .len()
        .checked_mul(4)
        .ok_or("RGBA image size overflow")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|error| error.to_string())?;
    for &index in image.pixels() {
        bytes.extend_from_slice(palette.color(index));
        bytes.push(255);
    }
    Ok(bytes)
}

fn inspect_args(args: &[OsString]) -> Result<(PathBuf, bool), &'static str> {
    let usage = "usage: dethrace-tools flic inspect PATH [--verbose]";
    if args.len() < 3
        || args.len() > 4
        || args[0] != OsStr::new("flic")
        || args[1] != OsStr::new("inspect")
    {
        return Err(usage);
    }
    if args.len() == 4 && args[3] != OsStr::new("--verbose") {
        return Err(usage);
    }
    if args[2].is_empty() {
        return Err(usage);
    }
    Ok((PathBuf::from(&args[2]), args.len() == 4))
}

#[cfg(test)]
mod tests {
    use super::{extract_args, inspect_args, rgba, select_frame};
    use dethrace_formats::image::{IndexedImage, Palette256};
    use std::ffi::{OsStr, OsString};

    #[test]
    fn converts_indices_to_opaque_rgba_and_selects_frames() {
        let mut image = IndexedImage::new(2, 1).unwrap();
        image.pixels_mut().copy_from_slice(&[0, 1]);
        let mut palette = Palette256::default();
        palette.set_color(0, [1, 2, 3]);
        palette.set_color(1, [4, 5, 6]);
        assert_eq!(
            rgba(&image, &palette).unwrap(),
            [1, 2, 3, 255, 4, 5, 6, 255]
        );
        assert_eq!(select_frame(OsStr::new("last"), 3).unwrap(), 2);
        assert_eq!(select_frame(OsStr::new("0"), 3).unwrap(), 0);
        assert!(select_frame(OsStr::new("3"), 3).is_err());
        assert!(select_frame(OsStr::new("last"), 0).is_err());
        let args = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(
            extract_args(&args(&[
                "flic", "extract", "a.fli", "--frame", "last", "--output", "a.png"
            ]))
            .is_ok()
        );
        assert!(extract_args(&args(&["flic", "extract", "a.fli", "--output", "a.png"])).is_err());
    }

    #[test]
    fn accepts_inspect_and_rejects_bad_arguments() {
        let args = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            inspect_args(&args(&["flic", "inspect", "movie.fli"]))
                .unwrap()
                .0
                .to_str(),
            Some("movie.fli")
        );
        assert!(
            inspect_args(&args(&["flic", "inspect", "movie.fli", "--verbose"]))
                .unwrap()
                .1
        );
        for bad in [
            args(&[]),
            args(&["flic", "inspect"]),
            args(&["flic", "extract", "movie.fli"]),
            args(&["flic", "inspect", "movie.fli", "--bad"]),
        ] {
            assert!(inspect_args(&bad).is_err());
        }
    }
}
