# FriedChip+ — Phase 2 Test Evidence & Verification Audit

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Test Suite Verification & Audit Evidence  
**Milestone:** Phase 2 Implementation Audit & Scope Freeze  
**Audit Date:** November 2026  
**Auditor:** Antigravity (Advanced Agentic Coding)  

---

## 1. Executive Summary: Audit of the "20 Passing Tests" Claim

The prior Phase 2 report recorded:
> *"20 tests passing with 0 failures: 11 workspace & safety integrity tests + 9 binary engine & extractor tests."*

This audit rigorously dissects that claim to eliminate confirmation bias, isolate synthetic assumptions, and establish an evidence-based quality baseline.

### Key Audit Verdict
1. **The Execution Count is Accurate:** The test harness executes exactly 20 test functions in `2.19s`, and all 20 return exit code 0 (`ok. 20 passed; 0 failed; 0 ignored`).
2. **The Scope is Substantially Synthetic:** Of the 20 tests, **15 are synthetic unit/integration tests**, **4 are negative/malformed input tests**, and **only 1 is a real-world test** (which executes conditionally).
3. **Zero Reference-Based Differential Tests Exist:** There are currently **no** automated differential tests running against golden outputs of upstream reference implementations (e.g. `BIOSUtilities/dell_pfs_extract.py`).
4. **Zero Fuzz Tests Exist:** No randomized fuzzer (e.g. `cargo-fuzz` / `libFuzzer`) is currently checking parser bounds against malformed or mutated inputs.
5. **Circularity Hazard Identified:** Several synthetic tests validate that the parser agrees with the `SyntheticFixtureBuilder`, rather than proving interoperability with real-world OEM firmware variations.

---

## 2. Test Execution Environment & Command

### Execution Command
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

### Full Test Execution Log
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.75s
     Running unittests src\lib.rs (src-tauri\target\debug\deps\lbe_desktop_lib-f070c3f4e1fb6454.exe)

running 20 tests
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_corrupt_signature_rejection ... ok
test tests::engine_tests::test_ifd_out_of_bounds_detection ... ok
test tests::engine_tests::test_ifd_boundary_collision_detection ... ok
test tests::engine_tests::test_dell_pfs_corrupt_xor_detection ... ok
test tests::engine_tests::test_uefi_firmware_volume_header_parsing ... ok
test tests::test_database_migrations_idempotency ... ok
test tests::test_sha256_correctness ... ok
test tests::test_file_import_and_metadata_persistence ... ok
test tests::test_source_deletion_or_relocation_detected ... ok
test tests::test_source_file_preservation ... ok
test tests::test_unknown_file_identification_not_inferred_from_extension ... ok
test tests::test_source_modification_detected_after_import ... ok
test tests::test_output_collision_and_atomic_overwrite_rejection ... ok
test tests::engine_tests::test_synthetic_dell_pfs_extraction_end_to_end ... ok
test tests::test_fresh_database_initialization ... ok
test tests::test_persistent_workspace_recovery ... ok
test tests::test_duplicate_import_after_application_restart ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.19s
```

---

## 3. Systematic Test Classification Register

The 20 tests are classified into five explicit engineering categories:

| Category | Count | Tests Included |
| :--- | :---: | :--- |
| **Synthetic Fixture Tests** | 15 | 4 engine tests + 11 workspace/persistence tests |
| **Negative & Malformed-Input Tests** | 4 | IFD corrupt sig, IFD collision, IFD out-of-bounds, PFS corrupt XOR |
| **Reference-Based Differential Tests** | **0** | *None implemented* |
| **Real-World Firmware Tests** | **1** | `test_real_world_dell_latitude_5x90_if_available` (Conditional) |
| **Fuzz Tests** | **0** | *None implemented* |

---

### Detailed Test Inventory

#### Part A: Firmware Engine Tests (`src-tauri/src/tests/engine_tests.rs`)

1. **`test_flreg_decoding_15bit_addressing`**
   - **Category:** Synthetic Unit Test
   - **What It Validates:** Asserts `decode_flreg(val)` masks bits 15 and 31, shifts base and limit by 12, sets lower 12 bits of limit to `0xFFF`, and identifies disabled regions where `limit < base`.
   - **Circularity Risk:** **Low**. The 15-bit FLREG formula is directly defined by Intel PCH hardware datasheets.

2. **`test_ifd_valid_image_parsing_and_gates`**
   - **Category:** Synthetic Fixture Test
   - **What It Validates:** Generates a synthetic 16MB image via `SyntheticFixtureBuilder::build_valid_ifd_image`, parses FLMAP0/1, extracts 5 regions (Descriptor, BIOS, ME, GbE, PlatformData), and asserts Gates 1, 2, and 3 pass.
   - **Circularity Risk:** **High**. The test validates that `IntelFlashDescriptor::parse` agrees with the offsets and layout generated by `SyntheticFixtureBuilder`. It does not prove that a dump from an ASPEED or Dell motherboard with non-standard component offsets will parse correctly.

3. **`test_ifd_corrupt_signature_rejection`**
   - **Category:** Negative / Malformed-Input Test
   - **What It Validates:** Overwrites offset `0x10` with `0xDEADBEEF` and asserts that `gate1_structural.passed == false` and `gate1_structural.has_valid_signature == false`.
   - **Circularity Risk:** **None**. Validates defensive rejection of invalid magic.

4. **`test_ifd_boundary_collision_detection`**
   - **Category:** Negative / Malformed-Input Test
   - **What It Validates:** Injects overlapping regions (Region 0 at `0x0000..0x1FFF` and Region 2 at `0x1000..0x3FFF`) and asserts `boundary_conflicts[0].overlap_bytes == 0x1000`.
   - **Circularity Risk:** **None**. Validates collision detection logic.

5. **`test_ifd_out_of_bounds_detection`**
   - **Category:** Negative / Malformed-Input Test
   - **What It Validates:** Sets a region limit to 16MB inside a 64KB image and asserts that `gate1_structural.issues` records an out-of-bounds error.
   - **Circularity Risk:** **None**. Validates image boundary enforcement.

6. **`test_synthetic_dell_pfs_extraction_end_to_end`**
   - **Category:** Synthetic Fixture Test
   - **What It Validates:** Constructs an in-memory Dell PFS container with a Name Info table and 3 payloads (System BIOS, Intel ME, EC), compresses with ZLIB, computes header XOR checksums and footer inverted CRC-32, extracts to a temporary directory, and verifies extracted file contents and hashes.
   - **Circularity Risk:** **High**. The fixture builder and parser share the same formula for entry headers, padding, and Name Info UTF-16LE decoding. If both builder and parser omit a real-world Dell flag or alternate revision, both pass cleanly.

7. **`test_dell_pfs_corrupt_xor_detection`**
   - **Category:** Negative / Malformed-Input Test
   - **What It Validates:** Inverts the 8-bit XOR checksum byte in the section header and asserts that `report.issues` records `"XOR checksum mismatch"`.
   - **Circularity Risk:** **None**. Validates defensive checksum rejection.

8. **`test_uefi_firmware_volume_header_parsing`**
   - **Category:** Synthetic Fixture Test
   - **What It Validates:** Builds a synthetic 128KB buffer with a valid `_FVH` signature, block map, and 16-bit additive inverse checksum, parses the header, and asserts fields match.
   - **Circularity Risk:** **High**. Tests only that the parser recognizes the synthetic header generated by the test harness. Does not test FFS parsing, corrupt block maps, or misaligned volumes.

9. **`test_real_world_dell_latitude_5x90_if_available`**
   - **Category:** Real-World Firmware Test (Conditional)
   - **What It Validates:** Inspects an authentic OEM installer (`Latitude_5X90_1.41.0.exe`), verifies section recognition, inflates ZLIB streams, validates inverted CRC-32, and extracts 14 component payloads.
   - **Circularity Risk:** **Zero**. Uses authentic vendor bytes compiled by Dell.
   - **Critical Test Flaw (DEF-01):** Uses an `if scratch_path.exists()` guard with an absolute path to a local scratch directory (`C:/Users/xtrar/.../scratch/Latitude_5X90_1.41.0.exe`). On any clean environment, CI server, or peer workstation, this test silently does nothing and passes without executing a single assertion.

---

#### Part B: Workspace & Persistence Tests (`src-tauri/src/tests/mod.rs`)

10. `test_fresh_database_initialization`: Asserts SQLite tables (`workspaces`, `firmware_files`, `operations`, `audit_logs`) are created on first launch.
11. `test_database_migrations_idempotency`: Asserts running migrations repeatedly does not fail or duplicate columns.
12. `test_persistent_workspace_recovery`: Asserts workspace records survive simulated application restart.
13. `test_sha256_correctness`: Validates SHA-256 calculation against known vector (`"Hello, world!"`).
14. `test_file_import_and_metadata_persistence`: Asserts importing a file stores hash, size, and metadata in SQLite.
15. `test_duplicate_import_after_application_restart`: Asserts re-importing the same hash is handled idempotently without duplicate records.
16. `test_source_deletion_or_relocation_detected`: Asserts engine detects when an imported source file has been moved or deleted from disk.
17. `test_source_modification_detected_after_import`: Asserts engine detects when an imported source file's bytes have been modified after ingestion.
18. `test_source_file_preservation`: Asserts source files are opened read-only and never modified in place.
19. `test_output_collision_and_atomic_overwrite_rejection`: Asserts engine rejects overwriting existing destination files unless explicitly requested, staging writes through `.part` files.
20. `test_unknown_file_identification_not_inferred_from_extension`: Asserts file format identification inspects binary magic bytes rather than trusting file extensions like `.bin` or `.rom`.

---

## 4. Analysis of the Circularity Problem

A central finding of this audit is that **synthetic tests alone cannot establish production correctness**.

```
                           The Synthetic Circularity Trap
┌─────────────────────────────────┐               ┌─────────────────────────────────┐
│     SyntheticFixtureBuilder     │               │          DellPfsParser          │
│                                 │               │                                 │
│  - Generates R1 entry (0x48)    │ ─────────────►│  - Expects R1 entry (0x48)      │
│  - Computes Inverted CRC-32     │  Byte Buffer  │  - Asserts Inverted CRC-32      │
│  - Writes UTF-16LE Name Table   │               │  - Decodes UTF-16LE Name Table  │
└─────────────────────────────────┘               └─────────────────────────────────┘
                 ▲                                                 │
                 └────────────── Shared Developer Assumptions ─────┘
                     (Both agree, but real-world OEM edge cases
                      such as R2 padding, multi-volume trailers,
                      or ThinOS LZMA wrappers fail undetected!)
```

### Concrete Consequences of Circularity
1. **Header Padding Variations:** In real Dell packages, older R1 entries sometimes contain 0x14 bytes of trailing metadata instead of 0x10. If the fixture builder only outputs 0x10, the test passes while the real parser would misalign on subsequent entries.
2. **Component Directory GUIDs:** The fixture builder only emits `GUID_NAME_INFO_1`. If an OEM package uses `GUID_NAME_INFO_2` or an unlisted directory GUID, synthetic tests will never catch the failure.
3. **ZLIB Compression Ratios:** Synthetic test payloads are short ASCII strings (`"FIXTURE_SYSTEM_BIOS_RAW_BYTES..."`), producing tiny ZLIB streams that cannot test chunk buffer exhaustion, stream resets, or decompression bomb protection under load.

---

## 5. Required Test Strategy Improvements

To transition FriedChip+ from "synthetic verification" to "commercial production verification", the following testing milestones are mandated for future sprints:

1. **Formalize Differential Testing:** Create automated tests that compare FriedChip+'s extraction output byte-for-byte against golden outputs generated by `dell_pfs_extract.py` and `bios_analyzer.py`.
2. **Eliminate Hardcoded Scratch Paths:** Relocate test firmware samples to a dedicated fixture registry (`tests/fixtures/`) and fail explicitly if an expected fixture is missing rather than skipping silently.
3. **Introduce Bounded Fuzzing:** Implement a `cargo-fuzz` target for `BinaryReader` and `DellPfsParser` that mutates container headers and feeds truncated ZLIB payloads to verify panic-freedom.
4. **Multi-Model Real-World Baseline:** Test at least three distinct manufacturer packages:
   - Dell Latitude 5490 (PFS R1/R2)
   - HP EliteBook 840 G5 (Insyde `$_IFLASH`)
   - Lenovo ThinkPad T480 (FL1/FL2)
