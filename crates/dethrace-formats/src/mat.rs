//! Neutral BRender MAT materials and named pixelmap/table references.
//!
//! Original MAT headers have unreliable declared chunk lengths. BRender's
//! `BrMaterialLoadMany` reads known fields and NUL-terminated names, then reads
//! the next header from the resulting stream position. This parser does likewise.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub identifier: String,
    pub colour: [u8; 3],
    pub opacity: u8,
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub power: f32,
    pub flags: u32,
    pub map_transform: [[f32; 2]; 3],
    pub index_base: u8,
    pub index_range: u8,
    pub colour_map: Option<String>,
    pub index_blend: Option<String>,
    pub index_shade: Option<String>,
    pub screendoor: Option<String>,
    pub index_fog: Option<String>,
}

impl Material {
    pub fn two_sided(&self) -> bool {
        self.flags & 0x1000 != 0
    }

    pub fn prelit(&self) -> bool {
        self.flags & 2 != 0
    }

    pub fn smooth(&self) -> bool {
        self.flags & 4 != 0
    }

    pub fn lit(&self) -> bool {
        self.flags & 1 != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatFile {
    pub materials: Vec<Material>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatError {
    pub offset: usize,
    pub detail: String,
}

impl MatError {
    fn new(offset: usize, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for MatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MAT offset {}: {}", self.offset, self.detail)
    }
}

impl std::error::Error for MatError {}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], MatError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| MatError::new(self.at, "offset overflow"))?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| MatError::new(self.at, format!("truncated {n}-byte field")))?;
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, MatError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, MatError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, MatError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f32(&mut self) -> Result<f32, MatError> {
        let at = self.at;
        let value = f32::from_bits(self.u32()?);
        if !value.is_finite() {
            return Err(MatError::new(at, "nonfinite material scalar"));
        }
        Ok(value)
    }

    fn name(&mut self) -> Result<String, MatError> {
        let start = self.at;
        let end = self
            .bytes
            .get(start..start.saturating_add(256).min(self.bytes.len()))
            .and_then(|data| data.iter().position(|&c| c == 0))
            .ok_or_else(|| MatError::new(start, "missing NUL-terminated name"))?
            + start;
        let name = std::str::from_utf8(&self.bytes[start..end])
            .map_err(|_| MatError::new(start, "name is not UTF-8"))?
            .to_owned();
        self.at = end + 1;
        Ok(name)
    }
}

fn material(cursor: &mut Cursor<'_>) -> Result<Material, MatError> {
    let colour = [cursor.u8()?, cursor.u8()?, cursor.u8()?];
    let opacity = cursor.u8()?;
    let ambient = cursor.f32()?;
    let diffuse = cursor.f32()?;
    let specular = cursor.f32()?;
    let power = cursor.f32()?;
    let flags = u32::from(cursor.u16()?);
    let mut map_transform = [[0.0; 2]; 3];
    for row in &mut map_transform {
        for value in row {
            *value = cursor.f32()?;
        }
    }
    let index_base = cursor.u8()?;
    let index_range = cursor.u8()?;
    let identifier = cursor.name()?;
    Ok(Material {
        identifier,
        colour,
        opacity,
        ambient,
        diffuse,
        specular,
        power,
        flags,
        map_transform,
        index_base,
        index_range,
        colour_map: None,
        index_blend: None,
        index_shade: None,
        screendoor: None,
        index_fog: None,
    })
}

impl MatFile {
    pub fn parse(bytes: &[u8]) -> Result<Self, MatError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.u32()? != 18 || cursor.u32()? != 8 {
            return Err(MatError::new(0, "missing MAT file-info chunk"));
        }
        let file_type = cursor.u32()?;
        let version = cursor.u32()?;
        if !matches!(file_type, 5 | 6) || version != 2 {
            return Err(MatError::new(8, "unsupported MAT file type or version"));
        }
        let mut materials = Vec::new();
        let mut current: Option<Material> = None;
        while cursor.at < bytes.len() {
            let at = cursor.at;
            let id = cursor.u32()?;
            let length = cursor.u32()?;
            if at
                .checked_add(8)
                .and_then(|end| end.checked_add(length as usize))
                .is_none_or(|end| end > bytes.len())
            {
                return Err(MatError::new(at, "declared MAT chunk exceeds file"));
            }
            match id {
                4 if current.is_none() => current = Some(material(&mut cursor)?),
                28 | 30 | 31 | 32 | 59 => {
                    let mat = current
                        .as_mut()
                        .ok_or_else(|| MatError::new(at, "reference without material"))?;
                    let name = cursor.name()?;
                    let target = match id {
                        28 => &mut mat.colour_map,
                        30 => &mut mat.index_blend,
                        31 => &mut mat.index_shade,
                        32 => &mut mat.screendoor,
                        _ => &mut mat.index_fog,
                    };
                    if target.replace(name).is_some() {
                        return Err(MatError::new(at, "duplicate material reference"));
                    }
                }
                0 if length == 0 => {
                    materials.push(
                        current
                            .take()
                            .ok_or_else(|| MatError::new(at, "end without material"))?,
                    );
                }
                _ => return Err(MatError::new(at, format!("unsupported MAT chunk {id}"))),
            }
        }
        if current.is_some() || materials.is_empty() {
            return Err(MatError::new(cursor.at, "incomplete MAT container"));
        }
        Ok(Self { materials })
    }
}

#[cfg(test)]
mod tests {
    use super::MatFile;

    fn chunk(out: &mut Vec<u8>, id: u32, declared: u32, payload: &[u8]) {
        out.extend(id.to_be_bytes());
        out.extend(declared.to_be_bytes());
        out.extend(payload);
    }

    #[test]
    fn parses_oldest_material_and_references_with_legacy_lengths() {
        let mut bytes = Vec::new();
        chunk(&mut bytes, 18, 8, &[0, 0, 0, 5, 0, 0, 0, 2]);
        let mut mat = vec![255, 128, 0, 200];
        for value in [0.2_f32, 0.7, 0.1, 20.0] {
            mat.extend(value.to_be_bytes());
        }
        mat.extend(0x1005_u16.to_be_bytes());
        for value in [1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0] {
            mat.extend(value.to_be_bytes());
        }
        mat.extend([0, 63]);
        mat.extend(b"M\0");
        chunk(&mut bytes, 4, (mat.len() + 1) as u32, &mat);
        chunk(&mut bytes, 28, 1, b"TEX.PIX\0");
        chunk(&mut bytes, 31, 1, b"SHADE.TAB\0");
        chunk(&mut bytes, 0, 0, &[]);
        let parsed = MatFile::parse(&bytes).unwrap();
        assert_eq!(parsed.materials.len(), 1);
        assert_eq!(parsed.materials[0].colour, [255, 128, 0]);
        assert_eq!(parsed.materials[0].colour_map.as_deref(), Some("TEX.PIX"));
        assert!(parsed.materials[0].two_sided());
        assert!(parsed.materials[0].lit());
        for cut in [0, 7, 15, bytes.len() - 1] {
            assert!(MatFile::parse(&bytes[..cut]).is_err());
        }
        let mut bad = bytes.clone();
        bad[28..32].copy_from_slice(&f32::INFINITY.to_be_bytes());
        assert!(MatFile::parse(&bad).is_err());
    }
}
