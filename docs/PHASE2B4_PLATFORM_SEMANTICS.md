# FriedChip+ — Phase 2B.4 Platform Semantics Assessment (DEF-47, DEF-48)

**Document Reference:** `docs/PHASE2B4_PLATFORM_SEMANTICS.md`  
**Status:** IMPLEMENTED AND TESTED  
**Scope:** Intel Flash Descriptor (IFD) Platform Semantics, FLMAP2 Interpretation, and Generation Classification  
**Upstream Provenance:**
- `flashrom`: commit `35c679c7827dd6ec34005e83ec5be870bfb43494`, `include/ich_descriptors.h` lines 80-96, `ich_descriptors.c` lines 1050-1165.
- `coreboot`: commit `66e704b123bb0d1737e42d7088b9ddfa6d27464d`, `util/ifdtool/ifdtool.c` lines 300-355, 450-475.

---

## 1. Executive Summary

Previous iterations of the Intel Flash Descriptor parser treated `FLMAP2[23:16]` universally as `ICCRIBA` (Integrated Clock Controller Register Initialization Base Address) and utilized physical available region space (`max_regions > 5`) or raw register values (`iccriba_raw > 0x30`) as heuristic proofs of modern (IFD v2.0 / Skylake+) platform generation.

Following independent review in Phase 2B.3, these practices were determined to violate Intel specifications and authoritative upstream implementations:
1. **Tiger Point (500-Series PCH) Semantic Inversion:** Intel repurposed `FLMAP2` on Tiger Point and newer platforms to describe CPU Soft Straps (`CSSO` and `CSSL`). Treating `FLMAP2[23:16]` as ICCRIBA on these platforms created fictitious ICCRIBA sections at arbitrary offsets, falsely restricting safe region-table extents and triggering spurious collisions.
2. **Unsupported Generation Inference:** In `coreboot`'s `ifdtool`, automatic generation classification is rejected; `ifdtool` mandates explicit platform selection via `-p <platform>` because descriptor headers alone cannot reliably prove chipset generation. Heuristic inference based on physical available space or raw register bytes is speculative.

Phase 2B.4 remedies both defects by introducing an explicit platform interpretation state machine (`Flmap2Interpretation`), ensuring unverified FLMAP2 interpretations never restrict region boundaries, and requiring converging independent evidence before classifying descriptor profiles.

---

## 2. DEF-47: Platform-Dependent FLMAP2 Interpretation

### 2.1 Upstream Source Inspection (`flashrom`)

In upstream `flashrom` (`include/ich_descriptors.h` lines 80-96):
```c
struct ich_desc_flmap2 {
    union {
        struct {
            uint32_t FMSBA : 8,
                     MSL : 8,
                     ICCRIBA : 8,
                     RIL : 8;
        };
        struct {
            uint32_t reserved1 : 2,
                     CSSO : 10,
                     reserved2 : 4,
                     CSSL : 8,
                     reserved3 : 8;
        } tiger;
    };
};
```

### 2.2 Semantic Field Comparison

| Bit Range | Legacy ICH8..10 / Ibex Peak | Cougar Point..Skylake (PCH 6-series..100/300) | Tiger Point (500-series PCH) & Newer |
|:---|:---|:---|:---|
| **[1:0]** | Reserved (0) | `FMSBA[1:0]` (Base Address bits) | **Reserved (Strictly 0)** |
| **[7:0]** | `FMSBA` (MCH Strap Base) | `FMSBA` (Master Strap Base) | `CSSO[5:0]` + Reserved[1:0] |
| **[11:2]** | `FMSBA[7:2]` + `MSL[3:0]` | `FMSBA[7:2]` + `MSL[3:0]` | **`CSSO` (CPU Soft Strap Offset, 10 bits)** |
| **[15:12]** | `MSL[7:4]` | `MSL[7:4]` | **Reserved (Strictly 0)** |
| **[15:8]** | `MSL` (MCH Strap Length) | `MSL` (Master Strap Length) | Split between `CSSO` and Reserved |
| **[23:16]** | Reserved (0) | **`ICCRIBA` (ICC Base Address)** | **`CSSL` (CPU Soft Strap Length, 8 bits)** |
| **[31:24]** | Reserved (0) | **`RIL` (Record Initial Length)** | **Reserved (Strictly 0)** |

### 2.3 Explicit Layout State Machine (`Flmap2Interpretation`)

FriedChip+ models FLMAP2 interpretation with a strongly-typed enum:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Flmap2Interpretation {
    /// Integrated Clock Controller Record Initialization (PCH 6-series through Skylake/Cannon Point era).
    /// FLMAP2[23:16] is ICCRIBA, FLMAP2[31:24] is RIL.
    IccribaRil,
    /// Tiger Point (500-series PCH) and later CPU Soft Straps.
    /// FLMAP2[11:2] is CSSO (10 bits), FLMAP2[23:16] is CSSL (8 bits).
    /// Reserved bits [1:0], [15:12], and [31:24] must be zero.
    CpuSoftStrap,
    /// Legacy ICH8-10 / Ibex Peak where bits [31:16] are reserved 0.
    LegacyMchStrap,
    /// Ambiguous, conflicting, or unknown layout.
    Unknown,
}
```

### 2.4 Resolution Algorithm and Boundary Safety

1. **Upper Half Check:** If `FLMAP2[31:16] == 0`, classified as `LegacyMchStrap`.
2. **Tiger Point Constraints:**
   - `reserved1 (bits 1:0) == 0`
   - `CSSO (bits 11:2) != 0`
   - `reserved2 (bits 15:12) == 0`
   - `CSSL (bits 23:16) != 0`
   - `reserved3 / RIL (bits 31:24) == 0`
   If all constraints match and `RIL == 0`, classified as `CpuSoftStrap`.
3. **ICCRIBA / RIL Constraints:**
   - `iccriba_raw (bits 23:16) != 0`
   - `ril_raw (bits 31:24) != 0`
   If both non-zero and within descriptor bounds, classified as `IccribaRil`.
4. **Boundary Safety Invariant:**
   - When `flmap2_interpretation != Flmap2Interpretation::IccribaRil`, `geometry.iccriba` is explicitly set to `None`.
   - On Tiger Point systems, `CSSL` (e.g. 0x04) is NOT shifted into an address (`0x40`) and does NOT collide with FRBA or restrict region table calculation.
   - On ambiguous systems, `has_unresolved_extents` is set to `true`, invalidating unconditional safe-geometry claims.

---

## 3. DEF-48: Removal of Unsupported Generation Inference

### 3.1 Prohibited Heuristics

The following rules have been permanently excised from `detect_ifd_profile_resolution`:
1. **`geometry.iccriba_raw > 0x30`:** On Tiger Point, `FLMAP2[23:16]` is `CSSL` (strap length), which can easily exceed 0x30 without implying Skylake generation. Furthermore, pre-Skylake PCHs (e.g., Cougar Point / Lynx Point) also feature ICCRIBA at variable offsets.
2. **`max_regions > 5`:** Physical available space between FRBA and the next succeeding section boundary is an artifact of descriptor padding (e.g., placing FMBA at 0x200 leaves 448 bytes = 112 region slots available). A legacy ICH8 descriptor with `NR=3` (4 regions) whose FMBA is placed at 0x80 has 64 bytes = 16 available region slots; this physical available space does not make it a modern platform.

### 3.2 Evidence-Backed Modern Profile Detection

Per `coreboot` `ifdtool` reference, automated profile detection without explicit caller specification (`-p`) requires strict converging evidence:
1. **Modern Master Permission Layout:**
   - Host CPU read mask at bits [15:0] (Descriptor Read at bit 8, BIOS Read at bit 9).
   - Must NOT match legacy layout (where bit 16 is Descriptor Read, bit 17 is BIOS Read, and bits [15:0] is Requester ID 0x0000).
2. **Converging Platform Evidence:**
   - `FLCOMP` read clock set to `17 MHz` (read frequency value 6, specific to Skylake 100-series PCH), OR
   - Verified Tiger Point CPU Soft Strap layout (`Flmap2Interpretation::CpuSoftStrap`).
3. **Default to Unknown:**
   - If modern FLMSTR markers are present without a converging platform indicator, or if legacy and modern markers conflict, profile resolution returns `IfdProfile::Unknown` with `ProfileConfidence::Unknown`.
4. **Explicit Caller Overrides:**
   - When a caller explicitly specifies an override (`IfdProfile::V1Legacy`, `IfdProfile::V2Modern`), confidence is recorded as `ProfileConfidence::UserSelected`, never `Confirmed`.
   - Quarantined profiles (`IfdProfile::V1_5Intermediate`) remain quarantined with `ProfileConfidence::Unknown`.
