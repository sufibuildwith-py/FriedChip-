# Phase 2B.5.2 — Implementation & Reconciliation Report

**Project:** FriedChip+  
**Release:** Phase 2B.5.2  
**Baseline Commit:** `682c9ae05bbe020a87e73be1e1341dd5b6a1a032` (Clean working tree, ancestor of HEAD)  
**Execution Timestamp:** 2026-11-11  
**Phase 3 Status:** **STRICTLY LOCKED** (No physical flashing, no live kernel drivers, no PFAT decapsulation)  
**New Firmware Formats:** **NONE (Not Authorized)**

---

## 1. Executive Summary & Preflight

Phase 2B.5.2 executes a narrow corrective engineering milestone addressing three independent review findings:
- **DEF-62:** Reconciled Tiger Point IFD profile identification against upstream `flashrom` source, removing the unsupported synthetic `0x00040040` heuristic and requiring authentic CSSL/CSSO pairs and `FLUMAP1` MDTBA descriptor context.
- **DEF-63:** Reconciled Dell Latitude 5X90 canonical fixture provenance and byte-size discrepancy, confirming that `13,662,488` bytes and SHA-256 `E24B8F8F...` are authentic on disk and correcting a clerical typo (`13,637,736` bytes) in prior report documentation.
- **DEF-64:** Reconciled third-party subprocess isolation and licensing claims across architecture documentation, distinguishing fault containment from OS sandboxing, documenting Windows sandboxing controls as `NOT_IMPLEMENTED` future requirements, and establishing that IPC process boundaries require independent legal review for GPL compliance (`PSPTool`).

Preflight verification confirmed:
- Baseline HEAD commit `682c9ae05bbe020a87e73be1e1341dd5b6a1a032` was clean and is an ancestor of the active working tree.
- Full test suite passes: **67 passed, 0 failed, 2 ignored** (real-world OEM integration tests pass when executed with `-- --ignored`).

---

## 2. DEF-62 — Correct Tiger Point Profile Identification

### 2.1 Upstream Investigation
- **Upstream Repository:** `flashrom` (local checkout under `research/third_party/flashrom`)
- **Pinned Commit SHA:** `35c679c7ecd1db67649b950ee0b0b4f74a540ef1`
- **Source Files & Locations:**
  * `include/ich_descriptors.h:88-94` (`FLMAP2` union with `CSSO` [11:2] and `CSSL` [23:16])
  * `include/ich_descriptors.h:515-524` (`FLUMAP1` with `MDTBA` [31:24] at offset `0x0EFC` = 3836 bytes)
  * `ich_descriptors.c:1084-1115` (`guess_ich_chipset_from_content`)
- **Exact Upstream Fields Used for Identification:**
  1. `FLMAP2` (at offset 0x1C):
     - `CSSL == 0x11` (bits 23:16 = 17 decimal)
     - `CSSO == 0x68` (bits 11:2 = 104 decimal, raw bitfield `0x68 << 2 = 0x01A0`)
     - Reserved bits: `res1` [1:0] == 0, `res2` [15:12] == 0, `res3` [31:24] == 0.
     - Combined `FLMAP2 = (0x11 << 16) | (0x68 << 2) = 0x001101A0`.
  2. `FLUMAP1` (at offset `0x0EFC` = 3836 bytes):
     - Bits 31:24: `MDTBA` (MIP Descriptor Table Base Address, introduced with Cannon Point / 300-series).
     - Upstream `guess_ich_chipset_from_content` checks `if (upper->MDTBA == 0x00)`: if zero, it classifies the descriptor into pre-Cannon Point families (Cougar Point, Lynx Point, Lewisburg, Sunrise Point 100-series). The modern Tiger Point/500-series branch is evaluated **only** when `upper->MDTBA != 0x00`.
  3. `FLMSTR1` (at FMBA):
     - Modern read mask bitfields: Host CPU reads Descriptor at bit 8 and BIOS at bit 9, with no legacy match.
- **Assumptions and Limitations:**
  * Upstream line 1113 emits a warning (`msg_pwarn("Unknown flash descriptor, assuming 500 series compatibility.\n");`) and assumes 500-series compatibility when an unrecognized soft-strap value is encountered.
  * FriedChip+ rejects speculative fallback heuristics. An unrecognized or incomplete soft-strap layout strictly returns `Unknown`.
- **Differences from Prior Implementation:**
  * The prior implementation introduced a synthetic `FLMAP2` value of `0x00040040` (CSSL=4, CSSO=0x10). In upstream flashrom, this value does not match the Tiger Point rule (`CSSL=0x11, CSSO=0x68`).
  * The prior classifier did not inspect `FLUMAP1` or check for non-zero `MDTBA`.

### 2.2 Technical Correction
1. **Removed Heuristic:** The synthetic value `0x00040040` no longer establishes Tiger Point identity. Any CPU soft-strap layout not matching the upstream pair `(0x11, 0x68)` strictly returns `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
2. **Context Requirement:** Added `flumap1: Option<u32>` to `detect_ifd_profile_resolution()`. If `flumap1` is missing or has `MDTBA == 0`, automatic detection returns `Unknown`.
3. **Descriptor Ingestion:** In `IntelFlashDescriptor::parse_with_profile`, `FLUMAP1` is read at offset `0x0EFC` and passed to profile resolution.
4. **Synthetic Fixture Builder:** Updated `build_valid_ifd_v2_image` in `src-tauri/src/engine/testing/fixture_builder.rs` with authentic `FLMAP2 = 0x001101A0` and `FLUMAP1 = 0x02000430` (MDTBA=2, VTL=4, VTBA=0x30).
5. **Decoding Gating:** Profile-specific decoding (`decode_flreg_with_profile`, `decode_master_rule_with_profile`) remains strictly disabled when the automatic profile is `Unknown`. User overrides remain explicitly tagged as `UserSelected`.

---

## 3. DEF-63 — Dell Latitude 5X90 Fixture Provenance & Size Reconciliation

### 3.1 Provenance Audit & Size Discrepancy Investigation
1. **Manifest Audit:**
   - Package: `Latitude_5X90_1.41.0.exe`
   - Manifest Size: `13,662,488` bytes
   - Expected SHA-256: `E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B`
2. **Disk Binary Audit:**
   - Fixture Location: `fixtures/realworld/Latitude_5X90_1.41.0.exe`
   - Actual Byte Size: `13,662,488` bytes (`(Get-Item ...).Length`)
   - Actual SHA-256: `E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B`
3. **Discrepancy Origin:**
   - The value `13,637,736` bytes in `docs/PHASE2B5_1_CORRECTION_REPORT.md` was an isolated clerical error / typo (difference: 24,752 bytes).
   - Neither the manifest nor the disk binary ever contained `13,637,736`. Both have always agreed on `13,662,488` bytes.

### 3.2 Test Decoupling & Binary Provenance
- Replaced the previous manifest-only test with two distinct tests:
  * `test_dell_fixture_manifest_metadata_consistency_def63`: Asserts JSON schema consistency, canonical size `13,662,488`, and external reference flag for the ~35 MB DUP wrapper.
  * `test_dell_fixture_binary_provenance_verification_def63`: Directly checks the filesystem binary, asserting exact size `13,662,488`, explicitly asserting `actual_size != 13,637,736` (to prevent typo reintroduction), and verifying SHA-256 against `E24B8F8F...`.

---

## 4. DEF-64 — Subprocess Isolation & Licensing Documentation Audit

### 4.1 Fault Separation vs OS-Enforced Sandboxing
Updated `docs/PHASE2B5_INTEGRATION_ARCHITECTURE.md` and `docs/PHASE2B5_NEXT_CAPABILITIES.md` to establish clear boundaries:
- **Fault Boundary (Process Separation):** Spawning an external tool isolates address spaces so child crashes/panics do not abort the host process.
- **What Subprocess Spawning Does NOT Prevent:** Calling `std::process::Command` does not prevent filesystem reads/writes, network sockets, token inheritance, or resource exhaustion.
- **Windows Sandboxing Requirements (`NOT_IMPLEMENTED`):** Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, memory limits), AppContainer / Restricted Low Integrity Tokens, explicit DACLs on temp directories, and network blocking are documented as future Phase 3 implementation requirements, not present capabilities.

### 4.2 Licensing & Copyleft Boundaries
- Documented that IPC process boundaries do not automatically establish GPL compliance or copyright separation under applicable legal standards.
- External tool candidates documented with explicit status labels:
  * `MEAnalyzer` (`d27ccdba0a`, BSD-2-Clause): `DESIGNED`, `NOT_IMPLEMENTED`, `LEGAL_REVIEW_REQUIRED`.
  * `PSPTool` (`5055403889`, GPL-3.0): `DESIGNED`, `NOT_IMPLEMENTED`, `LEGAL_REVIEW_REQUIRED` (GPL distribution analysis required before redistribution).
  * `IFRExtractor-RS` (`b1b9b36c00`, BSD-2-Clause): `DESIGNED`, `NOT_IMPLEMENTED`, `LEGAL_REVIEW_REQUIRED`.

---

## 5. Automated Test Suite Verification

```text
running 69 tests
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_conservative_profile_classification_rejects_ambiguous ... ok
test tests::engine_tests::test_dell_fixture_manifest_metadata_consistency_def63 ... ok
test tests::engine_tests::test_ifd_explicit_override_confidence_semantics ... ok
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_corrupt_signature_rejection ... ok
test tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling ... ok
test tests::engine_tests::test_ifd_flcomp_read_freq_6_not_standalone_v2_proof ... ok
test tests::engine_tests::test_ifd_flmap0_nr_reserved_bit27_isolation ... ok
test tests::engine_tests::test_ifd_flmap2_iccriba_accounting_and_safe_boundaries ... ok
test tests::engine_tests::test_ifd_flmap2_interpretation_semantics_def47 ... ok
test tests::engine_tests::test_ifd_flmap2_missing_read_error_propagation ... ok
test tests::engine_tests::test_ifd_flmap2_interpretation_edge_cases_def52 ... ok
test tests::engine_tests::test_ifd_boundary_collision_detection ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flmstr_older_layout_cannot_accidentally_trigger_modern ... ok
test tests::engine_tests::test_ifd_frba_out_of_bounds_handling ... ok
test tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks ... ok
test tests::engine_tests::test_ifd_geometry_known_vs_unknown_extents_def49 ... ok
test tests::engine_tests::test_ifd_geometry_adversarial_collision_checks ... ok
test tests::engine_tests::test_ifd_geometry_unknown_flmap2_safe_bounds_def54 ... ok
test tests::engine_tests::test_ifd_geometry_validation_detects_collisions_and_out_of_bounds ... ok
test tests::engine_tests::test_dell_pfs_truncated_zlib_stream ... ok
test tests::engine_tests::test_ifd_golden_vector_regression_against_coreboot_ifdtool ... ok
test tests::engine_tests::test_ifd_old_synthetic_0x00040040_cannot_confirm_tiger_point_def62 ... ok
test tests::engine_tests::test_dell_pfs_corrupt_xor_detection ... ok
test tests::engine_tests::test_dell_pfs_corrupted_inverted_crc32 ... ok
test tests::engine_tests::test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57 ... ok
test tests::engine_tests::test_ifd_tiger_point_conflicting_evidence_returns_unknown_def62 ... ok
test tests::engine_tests::test_ifd_out_of_bounds_detection ... ok
test tests::engine_tests::test_ifd_tiger_point_fully_evidenced_confirmed_def62 ... ok
test tests::engine_tests::test_ifd_tiger_point_missing_descriptor_context_returns_unknown_def62 ... ok
test tests::engine_tests::test_ifd_truncated_under_4kb_rejection ... ok
test tests::engine_tests::test_ifd_unknown_profile_blocks_speculative_decoding_def62 ... ok
test tests::engine_tests::test_ifd_unknown_profile_safe_fallback_no_speculative_decoding ... ok
test tests::engine_tests::test_ifd_unsupported_generation_inference_removed_def48 ... ok
test tests::engine_tests::test_ifd_user_override_produces_user_selected_def62 ... ok
test tests::engine_tests::test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding ... ok
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored
test tests::engine_tests::test_observer_targeted_path_fixtures_def50 ... ok
test tests::engine_tests::test_uefi_firmware_volume_header_parsing ... ok
test tests::engine_tests::test_uefi_fv_block_map_sentinel_missing ... ok
test tests::engine_tests::test_real_world_fixture_hash_mismatch_panics - should panic ... ok
test tests::engine_tests::test_uefi_fv_corrupt_signature_rejection ... ok
test tests::engine_tests::test_uefi_fv_invalid_checksum_rejection ... ok
test tests::engine_tests::test_uefi_fv_misaligned_volume ... ok
test tests::engine_tests::test_uefi_fv_truncated_header ... ok
test tests::engine_tests::test_uefi_fv_observer_single_site_accounting_def55 ... ok
test tests::test_database_migrations_idempotency ... ok
test tests::test_file_import_and_metadata_persistence ... ok
test tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end ... ok
test tests::test_output_collision_and_atomic_overwrite_rejection ... ok
test tests::test_sha256_correctness ... ok
test tests::test_fresh_database_initialization ... ok
test tests::test_source_deletion_or_relocation_detected ... ok
test tests::test_source_file_preservation ... ok
test tests::engine_tests::test_firmware_artifact_persistence_roundtrip ... ok
test tests::test_source_modification_detected_after_import ... ok
test tests::test_unknown_file_identification_not_inferred_from_extension ... ok
test tests::test_duplicate_import_after_application_restart ... ok
test tests::test_persistent_workspace_recovery ... ok
test tests::engine_tests::test_dell_fixture_binary_provenance_verification_def63 ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 67 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 25.74s
```

When real-world integration tests are executed explicitly against the authentic local fixture:
```text
cargo test test_real_world_dell_latitude_5x90_if_available -- --ignored -> ok. 1 passed; 0 failed
cargo test test_dell_pfs_differential_regression_against_biosutilities -- --ignored -> ok. 1 passed; 0 failed
```

---

## 6. Scope Boundaries & Phase 3 Declaration

- **Phase 3 Remains Strictly Locked:**
  - Zero live SPI flashing code.
  - Zero kernel driver interfaces.
  - PFAT decapsulation deferred.
  - No subprocess child process runner or sandbox harness executed.
- **Surgical Changes:** All changes were confined to IFD classification correctness, fixture builder accuracy, regression test suites, and documentation reconciliation.

**Phase 2B.5.2 is officially declared CLOSED and ready for final review.**
