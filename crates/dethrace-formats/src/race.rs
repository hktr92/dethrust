//! Race and opponent metadata needed to resolve a gallery roster.
//!
//! Retail `SelectOpponents` draws five distinct opponents from rank-band
//! strength buckets. A caller-supplied seed makes debug scenes reproducible;
//! the selection rules and source filenames still come from game data.

use crate::game_text::{GameText, TextError};

const OPPONENT_MIX: [[i32; 5]; 10] = [
    [3, 4, 4, 5, 5],
    [2, 3, 4, 5, 5],
    [2, 3, 4, 4, 5],
    [2, 2, 4, 4, 5],
    [2, 2, 3, 4, 5],
    [1, 2, 3, 4, 4],
    [1, 2, 3, 3, 4],
    [1, 2, 2, 3, 4],
    [1, 1, 2, 3, 3],
    [1, 1, 2, 2, 3],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Race {
    pub name: String,
    pub track_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceCatalog {
    pub races: Vec<Race>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opponent {
    pub name: String,
    pub car_number: i32,
    pub strength: i32,
    pub car_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentCatalog {
    pub opponents: Vec<Opponent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryRoster {
    pub race_name: String,
    pub track_file: String,
    pub player_car_file: String,
    pub opponents: Vec<Opponent>,
    pub seed: u64,
}

fn first_token(line: &str) -> &str {
    line.split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .next()
        .unwrap_or("")
}

fn int(text: &mut GameText, label: &str) -> Result<i32, TextError> {
    let line = text.next_line()?;
    first_token(&line.text)
        .parse()
        .map_err(|_| TextError::new(line.number, format!("invalid {label}")))
}

fn skip_text_chunks(text: &mut GameText) -> Result<(), TextError> {
    let chunks = text.count("text chunk")?;
    for _ in 0..chunks {
        text.next_line()?; // position
        text.next_line()?; // frame range
        let lines = text.count("text line")?;
        for _ in 0..lines {
            text.next_line()?;
        }
    }
    Ok(())
}

impl RaceCatalog {
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut text = GameText::parse(bytes)?;
        let mut races = Vec::new();
        loop {
            let name = text.next_line()?;
            if name.text == "END" {
                break;
            }
            text.next_line()?; // scene/map/info FLIC names
            let track = text.next_line()?;
            let track_file = first_token(&track.text);
            if !track_file.to_ascii_uppercase().ends_with(".TXT") {
                return Err(TextError::new(track.number, "invalid race track filename"));
            }
            skip_text_chunks(&mut text)?;
            races.push(Race {
                name: name.text,
                track_file: track_file.to_owned(),
            });
        }
        if races.is_empty() {
            return Err(TextError::new(1, "empty race list"));
        }
        Ok(Self { races })
    }

    pub fn find(&self, name: &str) -> Option<(usize, &Race)> {
        self.races
            .iter()
            .enumerate()
            .find(|(_, race)| race.name.eq_ignore_ascii_case(name))
    }
}

impl OpponentCatalog {
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut text = GameText::parse(bytes)?;
        let count = text.count("opponent")?;
        let mut opponents = Vec::with_capacity(count);
        for _ in 0..count {
            let name = text.next_line()?.text;
            if name == "END" {
                return Err(TextError::new(0, "opponent count exceeds entries"));
            }
            text.next_line()?; // short name
            let car_number = int(&mut text, "car number")?;
            let strength = int(&mut text, "strength rating")?;
            text.next_line()?; // network availability
            text.next_line()?; // mugshot
            let car_line = text.next_line()?;
            let car_file = first_token(&car_line.text);
            if !car_file.to_ascii_uppercase().ends_with(".TXT") {
                return Err(TextError::new(
                    car_line.number,
                    "invalid opponent car filename",
                ));
            }
            text.next_line()?; // stolen-car FLIC
            skip_text_chunks(&mut text)?;
            opponents.push(Opponent {
                name,
                car_number,
                strength,
                car_file: car_file.to_owned(),
            });
        }
        text.expect("END")?;
        Ok(Self { opponents })
    }
}

/// Initial rank and first basic car follow the fixed GENERAL.TXT scalar groups
/// read by `LoadGeneralParameters`; later physics/settings fields are ignored.
pub fn initial_player(bytes: &[u8]) -> Result<(i32, String), TextError> {
    let mut text = GameText::parse(bytes)?;
    for _ in 0..5 {
        text.next_line()?;
    } // camera and display setup
    let rank = int(&mut text, "initial rank")?;
    for _ in 0..2 {
        text.next_line()?;
    } // credits
    for _ in 0..6 {
        text.next_line()?;
    } // crush coefficients
    for _ in 0..3 {
        text.next_line()?;
    } // repair, recovery, pedestrian values
    for _ in 0..14 {
        text.next_line()?;
    } // two seven-line scoring tables
    for _ in 0..3 {
        text.next_line()?;
    } // fines, points, stunt bonus
    let line = text.next_line()?;
    let car = first_token(&line.text);
    if !car.to_ascii_uppercase().ends_with(".TXT") {
        return Err(TextError::new(line.number, "invalid initial car filename"));
    }
    Ok((rank, car.to_owned()))
}

impl GalleryRoster {
    pub fn resolve(
        races: &RaceCatalog,
        opponents: &OpponentCatalog,
        race_name: &str,
        player_car_file: &str,
        seed: u64,
    ) -> Result<Self, TextError> {
        let (race_index, race) = races
            .find(race_name)
            .ok_or_else(|| TextError::new(0, format!("race '{race_name}' not found")))?;
        if races.races.len() <= 3 {
            return Err(TextError::new(
                0,
                "race list too short for rank calculation",
            ));
        }
        let suggested_rank = 99 - 100 * (race_index as i32) / (races.races.len() as i32 - 3);
        let band = (suggested_rank / 10).clamp(0, 9) as usize;
        let mut state = seed;
        let mut picked = vec![false; opponents.opponents.len()];
        let mut had_scum = false;
        let mut selected = Vec::with_capacity(5);
        for strength in OPPONENT_MIX[band] {
            let eligible: Vec<usize> = opponents
                .opponents
                .iter()
                .enumerate()
                .filter(|(i, op)| {
                    op.strength == strength
                        && !op.car_file.eq_ignore_ascii_case(player_car_file)
                        && !picked[*i]
                        && (op.car_number >= 0 || !had_scum)
                })
                .map(|(i, _)| i)
                .collect();
            if eligible.is_empty() {
                return Err(TextError::new(
                    0,
                    format!("no eligible opponent with strength {strength}"),
                ));
            }
            // Stable debug selection. Upstream uses process-global IRandomBetween.
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let index = eligible[((state >> 32) as usize) % eligible.len()];
            picked[index] = true;
            let opponent = opponents.opponents[index].clone();
            had_scum |= opponent.car_number < 0;
            selected.push(opponent);
        }
        Ok(Self {
            race_name: race.name.clone(),
            track_file: race.track_file.clone(),
            player_car_file: player_car_file.to_owned(),
            opponents: selected,
            seed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GalleryRoster, OpponentCatalog, RaceCatalog, initial_player};
    #[test]
    fn parses_and_selects_deterministic_roster() {
        let races=RaceCatalog::parse(b"Maim Street\nA,B,C\nCITYA1.TXT\n0\nOther\nA,B,C\nOTHER.TXT\n0\nThird\nA,B,C\nTHIRD.TXT\n0\nFourth\nA,B,C\nFOURTH.TXT\n0\nEND\n").unwrap();
        let mut opp = b"6\n".to_vec();
        for (i, strength) in [1, 1, 2, 2, 3, 3].into_iter().enumerate() {
            opp.extend(
                format!(
                    "Driver {i}\nD{i}\n{i}\n{strength}\nall\nMUG.FLI\nCAR{i}.TXT\nSTOLEN.FLI\n0\n"
                )
                .as_bytes(),
            );
        }
        opp.extend(b"END\n");
        let opponents = OpponentCatalog::parse(&opp).unwrap();
        let a = GalleryRoster::resolve(&races, &opponents, "Maim Street", "PLAYER.TXT", 7).unwrap();
        let b = GalleryRoster::resolve(&races, &opponents, "Maim Street", "PLAYER.TXT", 7).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.opponents.len(), 5);
        assert_eq!(a.track_file, "CITYA1.TXT");
        assert!(GalleryRoster::resolve(&races, &opponents, "missing", "PLAYER.TXT", 7).is_err());
        let mut general = b"0\n".repeat(5);
        general.extend(b"99\n");
        general.extend(b"0\n".repeat(28));
        general.extend(b"PLAYER.TXT\n");
        assert_eq!(initial_player(&general).unwrap(), (99, "PLAYER.TXT".into()));
    }
}
