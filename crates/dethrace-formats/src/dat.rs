//! BRender DAT model geometry. Names and numeric fields remain in source coordinates.
//!
//! BRender reads material index names by count, not declared chunk size. Some
//! original car DATs declare one byte less than the actual name payload.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub vertices: [u16; 3],
    pub smoothing: u16,
    pub flags: u8,
    /// BRender material index: zero is no material, other entries are one based.
    pub material_index: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub identifier: String,
    pub flags: u16,
    pub pivot: Option<[f32; 3]>,
    pub crease_angle: Option<u16>,
    pub radius: Option<f32>,
    pub bounds: Option<[[f32; 3]; 2]>,
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatFile {
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatError {
    pub offset: usize,
    pub detail: String,
}

impl DatError {
    fn new(offset: usize, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for DatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DAT offset {}: {}", self.offset, self.detail)
    }
}

impl std::error::Error for DatError {}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DatError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| DatError::new(self.at, "offset overflow"))?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| DatError::new(self.at, format!("truncated {n}-byte field")))?;
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, DatError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DatError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, DatError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, DatError> {
        let at = self.at;
        let value = f32::from_bits(self.u32()?);
        if !value.is_finite() {
            return Err(DatError::new(at, "nonfinite model scalar"));
        }
        Ok(value)
    }
    fn vec3(&mut self) -> Result<[f32; 3], DatError> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    fn name(&mut self) -> Result<String, DatError> {
        let start = self.at;
        let end = self
            .bytes
            .get(start..start.saturating_add(256).min(self.bytes.len()))
            .and_then(|b| b.iter().position(|&v| v == 0))
            .ok_or_else(|| DatError::new(start, "missing NUL-terminated name"))?
            + start;
        let name = std::str::from_utf8(&self.bytes[start..end])
            .map_err(|_| DatError::new(start, "name is not UTF-8"))?
            .to_owned();
        self.at = end + 1;
        Ok(name)
    }
    fn counted(&mut self, size: usize, label: &str) -> Result<usize, DatError> {
        let at = self.at;
        let count =
            usize::try_from(self.u32()?).map_err(|_| DatError::new(at, "count overflow"))?;
        let bytes = count
            .checked_mul(size)
            .ok_or_else(|| DatError::new(at, "count multiplication overflow"))?;
        if bytes > self.bytes.len() - self.at {
            return Err(DatError::new(at, format!("truncated {label} array")));
        }
        Ok(count)
    }
}

fn active(current: &mut Option<Model>, at: usize) -> Result<&mut Model, DatError> {
    current
        .as_mut()
        .ok_or_else(|| DatError::new(at, "model data without model header"))
}

impl DatFile {
    pub fn parse(bytes: &[u8]) -> Result<Self, DatError> {
        let mut c = Cursor { bytes, at: 0 };
        if c.u32()? != 18 || c.u32()? != 8 || c.u32()? != 0xface || c.u32()? != 2 {
            return Err(DatError::new(0, "unsupported DAT file-info chunk"));
        }
        let mut models = Vec::new();
        let mut current: Option<Model> = None;
        while c.at < bytes.len() {
            let at = c.at;
            let id = c.u32()?;
            let declared =
                usize::try_from(c.u32()?).map_err(|_| DatError::new(at, "chunk size overflow"))?;
            if declared > bytes.len() - c.at {
                return Err(DatError::new(at, "declared chunk exceeds file"));
            }
            let start = c.at;
            match id {
                54 | 64 if current.is_none() => {
                    let flags = c.u16()?;
                    let (pivot, crease_angle, radius, bounds) = if id == 64 {
                        (
                            Some(c.vec3()?),
                            Some(c.u16()?),
                            Some(c.f32()?),
                            Some([c.vec3()?, c.vec3()?]),
                        )
                    } else {
                        (None, None, None, None)
                    };
                    current = Some(Model {
                        identifier: c.name()?,
                        flags,
                        pivot,
                        crease_angle,
                        radius,
                        bounds,
                        vertices: Vec::new(),
                        faces: Vec::new(),
                        materials: Vec::new(),
                    });
                }
                23 => {
                    let model = active(&mut current, at)?;
                    if !model.vertices.is_empty() {
                        return Err(DatError::new(at, "duplicate vertices"));
                    }
                    let count = c.counted(12, "vertex")?;
                    if count > u16::MAX as usize {
                        return Err(DatError::new(at, "too many vertices"));
                    }
                    model.vertices.reserve(count);
                    for _ in 0..count {
                        model.vertices.push(Vertex {
                            position: c.vec3()?,
                            uv: [0.0; 2],
                        });
                    }
                }
                24 => {
                    let model = active(&mut current, at)?;
                    let count = c.counted(8, "UV")?;
                    if count > model.vertices.len() {
                        return Err(DatError::new(at, "UV count exceeds vertex count"));
                    }
                    for vertex in model.vertices.iter_mut().take(count) {
                        vertex.uv = [c.f32()?, c.f32()?];
                    }
                }
                53 | 25 => {
                    let model = active(&mut current, at)?;
                    if !model.faces.is_empty() {
                        return Err(DatError::new(at, "duplicate faces"));
                    }
                    let count = c.counted(if id == 53 { 9 } else { 8 }, "face")?;
                    if count > u16::MAX as usize {
                        return Err(DatError::new(at, "too many faces"));
                    }
                    model.faces.reserve(count);
                    for _ in 0..count {
                        let vertices = [c.u16()?, c.u16()?, c.u16()?];
                        if vertices
                            .iter()
                            .any(|&index| usize::from(index) >= model.vertices.len())
                        {
                            return Err(DatError::new(c.at, "face vertex index out of range"));
                        }
                        let smoothing = if id == 53 {
                            c.u16()?
                        } else {
                            let old = c.u8()?;
                            if old == 0 {
                                u16::MAX
                            } else {
                                1u16 << ((old - 1) % 16)
                            }
                        };
                        let flags = c.u8()?;
                        model.faces.push(Face {
                            vertices,
                            smoothing,
                            flags,
                            material_index: 0,
                        });
                    }
                }
                22 => {
                    let model = active(&mut current, at)?;
                    if !model.materials.is_empty() {
                        return Err(DatError::new(at, "duplicate material index"));
                    }
                    let count = usize::try_from(c.u32()?)
                        .map_err(|_| DatError::new(c.at, "material count overflow"))?;
                    if count > u16::MAX as usize || count > bytes.len() - c.at {
                        return Err(DatError::new(at, "invalid material count"));
                    }
                    for _ in 0..count {
                        model.materials.push(c.name()?);
                    }
                }
                26 => {
                    let model = active(&mut current, at)?;
                    let count = c.counted(2, "face material")?;
                    let element_size = c.u32()?;
                    if element_size != 2 || count > model.faces.len() {
                        return Err(DatError::new(at, "invalid face material block"));
                    }
                    for face in model.faces.iter_mut().take(count) {
                        face.material_index = c.u16()?;
                        if usize::from(face.material_index) > model.materials.len() {
                            return Err(DatError::new(c.at, "face material index out of range"));
                        }
                    }
                }
                0 if declared == 0 => {
                    let model = current
                        .take()
                        .ok_or_else(|| DatError::new(at, "end without model"))?;
                    if model.vertices.is_empty() || model.faces.is_empty() {
                        return Err(DatError::new(at, "empty model geometry"));
                    }
                    models.push(model);
                }
                _ => return Err(DatError::new(at, format!("unsupported DAT chunk {id}"))),
            }
            if id != 22 && c.at - start != declared {
                return Err(DatError::new(at, format!("DAT chunk {id} length mismatch")));
            }
        }
        if current.is_some() || models.is_empty() {
            return Err(DatError::new(c.at, "incomplete DAT container"));
        }
        Ok(Self { models })
    }
}

#[cfg(test)]
mod tests {
    use super::DatFile;
    fn chunk(bytes: &mut Vec<u8>, id: u32, payload: &[u8]) {
        bytes.extend(id.to_be_bytes());
        bytes.extend((payload.len() as u32).to_be_bytes());
        bytes.extend(payload);
    }
    #[test]
    fn parses_geometry_and_rejects_bad_indexes_and_truncation() {
        let mut bytes = Vec::new();
        chunk(&mut bytes, 18, &[0, 0, 0xfa, 0xce, 0, 0, 0, 2]);
        chunk(&mut bytes, 54, b"\0\0M\0");
        let mut vertices = 3u32.to_be_bytes().to_vec();
        for xyz in [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
            for f in xyz {
                vertices.extend(f32::to_be_bytes(f));
            }
        }
        chunk(&mut bytes, 23, &vertices);
        let mut uvs = 3u32.to_be_bytes().to_vec();
        for uv in [[0., 0.], [1., 0.], [0., 1.]] {
            for f in uv {
                uvs.extend(f32::to_be_bytes(f));
            }
        }
        chunk(&mut bytes, 24, &uvs);
        let mut faces = 1u32.to_be_bytes().to_vec();
        faces.extend([0, 0, 0, 1, 0, 2, 0, 1, 0]);
        chunk(&mut bytes, 53, &faces);
        chunk(&mut bytes, 22, &[0, 0, 0, 1, b'A', 0]);
        chunk(&mut bytes, 26, &[0, 0, 0, 1, 0, 0, 0, 2, 0, 1]);
        chunk(&mut bytes, 0, &[]);
        let parsed = DatFile::parse(&bytes).unwrap();
        assert_eq!(parsed.models[0].vertices[1].uv, [1., 0.]);
        assert_eq!(parsed.models[0].faces[0].material_index, 1);
        for cut in [0, 15, 30, bytes.len() - 1] {
            assert!(DatFile::parse(&bytes[..cut]).is_err());
        }
        let mut bad = bytes.clone();
        let at = bad
            .windows(4)
            .position(|b| b == 53u32.to_be_bytes())
            .unwrap()
            + 12;
        bad[at + 2..at + 4].copy_from_slice(&3u16.to_be_bytes());
        assert!(DatFile::parse(&bad).is_err());
    }
}
