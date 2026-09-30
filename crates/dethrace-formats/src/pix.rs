//! BRender PIX containers. Pixel bytes retain their source format and row padding.
//!
//! Chunk IDs, big-endian fields, and the old/new pixelmap layouts follow
//! BRender `core/pixelmap/pmfile.c` and `core/fw/datafile.c`.

use std::fmt;

const FILE_INFO: u32 = 18;
const OLD_PIXELMAP: u32 = 3;
const PIXELMAP: u32 = 61;
const PIXELS: u32 = 33;
const END: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelType {
    Index8,
    Rgb888,
    Rgbx888,
    Rgba8888,
}

impl PixelType {
    fn parse(value: u8, offset: usize) -> Result<Self, PixError> {
        match value {
            3 => Ok(Self::Index8),
            6 => Ok(Self::Rgb888),
            7 => Ok(Self::Rgbx888),
            8 => Ok(Self::Rgba8888),
            _ => Err(PixError::new(
                offset,
                format!("unsupported pixel type {value}"),
            )),
        }
    }

    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Index8 => 1,
            Self::Rgb888 => 3,
            Self::Rgbx888 | Self::Rgba8888 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixelmap {
    pub identifier: String,
    pub pixel_type: PixelType,
    pub width: u16,
    pub height: u16,
    pub row_bytes: u16,
    pub origin_x: u16,
    pub origin_y: u16,
    pub mip_offset: u16,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixFile {
    pub pixelmaps: Vec<Pixelmap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixError {
    pub offset: usize,
    pub detail: String,
}

impl PixError {
    fn new(offset: usize, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for PixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PIX offset {}: {}", self.offset, self.detail)
    }
}

impl std::error::Error for PixError {}

fn be_u16(bytes: &[u8], at: usize, base: usize) -> Result<u16, PixError> {
    let pair = bytes
        .get(at..at + 2)
        .ok_or_else(|| PixError::new(base + at, "truncated u16"))?;
    Ok(u16::from_be_bytes([pair[0], pair[1]]))
}

fn be_u32(bytes: &[u8], at: usize, base: usize) -> Result<u32, PixError> {
    let four = bytes
        .get(at..at + 4)
        .ok_or_else(|| PixError::new(base + at, "truncated u32"))?;
    Ok(u32::from_be_bytes([four[0], four[1], four[2], four[3]]))
}

fn parse_pixelmap(bytes: &[u8], base: usize, modern: bool) -> Result<Pixelmap, PixError> {
    let pixel_type = PixelType::parse(
        *bytes
            .first()
            .ok_or_else(|| PixError::new(base, "empty pixelmap header"))?,
        base,
    )?;
    let row_bytes = be_u16(bytes, 1, base)?;
    let width = be_u16(bytes, 3, base)?;
    let height = be_u16(bytes, 5, base)?;
    let origin_x = be_u16(bytes, 7, base)?;
    let origin_y = be_u16(bytes, 9, base)?;
    let mip_offset = if modern { be_u16(bytes, 11, base)? } else { 0 };
    let name_at = if modern { 13 } else { 11 };
    let name = bytes
        .get(name_at..)
        .and_then(|rest| rest.strip_suffix(&[0]))
        .ok_or_else(|| PixError::new(base + name_at, "missing NUL-terminated identifier"))?;
    let identifier = std::str::from_utf8(name)
        .map_err(|_| PixError::new(base + name_at, "identifier is not UTF-8"))?
        .to_owned();
    if width == 0 || height == 0 {
        return Err(PixError::new(base + 3, "zero pixelmap dimension"));
    }
    let min_row = usize::from(width) * pixel_type.bytes_per_pixel();
    if usize::from(row_bytes) < min_row {
        return Err(PixError::new(
            base + 1,
            "row stride is shorter than pixel width",
        ));
    }
    if mip_offset != 0 {
        return Err(PixError::new(base + 11, "mipmapped PIX is unsupported"));
    }
    Ok(Pixelmap {
        identifier,
        pixel_type,
        width,
        height,
        row_bytes,
        origin_x,
        origin_y,
        mip_offset,
        pixels: Vec::new(),
    })
}

impl PixFile {
    pub fn parse(bytes: &[u8]) -> Result<Self, PixError> {
        let mut offset = 0usize;
        let mut seen_info = false;
        let mut current: Option<Pixelmap> = None;
        let mut pixelmaps = Vec::new();
        while offset < bytes.len() {
            let id = be_u32(bytes, offset, 0)?;
            let length = usize::try_from(be_u32(bytes, offset + 4, 0)?)
                .map_err(|_| PixError::new(offset + 4, "chunk length is too large"))?;
            let data_at = offset + 8;
            let end = data_at
                .checked_add(length)
                .ok_or_else(|| PixError::new(offset + 4, "chunk end overflows"))?;
            let data = bytes.get(data_at..end).ok_or_else(|| {
                PixError::new(offset, format!("truncated chunk {id} ({length} bytes)"))
            })?;
            if !seen_info && id != FILE_INFO {
                return Err(PixError::new(offset, "missing PIX file-info chunk"));
            }
            match id {
                FILE_INFO if !seen_info && length == 8 => {
                    if be_u32(data, 0, data_at)? != 2 || be_u32(data, 4, data_at)? != 2 {
                        return Err(PixError::new(
                            data_at,
                            "unsupported PIX file type or version",
                        ));
                    }
                    seen_info = true;
                }
                OLD_PIXELMAP | PIXELMAP if current.is_none() => {
                    current = Some(parse_pixelmap(data, data_at, id == PIXELMAP)?);
                }
                PIXELS => {
                    let map = current
                        .as_mut()
                        .ok_or_else(|| PixError::new(offset, "pixel data without pixelmap"))?;
                    if !map.pixels.is_empty() {
                        return Err(PixError::new(offset, "duplicate pixel data"));
                    }
                    let count = usize::try_from(be_u32(data, 0, data_at)?)
                        .map_err(|_| PixError::new(data_at, "pixel count is too large"))?;
                    let size = usize::try_from(be_u32(data, 4, data_at)?)
                        .map_err(|_| PixError::new(data_at + 4, "pixel size is too large"))?;
                    let expected = usize::from(map.row_bytes) * usize::from(map.height);
                    if size != map.pixel_type.bytes_per_pixel()
                        || count.checked_mul(size) != Some(expected)
                        || data.len() != 8 + expected
                    {
                        return Err(PixError::new(
                            offset,
                            format!("invalid pixel payload for {}", map.identifier),
                        ));
                    }
                    map.pixels = data[8..].to_vec();
                }
                END if length == 0 => {
                    let map = current
                        .take()
                        .ok_or_else(|| PixError::new(offset, "end chunk without pixelmap"))?;
                    if map.pixels.is_empty() {
                        return Err(PixError::new(
                            offset,
                            format!("missing pixels for {}", map.identifier),
                        ));
                    }
                    pixelmaps.push(map);
                }
                _ => return Err(PixError::new(offset, format!("unexpected chunk {id}"))),
            }
            offset = end;
        }
        if !seen_info || current.is_some() || pixelmaps.is_empty() {
            return Err(PixError::new(offset, "incomplete PIX container"));
        }
        Ok(Self { pixelmaps })
    }
}

#[cfg(test)]
mod tests {
    use super::{PixFile, PixelType};

    fn chunk(out: &mut Vec<u8>, id: u32, data: &[u8]) {
        out.extend(id.to_be_bytes());
        out.extend((data.len() as u32).to_be_bytes());
        out.extend(data);
    }

    fn sample() -> Vec<u8> {
        let mut out = Vec::new();
        chunk(&mut out, 18, &[0, 0, 0, 2, 0, 0, 0, 2]);
        for (name, pixels) in [
            (b"one\0".as_slice(), &[1, 2, 3, 4][..]),
            (b"two\0", &[5, 6, 7, 8][..]),
        ] {
            let mut header = vec![3, 0, 2, 0, 2, 0, 2, 0, 0, 0, 0];
            header.extend(name);
            chunk(&mut out, 3, &header);
            let mut payload = vec![0, 0, 0, 4, 0, 0, 0, 1];
            payload.extend(pixels);
            chunk(&mut out, 33, &payload);
            chunk(&mut out, 0, &[]);
        }
        out
    }

    #[test]
    fn parses_multiple_indexed_maps_and_rejects_bad_data() {
        let bytes = sample();
        let file = PixFile::parse(&bytes).unwrap();
        assert_eq!(file.pixelmaps.len(), 2);
        assert_eq!(file.pixelmaps[0].identifier, "one");
        assert_eq!(file.pixelmaps[0].pixel_type, PixelType::Index8);
        assert_eq!(file.pixelmaps[1].pixels, [5, 6, 7, 8]);
        for cut in [1, 7, 8, 15, bytes.len() - 1] {
            assert!(PixFile::parse(&bytes[..cut]).is_err());
        }
        let mut bad_stride = bytes.clone();
        bad_stride[26] = 1;
        assert!(PixFile::parse(&bad_stride).is_err());
        let mut bad_count = bytes.clone();
        bad_count[50] = 3;
        assert!(PixFile::parse(&bad_count).is_err());
    }
}
