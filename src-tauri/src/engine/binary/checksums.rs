use sha2::{Digest, Sha256};

/// Calculate 8-bit XOR checksum over byte slice.
///
/// Used by Dell PFS section headers and footers.
pub fn checksum_8_xor(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

/// Calculate inverted CRC-32 checksum with initial seed 0.
///
/// In Dell PFS footers, the payload checksum is computed such that
/// `footer.checksum == !crc32(payload)`.
pub fn inverted_crc32(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    !hasher.finalize()
}

/// Calculate standard CRC-32.
pub fn standard_crc32(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Calculate 16-bit additive inverse checksum over a slice of 16-bit words.
///
/// Used for EFI_FIRMWARE_VOLUME_HEADER where the sum of all 16-bit words in
/// the header modulo 0x10000 must equal 0.
pub fn efi_fv_checksum_16(header_bytes: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for chunk in header_bytes.chunks_exact(2) {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    // Additive inverse: (0x10000 - (sum & 0xFFFF)) & 0xFFFF
    ((0x10000u32.wrapping_sub(sum & 0xFFFF)) & 0xFFFF) as u16
}

/// Verify that 16-bit word sum over slice is zero mod 0x10000.
pub fn verify_efi_fv_checksum(header_bytes: &[u8]) -> bool {
    let mut sum: u32 = 0;
    for chunk in header_bytes.chunks_exact(2) {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    (sum & 0xFFFF) == 0
}

/// Compute SHA-256 hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute MD5 hex string (for legacy cross-reference only).
pub fn md5_hex(data: &[u8]) -> String {
    let digest = md5::compute(data);
    hex::encode(digest.0)
}
