//! Visual subset of resolution-independent `CARS/*.TXT`.
//!
//! Dethrace `LoadCar` reads a separate `REG/CARS` file for cockpit/UI data;
//! gallery geometry comes from the independent file's counted PIX/MAT/DAT/ACT
//! lists. Six counted damage programs are skipped without interpreting damage.

use crate::game_text::{GameText, TextError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualVariant {
    Austerity,
    Low,
    High,
}

impl VisualVariant {
    fn index(self) -> usize {
        match self {
            Self::Austerity => 0,
            Self::Low => 1,
            Self::High => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActorVariant {
    pub min_distance: f32,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CarVisualSpec {
    pub definition: String,
    pub pixelmaps: [Vec<String>; 3],
    pub shade_tables: Vec<String>,
    pub materials: [Vec<String>; 3],
    pub models: Vec<String>,
    pub actors: Vec<ActorVariant>,
    pub principal_actor: usize,
    pub screen_material: Option<String>,
}

impl CarVisualSpec {
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut text = GameText::parse(bytes)?;
        let definition = text
            .next_line()?
            .text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_owned();
        if !definition.to_ascii_uppercase().ends_with(".TXT") {
            return Err(TextError::new(1, "missing car definition filename"));
        }
        text.expect("START OF DRIVABLE STUFF")?;
        loop {
            let line = text.next_line()?;
            if line.text == "END OF DRIVABLE STUFF" {
                break;
            }
        }
        text.next_line()?; // engine sound IDs
        text.next_line()?; // stealability
        for _ in 0..6 {
            let clauses = text.count("damage clause")?;
            for _ in 0..clauses {
                text.next_line()?; // condition expression
                let effects = text.count("damage effect")?;
                for _ in 0..effects {
                    text.next_line()?;
                }
            }
        }
        text.next_line()?; // three grid icon names
        let pixelmaps = [
            text.list("austerity pixelmap")?,
            text.list("low pixelmap")?,
            text.list("high pixelmap")?,
        ];
        let shade_tables = text.list("shade table")?;
        let materials = [
            text.list("austerity material")?,
            text.list("low material")?,
            text.list("high material")?,
        ];
        let models = text.list("model")?;
        let actor_count = text.count("actor")?;
        let mut actors = Vec::with_capacity(actor_count);
        let mut principal_actor = None;
        for index in 0..actor_count {
            let line = text.next_line()?;
            let (distance, file) = line.text.split_once(',').ok_or_else(|| {
                TextError::new(line.number, "actor variant requires distance,filename")
            })?;
            let min_distance = distance
                .trim()
                .parse::<f32>()
                .map_err(|_| TextError::new(line.number, "invalid actor distance"))?;
            if !min_distance.is_finite() {
                return Err(TextError::new(line.number, "nonfinite actor distance"));
            }
            let file = file.split_whitespace().next().unwrap_or("");
            if !file.to_ascii_uppercase().ends_with(".ACT") {
                return Err(TextError::new(line.number, "invalid actor filename"));
            }
            if min_distance == 0.0 {
                principal_actor = Some(index);
            }
            actors.push(ActorVariant {
                min_distance,
                file: file.to_owned(),
            });
        }
        let principal_actor = principal_actor.ok_or_else(|| {
            TextError::new(
                text.lines.last().map_or(1, |l| l.number),
                "missing zero-distance principal actor",
            )
        })?;
        let line = text.next_line()?;
        let screen_material = line
            .text
            .split_whitespace()
            .next()
            .filter(|s| !s.eq_ignore_ascii_case("none"))
            .map(str::to_owned);
        Ok(Self {
            definition,
            pixelmaps,
            shade_tables,
            materials,
            models,
            actors,
            principal_actor,
            screen_material,
        })
    }

    pub fn pixelmaps_for(&self, variant: VisualVariant) -> &[String] {
        &self.pixelmaps[variant.index()]
    }
    pub fn materials_for(&self, variant: VisualVariant) -> &[String] {
        &self.materials[variant.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::{CarVisualSpec, VisualVariant};
    #[test]
    fn counts_sections_and_finds_principal_actor() {
        let text=b"CAR.TXT\nSTART OF DRIVABLE STUFF\nignored\nEND OF DRIVABLE STUFF\n1,2,3\nstealworthy\n0\n0\n0\n0\n0\n0\nA,B,C\n0\n1\nLOW.PIX\n0\n0\n0\n1\nLOW.MAT\n0\n1\nCAR.DAT\n2\n8,LOW.ACT\n0,HIGH.ACT\nnone\n";
        let spec = CarVisualSpec::parse(text).unwrap();
        assert_eq!(spec.pixelmaps_for(VisualVariant::Low), ["LOW.PIX"]);
        assert_eq!(spec.materials_for(VisualVariant::Low), ["LOW.MAT"]);
        assert_eq!(spec.principal_actor, 1);
        assert!(CarVisualSpec::parse(&text[..text.len() - 8]).is_err());
    }
}
