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
