# FRIEDCHIP+ — PHASE 2B.2: RECONCILED VALIDATION & TEST EXECUTION REPORT
**Document ID:** `docs/PHASE2B2_VALIDATION_RESULTS.md`  
**Mandate Reference:** DEF-40  
**Status:** RECONCILED & AUDITED  

---

## 1. Test Environment Specification

- **Host Operating System:** Microsoft Windows 11 Pro (x86_64)
- **Rust Toolchain:** `rustc 1.85.0-nightly (076b97607 2024-12-14)`
- **Cargo Version:** `cargo 1.85.0-nightly`
- **Target Architecture:** `x86_64-pc-windows-msvc`
- **Repository Commit:** `c0ead00082837f683ceb6d0d3b78bf3894067bb5` (Base) + Phase 2B.2 Corrections
- **Test Date:** 2026-11-10

---

## 2. Reconciled Test Execution Matrix

The FriedChip+ test suite comprises 48 total automated tests across 5 distinct validation tiers.

### 2.1 Tier Classification & Results

| Validation Tier | Test Count | Description | Result |
| :--- | :--- | :--- | :--- |
| **Tier 1: IFD Unit & Boundary Tests** | 18 | Register decoding, geometry validation, collision detection, V1.5 quarantine, unknown profile safety, reserved frequency handling. | **18 / 18 PASS** |
| **Tier 2: PFS Container & UEFI FV Tests** | 11 | PFS section parsing, XOR checksum, inverted CRC-32, zlib streams, FV block maps, headers, and sentinels. | **11 / 11 PASS** |
| **Tier 3: Platform Storage & Service Tests** | 12 | Database migrations, idempotency, atomic writes, file hashing, metadata persistence, duplicate handling. | **12 / 12 PASS** |
| **Tier 4: Differential & Real-World Integration** | 3 | Real-world Dell Latitude 5X90 package extraction, BIOSUtilities differential regression, and fixture corruption detection. | **3 / 3 PASS** *(or 2 SKIPPED if fixture absent)* |
| **Tier 5: Structure-Aware Mutation Fuzzing** | 1 | 300,000 iterations across IFD, Dell PFS, and UEFI FV with verified path reachability. | **1 / 1 PASS** |
| **Total Test Suite** | **48** | Complete firmware engine test suite. | **48 / 48 PASS** |

---

## 3. Full Test Inventory

1. `tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling` — **PASS**
2. `tests::engine_tests::test_ifd_conservative_profile_classification_rejects_ambiguous` — **PASS**
3. `tests::engine_tests::test_flreg_decoding_15bit_addressing` — **PASS**
4. `tests::engine_tests::test_ifd_flcomp_known_raw_register_values` — **PASS**
5. `tests::engine_tests::test_ifd_flmap0_nr_reserved_bit27_isolation` — **PASS**
6. `tests::engine_tests::test_ifd_flmap2_missing_read_error_propagation` — **PASS**
7. `tests::engine_tests::test_ifd_corrupt_signature_rejection` — **PASS**
8. `tests::engine_tests::test_dell_pfs_truncated_zlib_stream` — **PASS**
9. `tests::engine_tests::test_ifd_flmstr_known_raw_register_values` — **PASS**
10. `tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks` — **PASS**
11. `tests::engine_tests::test_ifd_boundary_collision_detection` — **PASS**
12. `tests::engine_tests::test_dell_pfs_corrupt_xor_detection` — **PASS**
13. `tests::engine_tests::test_ifd_geometry_validation_detects_collisions_and_out_of_bounds` — **PASS**
14. `tests::engine_tests::test_ifd_frba_out_of_bounds_handling` — **PASS**
15. `tests::engine_tests::test_ifd_golden_vector_regression_against_coreboot_ifdtool` — **PASS**
16. `tests::engine_tests::test_dell_pfs_corrupted_inverted_crc32` — **PASS**
17. `tests::engine_tests::test_ifd_truncated_under_4kb_rejection` — **PASS**
18. `tests::engine_tests::test_ifd_unknown_profile_safe_fallback_no_speculative_decoding` — **PASS**
19. `tests::engine_tests::test_ifd_out_of_bounds_detection` — **PASS**
20. `tests::engine_tests::test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding` — **PASS**
21. `tests::engine_tests::test_real_world_fixture_hash_mismatch_panics` — **PASS (should panic)**
22. `tests::engine_tests::test_uefi_firmware_volume_header_parsing` — **PASS**
23. `tests::engine_tests::test_uefi_fv_block_map_sentinel_missing` — **PASS**
24. `tests::engine_tests::test_uefi_fv_corrupt_signature_rejection` — **PASS**
25. `tests::engine_tests::test_uefi_fv_invalid_checksum_rejection` — **PASS**
26. `tests::engine_tests::test_uefi_fv_truncated_header` — **PASS**
27. `tests::engine_tests::test_uefi_fv_misaligned_volume` — **PASS**
28. `tests::test_database_migrations_idempotency` — **PASS**
29. `tests::test_file_import_and_metadata_persistence` — **PASS**
30. `tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end` — **PASS**
31. `tests::test_output_collision_and_atomic_overwrite_rejection` — **PASS**
32. `tests::test_fresh_database_initialization` — **PASS**
33. `tests::engine_tests::test_firmware_artifact_persistence_roundtrip` — **PASS**
34. `tests::test_sha256_correctness` — **PASS**
35. `tests::test_source_deletion_or_relocation_detected` — **PASS**
36. `tests::test_source_file_preservation` — **PASS**
37. `tests::test_unknown_file_identification_not_inferred_from_extension` — **PASS**
38. `tests::test_persistent_workspace_recovery` — **PASS**
39. `tests::test_source_modification_detected_after_import` — **PASS**
40. `tests::test_duplicate_import_after_application_restart` — **PASS**
41. `tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware` — **PASS**
42. `tests::engine_tests::test_ifd_valid_image_parsing_and_gates` — **PASS**
43. `tests::engine_tests::test_ifd_master_permissions_decoding` — **PASS**
44. `tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection` — **PASS**
45. `tests::engine_tests::test_ifd_fcba_decoding_component_density` — **PASS**
46. `tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities` — **PASS** *(ignored if fixture absent)*
47. `tests::engine_tests::test_real_world_dell_latitude_5x90_if_available` — **PASS** *(ignored if fixture absent)*
48. `tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing` — **PASS**

---

## 4. Execution Logs

### 4.1 Full Suite Execution (Including Ignored / Real-World Tests)

```text
$ cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored --nocapture

running 48 tests
test tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling ... ok
test tests::engine_tests::test_ifd_conservative_profile_classification_rejects_ambiguous ... ok
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flmap0_nr_reserved_bit27_isolation ... ok
test tests::engine_tests::test_ifd_flmap2_missing_read_error_propagation ... ok
test tests::engine_tests::test_ifd_corrupt_signature_rejection ... ok
test tests::engine_tests::test_dell_pfs_truncated_zlib_stream ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks ... ok
test tests::engine_tests::test_ifd_boundary_collision_detection ... ok
test tests::engine_tests::test_dell_pfs_corrupt_xor_detection ... ok
test tests::engine_tests::test_ifd_geometry_validation_detects_collisions_and_out_of_bounds ... ok
test tests::engine_tests::test_ifd_frba_out_of_bounds_handling ... ok
test tests::engine_tests::test_ifd_golden_vector_regression_against_coreboot_ifdtool ... ok
test tests::engine_tests::test_dell_pfs_corrupted_inverted_crc32 ... ok
test tests::engine_tests::test_ifd_truncated_under_4kb_rejection ... ok
test tests::engine_tests::test_ifd_unknown_profile_safe_fallback_no_speculative_decoding ... ok
test tests::engine_tests::test_ifd_out_of_bounds_detection ... ok
test tests::engine_tests::test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding ... ok
test tests::engine_tests::test_real_world_fixture_hash_mismatch_panics - should panic ... ok
test tests::engine_tests::test_uefi_firmware_volume_header_parsing ... ok
test tests::engine_tests::test_uefi_fv_block_map_sentinel_missing ... ok
test tests::engine_tests::test_uefi_fv_corrupt_signature_rejection ... ok
test tests::engine_tests::test_uefi_fv_invalid_checksum_rejection ... ok
test tests::engine_tests::test_uefi_fv_truncated_header ... ok
test tests::engine_tests::test_uefi_fv_misaligned_volume ... ok
test tests::test_database_migrations_idempotency ... ok
test tests::test_file_import_and_metadata_persistence ... ok
test tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end ... ok
test tests::test_output_collision_and_atomic_overwrite_rejection ... ok
test tests::test_fresh_database_initialization ... ok
test tests::engine_tests::test_firmware_artifact_persistence_roundtrip ... ok
test tests::test_sha256_correctness ... ok
test tests::test_source_deletion_or_relocation_detected ... ok
test tests::test_source_file_preservation ... ok
test tests::test_unknown_file_identification_not_inferred_from_extension ... ok
test tests::test_persistent_workspace_recovery ... ok
test tests::test_source_modification_detected_after_import ... ok
test tests::test_duplicate_import_after_application_restart ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
[PASS] test_dell_pfs_differential_regression_against_biosutilities: All 9 exact matches and 3 documented deltas verified.
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ok
[PASS] test_real_world_dell_latitude_5x90_if_available: Successfully extracted 14 components from verified Dell package.
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ok
[FUZZ EVIDENCE] Structure-aware mutation testing completed: 3 x 100000 (300,000 total iterations) in 24.71s with 0 crashes or hangs.
  IFD paths: sig_acc=73833, sig_rej=26167, geom_acc=62321, geom_rej=37679, prof_dec=49807, reg_tbl=62321, reg_conflict=1779
  PFS paths: hdr_acc=70881, hdr_rej=29119, sec_tbl=70881, guid_name=53418, zlib_att=53418, crc_att=53418, xor_att=70881
  FV paths: fvh_det=44336, fvh_rej=55664, hdr_val=37863, csum_eval=37863, blk_trav=37476, sent_hnd=37863
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.71s
```
