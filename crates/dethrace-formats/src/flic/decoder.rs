use std::fmt;

use crate::binary::{BinaryReadError, BinaryReader};
use crate::image::{IndexedImage, IndexedImageError, Palette256};

use super::{Flic, FlicFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlicDecodeErrorKind {
    InvalidImage(IndexedImageError),
    CopyPayloadTooShort {
        expected: usize,
        available: usize,
    },
    Read(BinaryReadError),
    PaletteOverflow {
        start: usize,
        count: usize,
    },
    RowOverflow {
        row: usize,
        start: usize,
        count: usize,
        width: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlicDecodeError {
    pub offset: Option<usize>,
    pub frame_index: Option<usize>,
    pub chunk_index: Option<usize>,
    pub kind: FlicDecodeErrorKind,
}

impl fmt::Display for FlicDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FLIC decode error: {:?}", self.kind)?;
        if let Some(offset) = self.offset {
            write!(f, " at offset {offset}")?;
        }
        if let Some(frame_index) = self.frame_index {
            write!(f, " in frame {frame_index}")?;
        }
        if let Some(chunk_index) = self.chunk_index {
            write!(f, " chunk {chunk_index}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FlicDecodeError {}

pub struct FlicDecoder<'flic, 'bytes> {
    frames: &'flic [FlicFrame<'bytes>],
    image: IndexedImage,
    palette: Palette256,
    frame_position: usize,
}

impl<'flic, 'bytes> FlicDecoder<'flic, 'bytes> {
    pub fn new(flic: &'flic Flic<'bytes>) -> Result<Self, FlicDecodeError> {
        let image = IndexedImage::new(
            usize::from(flic.header.width),
            usize::from(flic.header.height),
        )
        .map_err(|error| FlicDecodeError {
            offset: None,
            frame_index: None,
            chunk_index: None,
            kind: FlicDecodeErrorKind::InvalidImage(error),
        })?;

        Ok(Self {
            frames: &flic.frames,
            image,
            palette: Palette256::default(),
            frame_position: 0,
        })
    }

    pub fn image(&self) -> &IndexedImage {
        &self.image
    }

    pub fn palette(&self) -> &Palette256 {
        &self.palette
    }

    /// Number of frames already decoded, and the index of the next frame.
    pub fn frame_position(&self) -> usize {
        self.frame_position
    }

    /// Decode the next frame, returning false once all frames are consumed.
    pub fn decode_next_frame(&mut self) -> Result<bool, FlicDecodeError> {
        let Some(frame) = self.frames.get(self.frame_position) else {
            return Ok(false);
        };

        let mut image = self.image.clone();
        let mut palette = self.palette.clone();
        for (chunk_index, chunk) in frame.chunks.iter().enumerate() {
            let result = match chunk.chunk_type {
                4 | 11 => decode_palette(&mut palette, chunk.payload, chunk.chunk_type == 11),
                15 => decode_byte_run(&mut image, chunk.payload),
                13 => {
                    image.pixels_mut().fill(0);
                    Ok(())
                }
                16 => {
                    let expected = image.pixels().len();
                    if chunk.payload.len() < expected {
                        Err(FlicDecodeErrorKind::CopyPayloadTooShort {
                            expected,
                            available: chunk.payload.len(),
                        })
                    } else {
                        image
                            .pixels_mut()
                            .copy_from_slice(&chunk.payload[..expected]);
                        Ok(())
                    }
                }
                // Upstream DoMini skips the payload without changing the image.
                18 => Ok(()),
                _ => Ok(()),
            };
            result.map_err(|kind| FlicDecodeError {
                offset: Some(chunk.offset),
                frame_index: Some(self.frame_position),
                chunk_index: Some(chunk_index),
                kind,
            })?;
        }

        self.image = image;
        self.palette = palette;
        self.frame_position += 1;
        Ok(true)
    }
}

fn decode_palette(
    palette: &mut Palette256,
    bytes: &[u8],
    scale_64: bool,
) -> Result<(), FlicDecodeErrorKind> {
    let mut reader = BinaryReader::new(bytes);
    let packet_count = reader.read_u16_le().map_err(FlicDecodeErrorKind::Read)?;
    let mut current = 0usize;
    for _ in 0..packet_count {
        current += usize::from(reader.read_u8().map_err(FlicDecodeErrorKind::Read)?);
        let raw_count = reader.read_u8().map_err(FlicDecodeErrorKind::Read)?;
        let count = if raw_count == 0 {
            256
        } else {
            usize::from(raw_count)
        };
        let end = current
            .checked_add(count)
            .ok_or(FlicDecodeErrorKind::PaletteOverflow {
                start: current,
                count,
            })?;
        if end > Palette256::LEN {
            return Err(FlicDecodeErrorKind::PaletteOverflow {
                start: current,
                count,
            });
        }
        for index in current..end {
            let rgb = reader.take(3).map_err(FlicDecodeErrorKind::Read)?;
            // COLOR_64 stores 6-bit channels; upstream expands them by multiplication by four.
            let color = if scale_64 {
                [
                    rgb[0].wrapping_mul(4),
                    rgb[1].wrapping_mul(4),
                    rgb[2].wrapping_mul(4),
                ]
            } else {
                [rgb[0], rgb[1], rgb[2]]
            };
            palette.set_color(index as u8, color);
        }
        current = end;
    }
    Ok(())
}

fn decode_byte_run(image: &mut IndexedImage, bytes: &[u8]) -> Result<(), FlicDecodeErrorKind> {
    let mut reader = BinaryReader::new(bytes);
    let width = image.width();
    for y in 0..image.height() {
        let packet_count = reader.read_u8().map_err(FlicDecodeErrorKind::Read)?;
        let row = image.row_mut(y).expect("row index is within image height");
        let mut x = 0usize;
        for _ in 0..packet_count {
            let count = reader.read_i8().map_err(FlicDecodeErrorKind::Read)?;
            let len = usize::from(count.unsigned_abs());
            let end = x.checked_add(len).ok_or(FlicDecodeErrorKind::RowOverflow {
                row: y,
                start: x,
                count: len,
                width,
            })?;
            if end > width {
                return Err(FlicDecodeErrorKind::RowOverflow {
                    row: y,
                    start: x,
                    count: len,
                    width,
                });
            }
            if count < 0 {
                row[x..end].copy_from_slice(reader.take(len).map_err(FlicDecodeErrorKind::Read)?);
            } else {
                row[x..end].fill(reader.read_u8().map_err(FlicDecodeErrorKind::Read)?);
            }
            x = end;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flic::{FlicChunk, FlicFormat, FlicHeader};

    fn decode(chunks: Vec<FlicChunk<'_>>) -> Flic<'_> {
        Flic {
            header: FlicHeader {
                declared_file_size: 128,
                format: FlicFormat::Flc,
                frame_count: 1,
                width: 4,
                height: 2,
                bit_depth: 8,
                flags: 0,
                claimed_speed: 10,
            },
            frames: vec![FlicFrame {
                offset: 128,
                length: 16,
                chunks,
            }],
        }
    }

    fn chunk(kind: u16, payload: &[u8]) -> FlicChunk<'_> {
        FlicChunk {
            offset: 144,
            length: (payload.len() + 6) as u32,
            chunk_type: kind,
            payload,
        }
    }

    #[test]
    fn palette_packets_and_byte_runs() {
        let palette = [2, 0, 0, 1, 10, 20, 30, 1, 1, 40, 50, 60];
        let rle = [2, -2i8 as u8, 1, 2, 2, 3, 1, -4i8 as u8, 4, 5, 6, 7];
        let flic = decode(vec![chunk(4, &palette), chunk(15, &rle)]);
        let mut decoder = FlicDecoder::new(&flic).unwrap();
        decoder.decode_next_frame().unwrap();
        assert_eq!(decoder.palette().color(0), &[10, 20, 30]);
        assert_eq!(decoder.palette().color(2), &[40, 50, 60]);
        assert_eq!(decoder.image().pixels(), &[1, 2, 3, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn palette_64_scales_and_zero_count_means_256() {
        let mut payload = vec![1, 0, 0, 0];
        payload.extend([63, 0, 1]);
        payload.extend(vec![0; 255 * 3]);
        let flic = decode(vec![chunk(11, &payload)]);
        let mut decoder = FlicDecoder::new(&flic).unwrap();
        decoder.decode_next_frame().unwrap();
        assert_eq!(decoder.palette().color(0), &[252, 0, 4]);
    }

    #[test]
    fn malformed_packets_keep_prior_state() {
        for (kind, payload) in [
            (4, vec![1, 0, 255, 2]),
            (4, vec![1, 0, 0, 1]),
            (15, vec![1, 5, 9]),
            (15, vec![1, 0]),
        ] {
            let flic = decode(vec![chunk(kind, &payload)]);
            let mut decoder = FlicDecoder::new(&flic).unwrap();
            assert!(decoder.decode_next_frame().is_err());
            assert_eq!(decoder.frame_position(), 0);
            assert_eq!(decoder.image().pixels(), &[0; 8]);
        }
    }
}
