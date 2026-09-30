use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use dethrace_formats::flic::{Flic, FlicFormat};

fn main() {
    if let Err(error) = run(&env::args_os().skip(1).collect::<Vec<_>>()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: &[OsString]) -> Result<(), Box<dyn Error>> {
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
    use super::inspect_args;
    use std::ffi::OsString;

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
