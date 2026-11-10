use std::io::Write;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::engine::binary::checksums::{checksum_8_xor, efi_fv_checksum_16, inverted_crc32};
use crate::engine::binary::ifd::{IFD_SIGNATURE_MAGIC, IFD_SIGNATURE_OFFSET};
use crate::engine::vendor::dell_pfs::{DELL_PFS_FOOTER_PATTERN, DELL_PFS_SECTION_PATTERN, PFS_FTR_TAG, PFS_HDR_TAG};

/// Encode base and limit (inclusive) into FLREG u32 register value.
pub fn encode_flreg(base: usize, limit: usize) -> u32 {
    let base_page = ((base >> 12) & 0x7FFF) as u32;
    let limit_page = (((limit) >> 12) & 0x7FFF) as u32;
    (limit_page << 16) | base_page
}

pub struct SyntheticFixtureBuilder;

impl SyntheticFixtureBuilder {
    /// Builds a deterministic 16MB Intel SPI flash image with valid IFD v1.0, BIOS, ME, and GbE regions.
    pub fn build_valid_ifd_image(total_size: usize) -> Vec<u8> {
        let mut data = vec![0xFFu8; total_size];

        // 1. Signature at offset 0x10
        let sig_bytes = IFD_SIGNATURE_MAGIC.to_le_bytes();
        data[IFD_SIGNATURE_OFFSET..IFD_SIGNATURE_OFFSET + 4].copy_from_slice(&sig_bytes);

        // 2. FLMAP0 at 0x14:
        // Bits 7:0: FCBA = 0x08 (offset 0x80)
        // Bits 9:8: NC = 0 (1 component: NC+1 = 1)
        // Bits 23:16: FRBA = 0x04 (offset 0x40)
        // Bits 27:24: NR = 4 (5 regions: 0..4, NR+1 = 5)
        let flmap0: u32 = (0x04 << 24) | (0x04 << 16) | (0x00 << 8) | 0x08;
        data[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());

        // 3. FLMAP1 at 0x18:
        // Bits 7:0: FMBA = 0x06 (offset 0x60)
        // Bits 10:8: NM = 2 (3 masters: NM+1 = 3)
        // Bits 23:16: FISBA = 0x10 (offset 0x100)
        // Bits 31:24: ISL = 0x02
        let flmap1: u32 = (0x02 << 24) | (0x10 << 16) | (0x02 << 8) | 0x06;
        data[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());

        // 3b. FLMAP2 at 0x1C:
        // Bits 7:0: FMSBA = 0x00
        // Bits 15:8: MSL = 0x00
        // Bits 23:16: ICCRIBA = 0x00 (Legacy ICH/PCH indicator)
        let flmap2: u32 = 0x00000000;
        data[0x1C..0x20].copy_from_slice(&flmap2.to_le_bytes());

        // 4. FCBA at 0x80: FLCOMP register
        // Bits 2:0: comp1_density = 5 (16MB)
        // Bit 20: fastread = 1 (Fast Read Supported)
        // Bits 23:21: freq_fastread = 4 (50MHz)
        // Bits 26:24: freq_write = 1 (33MHz)
        // Bits 29:27: freq_read_id = 1 (33MHz)
        let fcba = 0x80;
        let flcomp: u32 = 0x05 | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27);
        data[fcba..fcba + 4].copy_from_slice(&flcomp.to_le_bytes());

        // 5. FMBA at 0x60: Master section
        let fmba = 0x60;
        // FLMSTR1 (Host CPU / BIOS): Read Descriptor(0), BIOS(1), ME(2), PlatData(4) -> 0x17 << 16; Write BIOS(1) -> 0x02 << 24
        let flmstr1: u32 = (0x17 << 16) | (0x02 << 24);
        data[fmba..fmba + 4].copy_from_slice(&flmstr1.to_le_bytes());

        // FLMSTR2 (Intel ME): Read Descriptor(0), ME(2) -> 0x05 << 16; Write ME(2) -> 0x04 << 24
        let flmstr2: u32 = (0x05 << 16) | (0x04 << 24);
        data[fmba + 4..fmba + 8].copy_from_slice(&flmstr2.to_le_bytes());

        // FLMSTR3 (GbE): Read GbE(3) -> 0x08 << 16; Write GbE(3) -> 0x08 << 24
        let flmstr3: u32 = (0x08 << 16) | (0x08 << 24);
        data[fmba + 8..fmba + 12].copy_from_slice(&flmstr3.to_le_bytes());

        // 6. Region table at FRBA (0x40)
        let frba = 0x40;
        let reg0 = encode_flreg(0x0000, 0x0FFF);             // Descriptor: 0x0000..0x0FFF (4KB)
        let reg1 = encode_flreg(0x600000, total_size - 1);    // BIOS: 6MB..16MB
        let reg2 = encode_flreg(0x1000, 0x5FFFFF);           // ME: 4KB..6MB
        let reg3: u32 = 0x00007FFF;                               // GbE: Disabled (limit < base)
        let reg4: u32 = 0x00007FFF;                               // Platform Data: Disabled

        data[frba..frba + 4].copy_from_slice(&reg0.to_le_bytes());
        data[frba + 4..frba + 8].copy_from_slice(&reg1.to_le_bytes());
        data[frba + 8..frba + 12].copy_from_slice(&reg2.to_le_bytes());
        data[frba + 12..frba + 16].copy_from_slice(&reg3.to_le_bytes());
        data[frba + 16..frba + 20].copy_from_slice(&reg4.to_le_bytes());

        // Fill regions with deterministic test patterns
        let fill_end = total_size.min(0x2000);
        for i in 0x1000..fill_end {
            data[i] = ((i * 13 + 0x37) & 0xFF) as u8;
        }

        data
    }

    /// Builds a deterministic modern IFD v2.0 SPI flash image (Skylake/Sunrise Point+ 100-series through 700-series).
    pub fn build_valid_ifd_v2_image(total_size: usize) -> Vec<u8> {
        let mut data = vec![0xFFu8; total_size];

        // 1. Signature at offset 0x10
        let sig_bytes = IFD_SIGNATURE_MAGIC.to_le_bytes();
        data[IFD_SIGNATURE_OFFSET..IFD_SIGNATURE_OFFSET + 4].copy_from_slice(&sig_bytes);

        // 2. FLMAP0 at 0x14:
        // FCBA = 0x0A (0xA0), NC = 0 (1 comp), FRBA = 0x04 (0x40), NR = 0x0F (16 regions)
        let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
        data[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());

        // 3. FLMAP1 at 0x18:
        // FMBA = 0x08 (0x80), NM = 2 (3 masters), FISBA = 0x10 (0x100), ISL = 0x20 (128 bytes, 0x100..0x180)
        let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
        data[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());

        // 4. FLMAP2 at 0x1C:
        // FMSBA = 0x20 (0x200), MSL = 0x02, ICCRIBA = 0x31, RIL = 0x01 (100-series Sunrise Point marker, DEF-47, DEF-49)
        let flmap2: u32 = (0x01 << 24) | (0x31 << 16) | (0x02 << 8) | 0x20;
        data[0x1C..0x20].copy_from_slice(&flmap2.to_le_bytes());

        // 5. FCBA at 0xA0: FLCOMP register (v2 4-bit densities)
        // Bits 3:0: comp1_density = 5 (16MB)
        // Bits 7:4: comp2_density = 0x0F (Unused)
        // Bits 19:17: freq_read = 6 (17MHz Skylake marker)
        // Bit 20: fastread = 1
        // Bits 23:21: freq_fastread = 4 (30MHz in IFD v2)
        // Bits 26:24: freq_write = 1 (33MHz)
        // Bits 29:27: freq_read_id = 1 (33MHz)
        let fcba = 0xA0;
        let flcomp: u32 = 0x05 | (0x0F << 4) | (6 << 17) | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27);
        data[fcba..fcba + 4].copy_from_slice(&flcomp.to_le_bytes());

        // 6. FMBA at 0x80: Master section (v2 bit shifts: rd_shift = 8, wr_shift = 20)
        let fmba = 0x80;
        // FLMSTR1 (Host CPU / BIOS): Read Descriptor(0), BIOS(1) -> 0x03 << 8; Write BIOS(1) -> 0x02 << 20
        let flmstr1: u32 = (0x02 << 20) | (0x03 << 8);
        data[fmba..fmba + 4].copy_from_slice(&flmstr1.to_le_bytes());

        // FLMSTR2 (Intel ME): Read Descriptor(0), ME(2) -> 0x05 << 8; Write ME(2) -> 0x04 << 20
        let flmstr2: u32 = (0x04 << 20) | (0x05 << 8);
        data[fmba + 4..fmba + 8].copy_from_slice(&flmstr2.to_le_bytes());

        // FLMSTR3 (GbE): Read GbE(3) -> 0x08 << 8; Write GbE(3) -> 0x08 << 20
        let flmstr3: u32 = (0x08 << 20) | (0x08 << 8);
        data[fmba + 8..fmba + 12].copy_from_slice(&flmstr3.to_le_bytes());

        // 7. Region table at FRBA (0x40): 16 regions with 15-bit addressing
        let frba = 0x40;
        let reg0 = encode_flreg(0x0000, 0x0FFF);             // Descriptor: 0x0000..0x0FFF (4KB)
        let reg1 = encode_flreg(0x600000, total_size - 1);    // BIOS: 6MB..16MB
        let reg2 = encode_flreg(0x1000, 0x5FFFFF);           // ME: 4KB..6MB
        data[frba..frba + 4].copy_from_slice(&reg0.to_le_bytes());
        data[frba + 4..frba + 8].copy_from_slice(&reg1.to_le_bytes());
        data[frba + 8..frba + 12].copy_from_slice(&reg2.to_le_bytes());

        // Regions 3..15 disabled (0x00007FFF)
        let disabled_reg: u32 = 0x00007FFF;
        for i in 3..16 {
            let offset = frba + (i * 4);
            data[offset..offset + 4].copy_from_slice(&disabled_reg.to_le_bytes());
        }

        data
    }

    /// Builds an IFD image where one region is disabled (`limit < base`).
    pub fn build_ifd_with_disabled_region() -> Vec<u8> {
        let mut data = Self::build_valid_ifd_image(0x100000); // 1MB
        // Overwrite Region 1 (BIOS) with disabled register value
        data[0x44..0x48].copy_from_slice(&0x00007FFFu32.to_le_bytes());
        data
    }

    /// Builds an IFD image with overlapping regions (collision detection).
    pub fn build_ifd_with_overlapping_regions() -> Vec<u8> {
        let mut data = Self::build_valid_ifd_image(0x100000); // 1MB
        // Region 0: 0x0000..0x1FFF (8KB)
        // Region 2: 0x1000..0x3FFF (overlaps Region 0 from 0x1000 to 0x1FFF)
        let reg0 = encode_flreg(0x0000, 0x1FFF);
        let reg2 = encode_flreg(0x1000, 0x3FFF);
        data[0x40..0x44].copy_from_slice(&reg0.to_le_bytes());
        data[0x48..0x4C].copy_from_slice(&reg2.to_le_bytes());
        data
    }

    /// Builds an IFD image with an out-of-bounds region.
    pub fn build_ifd_with_out_of_bounds_region() -> Vec<u8> {
        let mut data = Self::build_valid_ifd_image(0x10000); // 64KB image
        // Region 1: Limit at 0xFFFFFF (16MB), which exceeds 64KB image size
        let reg1 = encode_flreg(0x2000, 0xFFFFFF);
        data[0x44..0x48].copy_from_slice(&reg1.to_le_bytes());
        data
    }

    /// Builds an image with an invalid IFD signature.
    pub fn build_ifd_with_corrupt_signature() -> Vec<u8> {
        let mut data = Self::build_valid_ifd_image(0x100000);
        data[0x10..0x14].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        data
    }

    /// Builds a valid synthetic Dell PFS package containing given components:
    /// `components: &[ (friendly_name, version, payload_data) ]`
    pub fn build_synthetic_dell_pfs(
        components: &[(&str, &str, &[u8])],
    ) -> Vec<u8> {
        Self::build_synthetic_dell_pfs_with_options(components, false)
    }

    pub fn build_synthetic_dell_pfs_with_options(
        components: &[(&str, &str, &[u8])],
        corrupt_crc: bool,
    ) -> Vec<u8> {
        // 1. Build Name Info table payload
        let mut name_table_payload = Vec::new();

        for (idx, &(name, version, _)) in components.iter().enumerate() {
            let mut comp_guid_bytes = [0u8; 16];
            comp_guid_bytes[0..4].copy_from_slice(&(0xA1B2C3D4u32).to_le_bytes());
            comp_guid_bytes[4..8].copy_from_slice(&(0x4A5BE5F6u32).to_le_bytes());
            comp_guid_bytes[8..12].copy_from_slice(&(0x00008C9Du32).to_le_bytes());
            comp_guid_bytes[12..16].copy_from_slice(&((idx as u32 + 1)).to_le_bytes());

            // DellPfsInfo (0x14 bytes): HeaderVersion (1: u32), GUID (16 bytes)
            name_table_payload.extend_from_slice(&1u32.to_le_bytes());
            name_table_payload.extend_from_slice(&comp_guid_bytes);

            // DellPfsName (0x0E bytes): Version (4 x u16), VersionType (4 x u8), CharacterCount (u16)
            let mut v_parts = [1u16, 0, 0, 0];
            for (p_idx, p_str) in version.split('.').enumerate() {
                if p_idx < 4 {
                    v_parts[p_idx] = p_str.parse().unwrap_or(0);
                }
            }
            for v in v_parts {
                name_table_payload.extend_from_slice(&v.to_le_bytes());
            }
            name_table_payload.extend_from_slice(&[1, 1, 1, 1]); // VersionType: 4 bytes

            let utf16_chars: Vec<u16> = name.encode_utf16().collect();
            name_table_payload.extend_from_slice(&(utf16_chars.len() as u16).to_le_bytes());

            // UTF-16 string
            for ch in utf16_chars {
                name_table_payload.extend_from_slice(&ch.to_le_bytes());
            }

            // 2 padding bytes
            name_table_payload.extend_from_slice(&[0x00, 0x00]);
        }

        // 2. Build PFS volume payload containing entries
        let mut payload = Vec::new();

        // Add Name Info entry first
        let name_info_guid_bytes = [
            0x60, 0x19, 0x04, 0xFD, 0xC8, 0x0D, 0x9F, 0x4B,
            0x82, 0x25, 0xBB, 0xA9, 0xE3, 0x7C, 0x71, 0xE0,
        ];
        Self::append_pfs_entry(&mut payload, &name_info_guid_bytes, &name_table_payload);

        // Add component entries
        for (idx, &(_, _, comp_data)) in components.iter().enumerate() {
            let mut comp_guid_bytes = [0u8; 16];
            comp_guid_bytes[0..4].copy_from_slice(&(0xA1B2C3D4u32).to_le_bytes());
            comp_guid_bytes[4..8].copy_from_slice(&(0x4A5BE5F6u32).to_le_bytes());
            comp_guid_bytes[8..12].copy_from_slice(&(0x00008C9Du32).to_le_bytes());
            comp_guid_bytes[12..16].copy_from_slice(&((idx as u32 + 1)).to_le_bytes());

            Self::append_pfs_entry(&mut payload, &comp_guid_bytes, comp_data);
        }

        // 3. Assemble inner PFS volume (PFS.HDR. + payload + PFS.FTR.)
        let mut volume = Vec::new();
        volume.extend_from_slice(PFS_HDR_TAG);
        volume.extend_from_slice(&1u32.to_le_bytes()); // HeaderVersion = 1
        volume.extend_from_slice(&(payload.len() as u32).to_le_bytes()); // PayloadSize
        volume.extend_from_slice(&payload);

        // Footer: PayloadSize (u32), Inverted CRC-32 (u32), PFS.FTR. (8 bytes)
        let mut inv_crc = inverted_crc32(&payload);
        if corrupt_crc {
            inv_crc ^= 0xFFFFFFFF;
        }
        volume.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        volume.extend_from_slice(&inv_crc.to_le_bytes());
        volume.extend_from_slice(PFS_FTR_TAG);

        // 4. Compress volume with ZLIB
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&volume).unwrap();
        let compressed_volume = encoder.finish().unwrap();

        // 5. Wrap in Dell PFS section container
        let mut final_package = Vec::new();
        // Prefix with some dummy PE stub bytes
        final_package.extend_from_slice(b"MZ\x90\x00\x03\x00\x00\x00PE\x00\x00DUMMY_STUB_HEADER_DATA_12345");

        // Section header: 16 bytes
        let mut section_header = Vec::new();
        section_header.extend_from_slice(&(compressed_volume.len() as u32).to_le_bytes()); // 0x00..0x04
        section_header.push(0xAA); // Section Type: 0xAA (Firmware) at 0x04
        section_header.extend_from_slice(DELL_PFS_SECTION_PATTERN); // 0x05..0x0F (10 bytes)
        let hdr_xor = checksum_8_xor(&section_header[0..15]);
        section_header.push(hdr_xor); // Checksum at 0x0F

        final_package.extend_from_slice(&section_header);
        final_package.extend_from_slice(&compressed_volume);

        // Section footer: 16 bytes
        let mut section_footer = Vec::new();
        section_footer.extend_from_slice(&(compressed_volume.len() as u32).to_le_bytes()); // 0x00..0x04
        section_footer.extend_from_slice(DELL_PFS_FOOTER_PATTERN); // 0x04..0x0F (11 bytes)
        let ftr_xor = checksum_8_xor(&section_footer[0..15]);
        section_footer.push(ftr_xor); // Checksum at 0x0F

        final_package.extend_from_slice(&section_footer);

        final_package
    }

    fn append_pfs_entry(out: &mut Vec<u8>, guid_bytes: &[u8; 16], data: &[u8]) {
        // DellPfsEntryBase (Revision 1: 0x48 bytes)
        out.extend_from_slice(guid_bytes);                // 0x00..0x10: GUID
        out.extend_from_slice(&1u32.to_le_bytes());       // 0x10..0x14: HeaderVersion = 1
        out.extend_from_slice(&[1, 0, 0, 0]);             // 0x14..0x18: VersionType
        out.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]); // 0x18..0x20: Version (4 x u16)
        out.extend_from_slice(&0u64.to_le_bytes());       // 0x20..0x28: Reserved
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // 0x28..0x2C: DataSize
        out.extend_from_slice(&0u32.to_le_bytes());       // 0x2C..0x30: DataSigSize
        out.extend_from_slice(&0u32.to_le_bytes());       // 0x30..0x34: DataMetSize
        out.extend_from_slice(&0u32.to_le_bytes());       // 0x34..0x38: DataMetSigSize
        out.extend_from_slice(&[0u8; 16]);                // 0x38..0x48: R1 Unknown

        // Component data
        out.extend_from_slice(data);
    }

    /// Builds a Dell PFS image with invalid section XOR checksum.
    pub fn build_corrupt_xor_dell_pfs() -> Vec<u8> {
        let mut pkg = Self::build_synthetic_dell_pfs(&[("System BIOS", "1.0.0.0", b"FAKE_BIOS_PAYLOAD")]);
        // Locate section pattern and corrupt the checksum byte
        if let Some(pos) = pkg.windows(DELL_PFS_SECTION_PATTERN.len()).position(|w| w == DELL_PFS_SECTION_PATTERN) {
            let chk_offset = pos + DELL_PFS_SECTION_PATTERN.len();
            pkg[chk_offset] ^= 0xFF; // Invert checksum
        }
        pkg
    }

    /// Builds a Dell PFS image with invalid inverted CRC-32 checksum.
    pub fn build_corrupt_crc_dell_pfs() -> Vec<u8> {
        Self::build_synthetic_dell_pfs_with_options(&[("System BIOS", "1.0.0.0", b"FAKE_BIOS_PAYLOAD")], true)
    }

    /// Builds a valid synthetic EFI Firmware Volume header (0x48 bytes).
    pub fn build_valid_firmware_volume(fv_size: usize) -> Vec<u8> {
        let mut hdr = vec![0u8; 0x48];
        // 0x00..0x10: ZeroVector (all 0)
        // 0x10..0x20: FileSystemGuid
        hdr[0x10..0x20].copy_from_slice(&[0x7A, 0x86, 0x4C, 0x8C, 0x54, 0x00, 0x58, 0x47, 0x98, 0x58, 0x9F, 0x45, 0x24, 0x63, 0xD4, 0x12]);
        // 0x20..0x28: FvLength
        hdr[0x20..0x28].copy_from_slice(&(fv_size as u64).to_le_bytes());
        // 0x28..0x2C: Signature "_FVH"
        hdr[0x28..0x2C].copy_from_slice(b"_FVH");
        // 0x2C..0x30: Attributes
        hdr[0x2C..0x30].copy_from_slice(&0x4FEFF4FEu32.to_le_bytes());
        // 0x30..0x32: HeaderLength = 0x48
        hdr[0x30..0x32].copy_from_slice(&0x0048u16.to_le_bytes());
        // 0x32..0x34: Checksum (placeholder 0)
        hdr[0x32..0x34].copy_from_slice(&0x0000u16.to_le_bytes());
        // 0x34: ExtHeaderOffset = 0
        // 0x36: Reserved = 0
        // 0x37: Revision = 2
        hdr[0x37] = 0x02;
        // 0x38..0x40: BlockMap (1 block of fv_size)
        let num_blocks = (fv_size / 0x1000) as u32;
        hdr[0x38..0x3C].copy_from_slice(&num_blocks.to_le_bytes());
        hdr[0x3C..0x40].copy_from_slice(&0x1000u32.to_le_bytes());
        // 0x40..0x48: Terminator block (0, 0)
        // Checksum calculation over 0x48 bytes
        let chk = efi_fv_checksum_16(&hdr);
        hdr[0x32..0x34].copy_from_slice(&chk.to_le_bytes());

        let mut fv_image = hdr;
        fv_image.resize(fv_size, 0xFF);
        fv_image
    }
}
