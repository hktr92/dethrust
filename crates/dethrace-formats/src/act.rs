//! BRender ACT scene hierarchy in source coordinates.
//!
//! BRender serializes children before `ACTOR_ADD_CHILD`; no global registry is
//! needed to reconstruct the tree or resolve names against caller-owned data.

use std::fmt;

use crate::{dat::DatFile, mat::MatFile};

#[derive(Debug, Clone, PartialEq)]
pub enum ActorTransform {
    Identity,
    Matrix34([[f32; 3]; 4]),
    Translation([f32; 3]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Actor {
    pub identifier: String,
    /// BRender actor type (`NONE=0`, `MODEL=1`, `BOUNDS=5`, etc.).
    pub actor_type: u8,
    /// BRender render style (`DEFAULT=0`, `NONE=1`, `FACES=4`, etc.).
    pub render_style: u8,
    pub transform: ActorTransform,
    pub model: Option<String>,
    pub material: Option<String>,
    pub bounds: Option<[[f32; 3]; 2]>,
    pub children: Vec<Actor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActFile {
    pub roots: Vec<Actor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActError {
    pub offset: usize,
    pub detail: String,
}

impl ActError {
    fn new(offset: usize, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ActError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ACT offset {}: {}", self.offset, self.detail)
    }
}

impl std::error::Error for ActError {}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ActError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| ActError::new(self.at, "offset overflow"))?;
        let data = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| ActError::new(self.at, format!("truncated {n}-byte field")))?;
        self.at = end;
        Ok(data)
    }
    fn u8(&mut self) -> Result<u8, ActError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, ActError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, ActError> {
        let at = self.at;
        let f = f32::from_bits(self.u32()?);
        if !f.is_finite() {
            return Err(ActError::new(at, "nonfinite transform or bounds scalar"));
        }
        Ok(f)
    }
    fn vec3(&mut self) -> Result<[f32; 3], ActError> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    fn name(&mut self) -> Result<String, ActError> {
        let start = self.at;
        let end = self
            .bytes
            .get(start..start.saturating_add(256).min(self.bytes.len()))
            .and_then(|b| b.iter().position(|&v| v == 0))
            .ok_or_else(|| ActError::new(start, "missing NUL-terminated name"))?
            + start;
        let name = std::str::from_utf8(&self.bytes[start..end])
            .map_err(|_| ActError::new(start, "name is not UTF-8"))?
            .to_owned();
        self.at = end + 1;
        Ok(name)
    }
}

fn top(stack: &mut [Actor], at: usize) -> Result<&mut Actor, ActError> {
    stack
        .last_mut()
        .ok_or_else(|| ActError::new(at, "actor property without actor"))
}

impl ActFile {
    pub fn parse(bytes: &[u8]) -> Result<Self, ActError> {
        let mut c = Cursor { bytes, at: 0 };
        if c.u32()? != 18 || c.u32()? != 8 || c.u32()? != 1 || c.u32()? != 2 {
            return Err(ActError::new(0, "unsupported ACT file-info chunk"));
        }
        let mut roots = Vec::new();
        let mut stack: Vec<Actor> = Vec::new();
        let mut pending_transform: Option<ActorTransform> = None;
        let mut pending_bounds: Option<[[f32; 3]; 2]> = None;
        while c.at < bytes.len() {
            let at = c.at;
            let id = c.u32()?;
            let declared =
                usize::try_from(c.u32()?).map_err(|_| ActError::new(at, "chunk size overflow"))?;
            if declared > bytes.len() - c.at {
                return Err(ActError::new(at, "declared chunk exceeds file"));
            }
            let start = c.at;
            match id {
                35 => {
                    let actor_type = c.u8()?;
                    let render_style = c.u8()?;
                    if actor_type > 7 || render_style > 7 {
                        return Err(ActError::new(at, "unknown actor type or render style"));
                    }
                    stack.push(Actor {
                        identifier: c.name()?,
                        actor_type,
                        render_style,
                        transform: ActorTransform::Identity,
                        model: None,
                        material: None,
                        bounds: None,
                        children: Vec::new(),
                    });
                }
                36 => {
                    let actor = top(&mut stack, at)?;
                    if actor.model.replace(c.name()?).is_some() {
                        return Err(ActError::new(at, "duplicate model reference"));
                    }
                }
                38 => {
                    let actor = top(&mut stack, at)?;
                    if actor.material.replace(c.name()?).is_some() {
                        return Err(ActError::new(at, "duplicate material reference"));
                    }
                }
                43 | 44 => {
                    if pending_transform.is_some() {
                        return Err(ActError::new(at, "unattached transform"));
                    }
                    let mut matrix = [[0.0; 3]; 4];
                    for row in &mut matrix {
                        *row = c.vec3()?;
                    }
                    pending_transform = Some(ActorTransform::Matrix34(matrix));
                }
                48 => {
                    if pending_transform.is_some() {
                        return Err(ActError::new(at, "unattached transform"));
                    }
                    pending_transform = Some(ActorTransform::Translation(c.vec3()?));
                }
                49 => {
                    if pending_transform.is_some() {
                        return Err(ActError::new(at, "unattached transform"));
                    }
                    pending_transform = Some(ActorTransform::Identity);
                }
                37 if declared == 0 => {
                    let transform = pending_transform.take().ok_or_else(|| {
                        ActError::new(at, "actor transform without transform data")
                    })?;
                    top(&mut stack, at)?.transform = transform;
                }
                50 => {
                    if pending_bounds.is_some() {
                        return Err(ActError::new(at, "unattached bounds"));
                    }
                    pending_bounds = Some([c.vec3()?, c.vec3()?]);
                }
                41 if declared == 0 => {
                    let bounds = pending_bounds
                        .take()
                        .ok_or_else(|| ActError::new(at, "actor bounds without bounds data"))?;
                    top(&mut stack, at)?.bounds = Some(bounds);
                }
                42 if declared == 0 => {
                    if stack.len() < 2 {
                        return Err(ActError::new(at, "child without parent"));
                    }
                    let child = stack.pop().expect("length checked");
                    top(&mut stack, at)?.children.push(child);
                }
                0 if declared == 0 => {
                    if stack.len() != 1 || pending_transform.is_some() || pending_bounds.is_some() {
                        return Err(ActError::new(at, "incomplete actor hierarchy"));
                    }
                    roots.push(stack.pop().expect("length checked"));
                }
                _ => return Err(ActError::new(at, format!("unsupported ACT chunk {id}"))),
            }
            // BRender's name_size is inconsistent with name_write in original
            // ACT references, so read their NUL-terminated names semantically.
            if id != 36 && id != 38 && c.at - start != declared {
                return Err(ActError::new(at, format!("ACT chunk {id} length mismatch")));
            }
        }
        if !stack.is_empty()
            || roots.is_empty()
            || pending_transform.is_some()
            || pending_bounds.is_some()
        {
            return Err(ActError::new(c.at, "incomplete ACT container"));
        }
        Ok(Self { roots })
    }

    /// Resolve actor references against parsed, caller-owned files. Returned
    /// indices are `(file index, item index)` into the supplied slices.
    pub fn link(
        &self,
        models: &[&DatFile],
        materials: &[&MatFile],
    ) -> Result<Vec<LinkedActor>, ActError> {
        fn link_actor(
            actor: &Actor,
            models: &[&DatFile],
            materials: &[&MatFile],
        ) -> Result<LinkedActor, ActError> {
            let model = actor
                .model
                .as_ref()
                .map(|name| {
                    models
                        .iter()
                        .enumerate()
                        .find_map(|(file, dat)| {
                            dat.models
                                .iter()
                                .position(|m| m.identifier == *name)
                                .map(|index| (file, index))
                        })
                        .ok_or_else(|| {
                            ActError::new(
                                0,
                                format!("actor '{}' missing model '{name}'", actor.identifier),
                            )
                        })
                })
                .transpose()?;
            let material = actor
                .material
                .as_ref()
                .map(|name| {
                    materials
                        .iter()
                        .enumerate()
                        .find_map(|(file, mat)| {
                            mat.materials
                                .iter()
                                .position(|m| m.identifier == *name)
                                .map(|index| (file, index))
                        })
                        .ok_or_else(|| {
                            ActError::new(
                                0,
                                format!("actor '{}' missing material '{name}'", actor.identifier),
                            )
                        })
                })
                .transpose()?;
            let children = actor
                .children
                .iter()
                .map(|child| link_actor(child, models, materials))
                .collect::<Result<_, _>>()?;
            Ok(LinkedActor {
                model,
                material,
                children,
            })
        }
        self.roots
            .iter()
            .map(|root| link_actor(root, models, materials))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedActor {
    pub model: Option<(usize, usize)>,
    pub material: Option<(usize, usize)>,
    pub children: Vec<LinkedActor>,
}

#[cfg(test)]
mod tests {
    use super::{ActFile, ActorTransform};
    fn chunk(bytes: &mut Vec<u8>, id: u32, payload: &[u8]) {
        bytes.extend(id.to_be_bytes());
        bytes.extend((payload.len() as u32).to_be_bytes());
        bytes.extend(payload);
    }
    #[test]
    fn hierarchy_and_transform_are_checked() {
        let mut b = Vec::new();
        chunk(&mut b, 18, &[0, 0, 0, 1, 0, 0, 0, 2]);
        chunk(&mut b, 35, b"\0\0root\0");
        chunk(&mut b, 35, b"\x01\x04child\0");
        let mut matrix = Vec::new();
        for f in [1f32, 0., 0., 0., 1., 0., 0., 0., 1., 2., 3., 4.] {
            matrix.extend(f.to_be_bytes());
        }
        chunk(&mut b, 43, &matrix);
        chunk(&mut b, 37, &[]);
        chunk(&mut b, 42, &[]);
        chunk(&mut b, 0, &[]);
        let parsed = ActFile::parse(&b).unwrap();
        assert_eq!(
            parsed.roots[0].children[0].transform,
            ActorTransform::Matrix34([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [2., 3., 4.]])
        );
        assert!(parsed.link(&[], &[]).is_ok());
        let mut bad = b.clone();
        let at = bad
            .windows(4)
            .position(|x| x == 42u32.to_be_bytes())
            .unwrap();
        bad[at..at + 4].copy_from_slice(&36u32.to_be_bytes());
        assert!(ActFile::parse(&bad).is_err());
        for cut in [0, 15, 25, b.len() - 1] {
            assert!(ActFile::parse(&b[..cut]).is_err());
        }
    }
}
