# Phase 2B.5 — Validation Results & Evidence Report

**Project:** FriedChip+  
**Release:** Phase 2B.5  
**Execution Timestamp:** 2026-11-10  
**Test Suite Status:** 60 Passed; 0 Failed; 2 Ignored (Real-world OEM binary integration tests requiring optional local fixture); 0 Filtered Out.

---

## 1. Test Suite Execution Summary

```text
running 62 tests
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_dell_fixture_provenance_reconciliation_def56 ... ok
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_corrupt_signature_rejection ... ok
test tests::engine_tests::test_ifd_explicit_override_confidence_semantics ... ok
test tests::engine_tests::test_ifd_conservative_profile_classification_rejects_ambiguous ... ok
test tests::engine_tests::test_dell_pfs_truncated_zlib_stream ... ok
test tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling ... ok
test tests::engine_tests::test_ifd_flmap0_nr_reserved_bit27_isolation ... ok
test tests::engine_tests::test_ifd_flcomp_read_freq_6_not_standalone_v2_proof ... ok
test tests::engine_tests::test_ifd_flmap2_iccriba_accounting_and_safe_boundaries ... ok
test tests::engine_tests::test_ifd_boundary_collision_detection ... ok
test tests::engine_tests::test_ifd_flmap2_interpretation_edge_cases_def52 ... ok
test tests::engine_tests::test_ifd_flmap2_interpretation_semantics_def47 ... ok
test tests::engine_tests::test_ifd_flmap2_missing_read_error_propagation ... ok
test tests::engine_tests::test_dell_pfs_corrupted_inverted_crc32 ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks ... ok
test tests::engine_tests::test_dell_pfs_corrupt_xor_detection ... ok
test tests::engine_tests::test_ifd_frba_out_of_bounds_handling ... ok
test tests::engine_tests::test_ifd_flmstr_older_layout_cannot_accidentally_trigger_modern ... ok
test tests::engine_tests::test_ifd_geometry_unknown_flmap2_safe_bounds_def54 ... ok
test tests::engine_tests::test_ifd_geometry_adversarial_collision_checks ... ok
test tests::engine_tests::test_ifd_geometry_known_vs_unknown_extents_def49 ... ok
test tests::engine_tests::test_ifd_geometry_validation_detects_collisions_and_out_of_bounds ... ok
test tests::engine_tests::test_ifd_golden_vector_regression_against_coreboot_ifdtool ... ok
test tests::engine_tests::test_ifd_profile_read_freq_6_with_modern_flmstr_def53 ... ok
test tests::engine_tests::test_ifd_truncated_under_4kb_rejection ... ok
test tests::engine_tests::test_ifd_unknown_profile_safe_fallback_no_speculative_decoding ... ok
test tests::engine_tests::test_ifd_out_of_bounds_detection ... ok
test tests::engine_tests::test_ifd_unsupported_generation_inference_removed_def48 ... ok
test tests::engine_tests::test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding ... ok
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored
test tests::engine_tests::test_real_world_fixture_hash_mismatch_panics - should panic ... ok
test tests::engine_tests::test_uefi_firmware_volume_header_parsing ... ok
test tests::engine_tests::test_observer_targeted_path_fixtures_def50 ... ok
test tests::engine_tests::test_uefi_fv_block_map_sentinel_missing ... ok
test tests::engine_tests::test_uefi_fv_observer_single_site_accounting_def55 ... ok
test tests::engine_tests::test_uefi_fv_truncated_header ... ok
test tests::engine_tests::test_uefi_fv_misaligned_volume ... ok
test tests::engine_tests::test_uefi_fv_corrupt_signature_rejection ... ok
test tests::engine_tests::test_uefi_fv_invalid_checksum_rejection ... ok
test tests::test_database_migrations_idempotency ... ok
test tests::test_file_import_and_metadata_persistence ... ok
test tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end ... ok
test tests::test_output_collision_and_atomic_overwrite_rejection ... ok
test tests::test_sha256_correctness ... ok
test tests::test_source_deletion_or_relocation_detected ... ok
test tests::test_source_modification_detected_after_import ... ok
test tests::test_source_file_preservation ... ok
test tests::test_fresh_database_initialization ... ok
test tests::test_unknown_file_identification_not_inferred_from_extension ... ok
test tests::engine_tests::test_firmware_artifact_persistence_roundtrip ... ok
test tests::test_persistent_workspace_recovery ... ok
test tests::test_duplicate_import_after_application_restart ... ok
test tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 60 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 24.73s
```

---

## 2. Track A Defect Resolution Verification

### 2.1 DEF-52 — Conservative FLMAP2 Interpretation
- **Root Cause:** Upper half 0 and unknown layouts were previously falling through to legacy MCH strap extraction, speculatively populating FMSBA and MSL metadata.
- **Fix:** In `determine_flmap2_interpretation`:
  - `flmap2 == 0` evaluates strictly to `Flmap2Interpretation::Unknown`.
  - Non-zero layouts with upper half 0 and non-zero `FMSBA` evaluate to `LegacyMchStrap`.
  - Reserved-bit violations (`res1`, `res2`, `res3`) and out-of-bounds ICCRIBA/RIL fields evaluate strictly to `Unknown`.
  - `validate_descriptor_geometry` guarantees that when `flmap2_interpretation == Unknown`, `fmsba = None`, `msl = 0`, `iccriba = None`, and `ril = 0`.
- **Test Evidence:** `test_ifd_flmap2_interpretation_edge_cases_def52` passes, asserting that all-zero FLMAP2 and unknown layouts with plausible lower halves NEVER expose speculative FMSBA/MSL.

### 2.2 DEF-53 / DEF-57 — Conservative Profile Classification
- **Root Cause:**
  1. `test_ifd_flcomp_read_freq_6_not_standalone_v2_proof` was previously writing fast-read frequency `6 << 21` instead of read frequency `6 << 17`.
  2. Read-frequency 6 (17 MHz on Skylake) combined with modern FLMSTR was previously treated as sufficient to automatically infer `V2Modern`. However, flashrom documentation proves frequency 6 is repurposed on later platforms (e.g. 100 MHz on Cannon Point+), making generation ambiguous without hardware strap verification.
- **Fix (DEF-57):**
  - Updated `detect_ifd_profile_resolution` so that automatic frequency 6 + modern FLMSTR without verified Tiger Point soft straps returns `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
  - Prohibited profile-specific decoding (`decode_flreg_with_profile`, `decode_master_rule_with_profile`) from executing on ambiguous automatic profiles.
  - Preserved explicit user-selected profile overrides (`Some(IfdProfile::V2Modern)` produces `ProfileConfidence::UserSelected`).
- **Test Evidence:** `test_ifd_flcomp_read_freq_6_not_standalone_v2_proof`, `test_ifd_unsupported_generation_inference_removed_def48`, and `test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57` pass.

### 2.3 DEF-54 — Geometry Must Respect Unknown Layouts
- **Root Cause:** Unknown FLMAP2 interpretations could produce apparently valid layout geometry without flagging unresolved extents.
- **Fix:** When `flmap2 != 0 && flmap2_interpretation == Flmap2Interpretation::Unknown`, `validate_descriptor_geometry` flags `has_unresolved_extents = true`, pushes diagnostic notes, and invalidates geometry (`is_valid = false`), while preserving physical bounds calculation (`max_regions_extent`) without speculative register decoding.
- **Test Evidence:** `test_ifd_geometry_unknown_flmap2_safe_bounds_def54` passes.

### 2.4 DEF-55 — Firmware Volume Observer Single-Site Accounting
- **Root Cause:** Errors from `parse_at()` propagated into `find_volumes()`, resulting in double counting of candidate header rejections.
- **Fix:** Refactored `FirmwareVolume::parse_at` into `parse_at_internal` wrapped by `parse_at`, which acts as the sole instrumentation site for candidate header outcomes. `find_volumes()` does not duplicate recording for candidates evaluated by `parse_at`.
- **Test Evidence:** `test_uefi_fv_observer_single_site_accounting_def55` passes with 1:1 candidate to observer metric equality across all positive and negative execution paths.

### 2.5 DEF-56 / DEF-60 — Dell Fixture Provenance Decoupling
- **Root Cause:** Documentation referenced two distinct binaries (~13.6 MB PE SFX vs ~35 MB DUP wrapper) without clearly separating on-disk verified evidence from external metadata.
- **Resolution (DEF-60):** Reconciled in `fixtures/manifests/dell_latitude_5x90.json`. The canonical local fixture `Latitude_5X90_1.41.0.exe` (`E24B8F8F...`, 13.6 MB) was verified and hashed directly on disk (`VERIFIED_ON_DISK`). The ~35 MB DUP wrapper (`c5bb883c...`) is recorded as `UNVERIFIED_EXTERNALLY_REFERENCED` from vendor release logs; it is not present on disk, and dual binary verification is explicitly not claimed.
- **Test Evidence:** `test_dell_fixture_provenance_reconciliation_def56` passes.

---

## 3. Phase 2B.5.1 Remediation Summary

| Defect Reference | Subject | Status | Evidence |
|------------------|---------|--------|----------|
| **DEF-57** | Conservative Automatic Profile Selection | **RESOLVED** | Freq 6 + modern FLMSTR yields `Unknown` automatic profile; profile-specific decoders yield neutral/None; user override assigns `UserSelected`. Verified in `test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57`. |
| **DEF-58** | Repository Identity Reconciliation | **RESOLVED** | 5 repository remotes (`biosforge`, `bios_extract`, `efiSeek`, `PSPTool`, `Universal-IFR-Extractor`) reconciled against primary upstreams in `PHASE2B5_REPOSITORY_INVENTORY.md`. |
| **DEF-59** | Correct Integration Feasibility | **RESOLVED** | `IFRExtractor-RS` reclassified as CLI subprocess adapter (2015 edition binary, no lib.rs); `uefisettings` reclassified as host runtime tool; `PSPTool` elevated to Top Candidate alongside `MEAnalyzer`. |
| **DEF-60** | Fixture Provenance Evidence | **RESOLVED** | Canonical on-disk fixture (`E24B8F8F...`, 13.6 MB) verified on disk; ~35 MB DUP wrapper flagged `UNVERIFIED_EXTERNALLY_REFERENCED`; no dual binary verification claimed. |
| **DEF-61** | Integration Architecture Evidence Labels | **RESOLVED** | Specification banner added to `PHASE2B5_INTEGRATION_ARCHITECTURE.md`; designed vs implemented matrix established; Windows Job Objects/AppContainer sandboxing realities documented. |

---

## 4. IFD Profile Confidence Taxonomy (DEF-62)

To ensure consistent, evidence-backed platform identification, FriedChip+ defines four explicit confidence levels for `ProfileConfidence`:

1. **`Confirmed`:**
   - **Definition:** The platform profile is conclusively proven by converging, unambiguous hardware indicators matching authoritative upstream source implementations (e.g. `flashrom/ich_descriptors.c` at commit `35c679c7ec`).
   - **Criteria for `V2Modern`:** Authentic Tiger Point CPU soft strap identification pair (`CSSL == 0x11`, `CSSO == 0x68`, `FLMAP2 = 0x001101A0`), confirmed `FLUMAP1` descriptor context with non-zero `MDTBA` (MIP Descriptor Table Base Address, e.g. `0x02000430`), and modern FLMSTR bitfields (`rd_desc` at bit 8, `rd_bios` at bit 9) with zero legacy FLMSTR match.
   - **Criteria for `V1Legacy`:** Valid legacy `NR` (0..4, designating 1..5 regions), legacy SPI read frequency (20, 25, 33, or 50 MHz), legacy FLMSTR layout (`rd_desc` at bit 16, `rd_bios` at bit 17, reserved bits strictly zero), and valid legacy MCH straps (`FMSBA` non-zero, upper half 0).
   - **Decoding Behavior:** Profile-specific decoding (`decode_flreg_with_profile`, `decode_master_rule_with_profile`) is fully enabled.

2. **`Probable`:**
   - **Definition:** Heuristic or partial binary indicators suggest a particular platform generation, but cross-platform repurposing or generation overlap introduces ambiguity.
   - **Policy:** In accordance with the FriedChip+ conservative classification protocol, ambiguous heuristic indicators (such as frequency 6 without hardware soft straps, or CPU soft straps without upper map MDTBA context) **MUST NOT** proceed with speculative decoding under a `Probable` label. They are downgraded to `Unknown`.

3. **`UserSelected`:**
   - **Definition:** The profile was explicitly provided by the caller or analyst (`profile_override: Some(IfdProfile::...)`).
   - **Decoding Behavior:** Enables profile-specific decoding according to the caller's explicit instructions. The confidence level is explicitly recorded as `UserSelected` (never automatic `Confirmed`) to record that the classification relies on human or harness assertion rather than autonomous hardware proof.

4. **`Unknown`:**
   - **Definition:** Binary evidence is incomplete, conflicting, unverified, or unrecognized.
   - **Decoding Behavior:** Profile-specific decoding is strictly disabled. `decode_flreg_with_profile` returns `None` (preserving safe physical region bounding without speculative register interpretation). `decode_master_rule_with_profile` returns empty/neutral read and write masks.

---

## 5. Phase 2B.5.2 Corrective Engineering Verification (DEF-62, DEF-63, DEF-64)

### 5.1 DEF-62: Tiger Point Profile Identification Verification
- **Old Synthetic Value Rejected:** Synthetically constructed `FLMAP2 = 0x00040040` (CSSL=4, CSSO=0x10) does not match upstream flashrom rules. Tested in `test_ifd_old_synthetic_0x00040040_cannot_confirm_tiger_point_def62`, returning `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
- **Missing Descriptor Context Rejected:** CSSL=0x11 and CSSO=0x68 without `FLUMAP1` or with `MDTBA == 0` returns `Unknown`. Tested in `test_ifd_tiger_point_missing_descriptor_context_returns_unknown_def62`.
- **Conflicting Indicators Rejected:** Conflicting legacy FLMSTR combined with modern straps yields `Unknown`. Tested in `test_ifd_tiger_point_conflicting_evidence_returns_unknown_def62`.
- **Authentic Tiger Point Confirmed:** Populated with upstream values (`FLMAP2 = 0x001101A0`, `FLUMAP1 = 0x02000430`, modern FLMSTR) evaluates cleanly to `V2Modern` / `Confirmed`. Tested in `test_ifd_tiger_point_fully_evidenced_confirmed_def62` and `test_ifd_v2_valid_image_parsing_and_profile_detection`.

### 5.2 DEF-63: Dell Fixture Provenance & Size Discrepancy Reconciliation
- **Clerical Error Resolved:** The value `13,637,736` bytes in Phase 2B.5.1 documentation was verified as a report typo. Actual on-disk binary `fixtures/realworld/Latitude_5X90_1.41.0.exe` is `13,662,488` bytes, exactly matching the manifest.
- **Verification Decoupled:**
  - `test_dell_fixture_manifest_metadata_consistency_def63` asserts JSON manifest structure and external DUP reference status.
  - `test_dell_fixture_binary_provenance_verification_def63` performs direct byte-level filesystem inspection of the local fixture, asserting size `13,662,488`, asserting `actual_size != 13,637,736`, and asserting SHA-256 `E24B8F8F...`.

### 5.3 DEF-64: Subprocess Isolation & Licensing Documentation Audit
- **Fault Boundary vs Sandbox:** Documentation in `PHASE2B5_INTEGRATION_ARCHITECTURE.md` and `PHASE2B5_NEXT_CAPABILITIES.md` updated to separate fault containment from OS sandboxing.
- **Windows Sandboxing Reality:** Job Objects, AppContainer, and DACLs documented as future requirements (`NOT_IMPLEMENTED`).
- **Legal Compliance:** Removed claims that IPC process boundaries guarantee GPL separation. Marked `PSPTool` (GPL-3.0) and other candidates as `LEGAL_REVIEW_REQUIRED`.

---

## 6. Complete Test Suite Execution Results (Phase 2B.5.2)

```text
running 69 tests
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/
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
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/
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
