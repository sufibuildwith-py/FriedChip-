# Intel Flash Descriptor Geometry & Extent Validation Specification (Phase 2B.3)

**Document:** `docs/PHASE2B3_GEOMETRY_VALIDATION.md`  
**Status:** Approved Engineering Specification  
**Scope:** Complete Section Extent Accounting, Interval Collision Validation & ICCRIBA Integration (DEF-42, DEF-43)  
**Reference Code:** [`src-tauri/src/engine/binary/ifd.rs`](file:///c:/Users/xtrar/Desktop/LBE%20Re-engineer/src-tauri/src/engine/binary/ifd.rs#L105-L350)

---

## 1. Executive Summary

In descriptor-based Intel SPI flash systems, the Flash Descriptor Header (0x0000..0x001F) contains Flash Map registers (`FLMAP0`, `FLMAP1`, `FLMAP2`) specifying base addresses and length limits for subsequent functional sections. Previous iterations suffered from two significant safety gaps:
1. **Omission of ICCRIBA**: FLMAP2 bits 23:16 (`ICCRIBA`, Integrated Clock Controller Register Initialization Base Address) were ignored during safe region table boundary derivation and interval checking.
2. **Incomplete Interval & Collision Modeling**: Sections were treated as point offsets rather than contiguous intervals `[start, start + len)`, allowing overlapping section tables to bypass structural validation. Furthermore, unchecked arithmetic could theoretically permit integer wrapping when determining boundaries.

This specification documents the complete, checked interval extent validation implemented in FriedChip+ Phase 2B.3 under **DEF-42** and **DEF-43**.

---

## 2. Register Layout & Base Address Definitions

All base addresses in the Intel Flash Descriptor map registers are stored as 8-bit block multipliers shifted left by 4 bits (`offset = raw_byte << 4`), giving 16-byte alignment within the 4096-byte (0x1000) descriptor block.

### 2.1 FLMAP0 (Offset 0x14)
| Bit Range | Field Name | Description | FriedChip+ Handling |
|:---|:---|:---|:---|
| 7:0 | `FCBA` | Flash Component Base Address (`fcba_raw << 4`) | Mandatory section. Extent: 8 bytes (`FLCOMP0`, `FLCOMP1`) |
| 9:8 | `NC` | Number of Components (0 = 1 flash chip, 1 = 2 flash chips) | Decoded count `NC + 1` |
| 15:10 | Reserved | Reserved per Intel SPI Programming Guides | Checked / Ignored based on generation |
| 23:16 | `FRBA` | Flash Region Base Address (`frba_raw << 4`) | Mandatory section. Start of Flash Region table (`FLREG0..FLREGn`) |
| 26:24 | `NR` | Number of Regions (legacy count: `NR + 1`) | Preserved as `nr_raw` |
| 31:27 | Reserved | Reserved | Must be strictly validated |

### 2.2 FLMAP1 (Offset 0x18)
| Bit Range | Field Name | Description | FriedChip+ Handling |
|:---|:---|:---|:---|
| 7:0 | `FMBA` | Flash Master Base Address (`fmba_raw << 4`) | Mandatory section. Extent: `NM * 4` bytes |
| 10:8 | `NM` | Number of Masters (0 = 1, 1 = 2, 2 = 3, 3 = 4) | Decoded count `NM + 1` |
| 15:11 | Reserved | Reserved | Strictly validated |
| 23:16 | `FPSBA` | Flash PCH Strap Base Address (`fpsba_raw << 4`) | Optional section (`fpsba_raw != 0`). Extent: `ISL * 4` bytes |
| 31:24 | `ISL` | ICC / PCH Strap Length | Multiplier for FPSBA extent |

### 2.3 FLMAP2 (Offset 0x1C)
| Bit Range | Field Name | Description | FriedChip+ Handling |
|:---|:---|:---|:---|
| 7:0 | `FMSBA` | Flash Master Strap Base Address (`fmsba_raw << 4`) | Optional section (`fmsba_raw != 0`). Extent: `MSL * 4` bytes |
| 15:8 | `MSL` | Master Strap Length | Multiplier for FMSBA extent |
| 23:16 | `ICCRIBA` | Integrated Clock Controller Register Init Base Address | Optional section (`iccriba_raw != 0`). Extent: `RIL * 4` bytes |
| 31:24 | `RIL` | Register Init Length | Multiplier for ICCRIBA extent |

---

## 3. Checked Arithmetic & Section Extent Model (DEF-43)

### 3.1 `SectionExtent` Abstraction
Each active section is modeled as a closed-open interval `[start, end)` where `end = start + len`.

```rust
struct SectionExtent {
    name: &'static str,
    start: usize,
    len: usize,
    end: usize,
    is_known_length: bool,
}
```

### 3.2 Overlap Condition
Two intervals $A = [start_A, end_A)$ and $B = [start_B, end_B)$ intersect if and only if:
$$\max(start_A, start_B) < \min(end_A, end_B)$$
Or equivalently in code:
```rust
let overlaps = start_a < end_b && start_b < end_a;
```

### 3.3 Strict Arithmetic Overflow and Containment Rules
1. **Checked Additions**: All `start + len` calculations use `checked_add()`. Any `None` triggers immediate structural rejection.
2. **Containment within Descriptor Block**: Every active section must satisfy:
   $$end \le 0x1000$$
   Any section extending past offset `0x1000` (4096 bytes) is invalid, as the flash descriptor block is strictly 4KB in size.
3. **Absence Semantics**: An optional section with `raw_offset == 0` is absent (`None`). Absent sections are never added to `active_sections` and cannot cause false collision rejections.

---

## 4. ICCRIBA Accounting in Region Table Boundary Derivation (DEF-42)

The region table starts at `FRBA` and extends downward toward the next functional section. The safe region limit must be derived by finding the earliest succeeding section following `FRBA`:

$$\text{next\_boundary} = \min \{ s \in \text{active\_offsets} \mid s > \text{FRBA} \} \cup \{ 0x0EFC, 0x1000 \}$$

Where `0x0EFC` is `FLUMAP1_OFFSET` (Upper Map 1 offset present on V2 modern descriptors).

### 4.1 Upstream Provenance & Verification
- **Flashrom (`ich_descriptors.h` & `ich_descriptors.c`)**: Explicitly decodes `iccriba` from `flmap2[23:16]`. Flashrom uses `iccriba` to identify PCH clock register configuration.
- **Coreboot (`ifdtool.c`)**: Validates descriptor bounds up to 0x1000 and warns if region tables overlap downstream descriptor structures.
- **FriedChip+ Implementation**: `iccriba` is stored in `DescriptorGeometry { iccriba: Option<usize>, iccriba_raw: u8, ril: u8, ... }`. When present (`iccriba_raw > 0`), it participates both in interval collision checks and in the calculation of `max_regions_extent`.

---

## 5. Adversarial Test Matrix

The following edge cases are verified in `src-tauri/src/tests/engine_tests.rs`:

| Test Name | Condition Tested | Expected Outcome | Result |
|:---|:---|:---|:---|
| `test_ifd_geometry_adversarial_collision_checks` (Case 1) | FCBA (0x40..0x80) overlaps FMBA (0x60..0x80) | Structural validation fails (`is_valid = false`, collision error recorded) | PASS |
| `test_ifd_geometry_adversarial_collision_checks` (Case 2) | FPSBA (0x80) overlaps FRBA (0x80) | Structural validation fails (collision error recorded) | PASS |
| `test_ifd_geometry_adversarial_collision_checks` (Case 3) | FRBA=0x40, default 16 regions (0x40..0x80) crosses FMBA at 0x60 | Safe boundary validation fails (`boundary conflict` recorded) | PASS |
| `test_ifd_geometry_adversarial_collision_checks` (Case 4) | FMBA at 0xFF0 with len 0x20 reaches 0x1010 | Structural validation fails (`extends beyond 0x1000`) | PASS |
| `test_ifd_geometry_adversarial_collision_checks` (Case 5) | Abutting sections: 0x20..0x40, 0x40..0x80, 0x80..0xA0 | Valid geometry passes with zero errors | PASS |
| `test_ifd_flmap2_iccriba_accounting_and_safe_boundaries` (Case 1) | ICCRIBA before FRBA (ICCRIBA=0x80, FRBA=0xC0) | Valid geometry; ICCRIBA does not constrain FRBA | PASS |
| `test_ifd_flmap2_iccriba_accounting_and_safe_boundaries` (Case 2) | ICCRIBA after FRBA (FRBA=0x40, ICCRIBA=0x80) | Safe region limit strictly bounded to $(0x80 - 0x40) / 4 = 16$ | PASS |
| `test_ifd_flmap2_iccriba_accounting_and_safe_boundaries` (Case 3) | ICCRIBA adjacent to FRBA table end (FRBA=0x40..0x80, ICCRIBA=0x80) | Valid geometry passes without collision | PASS |
| `test_ifd_flmap2_iccriba_accounting_and_safe_boundaries` (Case 4) | ICCRIBA absent (`flmap2[23:16] == 0`) | `geom.iccriba == None`, no collision, no false constraint | PASS |
