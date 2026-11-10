# FRIEDCHIP+ — PHASE 2B.2: DESCRIPTOR GEOMETRY VALIDATION ARCHITECTURE
**Document ID:** `docs/PHASE2B2_GEOMETRY_VALIDATION.md`  
**Mandate Reference:** DEF-37, DEF-38  
**Status:** IMPLEMENTED & VERIFIED  

---

## 1. Problem Definition & Upstream Analysis

In previous iterations, the Intel Flash Descriptor parser extracted section offsets directly from `FLMAP0`, `FLMAP1`, and `FLMAP2` without rigorous boundary and alignment validation. If a malformed or malicious firmware image specified an invalid table offset (e.g. pointing into the header `< 0x20`, extending past the 4096-byte descriptor block, or overlapping adjacent tables), the parser risked:
- Overlapping memory reads.
- Silent fallback on truncated images.
- Unbounded region table traversal into adjacent data structures.

### Upstream Comparison

1. **`coreboot/util/ifdtool/ifdtool.c` (`max_regions_from_fdbar`)**:
   `ifdtool` determines the maximum number of regions by sorting table base addresses (`frba`, `fcba`, `fmba`, `fpsba`, `fmsba`) and finding the lowest base address strictly greater than `frba`.
   *Upstream Gaps Identified:*
   - Does not filter out zero-value offsets (e.g. absent `fpsba == 0` or `fmsba == 0`), which can cause `frba` comparisons to behave unexpectedly if offsets wrap.
   - Does not validate that offsets reside within the 4KB boundary (`0x1000`).
   - Does not verify that base addresses do not overlap the descriptor header (`0x00..0x20`).

2. **`flashrom/ich_descriptors.c`**:
   Validates signatures and component density, but assumes structurally valid descriptors once the signature passes.

---

## 2. Hardened Geometry Validation Design (DEF-37)

FriedChip+ Phase 2B.2 implements `validate_descriptor_geometry()` in `src-tauri/src/engine/binary/ifd.rs`, establishing rigorous structural validation prior to reading any downstream table:

```rust
pub fn validate_descriptor_geometry(reader: &BinaryReader) -> Result<DescriptorGeometry, AppError>
```

### 2.1 Boundary & Alignment Rules

1. **Descriptor Block Containment (`[0x20..0x1000]`):**
   The Intel Flash Descriptor resides entirely within the first 4096 bytes (`0x1000`) of the SPI flash. The first 32 bytes (`0x00..0x20`) are reserved for the Flash Descriptor Header (`FLVALSIG` and `FLMAP0..2`).
   Therefore, all base addresses must satisfy:
   $$0x0020 \le \text{offset} < 0x1000$$

2. **16-Byte Alignment:**
   Intel specifications define base addresses in 16-byte blocks (`<< 4`). Any decoded offset is inherently 16-byte aligned.

3. **Mandatory vs. Optional Section Validation:**
   - **Mandatory Sections:** Component Base (`FCBA`), Region Base (`FRBA`), and Master Base (`FMBA`) are required for any operational descriptor. If any mandatory offset is zero, `< 0x20`, or `>= 0x1000`, the descriptor is rejected.
   - **Optional Sections:** Processor Straps (`FPSBA`), PCH Straps (`FMSBA`), and ICC Records (`ICCRIBA`) may be absent on certain platforms. An offset of `0x00` is explicitly treated as **absent** and excluded from collision and extent calculations.

4. **Collision & Overlap Detection:**
   Every mandatory section must occupy a distinct, non-overlapping 16-byte boundary. If `FCBA == FRBA`, `FCBA == FMBA`, or `FRBA == FMBA`, geometry validation immediately flags a structural collision and halts table decoding.

5. **Checked Safe Extent Calculation:**
   The maximum safe region table extent is computed using checked arithmetic:
   ```rust
   let mut next_boundary = 0x1000usize; // End of 4KB descriptor block
   for &sec_offset in &active_sections {
       if sec_offset > frba && sec_offset < next_boundary {
           next_boundary = sec_offset;
       }
   }
   let max_safe_region_bytes = next_boundary.checked_sub(frba).unwrap_or(0);
   let max_regions_from_geom = max_safe_region_bytes / 4;
   ```
   This guarantees that reading up to `max_regions` entries can never overrun into neighboring tables or outside the 4KB block.

6. **Raw Register Preservation:**
   `DescriptorGeometry` preserves raw, unaltered bitfields:
   - `flmap0_raw: u32`
   - `flmap1_raw: u32`
   - `flmap2_raw: u32`
   - `fcba_raw: u8`, `frba_raw: u8`, `fmba_raw: u8`
   - `nc_raw: u8`, `nr_raw: u8`, `nm_raw: u8`

---

## 3. Strict Error Propagation (DEF-38)

Previous implementations permitted silent error masking when accessing `FLMAP2`:
```rust
// FORMER CODE (DEF-38 VULNERABILITY):
let flmap2 = reader.read_u32_le(0x1C).unwrap_or(0);
```
On images truncated below 32 bytes (or where offset `0x1C` is inaccessible), this silently yielded `0x00000000`, disguising a truncated image as a valid legacy descriptor.

**Phase 2B.2 Remediation:**
```rust
// REFACTORED CODE (STRICT ERROR PROPAGATION):
let flmap2 = reader.read_u32_le(0x1C)?;
```
All header and table reads now explicitly propagate errors via the `?` operator. Any truncated buffer under 32 bytes immediately terminates parsing with an explicit `AppError::Binary` failure.

---

## 4. Verification Evidence

Deterministic unit and regression tests in `src-tauri/src/tests/engine_tests.rs`:

1. **`test_ifd_geometry_validation_detects_collisions_and_out_of_bounds`**:
   - Out-of-bounds `FRBA` (`0x120` in 8-bit page -> `0x1200` > 4KB): Confirms `geom.is_valid == false` and `FRBA out of bounds` error recorded.
   - Header collision (`FRBA = 0x01` -> offset `0x10`, colliding with `FLVALSIG`): Confirms `geom.is_valid == false`.
   - Section collision (`FRBA == FCBA`): Confirms collision detected and rejected.
   - Absent optional section (`fpsba == 0`): Confirms optional section safely ignored without polluting next-boundary calculation.

2. **`test_ifd_flmap2_missing_read_error_propagation`**:
   - Supplies a 24-byte truncated descriptor (header missing `FLMAP2` at `0x1C`).
   - Confirms parser returns `Err(AppError::Binary)` with explicit bounds failure rather than silently assuming legacy descriptor.

3. **`test_ifd_boundary_collision_detection`**:
   - Validates that overlapping regions within the region table are detected and flagged in Gate 1 structural validation.
