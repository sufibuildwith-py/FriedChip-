# Parser-Path Mutation Coverage & Test-Only Observer Specification (Phase 2B.3)

**Document:** `docs/PHASE2B3_MUTATION_COVERAGE.md`  
**Status:** Completed Engineering Specification & Validation Report  
**Scope:** Direct Parser-Path Mutation Instrumentation & Boundary Safety Verification (DEF-44)  
**Reference Code:**
- [`src-tauri/src/engine/testing/observer.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/testing/observer.rs)
- [`src-tauri/src/engine/binary/ifd.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/binary/ifd.rs)
- [`src-tauri/src/engine/vendor/dell_pfs.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/vendor/dell_pfs.rs)
- [`src-tauri/src/engine/uefi/firmware_volume.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/uefi/firmware_volume.rs)
- [`src-tauri/src/tests/engine_tests.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/tests/engine_tests.rs#L535-L895)

---

## 1. Problem Statement & Audit Findings (DEF-44)

In previous Phase 2B iterations, the structure-aware mutation campaign (`test_in_process_bounded_parser_stress_fuzzing`) claimed coverage over critical parsing branches, but independent review identified that several counters were tautological or inferred outside the parser:
1. **Tautological CRC counter**: Dell PFS counted CRC attempts with `if s.payload_inverted_crc32_valid || !s.payload_inverted_crc32_valid`, which is mathematically identically true for any boolean value.
2. **External / Inferred Signature Counting**: IFD and UEFI FV signature detection was evaluated by inspecting buffer slices in the test harness rather than tracking whether the parser's internal decoding path actually accepted or rejected the signature.
3. **Missing Sentinel Traversal Check**: UEFI Firmware Volume block map traversal previously tolerated or synthesized terminator blocks without verifying that the mandatory `{NumBlocks = 0, Length = 0}` sentinel was encountered in the byte stream.

---

## 2. Observer Architecture (`engine/testing/observer.rs`)

To eliminate tautological counters while guaranteeing **zero production runtime overhead**, FriedChip+ Phase 2B.3 implements a dedicated thread-local test observer module guarded by `#[cfg(test)]`.

### 2.1 Thread-Local Counters
All execution metrics are tracked using thread-local `Cell<usize>` structures, ensuring:
- Safe, synchronization-free atomic updates per thread.
- Complete isolation between parallel test runners.
- Zero runtime overhead or memory allocations in release binaries.

```rust
thread_local! {
    static OBSERVER_ENABLED: Cell<bool> = Cell::new(false);
    static IFD_SIG_ACCEPTED: Cell<usize> = Cell::new(0);
    static IFD_SIG_REJECTED: Cell<usize> = Cell::new(0);
    static IFD_GEOM_ACCEPTED: Cell<usize> = Cell::new(0);
    static IFD_GEOM_REJECTED: Cell<usize> = Cell::new(0);
    static IFD_PROFILE_DECODER_REACHED: Cell<usize> = Cell::new(0);
    static IFD_REGION_TABLE_DECODED: Cell<usize> = Cell::new(0);
    static IFD_REGION_CONFLICT_REACHED: Cell<usize> = Cell::new(0);

    static PFS_CONTAINER_ACCEPTED: Cell<usize> = Cell::new(0);
    static PFS_CONTAINER_REJECTED: Cell<usize> = Cell::new(0);
    static PFS_SECTION_TABLE_REACHED: Cell<usize> = Cell::new(0);
    static PFS_GUID_NAME_REACHED: Cell<usize> = Cell::new(0);
    static PFS_ZLIB_ATTEMPTED: Cell<usize> = Cell::new(0);
    static PFS_ZLIB_SUCCEEDED: Cell<usize> = Cell::new(0);
    static PFS_ZLIB_REJECTED: Cell<usize> = Cell::new(0);
    static PFS_CRC_VERIFIED_MATCH: Cell<usize> = Cell::new(0);
    static PFS_CRC_VERIFIED_MISMATCH: Cell<usize> = Cell::new(0);
    static PFS_XOR_VERIFIED_MATCH: Cell<usize> = Cell::new(0);
    static PFS_XOR_VERIFIED_MISMATCH: Cell<usize> = Cell::new(0);

    static FV_FVH_DETECTED: Cell<usize> = Cell::new(0);
    static FV_FVH_REJECTED: Cell<usize> = Cell::new(0);
    static FV_HEADER_VALIDATED: Cell<usize> = Cell::new(0);
    static FV_CHECKSUM_EVALUATED: Cell<usize> = Cell::new(0);
    static FV_BLOCK_MAP_TRAVERSED: Cell<usize> = Cell::new(0);
    static FV_SENTINEL_FOUND: Cell<usize> = Cell::new(0);
    static FV_SENTINEL_MISSING: Cell<usize> = Cell::new(0);
}
```

---

## 3. Instrumentation Execution Sites

### 3.1 Intel Flash Descriptor (`IntelFlashDescriptor::parse`)
- `record_ifd_sig(valid)`: Triggered directly at `sig == IFD_SIGNATURE_MAGIC` check.
- `record_ifd_geom(valid)`: Triggered immediately following `validate_descriptor_geometry`.
- `record_ifd_profile_decoder()`: Triggered when invoking `detect_ifd_profile_resolution`.
- `record_ifd_region_table()`: Triggered upon populating the region table.
- `record_ifd_region_conflict()`: Triggered inside `validate_region_boundaries` when `boundary_conflicts` is non-empty.

### 3.2 Dell PFS Extraction (`DellPfsParser::extract`)
- `record_pfs_container(valid)`: Triggered at `is_dell_pfs()` entry check.
- `record_pfs_section_table()`: Triggered upon discovering active section headers.
- `record_pfs_guid_name()`: Triggered upon successfully extracting and mapping a component GUID to friendly name.
- `record_pfs_zlib_attempt()`, `record_pfs_zlib_result(success)`: Triggered at `ZlibDecoder::read_to_end()` execution site, cleanly distinguishing between successful decompression and corruption rejections.
- `record_pfs_crc(match)`: Triggered when evaluating `actual_crc == expected_crc`, cleanly separating matching checksums from corruption mismatches.
- `record_pfs_xor(match)`: Triggered when evaluating section header and footer XOR checksums.

### 3.3 UEFI Firmware Volumes (`FirmwareVolume::parse_at`)
- `record_fv_fvh(valid)`: Triggered at `hdr.signature == *b"_FVH"`.
- `record_fv_header_validated()`: Triggered when `header_length` and block counts are valid.
- `record_fv_checksum_evaluated()`: Triggered when verifying 16-bit summation checksum.
- `record_fv_block_map_traversed()`: Triggered during block map loop.
- `record_fv_sentinel(found)`: Triggered at the terminating block map check:
  - `true` when `{0, 0}` is found before header boundary.
  - `false` when block map reaches end of header without finding `{0, 0}`, returning `Err(AppError::Binary("UEFI Firmware Volume block map missing terminating {0, 0} sentinel"))`.

---

## 4. 300,000-Iteration Mutation Campaign Results

The bounded structure-aware stress test runs 100,000 iterations across each of the three primary Phase 2 firmware formats (total 300,000 iterations).

### 4.1 Execution Metrics Summary
| Parser | Metric / Path | Observed Count | Assertion Constraint | Status |
|:---|:---|:---|:---|:---|
| **IFD** | Signature Accepted | > 0 | `snap.ifd_sig_accepted > 0` | PASS |
| **IFD** | Signature Rejected | > 0 | `snap.ifd_sig_rejected > 0` | PASS |
| **IFD** | Geometry Accepted | > 0 | `snap.ifd_geom_accepted > 0` | PASS |
| **IFD** | Geometry Rejected | > 0 | `snap.ifd_geom_rejected > 0` | PASS |
| **IFD** | Profile Decoder Reached | > 0 | `snap.ifd_profile_decoder_reached > 0` | PASS |
| **IFD** | Region Table Decoded | > 0 | `snap.ifd_region_table_decoded > 0` | PASS |
| **IFD** | Region Conflict Reached | > 0 | `snap.ifd_region_conflict_reached > 0` | PASS |
| **Dell PFS** | Container Accepted | > 0 | `snap.pfs_container_accepted > 0` | PASS |
| **Dell PFS** | Container Rejected | > 0 | `snap.pfs_container_rejected > 0` | PASS |
| **Dell PFS** | Section Table Reached | > 0 | `snap.pfs_section_table_reached > 0` | PASS |
| **Dell PFS** | GUID Name Reached | > 0 | `snap.pfs_guid_name_reached > 0` | PASS |
| **Dell PFS** | ZLIB Attempted | > 0 | `snap.pfs_zlib_attempted > 0` | PASS |
| **Dell PFS** | ZLIB Succeeded | > 0 | `snap.pfs_zlib_succeeded > 0` | PASS |
| **Dell PFS** | ZLIB Rejected | > 0 | `snap.pfs_zlib_rejected > 0` | PASS |
| **Dell PFS** | CRC Verified Match | > 0 | `snap.pfs_crc_verified_match > 0` | PASS |
| **Dell PFS** | CRC Verified Mismatch | > 0 | `snap.pfs_crc_verified_mismatch > 0` | PASS |
| **Dell PFS** | XOR Verified Match | > 0 | `snap.pfs_xor_verified_match > 0` | PASS |
| **Dell PFS** | XOR Verified Mismatch | > 0 | `snap.pfs_xor_verified_mismatch > 0` | PASS |
| **UEFI FV** | `_FVH` Detected | > 0 | `snap.fv_fvh_detected > 0` | PASS |
| **UEFI FV** | `_FVH` Rejected | > 0 | `snap.fv_fvh_rejected > 0` | PASS |
| **UEFI FV** | Header Validated | > 0 | `snap.fv_header_validated > 0` | PASS |
| **UEFI FV** | Checksum Evaluated | > 0 | `snap.fv_checksum_evaluated > 0` | PASS |
| **UEFI FV** | Block Map Traversed | > 0 | `snap.fv_block_map_traversed > 0` | PASS |
| **UEFI FV** | Sentinel Found | > 0 | `snap.fv_sentinel_found > 0` | PASS |
| **UEFI FV** | Sentinel Missing | > 0 | `snap.fv_sentinel_missing > 0` | PASS |

### 4.2 Robustness Criteria
- **Total Iterations**: 300,000 (100,000 per engine format).
- **Crashes / Unhandled Panics**: 0.
- **Hangs / Infinite Loops**: 0.
- **Checked Rejections**: 100% of malformed inputs produced clean `Err` returns or invalid structural gate flags without panics.
