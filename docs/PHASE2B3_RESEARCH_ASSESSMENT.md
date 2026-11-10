# FRIEDCHIP+ — PHASE 2B.3: RESEARCH ASSESSMENT & UPSTREAM ALIGNMENT
**Document ID:** `docs/PHASE2B3_RESEARCH_ASSESSMENT.md`  
**Mandate Reference:** Phase 2B.3 Engineering Protocol  
**Starting Commit:** `ee02f14c0188eafe13e632d654d109cba7e02c2f`  
**Status:** IMPLEMENTATION GATE APPROVED  

---

## 1. Problem Statement

Phase 2B.2 implemented conservative IFD profile assignment and basic geometry checks. However, independent technical review identified remaining gaps across four engineering domains:
1. **Profile Classification Flaws (DEF-41):** `FLCOMP` read-frequency value `6` (17 MHz) was treated as a standalone modern V2 indicator, despite `flashrom` documentation establishing that `freq_read` is repurposed on 300-series (Cannon Lake) and newer platforms. Furthermore, FLMSTR inference did not prevent older legacy layouts (e.g. Host CPU Requester ID `0x0000` with Write BIOS bit set) from accidentally satisfying modern layout patterns when `NR == 0`.
2. **Missing FLMAP2 Accounting (DEF-42):** `FLMAP2` bits 23:16 define `ICCRIBA` (Integrated Clock Controller Register Init Base Address, introduced with Cougar Point 6-Series). The geometry validator omitted `ICCRIBA`, ignoring it during collision detection and upper boundary calculations for the region table.
3. **Incomplete Section Extents & Overlap Checks (DEF-43):** Collisions were checked primarily as identical base address equality (`fcba == frba`), rather than checking intervals `[start .. start + length)` across all pairs of active sections (`FCBA`, `FRBA`, `FMBA`, `FPSBA`, `FMSBA`, `ICCRIBA`, and `FLUMAP1_OFFSET`). Saturating arithmetic was used instead of checked arithmetic.
4. **Tautological Mutation Instrumentation (DEF-44):** The mutation harness contained inferred path counters, notably `payload_inverted_crc32_valid || !payload_inverted_crc32_valid` (tautology), and counted FV parsing as proof of sentinel detection without verifying that `{0, 0}` was actually encountered.
5. **Override Confidence Semantics (DEF-46):** User-selected profile overrides returned `ProfileConfidence::Confirmed`, falsely asserting independent binary evidence.

---

## 2. Upstream References & Source Inspection

### 2.1 Upstream Repositories Inspected

1. **`flashrom`**:
   - **Repository Path:** `research/third_party/flashrom/`
   - **Commit SHA:** `35c679c7ecd1db67649b950ee0b0b4f74a540ef1`
   - **Inspected Files:**
     - `ich_descriptors.c` (lines 338–345, 1030–1165):
       - Line 1143: Explicitly documents that for 300-series Cannon Point, 400-series Comet Point, 500-series Tiger Point, 600-series Alder Point, 700-series Raptor Point, and Meteor Lake:
         `/* freq_read was repurposed, so can't check on it any more. */`
       - Line 1149: Establishes that read frequency `6` (17 MHz) is valid for Skylake/Sunrise Point (100-series), Lewisburg (C620), and Apollo Lake.
       - Lines 1042–1085: Establishes `content->ICCRIBA == 0x00` indicates pre-Sandy Bridge (ICH8..Ibex Peak), while `ICCRIBA > 0x00` indicates Cougar Point (6-series) or newer.
     - `include/ich_descriptors.h` (lines 57–96, 188–241):
       - Lines 85–86: Defines `FLMAP2` structure:
         `uint32_t FMSBA:8, MSL:8, ICCRIBA:8, RIL:8;`
       - Lines 192–205: Defines pre-Skylake FLMSTR layout (Requester ID in `[15:0]`, Read mask in `[20:16]`, Write mask in `[28:24]`, with bits `[23:21]` and `[31:29]` strictly reserved `0`).
       - Lines 233–240: Defines post-Skylake modern FLMSTR layout (`ext_read:4`, `ext_write:4`, `read:12`, `write:12`).

2. **`coreboot` (`util/ifdtool`)**:
   - **Repository Path:** `research/third_party/coreboot/`
   - **Commit SHA:** `66e704b166fdf236f82c157a7bca21a2574304e8`
   - **Inspected Files:**
     - `util/ifdtool/ifdtool.c` (lines 23–26, 76–104, 230–298, 300–355, 450–475):
       - Line 25: Defines `FLUMAP1_OFFSET` as `(IFD_SIZE - 256) - 4` = `0x0EFC` (3836 bytes).
       - Lines 306–334: `is_platform_ifd_2()`: Coreboot never auto-detects IFD v2 from descriptor content alone without explicit platform flags (`-p <platform>`).
       - Lines 452–474: `max_regions_from_fdbar()`: Computes region boundaries by sorting active table base addresses against `FLUMAP1_OFFSET`.

3. **`EDK2` (`BaseTools`)**:
   - **Repository Path:** `research/third_party/edk2/`
   - **Inspected Files:**
     - `BaseTools/Source/C/Common/FirmwareVolumeBuffer.c` (lines 463–480):
       - Enforces that block map traversal must terminate strictly upon encountering `Length == 0 && NumBlocks == 0`.
       - If header bytes end before encountering this sentinel, the firmware volume is corrupted (`EFI_VOLUME_CORRUPTED`).

---

## 3. Reuse Decisions & Engineering Architecture

| Component / Issue | Upstream Concept | FriedChip+ Reuse Decision | Rationale |
| :--- | :--- | :--- | :--- |
| **Profile Classification (DEF-41)** | `flashrom` frequency repurposing & `coreboot` platform conservatism | **Adapt & Harden:** Reject frequency `6` as standalone V2 proof. Require multiple converging indicators (e.g. modern FLMSTR read permissions at bits 8 & 9 + valid modern range). Require legacy FLMSTR read permissions at bits 16 & 17 + reserved bit clearance. When ambiguous, return `Unknown`. | Prevents legacy descriptors with Requester ID `0x0000` from false-positive modern classification. |
| **ICCRIBA Section Accounting (DEF-42)** | `flashrom` `ich_descriptors.h` & `ich_descriptors.c` | **Adopt:** Decode `ICCRIBA = ((flmap2 >> 16) & 0xFF) << 4`. If raw `== 0`, mark absent. If raw `> 0`, include in geometry validation and boundary derivation. | Prevents silent overrun into ICC records section when calculating safe region extent. |
| **Complete Interval Collision Validation (DEF-43)** | `coreboot` `regions_collide()` concept extended to all IFD sections | **Independent Implementation:** Check every active section pair `(A, B)` for interval overlap: `start_a < end_b && start_b < end_a`. Use checked arithmetic (`checked_add`, `checked_sub`). Enforce descriptor containment `< 0x1000`. | Eliminates naive equality-only checks and prevents saturating arithmetic masking malformed geometry. |
| **Mutation Parser Instrumentation (DEF-44)** | Test observer pattern | **Independent Implementation:** Test-only observer hooks directly at calculation/branch sites in `DellPfsParser`, `FirmwareVolume`, and `IntelFlashDescriptor`. | Replaces tautological expressions (`valid || !valid`) with genuine execution site metrics without runtime production overhead. |
| **Override Confidence Semantics (DEF-46)** | Explicit provenance modeling | **Independent Implementation:** Add `ProfileConfidence::UserSelected`. Ensure user overrides preserve `UserOverride` source and never claim `Confirmed`. | Cleanly separates user assumptions from binary proof. |

---

## 4. License & Provenance Attributions

- `flashrom`: GNU General Public License v2 (GPLv2). Conceptual logic for `ICCRIBA` field definition and frequency repurposing is adapted as clean-room, independent Rust code under MIT license.
- `coreboot`: GNU General Public License v2 (GPLv2). `FLUMAP1_OFFSET` boundary concept is independently integrated.
- `edk2`: BSD-2-Clause-Patent. Sentinel validation rules aligned with UEFI Platform Initialization specification.
