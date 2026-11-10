# FriedChip+ — Phase 2A Reliability & Evidence Remediation Plan

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Reliability Engineering & Test Remediation Plan  
**Milestone:** Phase 2A Reliability & Evidence Remediation  
**Status:** **PROPOSED — AWAITING ENGINEERING APPROVAL**  
**Author:** Antigravity (Advanced Agentic Coding)  

---

## 1. Executive Summary & Remediation Mandate

Following the formal baseline establishment of the Phase 2 implementation audit, this document establishes the engineering workplan for **Phase 2A: Reliability and Evidence Remediation**.

### Core Governance Constraints
1. **Zero Product Scope Expansion:** No new firmware formats (Insyde, AMI, Phoenix, Apple), decompression algorithms, or hardware flasher drivers are introduced during Phase 2A.
2. **Confined Engine Scope:** All firmware logic remains strictly inside `src-tauri/src/engine/`. No Cargo multi-crate refactoring or dynamic plugin loading.
3. **Standstill Agreement:** In accordance with user directives, **no production code modifications will be executed until this remediation plan, test results, differential verification, and defect register are reviewed and explicitly approved.**
4. **Verifiable Engineering Terminology:** Every component, feature, and test is strictly designated under one of four distinct maturity states:
   - `DESIGNED`: Specified in documentation or architecture proposals only.
   - `IMPLEMENTED`: Concrete code exists in the repository.
   - `TESTED`: Verified against synthetic unit/negative fixtures.
   - `REAL_WORLD_VERIFIED`: Proved against authentic OEM firmware binaries or physical hardware dumps with documented differential parity.

---

## 2. Priority 1: Reproducible Real-World Testing Plan

### The Existing Flaw (DEF-01)
`test_real_world_dell_latitude_5x90_if_available` currently hardcodes an absolute path to a transient scratch folder:
`C:/Users/xtrar/.gemini/antigravity/brain/.../scratch/Latitude_5X90_1.41.0.exe`
If this path is absent (e.g. on clean developer machines or GitHub Actions CI runners), the test encounters `if scratch_path.exists()`, executes zero assertions, and exits with code 0 (`ok`). This can falsely present a skipped test as real-world verification.

### Remediation Blueprint
1. **Documented Fixture Cache Directory:**
   Establish standard local fixture resolution hierarchy:
   ```text
   Priority 1: Environment Variable `FRIEDCHIP_REALWORLD_FIXTURES_DIR`
   Priority 2: Environment Variable `DELL_PFS_REALWORLD_FIXTURE` (exact file path)
   Priority 3: Local workspace directory `fixtures/realworld/`
   ```
2. **Explicit Tri-State Test Outcomes (PASS / FAIL / SKIPPED):**
   - If the fixture is present and extracts valid partitions matching reference hashes: **PASS**.
   - If the fixture is present but extraction fails or checksums mismatch: **FAIL** (`panic!`).
   - If the fixture is absent: Emit a prominent test harness notice:
     ```text
     [SKIPPED] Real-world Dell Latitude fixture not found.
     To run real-world verification, set FRIEDCHIP_REALWORLD_FIXTURES_DIR or place
     Latitude_5X90_1.41.0.exe into fixtures/realworld/.
     ```
   - **Mandatory Reporting Rule:** A skipped test is strictly reported as `SKIPPED` and must never be tallied as `REAL_WORLD_VERIFIED`.
3. **Reference Asset Metadata Registration:**
   - **Manufacturer:** Dell Inc.
   - **Target Model Family:** Dell Latitude 5490 / 5590 / OptiPlex
   - **Executable Filename:** `Latitude_5X90_1.41.0.exe`
   - **Package Version:** 1.41.0
   - **Byte Size:** 13,662,488 bytes
   - **SHA-256 Hash:** `E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B`
   - **Acquisition Source:** Dell Open Catalog (`downloads.dell.com`) via `CatalogPC.xml`
   - **Acquisition Date:** November 2026

---

## 3. Priority 2: Dell PFS Differential Verification Plan

Using `platomav/BIOSUtilities` (`dell_pfs_extract.py`) as the independent reference baseline:

### Verification Criteria
1. Section Discovery: Ensure section counts, offsets, and types (`0xAA` Firmware vs `0xBB` Utilities) match.
2. Component Directory Resolution: Validate UTF-16LE name decoding and 4-part semantic version numbers.
3. Payload Boundary & Hash Verification: Verify extracted `.bin` sizes and SHA-256 hashes.
4. Investigation of Discrepancies:
   - Identify whether differences stem from PFAT / BIOS Guard header stripping, Python text-mode CRLF translations, or raw binary slicing.
   - Document all findings in [`docs/PHASE2A_DIFFERENTIAL_RESULTS.md`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/docs/PHASE2A_DIFFERENTIAL_RESULTS.md).

---

## 4. Priority 3: Intel Flash Descriptor (IFD) Enhancement Plan

### Missing Hardware Decoders to Implement
1. **Component Base Section (FCBA):**
   - Read FCBA offset from FLMAP1 bits 23:16 shifted left by 4 (typically offset `0x80`).
   - Decode Flash Components Register 0 (`FLCOMP`):
     - Component 1 Density: Bits 2:0 (`000b` = 512KB, `001b` = 1MB, `010b` = 2MB, `011b` = 4MB, `100b` = 8MB, `101b` = 16MB, `110b` = 32MB, `111b` = 64MB).
     - Component 2 Density: Bits 5:3.
     - Dual-chip SPI detection (`nc = 1` vs `nc = 2`).
     - Maximum Read Frequency caps (20MHz, 33MHz, 50MHz).
2. **Flash Master Access Permissions (FMBA / FRAP):**
   - Read FMBA offset from FLMAP1 bits 31:24 shifted left by 4 (typically offset `0x60`).
   - Decode Master Read/Write Access masks:
     - Master 1 (Host CPU / BIOS): Read mask (bits 7:0), Write mask (bits 15:8).
     - Master 2 (Intel ME): Read mask (bits 23:16), Write mask (bits 31:24).
     - Master 3 (GbE Controller): Read mask, Write mask.
   - Replace the static `host_flashing_locked: true` assumption with dynamic evaluation of Master 1 Region 1 (BIOS) and Region 2 (ME) write permissions.
3. **Chipset Heuristic Correction:**
   - Eliminate inaccurate claims of specific PCH generations (e.g. Alder Lake) based solely on region count (`nr >= 10`).
   - Unknown or unsupported layouts must return an explicit `UNCLASSIFIED` status:
     `platform_family: "Intel PCH (Unclassified Layout, Region Count: N)"`.

---

## 5. Priority 4: FirmwareArtifact Implementation Plan

### Current Status
- **Rust Status:** **NOT IMPLEMENTED** (The model currently exists only in documentation).
- **Codebase Reality:** Parsers return disparate structs (`DellPfsExtractionReport`, `IfdAnalysisReport`, `FlashRegion`).

### Remediation Plan
1. Define a minimal, vendor-agnostic Rust struct in `src-tauri/src/models/artifact.rs`:
   - Unique artifact ID and optional `parent_id`.
   - Source provenance (source file hash, source offset, extracted size).
   - Format identifier (`ArtifactFormat`).
   - Three-Gate validation results (`Gate1Structural`, `Gate2Platform`, `Gate3Programmer`).
   - Extensible format metadata dictionary (`HashMap<String, serde_json::Value>`).
2. Propose database schema migration (`002_firmware_artifacts.sql`) before applying schema changes.

---

## 6. Priority 5: UEFI Firmware Volume Hardening Plan

### Current Status: SCAFFOLDING
The module currently parses only the outer volume header (`_FVH`) and block maps. It does not parse FFS files or sections.

### Test Coverage Plan
Implement synthetic unit tests for:
1. `test_uefi_fv_corrupt_signature`: Rejects headers without `_FVH`.
2. `test_uefi_fv_checksum_failure`: Inverts checksum byte and asserts validation failure.
3. `test_uefi_fv_truncated_header`: Asserts error when buffer is smaller than `header_length`.
4. `test_uefi_fv_block_map_terminator_missing`: Tests defensive handling when block map array lacks `(0, 0)` sentinel.
5. `test_uefi_fv_misaligned_volume`: Tests detection when `_FVH` occurs at non-block-aligned offsets.

---

## 7. Priority 6: Parser Robustness & Fuzzing Setup Plan

### Malformed Boundary Tests to Add
- **BinaryReader:** Zero-length slices, integer overflow on 32-bit offset additions, EOF truncation.
- **Intel Flash Descriptor:** Out-of-bounds FRBA, overlapping region limits, component counts > 2, inverted regions.
- **Dell PFS:** Truncated ZLIB stream, 256MB zip bomb simulation, corrupted inverted CRC-32, unknown entry revisions, recursive nesting depth > 4.

### Fuzzing Blueprint (`cargo-fuzz` / libFuzzer)
Designate three fuzz targets under `src-tauri/fuzz/`:
1. `fuzz_binary_reader`: Feeds arbitrary byte streams to `slice` and integer decoders.
2. `fuzz_ifd_parser`: Feeds arbitrary 4KB–64MB buffers to `IntelFlashDescriptor::parse`.
3. `fuzz_dell_pfs`: Feeds mutated containers to `DellPfsParser::extract`.

---

## 8. Standstill & Implementation Sequence

```
                               Phase 2A Sequence
 ┌────────────────────────────────────────────────────────────────────────┐
 │ Step 1: Review & Sign-Off of Phase 2A Remediation Dossier             │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │ Approval Required
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Step 2: Implement Test Harness Improvements (Fixtures, DEF-01 Fix)    │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Step 3: Implement IFD FCBA & FRAP Decoders + Unclassified Status       │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Step 4: Implement Minimal Rust FirmwareArtifact Struct                 │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Step 5: Add Negative & Scaffolding Tests for UEFI FV & Parsers         │
 └────────────────────────────────────────────────────────────────────────┘
```
