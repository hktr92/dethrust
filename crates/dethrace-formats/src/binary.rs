use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryReadOperation {
    U8,
    I8,
    U16Le,
    I16Le,
    U32Le,
    Take,
    Skip,
    AlignTo2,
}

impl fmt::Display for BinaryReadOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::U8 => "read u8",
            Self::I8 => "read i8",
            Self::U16Le => "read u16_le",
            Self::I16Le => "read i16_le",
            Self::U32Le => "read u32_le",
            Self::Take => "take",
            Self::Skip => "skip",
            Self::AlignTo2 => "align to 2 bytes",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryReadError {
    pub operation: BinaryReadOperation,
    pub offset: usize,
    pub requested: usize,
    pub remaining: usize,
}

impl fmt::Display for BinaryReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at offset {} requested {} byte(s), but {} remain",
            self.operation, self.offset, self.requested, self.remaining
        )
    }
}

impl std::error::Error for BinaryReadError {}

pub struct BinaryReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub fn read_u8(&mut self) -> Result<u8, BinaryReadError> {
        Ok(u8::from_le_bytes(self.read_array(BinaryReadOperation::U8)?))
    }

    pub fn read_i8(&mut self) -> Result<i8, BinaryReadError> {
        Ok(i8::from_le_bytes(self.read_array(BinaryReadOperation::I8)?))
    }

    pub fn read_u16_le(&mut self) -> Result<u16, BinaryReadError> {
        Ok(u16::from_le_bytes(
            self.read_array(BinaryReadOperation::U16Le)?,
        ))
    }

    pub fn read_i16_le(&mut self) -> Result<i16, BinaryReadError> {
        Ok(i16::from_le_bytes(
            self.read_array(BinaryReadOperation::I16Le)?,
        ))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, BinaryReadError> {
        Ok(u32::from_le_bytes(
            self.read_array(BinaryReadOperation::U32Le)?,
        ))
    }

    pub fn take(&mut self, bytes: usize) -> Result<&'a [u8], BinaryReadError> {
        self.take_for(BinaryReadOperation::Take, bytes)
    }

    pub fn skip(&mut self, bytes: usize) -> Result<(), BinaryReadError> {
        self.take_for(BinaryReadOperation::Skip, bytes).map(|_| ())
    }

    pub fn align_to_2(&mut self) -> Result<(), BinaryReadError> {
        let padding = self.position % 2;
        self.take_for(BinaryReadOperation::AlignTo2, padding)
            .map(|_| ())
    }

    fn read_array<const N: usize>(
        &mut self,
        operation: BinaryReadOperation,
    ) -> Result<[u8; N], BinaryReadError> {
        let bytes = self.take_for(operation, N)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);
        Ok(array)
    }

    fn take_for(
        &mut self,
        operation: BinaryReadOperation,
        requested: usize,
    ) -> Result<&'a [u8], BinaryReadError> {
        let offset = self.position;
        let remaining = self.remaining();
        if requested > remaining {
            return Err(BinaryReadError {
                operation,
                offset,
                requested,
                remaining,
            });
        }

        let end = offset + requested;
        self.position = end;
        Ok(&self.bytes[offset..end])
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryReadOperation as Op, BinaryReader};

    #[test]
    fn reads_little_endian_integer_types() {
        let mut reader =
            BinaryReader::new(&[0xab, 0xff, 0x34, 0x12, 0xfe, 0xff, 0x78, 0x56, 0x34, 0x12]);

        assert_eq!(reader.read_u8(), Ok(0xab));
        assert_eq!(reader.read_i8(), Ok(-1));
        assert_eq!(reader.read_u16_le(), Ok(0x1234));
        assert_eq!(reader.read_i16_le(), Ok(-2));
        assert_eq!(reader.read_u32_le(), Ok(0x1234_5678));
        assert_eq!(reader.position(), 10);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn take_and_skip_advance_the_reader() {
        let mut reader = BinaryReader::new(&[1, 2, 3, 4]);

        assert_eq!(reader.take(2), Ok(&[1, 2][..]));
        assert_eq!(reader.position(), 2);
        assert_eq!(reader.skip(1), Ok(()));
        assert_eq!(reader.take(1), Ok(&[4][..]));
        assert_eq!(reader.position(), 4);
    }

    #[test]
    fn alignment_handles_even_and_odd_offsets() {
        let mut reader = BinaryReader::new(&[0xaa, 0xbb, 0xcc]);

        assert_eq!(reader.align_to_2(), Ok(()));
        assert_eq!(reader.position(), 0);
        reader.skip(1).unwrap();
        assert_eq!(reader.align_to_2(), Ok(()));
        assert_eq!(reader.position(), 2);
        assert_eq!(reader.align_to_2(), Ok(()));
        assert_eq!(reader.position(), 2);
    }

    #[test]
    fn exact_end_read_succeeds_and_one_byte_short_read_reports_context() {
        let mut exact = BinaryReader::new(&[0x34, 0x12]);
        assert_eq!(exact.read_u16_le(), Ok(0x1234));
        assert_eq!(exact.remaining(), 0);

        let mut short = BinaryReader::new(&[0x34]);
        let error = short.read_u16_le().unwrap_err();
        assert_eq!(error.operation, Op::U16Le);
        assert_eq!(error.offset, 0);
        assert_eq!(error.requested, 2);
        assert_eq!(error.remaining, 1);
        assert_eq!(short.position(), 0);
    }

    #[test]
    fn oversized_take_and_skip_report_the_current_offset() {
        let mut reader = BinaryReader::new(&[1, 2]);
        reader.skip(1).unwrap();

        let take_error = reader.take(2).unwrap_err();
        assert_eq!(take_error.operation, Op::Take);
        assert_eq!(take_error.offset, 1);
        assert_eq!(take_error.requested, 2);
        assert_eq!(take_error.remaining, 1);
        assert_eq!(reader.position(), 1);

        let skip_error = reader.skip(2).unwrap_err();
        assert_eq!(skip_error.operation, Op::Skip);
        assert_eq!(skip_error.offset, 1);
        assert_eq!(skip_error.requested, 2);
        assert_eq!(skip_error.remaining, 1);
        assert_eq!(reader.position(), 1);
    }

    #[test]
    fn odd_alignment_without_padding_reports_an_error() {
        let mut reader = BinaryReader::new(&[0xaa]);
        reader.skip(1).unwrap();

        let error = reader.align_to_2().unwrap_err();
        assert_eq!(error.operation, Op::AlignTo2);
        assert_eq!(error.offset, 1);
        assert_eq!(error.requested, 1);
        assert_eq!(error.remaining, 0);
        assert_eq!(reader.position(), 1);
    }
}
