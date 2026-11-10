# FriedChip+ — Phase 2B.4 Closure Recommendation

**Document Reference:** `docs/PHASE2B4_CLOSURE_RECOMMENDATION.md`  
**Assignment:** Phase 2B.4 Corrective Engineering Release  
**Status:** COMPLETE — SUBMITTED FOR INDEPENDENT AUDIT  
**Scope Compliance:** Phase 2 firmware engine only. Phase 3 remains strictly locked. No new firmware formats introduced.

---

## 1. Summary of Defect Resolutions

### DEF-47: Platform-Dependent FLMAP2 Interpretation
- Audited `flashrom` `ich_descriptors.h` (lines 80-96) and `ich_descriptors.c` (commit `35c679c7`).
- Implemented strongly-typed `Flmap2Interpretation` (`IccribaRil`, `CpuSoftStrap`, `LegacyMchStrap`, `Unknown`).
- Preserved all raw FLMAP2 fields (`fmsba_raw`, `msl_raw`, `iccriba_raw`, `ril_raw`, `csso_raw`, `cssl_raw`).
- Resolved Tiger Point semantic conflict: when `flmap2_interpretation == CpuSoftStrap`, neither ICCRIBA nor FMSBA is populated from the union, preventing spurious region-table collisions and fictitious bounds.

### DEF-48: Removal of Unsupported Generation Inference
- Removed `geometry.iccriba_raw > 0x30` and `max_regions > 5` as generation heuristics.
- Separated physical available space from generation classification.
- In accordance with `coreboot` `ifdtool` (commit `66e704b1`), which mandates `-p <platform>`, when binary indicators lack converging independent evidence, profile classification safely returns `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
- Maintained explicit user overrides as `ProfileSource::UserOverride` with `ProfileConfidence::UserSelected`.

### DEF-49: Geometry Validation Distinguishing Known and Unknown Extents
- Reassessed fixed 12-byte FCBA extent against upstream `struct ich_desc_component` (FLCOMP, FLILL, FLPB).
- Replaced automatic 4-byte fallbacks for zero-length fields (`ISL=0`, `MSL=0`, `RIL=0`) with explicit `ExtentLength::Unknown`.
- Descriptors containing unresolvable section extents cannot receive an unconditional safe-geometry result (`has_unresolved_extents = true`, `is_valid = false`).
- Moved legacy `NR` region-table boundary collision checking out of geometry validation into post-profile validation (`parse_with_profile`).
- Verified with adversarial test vectors covering zero lengths, adjacent sections, partial overlap, and complete containment.

### DEF-50: Observer Metric Semantics & Targeted Instrumentation
- Defined instruction-site semantics for every observer counter.
- Separated `is_dell_pfs` marker detection (`pfs_is_dell_pfs_matched`/`unmatched`) from `extract` container validation (`pfs_extract_accepted`/`rejected`).
- Enhanced UEFI FV signature rejection to account for invalid header positions (`< 0x28`) and candidate header validation errors.
- Separated region-table decoding entry (`ifd_region_table_entry`) from completed decoding (`ifd_region_table_completed`).
- Verified all observer paths individually in `test_observer_targeted_path_fixtures_def50`.
- Reported 300,000 iterations in `test_in_process_bounded_parser_stress_fuzzing` as deterministic structure-aware mutation stress testing, not code coverage.

### DEF-51: Test Inventory & Validation Reporting Accuracy
- Reconciled full test suite: 57 test functions (55 passed, 2 ignored, 0 failed).
- Documented that both `test_dell_pfs_differential_regression_against_biosutilities` and `test_real_world_dell_latitude_5x90_if_available` are ignored when proprietary OEM binaries are absent (Tier 4 tests).
- Confirmed exact fixture SHA-256 (`c5bb883ca09bd8eb38c64eb3d047ec6cb66847deef94e101f3e79e6f315a6e87`) and upstream commit SHAs (`flashrom` `35c679c7`, `coreboot` `66e704b1`, `BIOSUtilities` `a90cb541`).
- Preserved documented test commands with zero claims of unconfigured CI workflows.

---

## 2. Deliverables Index

1. `docs/PHASE2B4_RESEARCH_ASSESSMENT.md` — Upstream source inspection, Tiger Point FLMAP2 layout analysis, and coreboot platform selection audit.
2. `docs/PHASE2B4_PLATFORM_SEMANTICS.md` — FLMAP2 interpretation state machine, Tiger Point layout verification, and evidence-backed profile classification.
3. `docs/PHASE2B4_GEOMETRY_VALIDATION.md` — Section extent modeling, zero-length handling, and separation of physical geometry from profile semantics.
4. `docs/PHASE2B4_MUTATION_EVIDENCE.md` — Observer counter definitions and 300,000-iteration structure-aware mutation stress testing execution counts.
5. `docs/PHASE2B4_VALIDATION_RESULTS.md` — Full 57-test inventory reconciliation, Tier 1-4 breakdown, fixture hashes, and command outputs.
6. `docs/PHASE2B4_CLOSURE_RECOMMENDATION.md` — This engineering release summary.

---

## 3. Engineering Status Recommendation

All Phase 2B.4 mandates have been implemented, tested, and documented strictly within Phase 2 scope. Phase 2B is not declared closed; this release is submitted for independent audit.

READY FOR INDEPENDENT REVIEW
