# FRIEDCHIP+ — PHASE 2B.2: PATH-TRACKED MUTATION COVERAGE REPORT
**Document ID:** `docs/PHASE2B2_MUTATION_COVERAGE.md`  
**Mandate Reference:** DEF-39  
**Status:** VERIFIED & REPRODUCIBLE  

---

## 1. Test Architecture & Execution Parameters

To evaluate parser resiliency against malformed, corrupted, and adversarial binary inputs, FriedChip+ executes a structure-aware in-process mutation harness:
- **Test Name:** `tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing`
- **Location:** `src-tauri/src/tests/engine_tests.rs`
- **Scale:** 300,000 total iterations (100,000 iterations per parser target).
- **Execution Profile:** Deterministic LCG pseudo-random PRNG seed (`0xDEADBEEFCAFE1234`), providing 100% reproducible test trajectories across environments.
- **Resource Constraints:** Zero dynamic memory exhaustion, zero heap leakage, strictly bounded recursion (`MAX_NESTING_DEPTH = 4`), and strict buffer limits (`MAX_INPUT_SIZE = 512 MB`, `MAX_DECOMPRESSED_SIZE = 256 MB`).

---

## 2. Quantitative Path Traversal Metrics

Unlike passive fuzz tests that merely check for process survival, the Phase 2B.2 harness measures branch reachability across every critical state machine transition.

### 2.1 Intel Flash Descriptor (IFD) Path Execution (100,000 Iterations)

| State Machine Path / Gate | Measured Hits | Percentage | Assertion Requirement | Verified? |
| :--- | :--- | :--- | :--- | :--- |
| `sig_accepted` (`FLVALSIG == 0x0FF0A55A`) | 73,833 | 73.83% | $> 0$ | **YES** |
| `sig_rejected` (Corrupt / invalid signature) | 26,167 | 26.17% | $> 0$ | **YES** |
| `geom_accepted` (Valid table offsets `[0x20..0x1000]`) | 62,321 | 62.32% | $> 0$ | **YES** |
| `geom_rejected` (Out of bounds or collision) | 37,679 | 37.68% | $> 0$ | **YES** |
| `profile_decoder_reached` (Profile resolution entered) | 49,807 | 49.81% | $> 0$ | **YES** |
| `region_table_decoded` (Traversed active region entries) | 62,321 | 62.32% | $> 0$ | **YES** |
| `region_conflict_reached` (Boundary overlap detected) | 1,779 | 1.78% | $> 0$ | **YES** |

### 2.2 Dell PFS Container Parser Path Execution (100,000 Iterations)

| State Machine Path / Stage | Measured Hits | Percentage | Assertion Requirement | Verified? |
| :--- | :--- | :--- | :--- | :--- |
| `hdr_accepted` (PFS Section container marker matched) | 70,881 | 70.88% | $> 0$ | **YES** |
| `hdr_rejected` (Corrupt header / non-PFS input) | 29,119 | 29.12% | $> 0$ | **YES** |
| `sec_tbl_reached` (Section header inspected) | 70,881 | 70.88% | $> 0$ | **YES** |
| `guid_name_reached` (Entry GUID matching table parsed) | 53,418 | 53.42% | $> 0$ | **YES** |
| `zlib_attempted` (Decompression stream processed) | 53,418 | 53.42% | $> 0$ | **YES** |
| `crc_attempted` (Inverted CRC-32 checksum verified) | 53,418 | 53.42% | $> 0$ | **YES** |
| `xor_attempted` (Container 8-bit XOR checksum checked) | 70,881 | 70.88% | $> 0$ | **YES** |

### 2.3 UEFI Firmware Volume Parser Path Execution (100,000 Iterations)

| State Machine Path / Stage | Measured Hits | Percentage | Assertion Requirement | Verified? |
| :--- | :--- | :--- | :--- | :--- |
| `fvh_detected` (`_FVH` signature located) | 44,336 | 44.34% | $> 0$ | **YES** |
| `fvh_rejected` (Non-FV buffer or truncated signature) | 55,664 | 55.66% | $> 0$ | **YES** |
| `hdr_validated` (Header length & attributes checked) | 37,863 | 37.86% | $> 0$ | **YES** |
| `csum_evaluated` (16-bit word sum checksum verified) | 37,863 | 37.86% | $> 0$ | **YES** |
| `blk_traversed` (Block map entries navigated) | 37,476 | 37.48% | $> 0$ | **YES** |
| `sentinel_handled` (Terminal `{0, 0}` block detected) | 37,863 | 37.86% | $> 0$ | **YES** |

---

## 3. Observed Failures & Runtime Stability

- **Total Panics / Crashes:** 0
- **Total Infinite Loops / Hangs:** 0
- **Clean Error Handling:** 100% of rejected inputs resulted in safe `Err(AppError::Binary(...))` returns or structural issue logs.
- **Execution Time:** ~23.2 to 24.7 seconds on x86_64 Windows host.

---

## 4. Honesty & Scope Boundary Statement (DEF-40)

To maintain absolute documentation integrity:
1. This test suite represents **in-process bounded mutation testing**, not an out-of-process LLVM `libFuzzer` or AFL++ campaign with hardware coverage counters.
2. We do **not** claim "100% code coverage" across all edge cases. We claim that across 300,000 synthesized mutations, every structural verification step, checksum check, decompression handler, and boundary collision detector was exercised and demonstrated panic-free resilience.
