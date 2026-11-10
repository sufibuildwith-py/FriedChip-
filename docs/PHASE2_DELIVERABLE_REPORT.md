# FriedChip+ — Phase 2 Engineering Deliverable Report
## Reference-Informed Binary Engine & Dell PFS Extraction

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Date:** November 10, 2026  
**Status:** Completed & Formally Verified  
**Release Binary:** `src-tauri/target/release/lbe-desktop.exe`  
**Binary SHA-256:** `511406BCCCAB0DF21F872AECA93D5FD39DF28A42D66A1A6312AB03C05C48A440`  
**Regression Test Status:** 20 / 20 Tests Passing (`cargo test`)

---

## 1. Executive Summary

Phase 2 translates the engineering patterns and binary structure specifications audited from four mandatory public references (`BIOSUtilities`, `BIOSRecoverySuite`, `google/pawn`, and `dreamwhite/bios-extraction-guide`) into a pure-Rust, memory-safe, clean-room firmware engineering engine.

All deliverables mandated in the Phase 2 specification are fully implemented, deterministically tested with synthetic fixtures, and empirically validated against an authentic Dell Latitude 5X90 OEM BIOS package (`Latitude_5X90_1.41.0.exe`).

---

## 2. Architecture & File Organization

The engine is isolated under `src-tauri/src/engine/` with zero proprietary binary dependencies (`comextract.exe`, `7z.exe`, or external C dynamic libraries):

```text
src-tauri/src/engine/
├── binary/
│   ├── mod.rs               # Public re-exports and module definitions
│   ├── reader.rs            # Safe, bounded binary reader with checked arithmetic
│   ├── region.rs            # FlashRegion types, bounds calculation, boundary overlap detection
│   ├── ifd.rs               # Intel Flash Descriptor parser, FLREG decoder, 3 validation gates
│   ├── checksums.rs         # 8-bit XOR, inverted CRC-32, 16-bit additive inverse, SHA-256, MD5
│   └── verified_write.rs    # Canonical path distinctness, atomic write (.part), sync, readback SHA-256
├── vendor/
│   ├── mod.rs               # Vendor format re-exports
│   └── dell_pfs.rs          # Dell PFS container extractor, ZLIB stream decoder, entry parser
├── uefi/
│   ├── mod.rs               # UEFI module re-exports
│   └── firmware_volume.rs   # EFI_FIRMWARE_VOLUME_HEADER parser, checksum & block map validation
└── testing/
    ├── mod.rs               # Test harness re-exports
    └── fixture_builder.rs   # Deterministic in-memory synthetic fixture generator
```

---

## 3. Reference-Informed Technical Capabilities

### A. Dell PFS Update Extractor (`dell_pfs.rs`)
- **Container Structure Recognition:** Identifies section marker pattern `0xEE 0xAA 0x76 0x1B 0xEC 0xBB 0x20 0xF1 0xE6 0x51` and footer pattern `0xEE 0xAA 0xEE 0x8F 0x49 0x1B 0xE8 0xAE 0x14 0x37 0x90`.
- **Section Parsing & Checksums:** Computes and verifies 8-bit XOR checksum over section headers and footers (`checksum_8_xor`).
- **Pure-Rust Decompression:** Employs `flate2` with pure-Rust `rust_backend` (`miniz_oxide`). Enforces a strict 256 MB decompression ceiling to prevent decompression bomb vulnerabilities.
- **Inverted CRC-32 Validation:** Computes `!crc32(payload)` and verifies equality against the 32-bit footer checksum preceding `PFS.FTR.`.
- **Entry Revision Support:** Supports both `DellPfsEntryR1` (0x48 bytes) and `DellPfsEntryR2` (0x58 bytes) component headers.
- **Component Directory Resolution:** Reads the internal Name Information table (`E0717CE3A9BB25824B9F0DC8FD041960`) to pair raw entries with human-readable UTF-16LE names (e.g. `System BIOS with BIOS Guard`, `Embedded Controller`, `Intel Management Engine`) and semantic versions.
- **Artifact Classification Standard:** Extracted BIOS payloads are classified as `Dell Extracted BIOS Payload (Unverified SPI Layout)`, preserving the critical distinction between vendor update payloads and complete, programmer-ready SPI images.

### B. Intel Flash Descriptor (IFD) Engine (`ifd.rs`)
- **Signature Detection:** Verifies signature magic `0x0FF0A55A` (`5A A5 F0 0F` little-endian) at absolute offset `0x10`.
- **FLMAP0 / FLREG Decoding:** Decodes FLMAP0 region count (`NR`) and base address (`FRBA`). Decodes FLREG registers using 15-bit addressable page arithmetic (`base = (val & 0x7FFF) << 12`, `limit = ((val >> 16) & 0x7FFF) << 12 | 0xFFF`).
- **Unmapped / Disabled Region Handling:** Accurately classifies regions where `limit < base` (e.g. `0x00007FFF`) as disabled.
- **Boundary Overlap Detection:** Computes pairwise intersection among enabled regions to flag collisions or corrupted partition tables.
- **3 Distinct Verification Gates:**
  1. **Gate 1 — Structural Validity:** Evaluates signature magic, region table bounds within the file, and boundary non-overlap.
  2. **Gate 2 — Platform Compatibility:** Determines chipset family (ICH vs Series 6-9 vs modern PCH) and region slot count.
  3. **Gate 3 — Programmer Compatibility:** Verifies SPI capacity requirements, external flasher constraints (CH341A / Flashrom), and distinguishes external SPI flashing from host write permissions.

### C. UEFI Firmware Volume Parser (`firmware_volume.rs`)
- Detects `_FVH` signature at offset `0x28` of candidate firmware volumes.
- Computes and verifies 16-bit additive inverse checksum over the header (`verify_efi_fv_checksum`).
- Parses block map descriptors and attributes.

### D. Verified Write Pipeline (`verified_write.rs`)
- Verifies source and destination paths are distinct via OS-level canonicalization (`ensure_distinct_paths`).
- Atomically stages output to a temporary `.part` file, calls `sync_all()`, renames, and performs a full read-back SHA-256 verification against the in-memory buffer.

---

## 4. Synthetic Fixture Test Harness (`fixture_builder.rs`)

The test suite exercises all parsers against synthetic in-memory fixtures generated deterministically:

| Test Case | Module | Description | Result |
| :--- | :--- | :--- | :--- |
| `test_flreg_decoding_15bit_addressing` | `ifd.rs` | Verifies 15-bit page decoding, 128MB boundary, bit masking (bits 15, 31), and disabled detection (`limit < base`) | **PASSED** |
| `test_ifd_valid_image_parsing_and_gates` | `ifd.rs` | Full 16MB SPI image with Descriptor, BIOS, ME; exercises Gate 1, 2, and 3 | **PASSED** |
| `test_ifd_corrupt_signature_rejection` | `ifd.rs` | Rejects image with invalid signature at offset 0x10 | **PASSED** |
| `test_ifd_boundary_collision_detection` | `ifd.rs` | Flags overlapping regions (Region 0 & 2) in structural validation | **PASSED** |
| `test_ifd_out_of_bounds_detection` | `ifd.rs` | Detects region limit exceeding total image length | **PASSED** |
| `test_synthetic_dell_pfs_extraction_end_to_end` | `dell_pfs.rs` | Decompresses synthetic PFS container, validates XOR 8 and inverted CRC-32, extracts 4 components | **PASSED** |
| `test_dell_pfs_corrupt_xor_detection` | `dell_pfs.rs` | Detects corrupt section header XOR checksum | **PASSED** |
| `test_uefi_firmware_volume_header_parsing` | `firmware_volume.rs` | Validates `_FVH` header and 16-bit additive inverse checksum | **PASSED** |
| `test_sha256_correctness` | `checksums.rs` | Verifies SHA-256 hashing against standard test vectors | **PASSED** |
| `test_source_file_preservation` | `verified_write.rs` | Confirms source dump is never modified during analysis | **PASSED** |
| `test_output_collision_and_atomic_overwrite_rejection` | `verified_write.rs` | Verifies atomic overwrite protection and distinct paths | **PASSED** |
| `test_database_migrations_idempotency` | `db` | SQLite migration versioning and table consistency | **PASSED** |
| `test_real_world_dell_latitude_5x90_if_available` | Integration | End-to-end extraction against official OEM package | **PASSED** |

**Total Suite Result:** 20 passed, 0 failed, 0 ignored.

---

## 5. Real-World Package Validation

Tested against official Dell OEM update package:
- **Sample File:** `Latitude_5X90_1.41.0.exe` (13,662,488 bytes)
- **Container Recognition:** Confirmed 2 sections (Section `0xBB` Utilities, Section `0xAA` Firmware).
- **Extracted Components (14 total):**
  1. `Policy Payload`
  2. `Resource Payload`
  3. `UI Payload`
  4. `Component Directory Table` (Utilities)
  5. `System BIOS with BIOS Guard` (~9.00 MB)
  6. `Embedded Controller` (172 KB)
  7. `Backup Embedded Controller` (168 KB)
  8. `Gigabit Ethernet` (8 KB)
  9. `Intel Management Engine (VPro LP) Update` (~7.34 MB)
  10. `System Map`
  11. `PCR0 XML`
  12. `Main System TI Port Controller 0`
  13. `Model Information`
  14. `Component Directory Table` (Firmware)

---

## 6. Safety Boundary Enforcement

In strict compliance with project guidelines:
1. **Zero Copied Code:** All Rust code is written cleanly and independently.
2. **No Password/Credential Circumvention:** Master password algorithms (e.g. `dell_latitude.py` in BIOSRecoverySuite) are strictly excluded.
3. **No Kernel Drivers or Physical Memory Access:** Offline binary file analysis only; no `/dev/mem` access, direct PCI port I/O, or root driver hooks.
4. **Offline Durability:** Database and binary operations operate completely offline.

---

## 7. Desktop User Interface Integration

Implemented `src/components/ExtractorsView.tsx` with:
- Dual mode switcher: **Dell PFS Extractor** and **Intel Flash Descriptor (IFD)**.
- Section Breakdown Cards showing compressed size, decompressed size, Header XOR 8 status, and Inverted CRC-32 status.
- Extracted Components Table detailing name, version, raw size, GUID, and 6 distinct engineering gates (`PARSED`, `EXTRACTED`, `CHECKSUM OK`, `SPI UNVERIFIED`).
- One-click native package ingestion via `@tauri-apps/plugin-dialog`.
- Automatic SQLite persistence: extracted components are registered into the active workspace library.
- Verified in both Dark Graphite and Editorial Light workstation visual themes.
