# Phase 2B.2 — Research Assessment & Technical Evidence Dossier

**Document ID:** `RES-DOSS-2B2-001`  
**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE — MANDATORY RESEARCH DELIVERABLE  
**Scope:** Research-First Verification of IFD Profiles, Geometry Bounding & Upstream Tool Analysis (DEF-34, DEF-35, DEF-36, DEF-37, DEF-38)  

---

## 1. Upstream Research Repositories Inspected

In compliance with the **Mandatory Research-First Engineering Protocol**, direct source code inspection was conducted across the cloned third-party reference repositories in `research/third_party/`:

| Upstream Project | Cloned Repository Path | Commit SHA | License | Relevant Source Files Inspected |
|---|---|---|---|---|
| **coreboot** | `research/third_party/coreboot` | `66e704b166fdf236f82c157a7bca21a2574304e8` | GPL-2.0-only | `util/ifdtool/ifdtool.c` (lines 48–377, 452–474, 1090–1130, 2627–2664) |
| **flashrom** | `research/third_party/flashrom` | `35c679c7ecd1db67649b950ee0b0b4f74a540ef1` | GPL-2.0-or-later | `ich_descriptors.c` (lines 1030–1225), `include/ich_descriptors.h` (lines 20–241) |

---

## 2. Technical Findings from Source Inspection

### 2.1 coreboot `ifdtool` Geometry & Classification Architecture
Inspection of `util/ifdtool/ifdtool.c` revealed critical architectural truths:

1. **`max_regions_from_fdbar` (lines 452–474):**
   ```c
   static int max_regions_from_fdbar(const struct fdbar *fdb)
   {
       const size_t fcba = (fdb->flmap0 & 0xff) << 4;
       const size_t fmba = (fdb->flmap1 & 0xff) << 4;
       const size_t frba = ((fdb->flmap0 >> 16) & 0xff) << 4;
       const size_t fpsba = ((fdb->flmap1 >> 16) & 0xff) << 4;
       const size_t flumap = FLUMAP1_OFFSET;
       size_t sorted[5] = {fcba, fmba, frba, fpsba, flumap};

       qsort(sorted, ARRAY_SIZE(sorted), sizeof(size_t), sort_compare);

       for (size_t i = 0; i < 4; i++) {
           if (sorted[i] == frba)
               return MIN((sorted[i + 1] - sorted[i]) / 4, MAX_REGIONS);
       }
       return 0;
   }
   ```
   **Limitation Identified:** The coreboot helper is fundamentally a *best-effort geometric space calculation*, NOT a validation function. It blindly assumes:
   - All section offsets are non-zero (if `fpsba == 0`, sorting places `0` before `fcba`).
   - Section offsets do not overlap or collide.
   - All offsets are within the valid 4KB descriptor window (`0x20..0x0EFC`).
   - Sorting arbitrary offsets does not make an invalid geometry valid.
   **FriedChip+ Remediation (DEF-37):** Geometry must be validated (boundary checks, alignment, non-zero required sections, collision detection) *before* calculating maximum region extent.

2. **IFD Version & Platform Assignment (lines 320–355, 2627–2664):**
   - Coreboot does NOT automatically determine IFD v1 vs IFD v2 from binary geometry.
   - Coreboot requires an explicit CLI parameter: `-p <platform>` (e.g. `-p sklkbl`, `-p cnl`, `-p ifd2`, `-p wbg`).
   - If `-p` is omitted, `platform` defaults to `-1`, and `check_ifd_version` defaults unconditionally to `IFD_VERSION_1`.
   - `IFD_VERSION_1_5` is **exclusively** assigned when `platform == PLATFORM_WBG` (`CHIPSET_8_SERIES_WELLSBURG`). It is never assigned to mobile or consumer desktop chipsets.

### 2.2 flashrom Descriptor Detection & Generation Indicators
Inspection of `ich_descriptors.c` and `ich_descriptors.h` in flashrom confirmed:

1. **Skylake 100-Series Distinction via 17 MHz Read Frequency (lines 1118–1165):**
   ```c
   /*
    * As an additional measure, we check the read frequency like `ifdtool`.
    * The frequency value 6 (17MHz) was reserved before Skylake and is the
    * only valid value since. Skylake is currently the most important dis-
    * tinction because of the dropped number of regions field (NR).
    */
   ```
   Authoritative evidence establishes:
   - In `FLCOMP` bits 19:17 (`freq_read`), value `6` (`110b`, 17 MHz) was **strictly reserved** on all pre-Skylake PCHs (ICH8 through 9-Series Wildcat Point).
   - Intel introduced 17 MHz as the default descriptor read clock in Skylake 100-Series PCH.
   - Therefore, `freq_read == 6` is a high-confidence positive marker for modern descriptors.

2. **Dropped Number of Regions (`NR`) Field:**
   - In pre-Skylake descriptors, `FLMAP0[26:24]` is an architected 3-bit field representing $\text{Regions} - 1$ ($0 \le \text{NR} \le 4$, meaning 1 to 5 regions).
   - In Skylake 100-Series and newer, Intel designated bits 24 onwards as **Reserved**. The field reads 0.

3. **Master Permission Shifts (`FLMSTR1`):**
   - **Legacy (pre-Skylake):** Bits 15:0 = Requester ID. Bits 23:16 = Read permissions (8 regions). Bits 31:24 = Write permissions (8 regions).
   - **Modern (Skylake+):** Bits 7:0 = Reserved (or extended permissions on server/300+). Bits 19:8 = Read permissions (12 regions). Bits 31:20 = Write permissions (12 regions).

---

## 3. Authoritative Intel Specification Cross-References

| Specification Document | Document Number | Key Sections Verified |
|---|---|---|
| **Intel 100 Series and C230 Series Chipset Family PCH Datasheet Vol 1 & 2** | `332690`, `332691` | Section 28 (SPI Flash Controller): FLMAP0 bit 24+ reserved, FLCOMP clock frequencies, FLMSTR 12-bit masks |
| **Intel 200 Series and Z370 Chipset Series PCH Datasheet Vol 1 & 2** | `335195`, `335196` | Section 27: SPI memory organization, Flash Descriptor format, dropped NR verification |
| **Intel 300 Series and C240 Series Chipset Family PCH Datasheet Vol 1 & 2** | `337347`, `337348` | Section 28: Master permissions, MDTBA (Master Descriptor Table Base Address) upper map |
| **Intel BIOS Writer's Guide (BWG) — SPI Flash Architecture** | Proprietary OEM | Separation of descriptor permissions (FLMSTR1) from runtime flashing locks (BIOS_CNTL, SMM_BWP, PR0–PR4) |

---

## 4. Engineering Decisions & Implementation Directives

1. **Conservative Profile Classification (DEF-34):**
   - Separate geometry validation from generation classification.
   - Do NOT use region count or geometry to infer generation.
   - Eliminate undocumented ISL strap length heuristics and ad-hoc bitmask thresholds.
   - Return `IfdProfile::Unknown` whenever binary indicators do not converge on a single unique specification.
2. **Quarantine V1.5 (DEF-35):**
   - `IfdProfile::V1_5Intermediate` is confined to documentation as `UNRESOLVED / UNSUPPORTED`.
   - Prevent automatic detection from ever returning V1.5.
   - Mark V1.5 decoding as quarantined until an authentic hardware dump and verified vectors are acquired.
3. **Complete Unknown-Profile Safety (DEF-36):**
   - For `IfdProfile::Unknown`, withhold speculative interpretation of region addresses, sizes, enabled flags, densities, clocks, fast-read, and master permissions.
   - Model all profile-dependent decoded fields as typed `Option<T>` (e.g. `Option<usize>`, `Option<bool>`, `Option<u32>`) rather than zero-filled primitives.
   - Preserve raw 32-bit registers (`flreg_raw`, `flcomp_raw`, `flmstr_raw`) for transparent forensics.
4. **Pre-Traversal Geometry Validation (DEF-37):**
   - Implement `validate_descriptor_geometry`: verify 16-byte alignment, 4KB boundary limits, non-zero required sections (`fcba`, `frba`, `fmba`), absent optional section filtering, and collision/overlap detection.
   - Never assume sorting arbitrary offsets creates valid geometry.
5. **Parser Error Handling (DEF-38):**
   - Remove `.unwrap_or(0)` fallbacks (e.g. offset `0x1C`). Convert binary read errors to explicit `AppError`.
6. **Execution-Path Tracked Mutation Testing (DEF-39):**
   - Instrument in-process stress harness to track exact execution path reachability across IFD, Dell PFS, and UEFI FV.
