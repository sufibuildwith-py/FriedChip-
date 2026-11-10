# FRIEDCHIP+ — PHASE 2B.2: IFD PROFILE CLASSIFICATION & SAFETY ARCHITECTURE
**Document ID:** `docs/PHASE2B2_PROFILE_CLASSIFICATION.md`  
**Mandate Reference:** DEF-34, DEF-35, DEF-36  
**Status:** IMPLEMENTED & VERIFIED  

---

## 1. Executive Summary

In previous Phase 2 iterations (2A.1, 2B, 2B.1), Intel Flash Descriptor (IFD) generation assignment attempted heuristic classification using region table geometry, ISL thresholding, and speculative FLMSTR bit-density patterns. While motivated by a desire to classify diverse descriptors, this conflated **structural boundary safety** with **generation-specific semantic decoding**.

Following upstream analysis of `coreboot` (`util/ifdtool/ifdtool.c` commit `66e704b1`) and `flashrom` (`ich_descriptors.c` commit `35c679c7`), Phase 2B.2 establishes the foundational principle:
> *"Unknown is a valid result. A conservative parser is better than a confident wrong parser."*

IFD classification now strictly decouples geometry validation from generation classification. Where binary evidence cannot unambiguously establish a unique profile, the engine defaults to `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.

---

## 2. Separation of Geometry from Classification (DEF-34)

### 2.1 The Architectural Distinction

| Concept | Role in FriedChip+ | Decoupled Responsibilities | Upstream Equivalent |
| :--- | :--- | :--- | :--- |
| **Descriptor Geometry** | Structural Boundary Validation | Computes the memory boundaries of the 4KB descriptor block, ensures no overlapping sections, ensures all offsets reside within `[0x20..0x1000]`, and limits region table traversal so reading cannot run into neighboring tables. | `coreboot: max_regions_from_fdbar()` (hardened) |
| **Generation Classification** | Semantic Register Interpretation | Determines whether registers (`FLCOMP`, `FLMSTR`, `FLREG`) are decoded according to Legacy ICH8..9-Series or Modern 100..700-Series rules. | `flashrom: ich_descriptors.c` & `coreboot -p <platform>` |

### 2.2 Rejection of Speculative Heuristics

Previous versions relied on heuristic scoring rules that led to false confidence:
1. **Region Count as Generation Proof:** Legacy chipsets can theoretically define up to 5 regions, while modern chipsets define 10 to 16. However, corrupt images, truncated descriptors, or proprietary firmware may declare arbitrary region numbers. Region count establishes upper boundary traversal limits, **never** hardware generation.
2. **ISL (Instruction Set Limit) Heuristics:** Guessing generation from whether `ISL >= 0x02` was undocumented in Intel specifications. Flashrom and coreboot never use ISL for platform classification. This heuristic has been completely removed.
3. **FLMSTR Bit-Density Guessing:** Attempting to determine generation by counting set bits in master access masks produced erroneous matches on custom OEM vendor locks. Removed in favor of strict register field validation.

### 2.3 Concrete Grounding Rules

In Phase 2B.2, automatic profile identification requires unambiguous, spec-backed indicators:

```rust
// 1. V2 Modern Classification (Skylake 100-series through Raptor Lake / Meteor Lake)
// Requires:
//   - FLMAP0 NR indicating modern region layout (NR in 15..16)
//   - Modern FLMSTR layout where upper bits represent modern extended master/region masks
//   - Flash frequency: 17 MHz read frequency (0b110) or modern operating profile
if is_valid_v2_nr && is_v2_flmstr && (is_17mhz_read || is_valid_v2_freq) {
    return ProfileResolution {
        profile: IfdProfile::V2Modern,
        confidence: ProfileConfidence::Confirmed,
        source: ProfileSource::AutoDetected,
        max_regions: max_regions.min(16),
        indicators,
    };
}

// 2. V1 Legacy Classification (ICH8 through 9-Series / Bay Trail)
// Requires:
//   - Valid legacy NR (<= 4 -> 1..5 regions)
//   - Legacy FLMSTR layout (Requester ID in lower 16 bits, Read mask [23:16], Write mask [31:24])
//   - Read frequency in legacy set (20, 33, 48, 50 MHz; bit pattern 0b110 reserved/invalid)
if is_valid_legacy_nr && is_legacy_freq && is_legacy_flmstr && !is_17mhz_read {
    return ProfileResolution {
        profile: IfdProfile::V1Legacy,
        confidence: ProfileConfidence::Confirmed,
        source: ProfileSource::AutoDetected,
        max_regions: std::cmp::min(max_regions, legacy_count),
        indicators,
    };
}

// 3. Ambiguous Evidence -> UNKNOWN Safe Fallback
return ProfileResolution {
    profile: IfdProfile::Unknown,
    confidence: ProfileConfidence::Unknown,
    source: ProfileSource::AutoDetected,
    max_regions: max_regions.min(5),
    indicators,
};
```

---

## 3. Quarantining Profile V1.5 (DEF-35)

### 3.1 Background & Root Cause
Profile V1.5 (Wellsburg / Intel C610 Series Enterprise Chipset) was provisionally introduced during Phase 2B. However:
- Neither the project test suite nor upstream open-source firmware repositories have an authentic, hardware-verified C610 IFD binary fixture.
- Upstream `coreboot` supports Wellsburg (`-p wbg`) strictly as an explicit user command-line parameter; it never automatically detects Wellsburg from descriptor bytes alone.
- Intel C610 documentation indicates proprietary vendor variations in `FLMSTR` bit positions.

### 3.2 Quarantine Policy
In accordance with DEF-35:
1. **Automatic Detection Disabled:** `resolve_descriptor_profile()` will never auto-assign `IfdProfile::V1_5Intermediate`.
2. **Explicit Override Confined:** If a user or caller explicitly requests `IfdProfile::V1_5Intermediate`, the engine marks confidence as `ProfileConfidence::Unknown`.
3. **Speculative Decoding Suppressed:** All semantic decoders (`decode_flcomp_with_profile`, `decode_master_rule_with_profile`, `decode_flreg_with_profile`) treat `V1_5Intermediate` identically to `Unknown`, producing `None` for all interpreted properties.
4. **Prerequisite for Activation:** V1.5 remains classified as `UNRESOLVED / QUARANTINED` until an authentic C610 SPI dump fixture with verified hardware provenance and golden reference vectors is acquired.

---

## 4. Complete Unknown-Profile Safety Model (DEF-36)

When `profile == IfdProfile::Unknown` or `IfdProfile::V1_5Intermediate`:

1. **Zero Speculative Semantic Interpretation:** The engine refuses to guess region addresses, enabled status, flash component capacity, clock frequencies, fast read support, or master read/write access permissions.
2. **Type-Safe Representation via `Option<T>`:**
   Rather than populating fields with `0`, `0.0`, or `false` (which mislead consumers into believing a region has size 0, or that a chip is 0 MB, or that writes are actively blocked), uninterpreted properties are typed as `None`:
   - `FlashRegion.base`: `Option<usize>` (`None`)
   - `FlashRegion.limit`: `Option<usize>` (`None`)
   - `FlashRegion.size`: `Option<usize>` (`None`)
   - `FlashRegion.enabled`: `Option<bool>` (`None`)
   - `FlashComponentDensity.density_bytes`: `Option<usize>` (`None`)
   - `FlashComponentDensity.density_mb`: `Option<f64>` (`None`)
   - `FlashComponentsInfo.fast_read_supported`: `Option<bool>` (`None`)
   - `FlashComponentsInfo.read_clock_frequency_mhz`: `Option<u32>` (`None`)
   - `FlashComponentsInfo.write_erase_clock_frequency_mhz`: `Option<u32>` (`None`)
   - `MasterAccessRule.requester_id`: `Option<u16>` (`None`)
   - `MasterAccessRule.read_mask`: `Option<u16>` (`None`)
   - `MasterAccessRule.write_mask`: `Option<u16>` (`None`)
   - `MasterAccessRule.bios_readable`: `Option<bool>` (`None`)
   - `MasterAccessRule.bios_writable`: `Option<bool>` (`None`)
   - `Gate3ProgrammerCompatibility.descriptor_inferred_flash_capacity_bytes`: `Option<usize>` (`None`)
   - `Gate3ProgrammerCompatibility.descriptor_inferred_flash_capacity_mb`: `Option<f64>` (`None`)
   - `Gate3ProgrammerCompatibility.host_descriptor_bios_write_allowed`: `Option<bool>` (`None`)
   - `Gate3ProgrammerCompatibility.host_descriptor_write_disabled`: `Option<bool>` (`None`)
3. **Preservation of Raw Hardware Evidence:**
   Raw register values are unconditionally preserved and made available to expert analysts:
   - `FlashRegion.flreg_raw: u32`
   - `FlashComponentsInfo.flcomp_raw: u32`
   - `MasterAccessRule.flmstr_raw: u32`
   - `DescriptorGeometry.flmap0_raw: u32`
   - `DescriptorGeometry.flmap1_raw: u32`
   - `DescriptorGeometry.flmap2_raw: u32`

---

## 5. Verification Matrix

| Test Identifier | Tested Condition | Expected Result | Status |
| :--- | :--- | :--- | :--- |
| `test_ifd_unknown_profile_safe_fallback_no_speculative_decoding` | Truncated/ambiguous descriptor parsed | Resolves to `Unknown`, all semantic `Option` fields are `None`, raw registers preserved | **PASS** |
| `test_ifd_conservative_profile_classification_rejects_ambiguous` | Contradictory descriptor (V2 NR with reserved 17MHz read frequency) | Rejects classification, safely classifies as `Unknown` | **PASS** |
| `test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding` | Attempted auto-detection and explicit override of V1.5 | Never auto-detected; explicit override produces `None` for all decoded properties | **PASS** |
| `test_ifd_valid_image_parsing_and_gates` | Authentic V1 image | Correctly decoded with `Confirmed` Legacy profile | **PASS** |
| `test_ifd_v2_valid_image_parsing_and_profile_detection` | Authentic V2 image | Correctly decoded with `Confirmed` Modern profile | **PASS** |
