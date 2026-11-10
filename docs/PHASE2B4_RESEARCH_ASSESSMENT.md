# Phase 2B.4 Mandatory Research Assessment & Upstream Comparative Matrix

**Project:** FriedChip+  
**Document:** `docs/PHASE2B4_RESEARCH_ASSESSMENT.md`  
**Scope:** Research Assessment for Phase 2B.4 Platform Semantics, FLMAP2 Interpretation & Geometry Safety  
**Date:** November 10, 2026  
**Implementation Policy:** Mandatory Research-First Protocol (`AGENTS.md`)

---

## 1. Engineering Problem Statement

Independent review of Phase 2B.3 revealed remaining platform semantic and geometry safety deficiencies:
1. **Universal FLMAP2 ICCRIBA Fallacy (DEF-47)**: Previous implementation interpreted all `FLMAP2[23:16]` fields as `ICCRIBA` (Integrated Clock Controller Register Initialization Base Address). However, Intel chipset architectures from Tiger Point (500-series PCH) onwards completely redefined `FLMAP2`, repurposing bits `[11:2]` as `CSSO` (CPU Soft Strap Offset) and bits `[23:16]` as `CSSL` (CPU Soft Strap Length). Interpreting `CSSL` as `ICCRIBA` causes false base address calculations and erroneous safe region boundary calculations on modern platforms.
2. **Unsupported Generation Inferences (DEF-48)**: Heuristics such as `geometry.iccriba_raw > 0x30` or `max_regions > 5` were used as proof of a modern (IFD v2) generation. Geometry (available physical space) does not prove platform generation.
3. **Speculative Section Extents & Zero-Length Fallback (DEF-49)**: Fixed 12-byte extent assumptions for FCBA and automatic 4-byte fallbacks for zero length fields (`ISL=0`, `MSL=0`, `RIL=0`) masked unresolved section boundaries.
4. **Premature NR Interpretation (DEF-49)**: `FLMAP0[26:24]` (`NR`) was interpreted as a legacy region count during pre-traversal geometry validation, before the descriptor generation profile had been resolved.

---

## 2. Upstream Reference Source Code Inspection

### 2.1 flashrom (`research/third_party/flashrom`)
- **Pinned Commit**: `35c679c7827dd6ec34005e83ec5be870bfb43494`
- **File**: `include/ich_descriptors.h` (lines 80–96):
  ```c
  union { /* 0x0c */
      uint32_t FLMAP2;
      struct {
          uint32_t FMSBA   :8, /* Flash (G)MCH Strap Base Addr. */
                   MSL     :8, /* MCH Strap Length */
                   ICCRIBA :8, /* ICC Reg. Init Base Addr. (new since Sandy Bridge) */
                   RIL     :8; /* Register Init Length (new since Haswell) */
      };
      struct { /* new since Tiger Point */
          uint32_t         :2,
                   CSSO    :10, /* CPU Soft Strap Offset from PMC Base */
                           :4,
                   CSSL    :8,  /* CPU Soft Strap Length */
                           :8;
      };
  };
  ```
- **File**: `ich_descriptors.c` (lines 1050–1165):
  - Flashrom separates pre-Tiger Point and post-Tiger Point using upper map markers and explicit heuristics.
  - When upper map has `MDTBA != 0x00`:
    - If `ICCRIBA == 0x34`: Cannon Point (300-series).
    - Else: Interpreted using the Tiger Point soft-strap structure (`CSSL` and `CSSO`).
  - Flashrom explicitly notes:
    - `freq_read` was repurposed on 300-series Cannon Point and later.
    - `NR` was dropped / reserved from Skylake on.
  - Flashrom warns: `"Unknown flash descriptor, assuming 500 series compatibility"`.

### 2.2 coreboot (`research/third_party/coreboot`)
- **Pinned Commit**: `66e704b123bb0d1737e42d7088b9ddfa6d27464d`
- **File**: `util/ifdtool/ifdtool.c` (lines 300–355, 450–475):
  ```c
  /*
   * Some newer platforms have re-defined the FCBA field that was used to
   * distinguish IFD v1 v/s v2. Define a list of platforms that we know do not
   * have the required FCBA field, but are IFD v2 and return true if current
   * platform is one of them.
   */
  static int is_platform_ifd_2(void) { ... }
  ```
  - **Core Finding**: Coreboot *does not* attempt heuristic binary-only automatic generation classification for modern platforms. It requires an explicit command-line flag `-p <platform>` (e.g. `-p sklkbl`, `-p cnl`, `-p tgl`, `-p adl`), and explicitly prints:
    `"Warning: No platform specified. Output may be incomplete"`.
  - Coreboot's design demonstrates that firmware descriptors alone do not carry universal self-identifying generation tags across all Intel PCH eras. Automatic classification must be strictly conservative, returning `Unknown` unless primary register layout proof exists.

---

## 3. Comparative Matrix & Reuse Decisions

| Component / Subsystem | Upstream Implementation (`flashrom` / `coreboot`) | FriedChip+ Existing Implementation | Phase 2B.4 Decision | Rationale |
|:---|:---|:---|:---|:---|
| **FLMAP2 Interpretation** | Union of `(FMSBA, MSL, ICCRIBA, RIL)` vs `(CSSO, CSSL)` | Hardcoded universal `ICCRIBA` and `RIL` | **Adopt Upstream Union Model (DEF-47)** | Introduce explicit `Flmap2Interpretation` enum (`IccribaRil`, `CpuSoftStrap`, `LegacyMchStrap`, `Unknown`). |
| **Region Boundary Derivation** | Uses explicit platform selection or upper map markers | Uses `iccriba` unconditionally in `succeeding_offsets` | **Condition on Verified Layout (DEF-47)** | Only use `ICCRIBA` when `IccribaRil` interpretation is proven. Never use unverified FLMAP2 in boundary calculations. |
| **Profile Classification** | Requires explicit platform `-p` (coreboot) or multi-point heuristic (flashrom) | Relied on `iccriba_raw > 0x30` or `max_regions > 5` | **Remove Unsupported Heuristics (DEF-48)** | Strip geometry heuristics. Return `Unknown` if binary evidence is ambiguous. Preserve `UserSelected`. |
| **FCBA Extent** | 12 bytes (`FLCOMP`, `FLILL`, `FLPB`) minimum in `ich_desc_component` | Fixed 12 bytes with no distinction of unknown/variable extent | **Explicit Known vs Unknown Model (DEF-49)** | Standard component registers (12 bytes) are validated; variable/unknown extents modeled explicitly. |
| **Section Length Fallback** | Does not synthesize lengths | Replaced `len == 0` with `4` | **Remove Fallback (DEF-49)** | If `isl=0`, `msl=0`, or `ril=0`, extent is `ExtentLength::Unknown`. Do not certify unconditional safe geometry. |
| **Legacy NR Scope** | Bounded to legacy IFD v1 | Evaluated `nr_raw` in `validate_descriptor_geometry` pre-profile | **Post-Profile NR Validation (DEF-49)** | Physical space calculation is separated from declared regions. NR evaluated only when profile is resolved. |

---

## 4. Upstream License & Attribution

- **flashrom**: GPL-2.0-only. Clean-room conceptual reference and register bit-offset mapping; no code copied directly.
- **coreboot**: GPL-2.0-only. Clean-room reference for platform constraints and command-line architecture.
- **Intel Datasheets**: Public technical documentation (100-series PCH, 500-series PCH specifications).
