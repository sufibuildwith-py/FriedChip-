# FriedChip+ — Phase 2A Defect Register & Remediation Catalog

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Engineering Defect Register & Technical Debt Catalog  
**Milestone:** Phase 2A Reliability & Evidence Remediation  
**Status:** **AUDITED — REMEDIATION PROPOSED (AWAITING APPROVAL)**  
**Author:** Antigravity (Advanced Agentic Coding)  

---

## 1. Executive Summary & Defect Tracking Framework

In accordance with strict clean-room engineering discipline, this register records all concrete technical defects, architectural discrepancies, test limitations, and edge-case vulnerabilities identified during the Phase 2 audit and differential analysis.

### Defect Classification Severity Scale
- **Blocker:** Causes data corruption, hardware damage, or false positive test execution.
- **High:** Substantial functional omission, fragile test path, or misleading classification.
- **Medium:** Architectural inconsistency, heuristic inaccuracy, or format limitation.
- **Low:** Cosmetic naming discrepancy or non-critical edge-case variance.

---

## 2. Comprehensive Defect Register

| Defect ID | Severity | Component | Summary | Current State | Target State | Proposed Remediation |
| :--- | :---: | :--- | :--- | :---: | :---: | :--- |
| **DEF-01** | **High** | `engine_tests.rs` | Hardcoded scratch path causes silent test skip in clean checkouts. | `TESTED (Flawed)` | `REAL_WORLD_VERIFIED` | Introduce `FRIEDCHIP_REALWORLD_FIXTURES_DIR` resolution; report explicit `[SKIPPED]` notice when absent; never present skipped tests as verified. |
| **DEF-02** | **Medium** | `dell_pfs.rs` | GUID formatting uses reversed 32-hex string matching Dell table format instead of standard hyphenated UUID. | `IMPLEMENTED` | `REFINED` | Provide dual representation: `raw_pfs_guid` (for Dell directory table matching) and `standard_guid` (hyphenated EFI UUID for UI display). |
| **DEF-03** | **Medium** | `ifd.rs` | Region-count heuristic claims specific chipset generations (`nr >= 10` -> Alder Lake) without PCH hardware strap proof. | `IMPLEMENTED` | `REFINED` | Remove speculative chipset generation names; return explicit `platform_family: "Intel PCH (Unclassified)"` when silicon straps are unverified. |
| **DEF-04** | **Medium** | `ifd.rs` | Static `host_flashing_locked: true` assumption instead of decoding FMBA / FRAP permissions. | `IMPLEMENTED` | `IMPLEMENTED` | Parse FMBA offset from FLMAP1; decode Host CPU, ME, and GbE read/write bitmasks to dynamically evaluate flashing constraints. |
| **DEF-05** | **Medium** | `ifd.rs` | Missing FCBA component density decoding in IFD parser. | `NOT IMPLEMENTED` | `IMPLEMENTED` | Parse FCBA offset from FLMAP1; decode Component 1 & 2 density bits (512KB..64MB) and read frequency caps. |
| **DEF-06** | **High** | `models/mod.rs` | `FirmwareArtifact` model exists only in documentation, not implemented in Rust code. | `DESIGNED` | `IMPLEMENTED` | Implement `FirmwareArtifact` struct in Rust with source provenance, parent-child tracking, 3-gate results, and extensible metadata map. |
| **DEF-07** | **Medium** | `firmware_volume.rs` | Scaffolding-only test coverage; lacks malformed header, alignment, truncation, and block-map tests. | `SCAFFOLDING` | `TESTED (Hardened)` | Add 5 negative/boundary tests for corrupt signatures, missing block map terminators, invalid checksums, and truncated buffers. |
| **DEF-08** | **High** | `tests/` | Lack of automated reference differential tests and fuzzing harness. | `NOT IMPLEMENTED` | `TESTED` | Implement automated differential test against golden JSON vectors; set up `cargo-fuzz` targets for `BinaryReader`, `IFD`, and `DellPfsParser`. |

---

## 3. Detailed Technical Analysis of Defects

### DEF-01: Hardcoded Real-World Fixture Path
- **Exact Location:** [`src-tauri/src/tests/engine_tests.rs:174`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/tests/engine_tests.rs#L174)
- **Code:**
  ```rust
  let scratch_path = Path::new("C:/Users/xtrar/.gemini/antigravity/brain/2b8463af-e2de-426c-8983-bb9ac8466728/scratch/Latitude_5X90_1.41.0.exe");
  if scratch_path.exists() {
      // executes extraction
  }
  ```
- **Consequence:** In any checkout outside this specific Windows user session, the path does not exist. Cargo reports `test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ok`, giving the false impression that real-world firmware was verified when zero assertions executed.
- **Proposed Correction:**
  ```rust
  let fixture_path = std::env::var("DELL_PFS_REALWORLD_FIXTURE")
      .map(PathBuf::from)
      .unwrap_or_else(|_| PathBuf::from("fixtures/realworld/Latitude_5X90_1.41.0.exe"));

  if !fixture_path.exists() {
      println!("[SKIPPED] Real-world Dell Latitude fixture not found at {:?}. Skipping test.", fixture_path);
      return;
  }
  ```

---

### DEF-03 & DEF-04: IFD Heuristic Overreach & Missing FRAP Decoding
- **Exact Location:** [`src-tauri/src/engine/binary/ifd.rs:180-189`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/binary/ifd.rs#L180-L189)
- **Code:**
  ```rust
  let platform_family = if nr >= 10 {
      "Intel PCH (Skylake / Comet Lake / Tiger Lake / Alder Lake / Raptor Lake)".to_string()
  } ...
  ```
- **Consequence:** Claiming specific generation support (e.g. Alder Lake) based solely on `nr >= 10` is technically inaccurate. Certain low-power Gemini Lake and industrial Apollo Lake platforms define 10 regions but belong to entirely different microarchitectures.
- **Proposed Correction:**
  - Return `platform_family: "Intel PCH (Unclassified, Region Count: N)"`.
  - Parse FMBA (Flash Master Base Address) from `FLMAP1`:
    ```rust
    let fmba = ((flmap1 >> 24) & 0xFF) as usize * 16;
    // Decode Master 1 (Host CPU/BIOS) read & write bitmasks from FRAP register
    let frap = reader.read_u32_le(fmba)?;
    let bios_read_mask = (frap & 0xFF) as u8;
    let bios_write_mask = ((frap >> 8) & 0xFF) as u8;
    let host_flashing_locked = (bios_write_mask & (1 << 1)) == 0; // Check if Region 1 (BIOS) is writable by Host
    ```

---

### DEF-06: FirmwareArtifact Rust Implementation Gap
- **Finding:** The `FirmwareArtifact` model is specified in documentation, but does not exist in `src-tauri/src/models/`.
- **Consequence:** Parsers cannot be unified into a polymorphic extraction pipeline, and the UI must write custom rendering logic for each vendor format.
- **Proposed Correction:**
  - Create `src-tauri/src/models/artifact.rs`.
  - Implement serde serialization for all fields.
  - Propose database migration `002_firmware_artifacts.sql` to persist extracted artifacts with their parent file UUIDs.

---

## 4. Standstill & Review Requirement

In compliance with instructions:
- **No production code has been modified.**
- **All proposed corrections are formally cataloged herein.**
- **Awaiting explicit engineering review and approval before beginning Phase 2A implementation.**
