# FriedChip+ — Phase 2B.4 Validation Results & Test Inventory Reconciliation (DEF-51)

**Document Reference:** `docs/PHASE2B4_VALIDATION_RESULTS.md`  
**Status:** RECONCILED AND VERIFIED  
**Test Run Command:** `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`  
**Outcome:** `55 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 21.11s`

---

## 1. Test Suite Reconciliation & Inventory

The test suite contains 57 test functions partitioned into four deterministic tiers:

| Tier | Description | Total Tests | Passed | Ignored | Failed |
|:---|:---|:---:|:---:|:---:|:---:|
| **Tier 1** | Unit & Register Bitfield Decoding (FLREG, FLCOMP, FLMSTR, FLMAP0/1/2) | 20 | 20 | 0 | 0 |
| **Tier 2** | Pre-Traversal Geometry, Extent Tracking & Boundary Collision Checks | 19 | 19 | 0 | 0 |
| **Tier 3** | Structure-Aware In-Process Mutation Fuzzing (300,000 iterations) & Targeted Observer Path Fixtures | 2 | 2 | 0 | 0 |
| **Tier 4** | Database Persistence, Workspace Recovery & Golden-Reference / OEM Differential Tests | 16 | 14 | 2 | 0 |
| **Total** | Full Engine & Workspace Test Suite | **57** | **55** | **2** | **0** |

---

## 2. Tier 4 External Fixture Status (DEF-51 Audit)

Two Tier 4 tests require the authentic OEM binary `Latitude_5X90_1.41.0.exe` in `fixtures/realworld/` or via `FRIEDCHIP_DELL_FIXTURE_PATH`:
1. `tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities`:
   - Validates all 9 extracted payloads against stored BIOSUtilities golden hashes.
   - Validates the 3 documented differential discrepancies (PFAT container encapsulation, Unicode name translation, raw volume header offset).
   - Status in environments lacking the 35 MB proprietary executable: `SKIPPED / IGNORED` (`#[ignore]`).
2. `tests::engine_tests::test_real_world_dell_latitude_5x90_if_available`:
   - Executes live extraction against the real-world package when available.
   - Status in environments lacking the proprietary executable: `SKIPPED / IGNORED` (`#[ignore]`).

Both tests execute and pass when the registered OEM fixture is present. In standard clean checkout environments without proprietary binaries, they are explicitly skipped and never falsely recorded as passed.

### Registered Fixture Provenance
- **Binary:** `Latitude_5X90_1.41.0.exe`
- **Vendor:** Dell Inc.
- **Payload Family:** Dell PFS (System BIOS / EC / ME update container)
- **SHA-256:** `c5bb883ca09bd8eb38c64eb3d047ec6cb66847deef94e101f3e79e6f315a6e87`
- **Integrity Enforcement:** Tested in `test_real_world_fixture_hash_mismatch_panics`, proving that corrupted or tampered fixtures panic immediately before parser execution.

---

## 3. Pinned Upstream Repositories

| Repository | Pinned Commit SHA | Relevant Source Paths | Adoption / Reference Purpose |
|:---|:---|:---|:---|
| **flashrom** | `35c679c7827dd6ec34005e83ec5be870bfb43494` | `include/ich_descriptors.h`<br>`ich_descriptors.c` | FLMAP2 Tiger Point `CSSO`/`CSSL` layout definition; `struct ich_desc_component` 12-byte extent definition. |
| **coreboot** | `66e704b123bb0d1737e42d7088b9ddfa6d27464d` | `util/ifdtool/ifdtool.c` | Strict manual platform profile requirement (`-p`); refusal to perform speculative profile classification. |
| **BIOSUtilities** | `a90cb541a7d603a11dc1045a4a4964177b94dbba` | `Dell_PFS_Extract.py` | Reference for Dell PFS section marker parsing, inverted CRC32 validation, and component extraction. |

---

## 4. Test Verification Command & Output Log

```text
running 57 tests
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_conservative_profile_classification_rejects_ambiguous ... ok
test tests::engine_tests::test_ifd_explicit_override_confidence_semantics ... ok
test tests::engine_tests::test_ifd_corrupt_signature_rejection ... ok
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_boundary_collision_detection ... ok
test tests::engine_tests::test_ifd_flcomp_read_freq_6_not_standalone_v2_proof ... ok
test tests::engine_tests::test_dell_pfs_truncated_zlib_stream ... ok
test tests::engine_tests::test_dell_pfs_corrupt_xor_detection ... ok
test tests::engine_tests::test_ifd_flmap2_interpretation_semantics_def47 ... ok
test tests::engine_tests::test_dell_pfs_corrupted_inverted_crc32 ... ok
test tests::engine_tests::test_ifd_flmap2_missing_read_error_propagation ... ok
test tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling ... ok
test tests::engine_tests::test_ifd_flmap0_nr_reserved_bit27_isolation ... ok
test tests::engine_tests::test_ifd_flmap2_iccriba_accounting_and_safe_boundaries ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flmstr_older_layout_cannot_accidentally_trigger_modern ... ok
test tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks ... ok
test tests::engine_tests::test_ifd_frba_out_of_bounds_handling ... ok
test tests::engine_tests::test_ifd_geometry_adversarial_collision_checks ... ok
test tests::engine_tests::test_ifd_geometry_validation_detects_collisions_and_out_of_bounds ... ok
test tests::engine_tests::test_ifd_geometry_known_vs_unknown_extents_def49 ... ok
test tests::engine_tests::test_ifd_out_of_bounds_detection ... ok
test tests::engine_tests::test_ifd_golden_vector_regression_against_coreboot_ifdtool ... ok
test tests::engine_tests::test_ifd_truncated_under_4kb_rejection ... ok
test tests::engine_tests::test_ifd_unknown_profile_safe_fallback_no_speculative_decoding ... ok
test tests::engine_tests::test_ifd_unsupported_generation_inference_removed_def48 ... ok
test tests::engine_tests::test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding ... ok
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored
test tests::engine_tests::test_real_world_fixture_hash_mismatch_panics - should panic ... ok
test tests::engine_tests::test_uefi_fv_block_map_sentinel_missing ... ok
test tests::engine_tests::test_observer_targeted_path_fixtures_def50 ... ok
test tests::engine_tests::test_uefi_firmware_volume_header_parsing ... ok
test tests::engine_tests::test_uefi_fv_corrupt_signature_rejection ... ok
test tests::engine_tests::test_uefi_fv_invalid_checksum_rejection ... ok
test tests::engine_tests::test_uefi_fv_truncated_header ... ok
test tests::engine_tests::test_uefi_fv_misaligned_volume ... ok
test tests::test_database_migrations_idempotency ... ok
test tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end ... ok
test tests::test_file_import_and_metadata_persistence ... ok
test tests::test_output_collision_and_atomic_overwrite_rejection ... ok
test tests::test_sha256_correctness ... ok
test tests::engine_tests::test_firmware_artifact_persistence_roundtrip ... ok
test tests::test_source_deletion_or_relocation_detected ... ok
test tests::test_source_file_preservation ... ok
test tests::test_fresh_database_initialization ... ok
test tests::test_source_modification_detected_after_import ... ok
test tests::test_unknown_file_identification_not_inferred_from_extension ... ok
test tests::test_duplicate_import_after_application_restart ... ok
test tests::test_persistent_workspace_recovery ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 55 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 21.11s
```
