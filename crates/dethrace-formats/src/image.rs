use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedImage {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl IndexedImage {
    pub fn new(width: usize, height: usize) -> Result<Self, IndexedImageError> {
        if width == 0 || height == 0 {
            return Err(IndexedImageError::ZeroDimension);
        }

        let pixel_count = width
            .checked_mul(height)
            .ok_or(IndexedImageError::SizeOverflow)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(pixel_count)
            .map_err(|_| IndexedImageError::AllocationFailed)?;
        pixels.resize(pixel_count, 0);

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    pub fn row(&self, y: usize) -> Option<&[u8]> {
        let start = y.checked_mul(self.width)?;
        self.pixels.get(start..start.checked_add(self.width)?)
    }

    pub fn row_mut(&mut self, y: usize) -> Option<&mut [u8]> {
        let start = y.checked_mul(self.width)?;
        self.pixels.get_mut(start..start.checked_add(self.width)?)
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        self.row(y)?.get(x).copied()
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, index: u8) -> Option<()> {
        *self.row_mut(y)?.get_mut(x)? = index;
        Some(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedImageError {
    ZeroDimension,
    SizeOverflow,
    AllocationFailed,
}

impl fmt::Display for IndexedImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => f.write_str("image dimensions must be nonzero"),
            Self::SizeOverflow => f.write_str("image pixel count overflows usize"),
            Self::AllocationFailed => f.write_str("could not allocate image pixels"),
        }
    }
}

impl std::error::Error for IndexedImageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette256 {
    entries: [[u8; 3]; 256],
}

impl Palette256 {
    pub const LEN: usize = 256;

    pub const fn new(entries: [[u8; 3]; Self::LEN]) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[[u8; 3]; Self::LEN] {
        &self.entries
    }

    pub fn color(&self, index: u8) -> &[u8; 3] {
        &self.entries[usize::from(index)]
    }

    pub fn set_color(&mut self, index: u8, color: [u8; 3]) {
        self.entries[usize::from(index)] = color;
    }

    pub fn set_range(&mut self, start: usize, colors: &[[u8; 3]]) -> Result<(), PaletteRangeError> {
        let range = start..start.checked_add(colors.len()).ok_or(PaletteRangeError {
            start,
            len: colors.len(),
        })?;
        self.entries
            .get_mut(range)
            .ok_or(PaletteRangeError {
                start,
                len: colors.len(),
            })?
            .copy_from_slice(colors);
        Ok(())
    }
}

impl Default for Palette256 {
    fn default() -> Self {
        Self {
            entries: [[0; 3]; Self::LEN],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteRangeError {
    pub start: usize,
    pub len: usize,
}

impl fmt::Display for PaletteRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "palette range {}..{} is outside 0..{}",
            self.start,
            self.start.saturating_add(self.len),
            Palette256::LEN
        )
    }
}

impl std::error::Error for PaletteRangeError {}

#[cfg(test)]
mod tests {
    use super::{IndexedImage, IndexedImageError, Palette256, PaletteRangeError};

    #[test]
    fn image_is_zeroed_with_contiguous_buffer_size() {
        let image = IndexedImage::new(3, 2).unwrap();

        assert_eq!((image.width(), image.height()), (3, 2));
        assert_eq!(image.pixels(), &[0; 6]);
    }

    #[test]
    fn rejects_zero_and_overflowing_dimensions() {
        assert_eq!(
            IndexedImage::new(0, 1),
            Err(IndexedImageError::ZeroDimension)
        );
        assert_eq!(
            IndexedImage::new(usize::MAX, 2),
            Err(IndexedImageError::SizeOverflow)
        );
        assert_eq!(
            IndexedImage::new(usize::MAX, 1),
            Err(IndexedImageError::AllocationFailed)
        );
    }

    #[test]
    fn row_and_pixel_access_stay_inside_image_boundaries() {
        let mut image = IndexedImage::new(2, 2).unwrap();
        image.row_mut(1).unwrap().copy_from_slice(&[3, 4]);
        image.set_pixel(0, 0, 1).unwrap();

        assert_eq!(image.row(0), Some(&[1, 0][..]));
        assert_eq!(image.row(1), Some(&[3, 4][..]));
        assert_eq!(image.row(2), None);
        assert_eq!(image.pixel(1, 1), Some(4));
        assert_eq!(image.pixel(2, 1), None);
        assert_eq!(image.pixel(0, 2), None);
        assert_eq!(image.set_pixel(2, 0, 9), None);
    }

    #[test]
    fn palette_entries_and_ranges_can_be_updated_safely() {
        let mut palette = Palette256::default();
        palette.set_color(255, [1, 2, 3]);
        palette.set_range(0, &[[4, 5, 6], [7, 8, 9]]).unwrap();

        assert_eq!(palette.color(0), &[4, 5, 6]);
        assert_eq!(palette.color(1), &[7, 8, 9]);
        assert_eq!(palette.color(255), &[1, 2, 3]);
        assert_eq!(palette.entries().len(), 256);
        assert_eq!(
            palette.set_range(255, &[[0, 0, 0], [0, 0, 0]]),
            Err(PaletteRangeError { start: 255, len: 2 })
        );
    }
}
