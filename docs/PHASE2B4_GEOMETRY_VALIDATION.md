# FriedChip+ — Phase 2B.4 Geometry Validation Assessment (DEF-49)

**Document Reference:** `docs/PHASE2B4_GEOMETRY_VALIDATION.md`  
**Status:** IMPLEMENTED AND TESTED  
**Scope:** Intel Flash Descriptor Geometry Validation, Extent Representation, and Boundary Collision Checking  
**Upstream Provenance:**
- `flashrom`: commit `35c679c7827dd6ec34005e83ec5be870bfb43494`, `include/ich_descriptors.h` lines 120-145 (`struct ich_desc_component`).
- `coreboot`: commit `66e704b123bb0d1737e42d7088b9ddfa6d27464d`, `util/ifdtool/ifdtool.c` lines 300-355.

---

## 1. Engineering Problem

Prior to Phase 2B.4, geometry validation suffered from several correctness defects:
1. **Speculative Extent Fallbacks:** When optional section length fields were zero (`ISL=0`, `MSL=0`, `RIL=0`), the validator automatically synthesized a 4-byte extent fallback (`if len > 0 { len * 4 } else { 4 }`). This masked corrupted, truncated, or incomplete descriptors by inventing geometry not present in the hardware descriptor.
2. **Conflation of Physical Geometry with Profile-Specific Semantics:** Geometry validation calculated `declared_regions = (nr_raw + 1)` and rejected descriptors if this exceeded physical available space *before* the descriptor generation profile was identified. On modern V2 platforms, `NR` is deprecated/reused and does not represent region count.
3. **Unverified Extents Receiving Safe Geometry:** Descriptors with unresolvable or missing length fields were marked `is_valid = true` if no other sections collided.

---

## 2. Technical Remediation

### 2.1 Reassessment of FCBA Extent
In upstream `flashrom` (`include/ich_descriptors.h` lines 120-145):
```c
struct ich_desc_component {
    union { uint32_t FLCOMP; ... };
    union { uint32_t FLILL;  ... };
    union { uint32_t FLPB;   ... };
};
```
The component block structure contains exactly three 32-bit registers (FLCOMP at 0x00, FLILL at 0x04, FLPB at 0x08), totaling exactly 12 bytes (`sizeof(struct ich_desc_component) == 12`). Regardless of whether 1 or 2 components are populated (`NC=0` or `NC=1`), the register header footprint in the flash descriptor is 12 bytes.

### 2.2 Explicit Extent Representation (`ExtentLength`)
FriedChip+ now models section lengths explicitly:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExtentLength {
    Exact(usize),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionExtent {
    pub name: String,
    pub start: usize,
    pub length: ExtentLength,
    pub end: Option<usize>,
}
```

When an active section specifies length 0 (`ISL=0`, `MSL=0`, or unverified `FLMAP2` upper bits), it is assigned `ExtentLength::Unknown` with `end: None`. Automatic 4-byte fallbacks have been removed.

### 2.3 Unresolved Extent Invariant
Any unresolved section extent marks `has_unresolved_extents = true`. Under DEF-49:
```rust
let is_valid = errors.is_empty() && !has_unresolved_extents && max_regions_extent > 0 && frba < image_size;
```
A descriptor containing unresolvable section extents cannot receive an unconditional safe-geometry result (`is_valid = false`).

### 2.4 Separation of Physical Space from Declared Regions
1. **Geometry Stage (Pre-Profile):**
   - Calculates physical gap: `available_bytes = next_boundary.saturating_sub(frba)`.
   - Derives maximum physical region slots: `max_regions_extent = (available_bytes / 4).min(16)`.
   - Does not evaluate `NR` register semantics.
2. **Profile Resolution Stage:**
   - Detects profile (`V1Legacy`, `V2Modern`, or `Unknown`).
3. **Post-Profile Boundary Collision Stage:**
   - For `V1Legacy`: checks declared legacy region table `((nr_raw + 1).min(5) * 4)` against `next_boundary`.
   - For `V2Modern`: checks modern region table `(max_regions.min(16) * 4)` against `next_boundary`.
   - If declared extent exceeds physical boundary, records a structural violation.

---

## 3. Adversarial Validation Suite

The test suite in `src-tauri/src/tests/engine_tests.rs` tests:
- `test_ifd_geometry_known_vs_unknown_extents_def49`:
  - Zero-length fields (`ISL=0`, `MSL=0`) produce `ExtentLength::Unknown`, `has_unresolved_extents == true`, and `is_valid == false`.
  - Non-overlapping adjacent sections (e.g. FCBA 0x20..0x2C, FRBA 0x30..0x34, FMBA 0x40..0x48) pass without collision.
  - Partial section overlap and identical base address collisions are reliably detected.
- `test_ifd_geometry_adversarial_collision_checks`:
  - Verifies that physical geometry calculates 4 available regions while post-profile validation catches legacy NR=7 extending into FMBA at 0x50.
