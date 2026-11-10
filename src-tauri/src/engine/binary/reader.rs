use crate::error::AppError;

/// A bounded reader over an immutable byte slice.
///
/// Designed to prevent buffer overflows, panics on truncated inputs,
/// and out-of-bounds reads when parsing untrusted binary firmware dumps.
#[derive(Debug, Clone, Copy)]
pub struct BinaryReader<'a> {
    data: &'a [u8],
}

impl<'a> BinaryReader<'a> {
    /// Create a new BinaryReader wrapping the given buffer.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Total length of the underlying buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get reference to full underlying slice.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    /// Read a single byte at the given offset.
    pub fn read_u8(&self, offset: usize) -> Result<u8, AppError> {
        self.data
            .get(offset)
            .copied()
            .ok_or_else(|| AppError::Binary(format!("Offset 0x{:X} out of bounds (len: 0x{:X})", offset, self.data.len())))
    }

    /// Read a 16-bit unsigned integer in Little-Endian format.
    pub fn read_u16_le(&self, offset: usize) -> Result<u16, AppError> {
        let bytes = self.slice(offset, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read a 32-bit unsigned integer in Little-Endian format.
    pub fn read_u32_le(&self, offset: usize) -> Result<u32, AppError> {
        let bytes = self.slice(offset, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a 64-bit unsigned integer in Little-Endian format.
    pub fn read_u64_le(&self, offset: usize) -> Result<u64, AppError> {
        let bytes = self.slice(offset, 8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read a slice of bytes with checked start and length.
    pub fn slice(&self, start: usize, len: usize) -> Result<&'a [u8], AppError> {
        let end = start
            .checked_add(len)
            .ok_or_else(|| AppError::Binary(format!("Overflow calculating slice end: 0x{:X} + 0x{:X}", start, len)))?;

        if end > self.data.len() {
            return Err(AppError::Binary(format!(
                "Slice range 0x{:X}..0x{:X} exceeds buffer length 0x{:X}",
                start, end, self.data.len()
            )));
        }

        Ok(&self.data[start..end])
    }

    /// Read a GUID formatted as a standard 16-byte hex string (UUID/GUID).
    ///
    /// Parses first 3 components as Little-Endian (EFI standard) and remaining 8 as big-endian.
    pub fn read_guid_le(&self, offset: usize) -> Result<String, AppError> {
        let bytes = self.slice(offset, 16)?;
        let d1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let d2 = u16::from_le_bytes([bytes[4], bytes[5]]);
        let d3 = u16::from_le_bytes([bytes[6], bytes[7]]);
        let d4 = &bytes[8..10];
        let d5 = &bytes[10..16];

        Ok(format!(
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            d1, d2, d3, d4[0], d4[1], d5[0], d5[1], d5[2], d5[3], d5[4], d5[5]
        ))
    }

    /// Search for needle starting at `start` offset, searching at most `max_search_len` bytes.
    pub fn find_subslice(&self, needle: &[u8], start: usize, max_search_len: Option<usize>) -> Option<usize> {
        if needle.is_empty() || start >= self.data.len() {
            return None;
        }

        let search_limit = match max_search_len {
            Some(limit) => (start + limit).min(self.data.len()),
            None => self.data.len(),
        };

        if search_limit < start + needle.len() {
            return None;
        }

        let window = &self.data[start..search_limit];
        window
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|pos| start + pos)
    }

    /// Find all occurrences of needle within `[start, limit)`.
    pub fn find_all(&self, needle: &[u8], start: usize, max_search_len: Option<usize>) -> Vec<usize> {
        let mut occurrences = Vec::new();
        let mut curr = start;

        while let Some(pos) = self.find_subslice(needle, curr, max_search_len) {
            occurrences.push(pos);
            curr = pos.saturating_add(1);
            if curr >= self.data.len() {
                break;
            }
        }

        occurrences
    }
}
