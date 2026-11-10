# FriedChip+ — Phase 2B Defect Register & Resolution Ledger

**Document ID:** `REG-DEF-007`  
**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** ALL DEFECTS RESOLVED & VERIFIED (DEF-15 through DEF-24)  
**Audit Milestone:** Phase 2B Engineering Closure  

---

## 1. Defect Summary Table

| Defect ID | Severity | Subsystem | Title | Status |
|---|---|---|---|---|
| **DEF-15** | Critical | IFD Parser | Brittle `nr >= 10` Descriptor Profile Selection Heuristic | **RESOLVED** |
| **DEF-16** | Critical | IFD Parser | Unconditional 15-Bit Address Masking & Boundary Inaccuracies | **RESOLVED** |
| **DEF-17** | Major | IFD Parser | Silent Fallback for Reserved Frequencies & Density Bit Bleed | **RESOLVED** |
| **DEF-18** | Major | IFD Security Gates | Master Shift Errors & False Flashing Lock Claims | **RESOLVED** |
| **DEF-19** | Major | SPI Hardware Gates | Over-Confident Programmer Compatibility Assertion | **RESOLVED** |
| **DEF-20** | Major | Testing & Fixtures | Brittle Real-World Fixture Path Resolution | **RESOLVED** |
| **DEF-21** | Major | Differential Testing | Lack of Automated Upstream Differential Assertions | **RESOLVED** |
| **DEF-22** | Major | Fuzzing & Stress | Shallow Fuzzing Lacking Deep Structural Mutation | **RESOLVED** |
| **DEF-23** | Normal | Persistence & DB | Schema Idempotency & Artifact Recovery Verification | **RESOLVED** |
| **DEF-24** | Normal | Forensic Safety | Source File Immutability & Collision Prevention | **RESOLVED** |

---

## 2. Detailed Defect Investigations & Resolutions

### DEF-15: Brittle `nr >= 10` Descriptor Profile Selection Heuristic
- **Subsystem:** `src-tauri/src/engine/binary/ifd.rs`
- **Upstream Reference:** `coreboot/ifdtool.c` (`check_ifd_version`), `flashrom/ich_descriptors.c`
- **Root Cause:** In Intel 100-Series PCH (Skylake) and newer, Intel removed `NR` from `FLMAP0[27:24]`. Evaluating `nr >= 10` falsely classified modern descriptors as legacy if bits happened to be low, truncating regions past index 4.
- **Remediation:** Introduced explicit `IfdProfile` enum (`V1Legacy`, `V1_5Intermediate`, `V2Modern`, `Unknown`) and evidence-backed detection checking `FLMAP2.ICCRIBA`, strap length `ISL`, and section geometry.
- **Verification Test:** `test_ifd_v2_valid_image_parsing_and_profile_detection`
- **Status:** **RESOLVED**

---

### DEF-16: Unconditional 15-Bit Address Masking & Boundary Inaccuracies
- **Subsystem:** `src-tauri/src/engine/binary/ifd.rs`
- **Upstream Reference:** `coreboot/ifdtool.c` (lines 357-386)
- **Root Cause:** A universal 15-bit mask (`0x7FFF`) was applied to legacy descriptors where registers only allocate 12 bits (`0x0FFF`), reading reserved bits. Additionally, disabled regions were not distinguished from Region 0 (`0x00000000`).
- **Remediation:** Profile-aware masking in `decode_flreg_with_profile` (12-bit for `V1Legacy`, 15-bit for `V2Modern`). Enforced inclusive limit semantics `(limit << 12) | 0xFFF` with checked shifts. Disabled rule strictly set to $\text{limit\_raw} < \text{base\_raw}$.
- **Verification Test:** `test_ifd_flreg_profile_aware_decoding_and_boundary_checks`
- **Status:** **RESOLVED**

---

### DEF-17: Silent Fallback for Reserved Frequencies & Density Bit Bleed
- **Subsystem:** `src-tauri/src/engine/binary/ifd.rs`
- **Upstream Reference:** Intel SPI Programming Guide Section 4.2; `coreboot/ifdtool.c`
- **Root Cause:** Frequency bitfield value 3 or 5 was silently replaced with 20 MHz. In legacy descriptors, component densities are 3 bits (bits 2:0 and 5:3); reading 4 bits caused frequency bits to bleed into density. Clock frequency 4 represents 50 MHz on v1 but 30 MHz on v2.
- **Remediation:** Replaced raw integer with `SpiFrequency` enum containing explicit `Reserved(u8)` and `RepurposedEspi(u8)`. Separated 3-bit vs 4-bit density decoding by profile.
- **Verification Test:** `test_ifd_flcomp_reserved_frequency_handling`, `test_ifd_fcba_decoding_component_density`
- **Status:** **RESOLVED**

---

### DEF-18: Master Shift Errors & False Flashing Lock Claims
- **Subsystem:** `src-tauri/src/engine/binary/ifd.rs`
- **Upstream Reference:** `coreboot/ifdtool.h` (`FLMSTR_WR_SHIFT_V1`, `FLMSTR_WR_SHIFT_V2`)
- **Root Cause:** Fixed bit shifts were used across generations. Field `host_flashing_locked` gave users the impression that BIOS write protect was evaluated, when only descriptor permissions were checked.
- **Remediation:** Profile-aware shifts in `decode_master_rule_with_profile` (Write shift 24 vs 20, Read shift 16 vs 8). Renamed field to `host_descriptor_bios_write_allowed`. Added `RuntimeChipsetLockStatus::Indeterminate` to Gate 2.
- **Verification Test:** `test_ifd_master_permissions_decoding`, `test_ifd_differential_regression_against_coreboot_ifdtool`
- **Status:** **RESOLVED**

---

### DEF-19: Over-Confident Programmer Compatibility Assertion
- **Subsystem:** `src-tauri/src/engine/binary/ifd.rs`
- **Upstream Reference:** JEDEC SPI NOR Standard (JESD216); Flashrom hardware drivers
- **Root Cause:** Gate 3 claimed `passed = true` based purely on parsing static descriptor bytes. External programmer safety requires hardware telemetry (voltage, pinout, bus contention, JEDEC RDID).
- **Remediation:** Enforced strict tri-state `ProgrammerCompatibilityStatus::Indeterminate` with `passed = false` until physical telemetry is supplied.
- **Verification Test:** `test_ifd_gate3_programmer_indeterminate_without_hardware`
- **Status:** **RESOLVED**

---

### DEF-20: Brittle Real-World Fixture Path Resolution
- **Subsystem:** `src-tauri/src/tests/engine_tests.rs`, `fixtures/manifests/`
- **Root Cause:** Tests relied on hardcoded absolute development paths (`C:/Users/xtrar/.../scratch/`), breaking on any other developer machine or CI runner. Missing fixtures passed silently.
- **Remediation:** Multi-level fixture resolver checking `FRIEDCHIP_FIXTURE_DIR`, relative `fixtures/realworld/`, and directory walk. Strict SHA-256 pre-flight check. Missing fixtures properly marked with `#[ignore = "..."]`. Altered fixtures immediately panic.
- **Verification Test:** `test_real_world_fixture_hash_mismatch_panics`, `test_real_world_dell_latitude_5x90_if_available`
- **Status:** **RESOLVED**

---

### DEF-21: Lack of Automated Upstream Differential Assertions
- **Subsystem:** `src-tauri/src/tests/engine_tests.rs`
- **Upstream Reference:** `BIOSUtilities` (`dell_pfs_extract.py`), `coreboot` (`ifdtool.c`)
- **Root Cause:** Assertions compared only internal synthetic fixtures without continuous differential verification against outputs of upstream tools.
- **Remediation:** Created `test_dell_pfs_differential_regression_against_biosutilities` verifying all 9 exact matches and 3 documented deltas. Created `test_ifd_differential_regression_against_coreboot_ifdtool` verifying register decode parity.
- **Verification Test:** `test_dell_pfs_differential_regression_against_biosutilities`, `test_ifd_differential_regression_against_coreboot_ifdtool`
- **Status:** **RESOLVED**

---

### DEF-22: Shallow Fuzzing Lacking Deep Structural Mutation
- **Subsystem:** `src-tauri/src/tests/engine_tests.rs`
- **Root Cause:** Previous fuzzing relied on purely randomized buffers, testing only shallow preamble rejections and missing deep parser branches.
- **Remediation:** Built an in-process structure-aware fuzz harness mutating valid IFD, Dell PFS, and UEFI FV seeds with XOR bit flips, header truncations, and boundary shifts across 300,000 bounded iterations.
- **Verification Test:** `test_in_process_bounded_parser_stress_fuzzing` (0 crashes in 17.5s)
- **Status:** **RESOLVED**

---

### DEF-23: Schema Idempotency & Artifact Recovery Verification
- **Subsystem:** `src-tauri/src/db/`
- **Root Cause:** Database migrations lacked automated tests verifying idempotency and cross-restart workspace recovery.
- **Remediation:** Implemented `test_database_migrations_idempotency` and `test_persistent_workspace_recovery`, verifying clean schema setup, duplicate import resilience, and artifact query roundtrips.
- **Verification Test:** `test_database_migrations_idempotency`, `test_persistent_workspace_recovery`, `test_firmware_artifact_persistence_roundtrip`
- **Status:** **RESOLVED**

---

### DEF-24: Source File Immutability & Collision Prevention
- **Subsystem:** `src-tauri/src/fs/`
- **Root Cause:** Risk of accidental in-place alteration of imported original firmware dumps or overwrite collisions.
- **Remediation:** Implemented atomic output writers, read-only original file handles, SHA-256 tamper verification, and collision rejection.
- **Verification Test:** `test_source_file_preservation`, `test_source_modification_detected_after_import`, `test_output_collision_and_atomic_overwrite_rejection`
- **Status:** **RESOLVED**

---

## 3. Phase 2B.5.2 Corrective Engineering Ledger (DEF-62, DEF-63, DEF-64)

### DEF-62: Tiger Point Profile Identification Evidence
1. **Original Problem:** The synthetic FLMAP2 value `0x00040040` (CSSL=4, CSSO=0x10) was claimed as proof of Tiger Point identity to assign `V2Modern / Confirmed`. Upstream flashrom inspection revealed that `0x00040040` does not match the upstream identification pair, lacks required FLUMAP1 MDTBA descriptor context, and was previously falling through to a generic compatibility warning.
2. **Root Cause:** Over-reliance on a syntactically plausible FLMAP2 pattern rather than the exact upstream chipset identification rules in `flashrom/ich_descriptors.c` (`guess_ich_chipset_from_content`).
3. **Files Changed:**
   - `src-tauri/src/engine/binary/ifd.rs`
   - `src-tauri/src/engine/testing/fixture_builder.rs`
   - `src-tauri/src/tests/engine_tests.rs`
4. **Technical Correction:**
   - Added `flumap1: Option<u32>` to `detect_ifd_profile_resolution`.
   - Populated and evaluated upstream Tiger Point requirements: `CSSL == 0x11` (17) and `CSSO == 0x68` (104), corresponding to `FLMAP2 = 0x001101A0`.
   - Required non-zero `MDTBA` from `FLUMAP1` at offset `0x0EFC` (3836 bytes).
   - The old `0x00040040` value and any incomplete/unverified soft-strap layouts strictly return `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
   - Updated synthetic fixture builder `build_valid_ifd_v2_image` with authentic FLMAP2 (`0x001101A0`) and FLUMAP1 (`0x02000430`).
5. **Tests Added / Changed:**
   - `test_ifd_old_synthetic_0x00040040_cannot_confirm_tiger_point_def62` (Added)
   - `test_ifd_tiger_point_missing_descriptor_context_returns_unknown_def62` (Added)
   - `test_ifd_tiger_point_conflicting_evidence_returns_unknown_def62` (Added)
   - `test_ifd_tiger_point_fully_evidenced_confirmed_def62` (Added)
   - `test_ifd_unknown_profile_blocks_speculative_decoding_def62` (Added)
   - `test_ifd_user_override_produces_user_selected_def62` (Added)
   - `test_ifd_v2_valid_image_parsing_and_profile_detection` (Updated)
   - `test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57` (Updated)
6. **Test Result:** All 8 tests passed cleanly.
7. **Remaining Limitations:** Identification is specific to Tiger Point / 500-Series rules from flashrom `35c679c7ec`. Other 600/700 series rules remain in research documentation until authentic fixtures are acquired.
8. **Evidence Status:** Upstream source verified (`flashrom/ich_descriptors.c:1087-1092`); automated test execution confirmed.
9. **Closure Assessment:** **CLOSED**.

---

### DEF-63: Dell Latitude 5X90 Fixture Provenance Discrepancy
1. **Original Problem:** Phase 2B.5.1 correction report recorded the canonical fixture size as `13,637,736` bytes, whereas the manifest records `13,662,488` bytes (discrepancy of 24,752 bytes). Test suites previously evaluated only JSON manifest strings without distinguishing actual disk binary verification.
2. **Root Cause:** A clerical error/typo in the Phase 2B.5.1 report draft. The actual on-disk binary `fixtures/realworld/Latitude_5X90_1.41.0.exe` has always measured exactly `13,662,488` bytes with SHA-256 `E24B8F8F...`.
3. **Files Changed:**
   - `fixtures/manifests/dell_latitude_5x90.json`
   - `src-tauri/src/tests/engine_tests.rs`
4. **Technical Correction:**
   - Verified actual file size on disk: `(Get-Item fixtures/realworld/Latitude_5X90_1.41.0.exe).Length == 13662488`.
   - Verified actual file SHA-256: `E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B`.
   - Reconciled manifest notes explaining the clerical typo in report documentation.
   - Separated tests into manifest metadata consistency vs actual on-disk binary provenance verification.
5. **Tests Added / Changed:**
   - `test_dell_fixture_manifest_metadata_consistency_def63` (Updated)
   - `test_dell_fixture_binary_provenance_verification_def63` (Added: direct filesystem inspection, size check, SHA-256 hash check, asserting against typo).
6. **Test Result:** Both tests passed.
7. **Remaining Limitations:** Alternative ~35 MB DUP package remains unacquired and is documented as `UNVERIFIED_EXTERNALLY_REFERENCED`.
8. **Evidence Status:** Byte-level filesystem audit and hash calculation verified against disk file.
9. **Closure Assessment:** **CLOSED**.

---

### DEF-64: Subprocess Isolation and Licensing Documentation
1. **Original Problem:** Documentation in `PHASE2B5_NEXT_CAPABILITIES.md` claimed that subprocess execution "guarantees total memory safety, process isolation" and "respects GPL-3.0 process boundaries", conflating fault containment with OS sandboxing and presenting architectural separation as a legal conclusion.
2. **Root Cause:** Imprecise phrasing in architecture summaries that overlooked OS-level child process token inheritance and FSF legal interpretations of combined works.
3. **Files Changed:**
   - `docs/PHASE2B5_INTEGRATION_ARCHITECTURE.md`
   - `docs/PHASE2B5_NEXT_CAPABILITIES.md`
4. **Technical Correction:**
   - Differentiated Fault Boundary (process crash containment) from OS-Enforced Sandboxing (filesystem DACLs, AppContainer, Job Objects, network blocking).
   - Documented Windows sandboxing controls as future requirements (`NOT_IMPLEMENTED`).
   - Applied explicit status labels: `DESIGNED`, `IMPLEMENTED`, `TESTED`, `NOT_IMPLEMENTED`, `LEGAL_REVIEW_REQUIRED`.
   - Stated clearly that IPC process boundaries do not automatically establish GPL compliance; marked `PSPTool` integration as `LEGAL_REVIEW_REQUIRED`.
   - Added external tool licensing ledger with pinned commits and license types.
5. **Tests Added / Changed:** Pure documentation and architectural contract correction; no code runner exists in Phase 2.
6. **Test Result:** N/A (Documentation audit).
7. **Remaining Limitations:** Child process isolation runner is deferred to Phase 3.
8. **Evidence Status:** Architectural review and legal disclaimer reconciled across documentation.
9. **Closure Assessment:** **CLOSED**.

---

## 4. Reconciled Test Suite Ledger

| Test Category | Count | Status | Notes |
|---------------|-------|--------|-------|
| **Runnable Unit & Integration Tests** | 67 | **PASSED** | All 67 in-tree automated tests pass in ~25.7s. |
| **Real-World OEM Integration Tests** | 2 | **IGNORED (Passed when executed)** | `test_real_world_dell_latitude_5x90_if_available` and `test_dell_pfs_differential_regression_against_biosutilities` require local proprietary fixture. Both execute and pass cleanly when invoked with `-- --ignored`. |
| **Failed Tests** | 0 | **NONE** | Zero failures across all suites. |
| **Filtered Out** | 0 | **NONE** | Zero tests filtered out during full test run. |
| **Total Test Catalog** | 69 | **VERIFIED** | 67 passed; 0 failed; 2 ignored. |
