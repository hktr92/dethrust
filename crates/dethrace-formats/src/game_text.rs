//! Decoded Carmageddon game text lines, independent of filesystem and Bevy.
//!
//! The original retail files use method 1 of Dethrace `DecodeLine2` (selected
//! by the encoded `GENERAL.TXT` header). Decoding precedes comment removal.

use std::fmt;

const KEY: [u8; 16] = [
    0x6c, 0x1b, 0x99, 0x5f, 0xb9, 0xcd, 0x5f, 0x13, 0xcb, 0x04, 0x20, 0x0e, 0x5e, 0x1c, 0xa1, 0x0e,
];
const COMMENT_KEY: [u8; 16] = [
    0x67, 0xa8, 0xd6, 0x26, 0xb6, 0xdd, 0x45, 0x1b, 0x32, 0x7e, 0x22, 0x13, 0x15, 0xc2, 0x94, 0x37,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextError {
    pub line: usize,
    pub detail: String,
}

impl TextError {
    pub fn new(line: usize, detail: impl Into<String>) -> Self {
        Self {
            line,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "game text line {}: {}", self.line, self.detail)
    }
}

impl std::error::Error for TextError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLine {
    pub number: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameText {
    pub lines: Vec<DataLine>,
    at: usize,
}

impl GameText {
    pub fn parse(bytes: &[u8]) -> Result<Self, TextError> {
        let mut lines = Vec::new();
        for (index, raw) in bytes.split(|&b| b == b'\n').enumerate() {
            let line = index + 1;
            let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
            if raw.len() > 255 {
                return Err(TextError::new(line, "line exceeds original 255-byte limit"));
            }
            let mut data = if let Some(encoded) = raw.strip_prefix(b"@") {
                let mut decoded = encoded.to_vec();
                let mut key = &KEY;
                let mut seed = decoded.len() % 16;
                for i in 0..decoded.len() {
                    if i >= 2 && decoded[i - 1] == b'/' && decoded[i - 2] == b'/' {
                        key = &COMMENT_KEY;
                    }
                    if decoded[i] == b'\t' {
                        decoded[i] = 0x9f;
                    }
                    decoded[i] =
                        ((key[seed] ^ (decoded[i].wrapping_sub(32))) & 0x7f).wrapping_add(32);
                    seed = (seed + 7) % 16;
                    if decoded[i] == 0x9f {
                        decoded[i] = b'\t';
                    }
                }
                decoded
            } else {
                raw.to_vec()
            };
            if let Some(comment) = data.windows(2).position(|pair| pair == b"//") {
                data.truncate(comment);
            }
            let text = std::str::from_utf8(&data)
                .map_err(|_| TextError::new(line, "non-UTF-8 data line"))?
                .trim();
            if !text.is_empty() {
                lines.push(DataLine {
                    number: line,
                    text: text.to_owned(),
                });
            }
        }
        Ok(Self { lines, at: 0 })
    }

    pub fn next_line(&mut self) -> Result<DataLine, TextError> {
        let line = self
            .lines
            .get(self.at)
            .ok_or_else(|| {
                TextError::new(
                    self.lines.last().map_or(1, |l| l.number + 1),
                    "unexpected end of file",
                )
            })?
            .clone();
        self.at += 1;
        Ok(line)
    }

    pub fn expect(&mut self, marker: &str) -> Result<(), TextError> {
        let line = self.next_line()?;
        if line.text != marker {
            return Err(TextError::new(
                line.number,
                format!("expected '{marker}', found '{}'", line.text),
            ));
        }
        Ok(())
    }

    pub fn count(&mut self, label: &str) -> Result<usize, TextError> {
        let line = self.next_line()?;
        let token = line
            .text
            .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
            .next()
            .unwrap_or("");
        let count = token
            .parse::<usize>()
            .map_err(|_| TextError::new(line.number, format!("invalid {label} count")))?;
        if count > 10_000 {
            return Err(TextError::new(
                line.number,
                format!("{label} count too large"),
            ));
        }
        Ok(count)
    }

    pub fn list(&mut self, label: &str) -> Result<Vec<String>, TextError> {
        let count = self.count(label)?;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let line = self.next_line()?;
            let name = line
                .text
                .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
                .next()
                .unwrap_or("");
            if name.is_empty() {
                return Err(TextError::new(
                    line.number,
                    format!("empty {label} filename"),
                ));
            }
            items.push(name.to_owned());
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::GameText;
    #[test]
    fn strips_comments_and_rejects_bad_count() {
        let mut text = GameText::parse(b"// comment\n  2 // count\nA.PIX\nB.PIX\n").unwrap();
        assert_eq!(text.list("PIX").unwrap(), vec!["A.PIX", "B.PIX"]);
        assert!(GameText::parse(b"10001\n").unwrap().count("items").is_err());
    }
}
