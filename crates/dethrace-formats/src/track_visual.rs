//! Static visual subset of `RACES/*.TXT` version 6.
//!
//! `world.c::LoadTrack` consumes the counted normal-resolution resource groups
//! after the start-grid and checkpoint records. Later fields describe sky,
//! grooves, peds, collision, and gameplay, so this parser stops at the main ACT.

use crate::game_text::{GameText, TextError};

#[derive(Debug, Clone, PartialEq)]
pub struct TrackVisualSpec {
    pub track_file: String,
    pub start_position: [f32; 3],
    pub start_yaw_degrees: f32,
    pub pixelmaps: Vec<String>,
    pub materials: Vec<String>,
    pub models: Vec<String>,
    pub actor_file: String,
}

fn first_token(line: &str) -> &str {
    line.split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .next()
        .unwrap_or("")
}

fn number(text: &mut GameText, field: &str) -> Result<f32, TextError> {
    let line = text.next_line()?;
    let value = first_token(&line.text)
        .parse::<f32>()
        .map_err(|_| TextError::new(line.number, format!("invalid {field}")))?;
    if !value.is_finite() {
        return Err(TextError::new(line.number, format!("non-finite {field}")));
    }
    Ok(value)
}

fn position(text: &mut GameText) -> Result<[f32; 3], TextError> {
    let line = text.next_line()?;
    let parts = line
        .text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(TextError::new(
            line.number,
            "expected three start coordinates",
        ));
    }
    let mut values = [0.0; 3];
    for (index, part) in parts.into_iter().enumerate() {
        values[index] = part
            .parse::<f32>()
            .map_err(|_| TextError::new(line.number, "invalid start coordinate"))?;
        if !values[index].is_finite() {
            return Err(TextError::new(line.number, "non-finite start coordinate"));
        }
    }
    Ok(values)
}

fn resource_list(
    text: &mut GameText,
    label: &str,
    extension: &str,
) -> Result<Vec<String>, TextError> {
    let count = text.count(label)?;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let line = text.next_line()?;
        let file = first_token(&line.text);
        if !file.to_ascii_uppercase().ends_with(extension) {
            return Err(TextError::new(
                line.number,
                format!("invalid {label} filename: {file}"),
            ));
        }
        files.push(file.to_owned());
    }
    Ok(files)
}

fn skip_checkpoints(text: &mut GameText, version: usize) -> Result<(), TextError> {
    let count = text.count("checkpoint")?;
    for _ in 0..count {
        text.next_line()?; // target times
        text.next_line()?; // bonus times
        let quads = text.count("checkpoint quad")?;
        for _ in 0..quads {
            for _ in 0..4 {
                text.next_line()?; // corner
            }
        }
        if version > 1 {
            text.next_line()?; // map left, low and high resolution
            text.next_line()?; // map top, low and high resolution
        }
    }
    Ok(())
}

impl TrackVisualSpec {
    pub fn parse(bytes: &[u8], track_file: &str) -> Result<Self, TextError> {
        if !track_file.to_ascii_uppercase().ends_with(".TXT") {
            return Err(TextError::new(0, "invalid track filename"));
        }
        let mut text = GameText::parse(bytes)?;
        let version_line = text.next_line()?;
        let version = version_line
            .text
            .strip_prefix("VERSION ")
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or_else(|| TextError::new(version_line.number, "missing track VERSION"))?;
        if version != 6 {
            return Err(TextError::new(
                version_line.number,
                format!("unsupported track version {version}"),
            ));
        }
        let start_position = position(&mut text)?;
        let start_yaw_degrees = number(&mut text, "start yaw")?;
        text.next_line()?; // initial timers
        text.next_line()?; // lap count
        for _ in 0..3 {
            text.next_line()?; // bonus scores by difficulty
        }
        text.next_line()?; // checkpoint map rectangle widths
        text.next_line()?; // checkpoint map rectangle heights
        skip_checkpoints(&mut text, version)?;
        text.list("austerity pixelmap")?;
        let pixelmaps = resource_list(&mut text, "normal pixelmap", ".PIX")?;
        text.list("shade table")?;
        text.list("austerity material")?;
        let materials = resource_list(&mut text, "normal material", ".MAT")?;
        let models = resource_list(&mut text, "normal model", ".DAT")?;
        text.list("austerity model")?;
        let actor_line = text.next_line()?;
        let actor_file = first_token(&actor_line.text);
        if !actor_file.to_ascii_uppercase().ends_with(".ACT") {
            return Err(TextError::new(
                actor_line.number,
                "invalid main actor filename",
            ));
        }
        Ok(Self {
            track_file: track_file.to_owned(),
            start_position,
            start_yaw_degrees,
            pixelmaps,
            materials,
            models,
            actor_file: actor_file.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TrackVisualSpec;

    #[test]
    fn parses_version_six_visual_groups_after_counted_checkpoints() {
        let data = b"VERSION 6\n1,2,3\n90\n1,2,3\n1\n0\n0\n0\n9,18\n11,26\n1\n20,10,5\n60,30,10\n1\n0,0,0\n1,0,0\n1,1,0\n0,1,0\n1,2\n3,4\n1\nLOW.PIX\n1\nMAIN.PIX\n0\n1\nLOW.MAT\n1\nMAIN.MAT\n1\nMAIN.DAT\n0\nMAIN.ACT\n";
        let spec = TrackVisualSpec::parse(data, "TRACK.TXT").unwrap();
        assert_eq!(spec.start_position, [1.0, 2.0, 3.0]);
        assert_eq!(spec.pixelmaps, ["MAIN.PIX"]);
        assert_eq!(spec.materials, ["MAIN.MAT"]);
        assert_eq!(spec.models, ["MAIN.DAT"]);
        assert_eq!(spec.actor_file, "MAIN.ACT");
        assert!(TrackVisualSpec::parse(&data[..data.len() - 5], "TRACK.TXT").is_err());
        assert!(TrackVisualSpec::parse(b"VERSION 7\n", "TRACK.TXT").is_err());
    }
}
