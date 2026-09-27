use std::fmt;

use crate::image::{IndexedImage, IndexedImageError, Palette256};

use super::{Flic, FlicFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlicDecodeErrorKind {
    InvalidImage(IndexedImageError),
    CopyPayloadTooShort { expected: usize, available: usize },
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

        for (chunk_index, chunk) in frame.chunks.iter().enumerate() {
            match chunk.chunk_type {
                13 => self.image.pixels_mut().fill(0),
                16 => {
                    let expected = self.image.pixels().len();
                    if chunk.payload.len() < expected {
                        return Err(FlicDecodeError {
                            offset: Some(chunk.offset),
                            frame_index: Some(self.frame_position),
                            chunk_index: Some(chunk_index),
                            kind: FlicDecodeErrorKind::CopyPayloadTooShort {
                                expected,
                                available: chunk.payload.len(),
                            },
                        });
                    }
                    self.image
                        .pixels_mut()
                        .copy_from_slice(&chunk.payload[..expected]);
                }
                // Upstream DoMini skips the payload without changing the image.
                18 => {}
                _ => {}
            }
        }

        self.frame_position += 1;
        Ok(true)
    }
}
