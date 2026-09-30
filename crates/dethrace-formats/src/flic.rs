mod decoder;

pub use decoder::{FlicDecodeError, FlicDecodeErrorKind, FlicDecoder};

use std::fmt;

use crate::binary::{BinaryReadError, BinaryReadOperation, BinaryReader};

const FLIC_HEADER_SIZE: usize = 128;
const FLIC_FRAME_HEADER_SIZE: usize = 16;
const FLIC_CHUNK_HEADER_SIZE: usize = 6;
const FRAME_MAGIC: u16 = 0xf1fa;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlicFormat {
    Fli,
    Flc,
}

impl FlicFormat {
    fn from_magic(magic: u16) -> Option<Self> {
        match magic {
            0xaf11 => Some(Self::Fli),
            0xaf12 => Some(Self::Flc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlicHeader {
    pub declared_file_size: u32,
    pub format: FlicFormat,
    pub frame_count: u16,
    pub width: u16,
    pub height: u16,
    pub bit_depth: u16,
    pub flags: u16,
    pub claimed_speed: u16,
}

impl FlicHeader {
    /// Returns the frame period using the timing conversion used by upstream.
    pub fn frame_delay_ms(self) -> u32 {
        match self.format {
            FlicFormat::Fli => u32::from(self.claimed_speed) * 14,
            FlicFormat::Flc => u32::from(self.claimed_speed),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlicChunk<'a> {
    pub offset: usize,
    pub length: u32,
    pub chunk_type: u16,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlicFrame<'a> {
    pub offset: usize,
    pub length: u32,
    pub chunks: Vec<FlicChunk<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flic<'a> {
    pub header: FlicHeader,
    pub frames: Vec<FlicFrame<'a>>,
}

impl<'a> Flic<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, FlicParseError> {
        let mut reader = BinaryReader::new(bytes);
        let declared_file_size = reader
            .read_u32_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        if declared_file_size < FLIC_HEADER_SIZE as u32 {
            return Err(FlicParseError::new(
                0,
                None,
                None,
                FlicParseErrorKind::FileSizeTooSmall {
                    declared: declared_file_size,
                    minimum: FLIC_HEADER_SIZE,
                },
            ));
        }

        let file_size = usize::try_from(declared_file_size).map_err(|_| {
            FlicParseError::new(0, None, None, FlicParseErrorKind::ArithmeticOverflow)
        })?;
        if file_size > bytes.len() {
            return Err(FlicParseError::new(
                0,
                None,
                None,
                FlicParseErrorKind::FileSizeExceedsInput {
                    declared: file_size,
                    available: bytes.len(),
                },
            ));
        }

        let mut reader = BinaryReader::new(&bytes[..file_size]);
        reader
            .read_u32_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let magic_offset = reader.position();
        let magic = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let format = FlicFormat::from_magic(magic).ok_or_else(|| {
            FlicParseError::new(
                magic_offset,
                None,
                None,
                FlicParseErrorKind::UnsupportedMagic { magic },
            )
        })?;
        let frame_count = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let width = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let height = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let depth_offset = reader.position();
        let bit_depth = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        if bit_depth != 8 {
            return Err(FlicParseError::new(
                depth_offset,
                None,
                None,
                FlicParseErrorKind::UnsupportedBitDepth { bit_depth },
            ));
        }
        let flags = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let claimed_speed = reader
            .read_u16_le()
            .map_err(|error| FlicParseError::from_read(error, None, None))?;
        let header_padding = FLIC_HEADER_SIZE - reader.position();
        reader
            .skip(header_padding)
            .map_err(|error| FlicParseError::from_read(error, None, None))?;

        let header = FlicHeader {
            declared_file_size,
            format,
            frame_count,
            width,
            height,
            bit_depth,
            flags,
            claimed_speed,
        };
        let mut frames = Vec::new();

        for frame_index in 0..usize::from(frame_count) {
            let frame_offset = reader.position();
            let frame_length = reader
                .read_u32_le()
                .map_err(|error| FlicParseError::from_read(error, Some(frame_index), None))?;
            if frame_length < FLIC_FRAME_HEADER_SIZE as u32 {
                return Err(FlicParseError::new(
                    frame_offset,
                    Some(frame_index),
                    None,
                    FlicParseErrorKind::FrameSizeTooSmall {
                        length: frame_length,
                    },
                ));
            }
            let frame_length_usize = usize::try_from(frame_length).map_err(|_| {
                FlicParseError::new(
                    frame_offset,
                    Some(frame_index),
                    None,
                    FlicParseErrorKind::ArithmeticOverflow,
                )
            })?;
            let frame_end = frame_offset
                .checked_add(frame_length_usize)
                .ok_or_else(|| {
                    FlicParseError::new(
                        frame_offset,
                        Some(frame_index),
                        None,
                        FlicParseErrorKind::ArithmeticOverflow,
                    )
                })?;
            if frame_end > file_size {
                return Err(FlicParseError::new(
                    frame_offset,
                    Some(frame_index),
                    None,
                    FlicParseErrorKind::FrameExceedsFile {
                        end: frame_end,
                        file_size,
                    },
                ));
            }

            let frame_magic_offset = reader.position();
            let frame_magic = reader
                .read_u16_le()
                .map_err(|error| FlicParseError::from_read(error, Some(frame_index), None))?;
            if frame_magic != FRAME_MAGIC {
                return Err(FlicParseError::new(
                    frame_magic_offset,
                    Some(frame_index),
                    None,
                    FlicParseErrorKind::InvalidFrameMagic { magic: frame_magic },
                ));
            }
            let chunk_count = reader
                .read_u16_le()
                .map_err(|error| FlicParseError::from_read(error, Some(frame_index), None))?;
            reader
                .skip(8)
                .map_err(|error| FlicParseError::from_read(error, Some(frame_index), None))?;

            let mut chunks = Vec::new();
            for chunk_index in 0..usize::from(chunk_count) {
                let chunk_offset = reader.position();
                let remaining_in_frame = frame_end.saturating_sub(chunk_offset);
                if remaining_in_frame < FLIC_CHUNK_HEADER_SIZE {
                    return Err(FlicParseError::new(
                        chunk_offset,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::UnexpectedEnd {
                            operation: BinaryReadOperation::Take,
                            requested: FLIC_CHUNK_HEADER_SIZE,
                            remaining: remaining_in_frame,
                        },
                    ));
                }

                let chunk_length = reader.read_u32_le().map_err(|error| {
                    FlicParseError::from_read(error, Some(frame_index), Some(chunk_index))
                })?;
                let kind = reader.read_u16_le().map_err(|error| {
                    FlicParseError::from_read(error, Some(frame_index), Some(chunk_index))
                })?;
                if chunk_length < FLIC_CHUNK_HEADER_SIZE as u32 {
                    return Err(FlicParseError::new(
                        chunk_offset,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::ChunkSizeTooSmall {
                            length: chunk_length,
                        },
                    ));
                }

                let chunk_length_usize = usize::try_from(chunk_length).map_err(|_| {
                    FlicParseError::new(
                        chunk_offset,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::ArithmeticOverflow,
                    )
                })?;
                let chunk_end = chunk_offset
                    .checked_add(chunk_length_usize)
                    .ok_or_else(|| {
                        FlicParseError::new(
                            chunk_offset,
                            Some(frame_index),
                            Some(chunk_index),
                            FlicParseErrorKind::ArithmeticOverflow,
                        )
                    })?;
                if chunk_end > frame_end {
                    return Err(FlicParseError::new(
                        chunk_offset,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::ChunkExceedsFrame {
                            end: chunk_end,
                            frame_end,
                        },
                    ));
                }

                let payload_length = chunk_length_usize - FLIC_CHUNK_HEADER_SIZE;
                let payload = reader.take(payload_length).map_err(|error| {
                    FlicParseError::from_read(error, Some(frame_index), Some(chunk_index))
                })?;
                let aligned_end = chunk_end.checked_add(chunk_end % 2).ok_or_else(|| {
                    FlicParseError::new(
                        chunk_offset,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::ArithmeticOverflow,
                    )
                })?;
                if aligned_end > frame_end {
                    return Err(FlicParseError::new(
                        chunk_end,
                        Some(frame_index),
                        Some(chunk_index),
                        FlicParseErrorKind::ChunkAlignmentExceedsFrame { frame_end },
                    ));
                }
                reader.skip(aligned_end - chunk_end).map_err(|error| {
                    FlicParseError::from_read(error, Some(frame_index), Some(chunk_index))
                })?;

                chunks.push(FlicChunk {
                    offset: chunk_offset,
                    length: chunk_length,
                    chunk_type: kind,
                    payload,
                });
            }

            let frame_padding = frame_end - reader.position();
            reader
                .skip(frame_padding)
                .map_err(|error| FlicParseError::from_read(error, Some(frame_index), None))?;
            frames.push(FlicFrame {
                offset: frame_offset,
                length: frame_length,
                chunks,
            });
        }

        Ok(Self { header, frames })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlicParseErrorKind {
    UnexpectedEnd {
        operation: BinaryReadOperation,
        requested: usize,
        remaining: usize,
    },
    FileSizeTooSmall {
        declared: u32,
        minimum: usize,
    },
    FileSizeExceedsInput {
        declared: usize,
        available: usize,
    },
    UnsupportedMagic {
        magic: u16,
    },
    UnsupportedBitDepth {
        bit_depth: u16,
    },
    FrameSizeTooSmall {
        length: u32,
    },
    FrameExceedsFile {
        end: usize,
        file_size: usize,
    },
    InvalidFrameMagic {
        magic: u16,
    },
    ChunkSizeTooSmall {
        length: u32,
    },
    ChunkExceedsFrame {
        end: usize,
        frame_end: usize,
    },
    ChunkAlignmentExceedsFrame {
        frame_end: usize,
    },
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlicParseError {
    pub offset: usize,
    pub frame_index: Option<usize>,
    pub chunk_index: Option<usize>,
    pub kind: FlicParseErrorKind,
}

impl FlicParseError {
    fn new(
        offset: usize,
        frame_index: Option<usize>,
        chunk_index: Option<usize>,
        kind: FlicParseErrorKind,
    ) -> Self {
        Self {
            offset,
            frame_index,
            chunk_index,
            kind,
        }
    }

    fn from_read(
        error: BinaryReadError,
        frame_index: Option<usize>,
        chunk_index: Option<usize>,
    ) -> Self {
        Self::new(
            error.offset,
            frame_index,
            chunk_index,
            FlicParseErrorKind::UnexpectedEnd {
                operation: error.operation,
                requested: error.requested,
                remaining: error.remaining,
            },
        )
    }
}

impl fmt::Display for FlicParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FLIC parse error at offset {}: {:?}",
            self.offset, self.kind
        )?;
        if let Some(frame_index) = self.frame_index {
            write!(f, " in frame {frame_index}")?;
        }
        if let Some(chunk_index) = self.chunk_index {
            write!(f, " chunk {chunk_index}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FlicParseError {}

#[cfg(test)]
mod tests {
    use super::{Flic, FlicDecodeErrorKind, FlicDecoder, FlicFormat, FlicParseErrorKind};

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn chunk(kind: u16, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(6_u32 + payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn frame(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        put_u16(&mut bytes, 4, 0xf1fa);
        put_u16(&mut bytes, 6, chunks.len() as u16);
        for chunk in chunks {
            bytes.extend_from_slice(chunk);
            if !bytes.len().is_multiple_of(2) {
                bytes.push(0);
            }
        }
        let frame_size = bytes.len() as u32;
        put_u32(&mut bytes, 0, frame_size);
        bytes
    }

    fn file(magic: u16, frames: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = vec![0; 128];
        for frame in frames {
            bytes.extend_from_slice(frame);
        }
        let file_size = bytes.len() as u32;
        put_u32(&mut bytes, 0, file_size);
        put_u16(&mut bytes, 4, magic);
        put_u16(&mut bytes, 6, frames.len() as u16);
        put_u16(&mut bytes, 8, 320);
        put_u16(&mut bytes, 10, 200);
        put_u16(&mut bytes, 12, 8);
        put_u16(&mut bytes, 14, 0);
        put_u16(&mut bytes, 16, 25);
        bytes
    }

    fn decoder_file(frames: &[Vec<u8>], width: u16, height: u16) -> Vec<u8> {
        let mut bytes = file(0xaf12, frames);
        put_u16(&mut bytes, 8, width);
        put_u16(&mut bytes, 10, height);
        bytes
    }

    #[test]
    fn parses_minimal_fli_and_flc_files() {
        let empty_frame = frame(&[]);
        let fli_bytes = file(0xaf11, std::slice::from_ref(&empty_frame));
        let flc_bytes = file(0xaf12, &[empty_frame]);
        let fli = Flic::parse(&fli_bytes).unwrap();
        let flc = Flic::parse(&flc_bytes).unwrap();

        assert_eq!(fli.header.format, FlicFormat::Fli);
        assert_eq!(fli.header.frame_delay_ms(), 350);
        assert_eq!(flc.header.format, FlicFormat::Flc);
        assert_eq!(flc.header.frame_delay_ms(), 25);
        assert_eq!((fli.header.width, fli.header.height), (320, 200));
        assert_eq!(fli.frames.len(), 1);
        assert!(fli.frames[0].chunks.is_empty());
    }

    #[test]
    fn parses_multiple_frames_and_chunks() {
        let first = frame(&[chunk(4, &[1, 2]), chunk(11, &[3, 4, 5, 6])]);
        let second = frame(&[chunk(16, &[7])]);
        let bytes = file(0xaf11, &[first, second]);
        let flic = Flic::parse(&bytes).unwrap();

        assert_eq!(flic.frames.len(), 2);
        assert_eq!(flic.frames[0].chunks.len(), 2);
        assert_eq!(flic.frames[0].chunks[0].chunk_type, 4);
        assert_eq!(flic.frames[0].chunks[1].chunk_type, 11);
        assert_eq!(flic.frames[1].chunks[0].chunk_type, 16);
        assert_eq!(flic.frames[0].chunks[0].payload, &[1, 2]);
        assert_eq!(flic.frames[1].chunks[0].payload, &[7]);
    }

    #[test]
    fn odd_chunk_payload_is_followed_by_even_alignment() {
        let bytes = file(0xaf12, &[frame(&[chunk(99, &[0xaa]), chunk(100, &[0xbb])])]);
        let flic = Flic::parse(&bytes).unwrap();
        let chunks = &flic.frames[0].chunks;

        assert_eq!(chunks[0].length, 7);
        assert_eq!(chunks[0].payload, &[0xaa]);
        assert_eq!(chunks[1].payload, &[0xbb]);
        assert_eq!(chunks[1].offset % 2, 0);
    }

    #[test]
    fn rejects_unsupported_file_magic() {
        let error = Flic::parse(&file(0x1234, &[frame(&[])])).unwrap_err();
        assert_eq!(error.offset, 4);
        assert_eq!(
            error.kind,
            FlicParseErrorKind::UnsupportedMagic { magic: 0x1234 }
        );
    }

    #[test]
    fn rejects_non_eight_bit_files() {
        let mut bytes = file(0xaf11, &[frame(&[])]);
        put_u16(&mut bytes, 12, 16);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(error.offset, 12);
        assert_eq!(
            error.kind,
            FlicParseErrorKind::UnsupportedBitDepth { bit_depth: 16 }
        );
    }

    #[test]
    fn rejects_bad_frame_magic() {
        let mut bytes = file(0xaf11, &[frame(&[])]);
        put_u16(&mut bytes, 128 + 4, 0);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(error.frame_index, Some(0));
        assert_eq!(
            error.kind,
            FlicParseErrorKind::InvalidFrameMagic { magic: 0 }
        );
    }

    #[test]
    fn rejects_truncated_frame() {
        let mut bytes = file(0xaf11, &[frame(&[])]);
        put_u32(&mut bytes, 128, 100);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(error.frame_index, Some(0));
        assert!(matches!(
            error.kind,
            FlicParseErrorKind::FrameExceedsFile { .. }
        ));
    }

    #[test]
    fn rejects_truncated_chunk() {
        let mut bytes = file(0xaf11, &[frame(&[chunk(4, &[1])])]);
        put_u32(&mut bytes, 128 + 16, 100);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(error.frame_index, Some(0));
        assert_eq!(error.chunk_index, Some(0));
        assert!(matches!(
            error.kind,
            FlicParseErrorKind::ChunkExceedsFrame { .. }
        ));
    }

    #[test]
    fn rejects_impossible_chunk_size() {
        let mut bytes = file(0xaf12, &[frame(&[chunk(4, &[])])]);
        put_u32(&mut bytes, 128 + 16, u32::MAX);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(error.chunk_index, Some(0));
        assert!(matches!(
            error.kind,
            FlicParseErrorKind::ChunkExceedsFrame { .. }
        ));
    }

    #[test]
    fn rejects_declared_file_size_past_input() {
        let mut bytes = file(0xaf11, &[frame(&[])]);
        let declared = bytes.len() as u32 + 1;
        put_u32(&mut bytes, 0, declared);
        let error = Flic::parse(&bytes).unwrap_err();
        assert_eq!(
            error.kind,
            FlicParseErrorKind::FileSizeExceedsInput {
                declared: declared as usize,
                available: bytes.len(),
            }
        );
    }
    #[test]
    fn decodes_uncompressed_black_and_mini_frames_in_sequence() {
        let frames = [
            frame(&[chunk(16, &[1, 2, 3, 4, 5, 6])]),
            frame(&[chunk(18, &[0xaa, 0xbb])]),
            frame(&[chunk(13, &[])]),
        ];
        let bytes = decoder_file(&frames, 3, 2);
        let flic = Flic::parse(&bytes).unwrap();
        let mut decoder = FlicDecoder::new(&flic).unwrap();

        assert_eq!(decoder.frame_position(), 0);
        assert!(decoder.decode_next_frame().unwrap());
        assert_eq!(decoder.image().pixels(), &[1, 2, 3, 4, 5, 6]);
        assert!(decoder.decode_next_frame().unwrap());
        assert_eq!(decoder.image().pixels(), &[1, 2, 3, 4, 5, 6]);
        assert!(decoder.decode_next_frame().unwrap());
        assert_eq!(decoder.image().pixels(), &[0; 6]);
        assert_eq!(decoder.frame_position(), 3);
        assert!(!decoder.decode_next_frame().unwrap());
        assert_eq!(decoder.palette().entries().len(), 256);
    }

    #[test]
    fn copy_chunk_ignores_extra_payload_bytes_and_keeps_chunks_separate() {
        let frames = [frame(&[
            chunk(16, &[1, 2, 3, 4, 5, 6, 0xee]),
            chunk(16, &[7, 8, 9, 10, 11, 12]),
        ])];
        let bytes = decoder_file(&frames, 3, 2);
        let flic = Flic::parse(&bytes).unwrap();
        let mut decoder = FlicDecoder::new(&flic).unwrap();

        assert!(decoder.decode_next_frame().unwrap());
        assert_eq!(decoder.image().pixels(), &[7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn truncated_copy_does_not_read_from_the_following_chunk() {
        let frames = [frame(&[chunk(16, &[1, 2, 3, 4, 5]), chunk(99, &[6])])];
        let bytes = decoder_file(&frames, 3, 2);
        let flic = Flic::parse(&bytes).unwrap();
        let mut decoder = FlicDecoder::new(&flic).unwrap();

        let error = decoder.decode_next_frame().unwrap_err();
        assert_eq!(error.frame_index, Some(0));
        assert_eq!(error.chunk_index, Some(0));
        assert_eq!(
            error.kind,
            FlicDecodeErrorKind::CopyPayloadTooShort {
                expected: 6,
                available: 5,
            }
        );
        assert_eq!(decoder.frame_position(), 0);
    }
}
