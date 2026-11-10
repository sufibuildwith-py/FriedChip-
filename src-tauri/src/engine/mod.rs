pub mod binary;
pub mod vendor;
pub mod uefi;
pub mod testing;

pub use binary::{
    checksums, ifd, reader, region, verified_write,
    FlashRegion, IfdAnalysisReport, IntelFlashDescriptor, RegionType,
};
pub use vendor::{
    dell_pfs, DellPfsExtractedComponent, DellPfsExtractionReport, DellPfsParser, DellPfsSectionInfo,
};
pub use uefi::{firmware_volume, FirmwareVolume, FirmwareVolumeHeader};
pub use testing::{fixture_builder, SyntheticFixtureBuilder};
