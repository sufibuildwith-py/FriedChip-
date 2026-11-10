use serde::{Deserialize, Serialize};
use crate::engine::binary::checksums::verify_efi_fv_checksum;
use crate::engine::binary::reader::BinaryReader;
use crate::error::AppError;

pub const EFI_FVH_SIGNATURE: &[u8; 4] = b"_FVH";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareVolumeBlockMap {
    pub num_blocks: u32,
    pub block_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareVolumeHeader {
    pub offset: usize,
    pub fv_length: u64,
    pub signature: String,
    pub attributes: u32,
    pub header_length: u16,
    pub checksum: u16,
    pub checksum_valid: bool,
    pub revision: u8,
    pub block_maps: Vec<FirmwareVolumeBlockMap>,
}

pub struct FirmwareVolume;

impl FirmwareVolume {
    /// Scan and parse EFI Firmware Volume headers in a given byte buffer.
    pub fn find_volumes(data: &[u8]) -> Vec<FirmwareVolumeHeader> {
        let reader = BinaryReader::new(data);
        let mut volumes = Vec::new();

        // Search for "_FVH" signature occurrences
        let sig_offsets = reader.find_all(EFI_FVH_SIGNATURE, 0, None);

        for sig_offset in sig_offsets {
            // Header starts 0x28 bytes before the signature
            if sig_offset < 0x28 {
                continue;
            }
            let hdr_offset = sig_offset - 0x28;

            if let Ok(hdr) = Self::parse_at(&reader, hdr_offset) {
                volumes.push(hdr);
            }
        }

        volumes
    }

    /// Parse a single firmware volume header at the specified offset.
    pub fn parse_at(reader: &BinaryReader, offset: usize) -> Result<FirmwareVolumeHeader, AppError> {
        // Minimum header size is 0x38 (up to first block map)
        if offset + 0x38 > reader.len() {
            return Err(AppError::Binary("Offset too close to buffer end for FV header".to_string()));
        }

        // Verify "_FVH" signature at offset + 0x28
        let sig_bytes = reader.slice(offset + 0x28, 4)?;
        if sig_bytes != EFI_FVH_SIGNATURE {
            return Err(AppError::Binary(format!(
                "Invalid FV signature at 0x{:X}", offset + 0x28
            )));
        }

        let fv_length = reader.read_u64_le(offset + 0x20)?;
        let attributes = reader.read_u32_le(offset + 0x2C)?;
        let header_length = reader.read_u16_le(offset + 0x30)?;
        let checksum = reader.read_u16_le(offset + 0x32)?;
        let revision = reader.read_u8(offset + 0x37)?;

        if (header_length as usize) < 0x38 || offset + (header_length as usize) > reader.len() {
            return Err(AppError::Binary("Invalid header length in FV header".to_string()));
        }

        let hdr_bytes = reader.slice(offset, header_length as usize)?;
        let checksum_valid = verify_efi_fv_checksum(hdr_bytes);

        // Parse block map entries starting at offset + 0x38
        let mut block_maps = Vec::new();
        let mut curr = offset + 0x38;

        while curr + 8 <= offset + (header_length as usize) {
            let num_blocks = reader.read_u32_le(curr)?;
            let block_length = reader.read_u32_le(curr + 4)?;

            if num_blocks == 0 && block_length == 0 {
                // End of block map array
                break;
            }

            block_maps.push(FirmwareVolumeBlockMap {
                num_blocks,
                block_length,
            });

            curr += 8;
        }

        Ok(FirmwareVolumeHeader {
            offset,
            fv_length,
            signature: "_FVH".to_string(),
            attributes,
            header_length,
            checksum,
            checksum_valid,
            revision,
            block_maps,
        })
    }
}
