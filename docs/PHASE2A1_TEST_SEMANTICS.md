# FriedChip+ — Test Outcome Semantics & Integration Testing Framework (DEF-13)

**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE — ZERO SILENT PASSES  
**Target Milestone:** Phase 2A.1 Reliability Architecture  
**Affected Module:** `src-tauri/src/tests/engine_tests.rs`

---

## 1. Executive Summary & Defect Remediation (DEF-13)

Prior to Phase 2A.1, the real-world Dell PFS extraction test inspected the file system, and if the registered Dell firmware binary was missing, it emitted an informational print statement and returned early:
```rust
// Former defective pattern:
if !fixture_path.exists() {
    println!("Skipping real-world test: fixture missing");
    return; // Rust counted this as PASS!
}
```

This pattern violated foundational testing integrity:
1. **False Positive Evidence:** When run in clean CI environments or on developer machines without the heavy 40 MB OEM binary, the test reported `test result: ok`, giving the false appearance of passing real-world firmware regression tests.
2. **Ambiguous Test Accounting:** Skipped tests were counted in the passed tally (`X passed, 0 failed`), obscuring test gaps.

Under Phase 2A.1 remediation, this defect was eradicated:
- **Zero Silent Passes:** A skipped test is NEVER reported as passed.
- **Idiomatic Cargo Attribute:** Real-world integration tests that require external OEM firmware fixtures are annotated with Rust's native `#[ignore = "..."]` attribute.
- **Mandatory Failure on Explicit Execution:** When tests are executed with `--include-ignored`, missing or corrupted fixtures trigger an immediate `panic!`, producing a hard `FAILED` result.

---

## 2. Test Execution Tiers & Cargo Semantics

The test suite is structured into two clearly delineated tiers:

```
+-------------------------------------------------------------------+
|                        CARGO TEST MATRIX                          |
+-------------------------------------------------------------------+
| 1. Default Verification Suite (cargo test)                        |
|    - Fast, hermetic, self-contained                               |
|    - Synthetic fixtures, parser logic, register decoding          |
|    - Negative testing (corruption, truncation, boundary collision)|
|    - In-process stress fuzzing (300,000 bounded iterations)       |
|    - Real-world tests marked as 'ignored' with explicit reason   |
|    - Output: 36 passed; 0 failed; 2 ignored                       |
+-------------------------------------------------------------------+
| 2. Full Real-World Integration Suite (cargo test -- --include-ignored)|
|    - Requires physical OEM firmware fixture in fixtures/realworld/|
|    - Cryptographic SHA-256 pre-flight validation                 |
|    - Differential regression testing against BIOSUtilities golden |
|    - End-to-end component decomposition                          |
|    - Missing/altered fixture causes immediate hard failure        |
|    - Output: 38 passed; 0 failed; 0 ignored                       |
+-------------------------------------------------------------------+
```

### 2.1 Default Invocation Behavior
When a developer or CI pipeline runs `cargo test`, Cargo natively reports:
```text
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/

test result: ok. 36 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```
The two OEM tests are explicitly identified as **ignored**, not passed.

### 2.2 Explicit Invocation Behavior
When executed with `--include-ignored`:
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored
```

#### Scenario A: Registered Fixture is Present & Valid
```text
[PASS] test_dell_pfs_differential_regression_against_biosutilities: All 9 exact matches and 3 documented deltas verified.
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ok
[PASS] test_real_world_dell_latitude_5x90_if_available: Successfully extracted 14 components from verified Dell package.
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

#### Scenario B: Registered Fixture is Missing
If the developer runs `--include-ignored` without placing `Latitude_5X90_1.41.0.exe` in `fixtures/realworld/`:
```text
thread 'tests::engine_tests::test_real_world_dell_latitude_5x90_if_available' panicked at src\tests\engine_tests.rs:520:9:
Explicit real-world test requested via --include-ignored, but Dell fixture was not found in:
1. Environment variable DELL_FIXTURE_PATH
2. Relative workspace directory 'fixtures/realworld/Latitude_5X90_1.41.0.exe'
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... FAILED
```
The test suite fails completely, preventing any untested release build from slipping through.

#### Scenario C: Registered Fixture Has Hash Mismatch
If the binary is damaged or altered:
```text
thread 'tests::engine_tests::test_real_world_fixture_hash_mismatch_panics' panicked at src\tests\engine_tests.rs:548:9:
Dell real-world fixture SHA-256 verification failed! Corrupted or altered fixture binary.
Expected: e17b6291a13437e54f3b5fb3170e1a8fa4c379a0cf19230558cf7ee847dbef84
```
Verified via automated negative test `test_real_world_fixture_hash_mismatch_panics` (`- should panic ... ok`).

---

## 3. Exhaustive Engine Test Inventory (38 Tests)

| # | Test Function Name | Test Type | Target Component | Description / Assertion |
|---|---|---|---|---|
| 1 | `test_flreg_decoding_15bit_addressing` | Unit | IFD Parser | Validates 15-bit address shifts (`base << 12`, `limit << 12 \| 0xFFF`) |
| 2 | `test_ifd_flcomp_known_raw_register_values` | Golden Unit | IFD Component | Verifies raw registers `0x08900005` (16MB, 50MHz) & `0x0000002B` (Dual) |
| 3 | `test_ifd_flmstr_known_raw_register_values` | Golden Unit | IFD Master | Verifies IFD v1 `0x02170000`, full-access `0xFFFFFFFF`, and IFD v2 `0x00200300` |
| 4 | `test_ifd_fcba_decoding_component_density` | Synthetic | IFD Component | Parses synthetic image with FCBA density and clock fields |
| 5 | `test_ifd_master_permissions_decoding` | Synthetic | IFD Master | Verifies master permission breakdown across Host CPU, ME, and GbE |
| 6 | `test_ifd_valid_image_parsing_and_gates` | Synthetic | IFD Gates | Verifies Gate 1 (passed) and Gate 2 platform validation |
| 7 | `test_ifd_gate3_programmer_indeterminate_without_hardware` | Security Unit | IFD Gate 3 | Asserts Gate 3 is `Indeterminate` and `passed = false` without hardware |
| 8 | `test_ifd_corrupt_signature_rejection` | Negative | IFD Parser | Rejects bad signature magic at offset `0x10` |
| 9 | `test_ifd_truncated_under_4kb_rejection` | Negative | IFD Parser | Rejects images under 4 KB |
| 10 | `test_ifd_out_of_bounds_detection` | Negative | IFD Parser | Detects regions exceeding image boundary |
| 11 | `test_ifd_boundary_collision_detection` | Negative | IFD Parser | Detects overlapping flash regions |
| 12 | `test_ifd_frba_out_of_bounds_handling` | Negative | IFD Parser | Handles corrupt FRBA pointing outside binary |
| 13 | `test_synthetic_dell_pfs_extraction_end_to_end` | Synthetic | Dell PFS | Extracts synthetic PFS container with ZLIB decompression |
| 14 | `test_dell_pfs_corrupt_xor_detection` | Negative | Dell PFS | Detects corrupted byte in XOR checksum calculation |
| 15 | `test_dell_pfs_corrupted_inverted_crc32` | Negative | Dell PFS | Detects corrupt inverted CRC-32 in PFS footer |
| 16 | `test_dell_pfs_truncated_zlib_stream` | Negative | Dell PFS | Detects truncated ZLIB payload before EOF |
| 17 | `test_uefi_firmware_volume_header_parsing` | Synthetic | UEFI FV | Parses valid UEFI FV with block maps and attributes |
| 18 | `test_uefi_fv_corrupt_signature_rejection` | Negative | UEFI FV | Rejects volume missing `_FVH` signature magic |
| 19 | `test_uefi_fv_invalid_checksum_rejection` | Negative | UEFI FV | Rejects volume with invalid uint16 checksum |
| 20 | `test_uefi_fv_truncated_header` | Negative | UEFI FV | Rejects truncated FV header |
| 21 | `test_uefi_fv_block_map_sentinel_missing` | Negative | UEFI FV | Rejects FV missing terminating `{0, 0}` block map sentinel |
| 22 | `test_uefi_fv_misaligned_volume` | Negative | UEFI FV | Rejects unaligned FV start offset |
| 23 | `test_real_world_fixture_hash_mismatch_panics` | Negative | Fixture Harness | Asserts that corrupt fixture SHA-256 triggers hard panic |
| 24 | `test_firmware_artifact_persistence_roundtrip` | Integration | Storage / DB | Roundtrip artifact persistence in SQLite database |
| 25 | `test_fresh_database_initialization` | Unit | Database | Validates fresh SQLite schema and migrations |
| 26 | `test_database_migrations_idempotency` | Unit | Database | Validates migration idempotency on repeated init |
| 27 | `test_file_import_and_metadata_persistence` | Integration | Workspace | Imports file into workspace and verifies DB record |
| 28 | `test_source_file_preservation` | Security Unit | Workspace | Verifies original source file is read-only and preserved |
| 29 | `test_source_deletion_or_relocation_detected` | Integration | Workspace | Detects deletion or moving of imported source file |
| 30 | `test_source_modification_detected_after_import` | Integration | Workspace | Detects tampering with source file post-import |
| 31 | `test_sha256_correctness` | Unit | Crypto | Validates SHA-256 implementation against standard test vectors |
| 32 | `test_unknown_file_identification_not_inferred_from_extension` | Unit | Workspace | Verifies file format is identified by magic, not extension |
| 33 | `test_duplicate_import_after_application_restart` | Integration | Workspace | Prevents duplicate imports on restarted workspace |
| 34 | `test_persistent_workspace_recovery` | Integration | Workspace | Validates state restoration from persistent workspace |
| 35 | `test_output_collision_and_atomic_overwrite_rejection` | Security Unit | Storage | Validates atomic file writes and overwrite protection |
| 36 | `test_in_process_bounded_parser_stress_fuzzing` | Fuzz Stress | Multi-Engine | Executes 300,000 deterministic pseudo-random fuzz iterations |
| 37 | `test_dell_pfs_differential_regression_against_biosutilities` | Differential | Dell PFS | Verifies 9 exact matches & 3 documented deltas vs BIOSUtilities (`#[ignore]`) |
| 38 | `test_real_world_dell_latitude_5x90_if_available` | Real-World | Dell PFS | Extracts 14 components from verified Latitude 5X90 package (`#[ignore]`) |

---

## 4. Verification Instructions for CI / Local Development

### Local Fast Check (Default)
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Full Integration Run with Fixtures
```bash
# 1. Ensure Latitude_5X90_1.41.0.exe is present in fixtures/realworld/
# 2. Run with --include-ignored
cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored --nocapture
```
Both modes execute cleanly with zero errors.
