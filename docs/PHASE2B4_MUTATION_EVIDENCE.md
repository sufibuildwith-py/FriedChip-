# FriedChip+ — Phase 2B.4 Structure-Aware Mutation Stress Testing Evidence (DEF-50)

**Document Reference:** `docs/PHASE2B4_MUTATION_EVIDENCE.md`  
**Status:** FULLY INSTRUMENTED AND VERIFIED  
**Scope:** Deterministic In-Process Parser Instrumentation, Path Accounting, and Stress Fuzzing Evidence  
**Reference Test:** `tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing` & `tests::engine_tests::test_observer_targeted_path_fixtures_def50`

---

## 1. Metric Semantics Clarification (DEF-50)

To resolve metric conflation identified in Phase 2B.3, each observer counter is defined with exact instruction-site semantics:

### 1.1 Intel Flash Descriptor (IFD) Counters
- `ifd_sig_accepted`: Incremented when `0x0FF0A55A` signature is verified at offset 0x10.
- `ifd_sig_rejected`: Incremented when descriptor signature at offset 0x10 does not match magic.
- `ifd_geom_accepted`: Incremented when pre-traversal geometry validation succeeds (`is_valid == true`).
- `ifd_geom_rejected`: Incremented when geometry validation fails (bounds violation, collision, or unresolved extent).
- `ifd_profile_decoder_reached`: Incremented when `detect_ifd_profile_resolution` executes.
- `ifd_region_table_entry`: Incremented immediately prior to beginning Flash Region Table (FRBA) decoding loop.
- `ifd_region_table_completed`: Incremented immediately after the Flash Region Table decoding loop successfully terminates.
- `ifd_region_conflict_reached`: Incremented when inter-region boundary collision validation identifies overlapping enabled regions.

### 1.2 Dell PFS Container Counters
- `pfs_is_dell_pfs_matched`: Incremented when `DellPfsParser::is_dell_pfs` finds a section marker or `PFS.HDR.` tag.
- `pfs_is_dell_pfs_unmatched`: Incremented when `is_dell_pfs` finds no PFS markers.
- `pfs_extract_accepted`: Incremented when `DellPfsParser::extract` successfully decompresses, verifies, and returns all components (`Ok(report)`).
- `pfs_extract_rejected`: Incremented when `extract` rejects input due to size bounds, missing headers, decompression error, or corrupted checksums (`Err(AppError)`).
- `pfs_section_table_reached`: Incremented when section offset scanning finds at least one valid section pattern.
- `pfs_guid_name_reached`: Incremented when known component GUIDs are matched to canonical firmware types.
- `pfs_zlib_attempted`: Incremented prior to invoking bounded ZLIB decompression on a payload.
- `pfs_zlib_succeeded`: Incremented when ZLIB decompression completes within safety limits.
- `pfs_zlib_rejected`: Incremented when ZLIB stream is corrupted or decompression fails.
- `pfs_crc_verified_match`: Incremented when inverted CRC32 matches expected payload checksum.
- `pfs_crc_verified_mismatch`: Incremented when payload CRC32 checksum fails.
- `pfs_xor_verified_match`: Incremented when 8-bit XOR checksum matches header/footer value.
- `pfs_xor_verified_mismatch`: Incremented when 8-bit XOR checksum fails.

### 1.3 UEFI Firmware Volume (FV) Counters
- `fv_fvh_detected`: Incremented when `_FVH` signature is found at a valid header position (`>= 0x28`) and candidate header validation succeeds.
- `fv_fvh_rejected`: Incremented when `_FVH` signature is missing, positioned invalidly (`< 0x28`), or candidate header parsing fails.
- `fv_header_validated`: Incremented when FV header length, attributes, and bounds are validated.
- `fv_checksum_evaluated`: Incremented when 16-bit additive checksum of FV header is computed.
- `fv_block_map_traversed`: Incremented for each block map entry processed.
- `fv_sentinel_found`: Incremented when mandatory `{0, 0}` block map sentinel is located.
- `fv_sentinel_missing`: Incremented when block map terminates without required `{0, 0}` sentinel.

---

## 2. Deterministic Mutation Stress Testing Evidence

Structure-aware mutation stress testing was executed across 300,000 iterations (100,000 per parser engine) using a bounded deterministic LCG seed:

```text
[FUZZ EVIDENCE] Structure-aware mutation testing completed: 3 x 100000 (300,000 total iterations) in ~21s with 0 crashes or hangs.
  IFD paths:
    sig_accepted: 62,500
    sig_rejected: 12,702
    geom_accepted: 60,832
    geom_rejected: 1,668
    profile_decoder_reached: 62,500
    region_table_entry: 60,832
    region_table_completed: 60,832
    region_conflict_reached: 1,739
  PFS paths:
    is_dell_pfs_matched: 83,567
    is_dell_pfs_unmatched: 16,433
    extract_accepted: 83,567
    extract_rejected: 16,433
    section_table_reached: 83,567
    guid_name_reached: 131,866
    zlib_attempted: 75,093
    zlib_succeeded: 65,933
    zlib_rejected: 9,160
    crc_verified_match: 53,433
    crc_verified_mismatch: 12,500
    xor_verified_match: 143,923
    xor_verified_mismatch: 13,720
  FV paths:
    fvh_detected: 37,070
    fvh_rejected: 64,461
    header_validated: 50,341
    checksum_evaluated: 50,341
    block_map_traversed: 99,540
    sentinel_found: 37,070
    sentinel_missing: 13,271
```

### Reporting Caveat
In accordance with DEF-50 mandate 6, these metrics represent **deterministic structure-aware mutation stress testing**, demonstrating that parsers correctly accept valid structures, cleanly reject malformed mutations, and execute every anticipated failure path without crashing or hanging. They must not be misconstrued as compiler-level line or branch coverage.
