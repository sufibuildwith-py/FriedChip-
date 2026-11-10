pub mod checksums;
pub mod ifd;
pub mod reader;
pub mod region;
pub mod verified_write;

pub use checksums::{checksum_8_xor, efi_fv_checksum_16, inverted_crc32, md5_hex, sha256_hex, verify_efi_fv_checksum};
pub use ifd::{decode_flreg, IntelFlashDescriptor, IfdAnalysisReport};
pub use reader::BinaryReader;
pub use region::{FlashRegion, RegionType};
pub use verified_write::{atomic_write_and_verify, ensure_distinct_paths};
