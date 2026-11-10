# Phase 2B.5.1 — Independent Review Corrections & Research Audit Reconciliation Report

**Project:** FriedChip+  
**Release:** Phase 2B.5.1  
**Starting Commit:** `67ff4c0d2790c7e7b781f3aba2b20de74d944875`  
**Execution Timestamp:** 2026-11-10  
**Phase 3 Status:** **STRICTLY LOCKED** (No physical flashing, no live kernel drivers, no PFAT decapsulation)  
**New Firmware Formats:** **NONE (Not Authorized)**

---

## 1. Executive Summary

Following the completion of Phase 2B.5, an independent review identified five engineering and documentation discrepancies requiring formal reconciliation:
- **DEF-57:** Profile classification assigned `Probable` confidence to frequency 6 + modern FLMSTR when platform generation is ambiguous across Skylake and Cannon Point+.
- **DEF-58:** Repository inventory listed upstream remote URLs that diverged from the canonical clones present in `research/third_party/` for five projects.
- **DEF-59:** Architectural audit overclaimed library reusability for `IFRExtractor-RS` (a 2015-edition CLI binary without library exports) and `uefisettings` (a Linux host runtime tool unable to parse offline SPI flash NVRAM images on Windows).
- **DEF-60:** Dell fixture documentation asserted dual-binary verification when only the canonical ~13.6 MB PE SFX binary was hashed and verified on disk.
- **DEF-61:** External adapter architecture lacked an explicit disclaimer distinguishing designed protocol specifications from implemented runtime code, and omitted Windows subprocess sandboxing realities.

All five findings have been completely resolved, verified through automated regression tests, and audited across documentation artifacts.

---

## 2. Detailed Technical Remediation

### 2.1 DEF-57 — Conservative Automatic Profile Selection
- **Problem Statement:** In Phase 2B.5, when FLCOMP read frequency was `6` (17 MHz on Skylake) and FLMSTR had modern bitfields, `detect_ifd_profile_resolution()` returned `IfdProfile::V2Modern` with `ProfileConfidence::Probable`. However, flashrom documentation (`ich_descriptors.h`) proves frequency encoding `6` was repurposed on Cannon Point+ to denote 100 MHz fast-read. Because generation is ambiguous without hardware strap verification, automatically inferring `V2Modern` violated conservative engineering principles.
- **Code Remediation:**
  - Modified `detect_ifd_profile_resolution()` in `src-tauri/src/engine/binary/ifd.rs`:
    * When read frequency is 6 and modern FLMSTR is observed without verified hardware layout (Tiger Point CPU soft straps), automatic detection returns `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
    * Profile-specific decoding methods (`decode_flreg_with_profile`, `decode_master_rule_with_profile`) strictly refuse to decode on `IfdProfile::Unknown` (returning neutral bounds and empty permission masks).
    * Explicit user profile overrides (`Some(IfdProfile::V2Modern)`) are preserved and assign `ProfileConfidence::UserSelected`.
    * Synthetic V2 fixture builder (`build_valid_ifd_v2_image`) was updated to include authentic Tiger Point soft strap indicators (`(0x04 << 16) | 0x40`) to ensure positive V2 fixtures unambiguously evaluate to `IfdProfile::V2Modern` with `ProfileConfidence::Confirmed`.
- **Test Evidence:**
  - `test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57`: Verifies that ambiguous automatic evidence yields `Unknown`, profile-specific decoding returns `None` / neutral masks, user override yields `UserSelected`, and Tiger Point hardware layout yields `Confirmed`.
  - `test_ifd_unsupported_generation_inference_removed_def48`: Verifies that freq 6 on ICCRIBA layout returns `Unknown`.

### 2.2 DEF-58 — Repository Identity Reconciliation
- **Problem Statement:** Documentation in `PHASE2B5_REPOSITORY_INVENTORY.md` listed secondary mirrors or personal fork URLs for five repositories rather than the authentic remotes of the local checkouts in `research/third_party/`.
- **Remediation:**
  - All 25 repositories were inspected using `git remote get-url origin`, `git log -1`, author attributions, and license texts.
  - The five repository remotes and authors were reconciled in `PHASE2B5_REPOSITORY_INVENTORY.md` without deleting or modifying local clones:
    1. `biosforge`: Remote `https://github.com/GPU-Solution/biosforge.git`, commit `535309e407`, authored by Lucas Efler / Misio.
    2. `bios_extract`: Remote `https://github.com/coreboot/bios_extract.git`, commit `0a7bc1d717`, primary upstream coreboot repository maintained by Nicholas Chin.
    3. `efiSeek`: Remote `https://github.com/DSecurity/efiSeek.git`, commit `573f4b9b5b`, primary authoritative repository authored by Evgeny Rasskazov (DSec).
    4. `PSPTool`: Remote `https://github.com/PSPReverse/PSPTool.git`, commit `5055403889`, primary tool repository authored by Christian Werling.
    5. `Universal-IFR-Extractor`: Remote `https://github.com/donovan6000/Universal-IFR-Extractor.git`, commit `8f55681108`, original C++ implementation authored by Donovan Cudney.

### 2.3 DEF-59 — Integration Feasibility Reclassification
- **Problem Statement:** Previous reports classified `IFRExtractor-RS` and `uefisettings` as `INTEGRATE_AS_LIBRARY` (direct Cargo crate dependencies) without inspecting their actual package manifests, entry points, and OS requirements.
- **Remediation:**
  - `IFRExtractor-RS`: Source inspection revealed Cargo package `ifrextractor` v1.6.1 is Rust 2015 Edition with dependencies `nom = 4.2.3` and `sha256 = 1.6.0`. It contains `src/main.rs`, `src/framework_parser.rs`, and `src/uefi_parser.rs`, but NO `src/lib.rs` and zero unit tests. It is an executable CLI binary, not an importable library crate. Reclassified as `INTEGRATE_AS_EXTERNAL_ADAPTER` (CLI Subprocess Adapter) or candidate for clean-room modern Rust rewrite.
  - `uefisettings`: Source inspection revealed it interacts with Linux kernel sysfs (`/sys/firmware/efi/efivars`), HiiDB, or UEFI Shell runtime boot services (`GetVariable()`). It does not parse raw offline SPI flash NVRAM binary stores (NVAR/VSS). Reclassified as `RESEARCH_ONLY` (not an offline SPI NVRAM parser).
  - `MEAnalyzer`: Retained as `INTEGRATE_AS_EXTERNAL_ADAPTER`. Reconciled documentation to explicitly note that its native script (`MEA.py`) outputs terminal ASCII text, necessitating an adapter wrapper script to bridge to FriedChip+'s JSON protocol.
  - `PSPTool`: Cloned under `research/third_party/PSPTool` (`5055403889`, GPL-3.0, Christian Werling), parsing AMD PSP Directory Tables 1 and 2, BIOS directories, and entry types. Elevated to Top Candidate for Phase 3 alongside MEAnalyzer.
  - Summary classification table in `PHASE2B5_REPOSITORY_AUDIT.md` updated: `INTEGRATE_AS_LIBRARY` count is 0; `INTEGRATE_AS_EXTERNAL_ADAPTER` count is 4.

### 2.4 DEF-60 — Fixture Provenance Decoupling
- **Problem Statement:** Manifest and validation documentation referenced a ~35 MB Dell Update Package (DUP) installer alongside the ~13.6 MB PE SFX binary, implying dual-binary local verification.
- **Remediation:**
  - In `fixtures/manifests/dell_latitude_5x90.json`, clearly separated verified on-disk artifacts from external reference metadata:
    * `canonical_verification_status`: `"VERIFIED_ON_DISK"` (SHA-256 `E24B8F8F...`, size 13,637,736 bytes).
    * `alternative_verification_status`: `"UNVERIFIED_EXTERNALLY_REFERENCED"` (SHA-256 `c5bb883c...`, size ~35 MB, referenced from Dell support logs).
  - Explicitly stated that dual-binary verification was not performed because only the canonical SFX binary is present locally.

### 2.5 DEF-61 — Integration Architecture Labels & Windows Subprocess Realities
- **Problem Statement:** `PHASE2B5_INTEGRATION_ARCHITECTURE.md` did not emphasize that the external adapter JSON protocol is a design specification, and lacked analysis of Windows OS security primitives for child process isolation.
- **Remediation:**
  - Added a prominent specification warning banner at the top of `PHASE2B5_INTEGRATION_ARCHITECTURE.md`.
  - Added an Implementation Status Matrix distinguishing:
    * Designed Adapter Contract: SPECIFIED
    * Implemented Process Runner: NOT IMPLEMENTED
    * Tested Process Isolation: NOT IMPLEMENTED
    * Real-World Adapter Execution: NOT AUTHORIZED
  - Documented Windows subprocess security realities: `std::process::Command` does not sandbox processes on Windows; production sandboxing requires Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, memory limits), AppContainer / Restricted Low Integrity Tokens, explicit DACLs on scratch directories, and network socket isolation.

---

## 3. Automated Test Suite Verification

The complete test suite was executed across all unit, differential, regression, and observer tests:

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
test tests::engine_tests::test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57 ... ok
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

test result: ok. 60 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 24.32s
```

All 60 tests pass. The two ignored tests are the documented real-world OEM binary tests requiring the optional local fixture.

---

## 4. Scope & Milestone Closure

- **Phase 3 Remains Strictly Locked:**
  - No physical SPI flashing routines implemented.
  - No kernel driver interfaces added.
  - PFAT decapsulation remains deferred.
  - No child process runner or sandbox harness executed.
- **Repository Integrity:** All 25 repositories under `research/third_party/` verified and accounted for.
- **Documentation Reconciliation:** All 7 Phase 2B.5 documents updated and synchronized with verified reality.

**Phase 2B.5.1 is officially declared CLOSED and ready for final review.**
