# FriedChip+ — Phase 2B Intel Flash Descriptor Corrections Report

**Document ID:** `CR-IFD-002`  
**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE & VERIFIED (42/42 Tests Passing)  
**Modules Modified:**
- `src-tauri/src/engine/binary/ifd.rs`
- `src-tauri/src/engine/testing/fixture_builder.rs`
- `src-tauri/src/tests/engine_tests.rs`

---

## 1. Executive Summary

During Phase 2A and Phase 2A.1, the Intel Flash Descriptor parser demonstrated functional progress, successfully extracting tables from synthetic descriptors. However, an in-depth source review against upstream firmware engineering references (`coreboot`, `flashrom`) revealed critical architectural and bitfield defects:

1. **Ad-Hoc Profile Heuristic (DEF-15)**: The parser used a brittle heuristic (`nr >= 10`) to switch between legacy and modern descriptor logic, misidentifying Skylake+ descriptors (where Intel dropped `NR`) or legacy descriptors with non-standard strap counts.
2. **FLREG Bitfield & Boundary Inaccuracies (DEF-16)**: Unconditionally applied a 15-bit address mask across all chipsets (reading reserved bits on ICH8..Wildcat Point), lacked checked arithmetic, used inconsistent upper bound semantics, and failed to correctly identify disabled regions.
3. **FLCOMP Frequency & Density Corruption (DEF-17)**: Silently mapped reserved SPI frequencies to 20 MHz and applied uniform density bit masks ignoring 3-bit vs 4-bit generation differences.
4. **FLMSTR Shift Errors & False Runtime Claims (DEF-18)**: Applied fixed bit shifts to master read/write masks across generations and incorrectly claimed "flashing locked" based purely on descriptor permissions without checking runtime chipset registers.
5. **Programmer Compatibility Gate Over-Confidence (DEF-19)**: Claimed positive programmer compatibility on static binary analysis without physical SPI telemetry.

All five defects have been completely remedied in Phase 2B with clean-room, profile-aware Rust implementations.

---

## 2. Itemized Remediation Breakdown

### 2.1 DEF-15: Explicit IFD Profile Architecture

#### Defect Description
Prior implementation relied on `let is_modern = nr >= 10;`. In modern Intel descriptors (100-Series Skylake through 700-Series Raptor Lake), Intel explicitly repurposed bits 27:24 of `FLMAP0`, causing `NR` to read as 0 or arbitrary strap values. This caused modern images to be misclassified as legacy, truncating regions past region 4.

#### Remediation Implemented
Implemented a first-class `IfdProfile` enum and profile detection engine inspecting multiple physical hardware indicators:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfdProfile {
    V1Legacy,
    V1_5Intermediate,
    V2Modern,
    Unknown,
}
```

The new `detect_ifd_profile(header_bytes)` function inspects:
1. `FLMAP2` ICC Register Initialization Base Address (`ICCRIBA >= 0x31` indicates Skylake+).
2. PCH Strap Length (`ISL > 16` indicates extended modern straps).
3. Section table geometric spacing between `FRBA` and successor tables.
4. Fallback gracefully to `IfdProfile::Unknown` without panicking.

---

### 2.2 DEF-16: Profile-Aware FLREG Address Translation & Boundary Semantics

#### Defect Description
- Legacy IFD v1 uses 12-bit address page masks (`0x0FFF`), while IFD v2 uses 15-bit masks (`0x7FFF`). Prior code used a universal 15-bit mask, causing bit bleeding on legacy hardware.
- Upper boundary calculation was inconsistent (exclusive vs inclusive), causing off-by-one byte size mismatches.
- Disabled region detection was incomplete: in Intel hardware, a region is disabled if and only if $\text{limit\_raw} < \text{base\_raw}$. Region 0 with $\text{base}=0, \text{limit}=0$ is an active 4 KB region.

#### Remediation Implemented
Implemented `decode_flreg_with_profile`:
```rust
pub fn decode_flreg_with_profile(val: u32, profile: IfdProfile) -> (usize, usize, bool) {
    let (base_mask, limit_mask) = match profile {
        IfdProfile::V1Legacy => (0x0FFF, 0x0FFF),
        IfdProfile::V1_5Intermediate => (0x1FFF, 0x1FFF),
        IfdProfile::V2Modern | IfdProfile::Unknown => (0x7FFF, 0x7FFF),
    };

    let base_raw = (val & base_mask) as usize;
    let limit_raw = ((val >> 16) & limit_mask) as usize;

    let base = base_raw.checked_shl(12).unwrap_or(0);
    let limit = limit_raw.checked_shl(12).map(|l| l | 0xFFF).unwrap_or(0);

    // A region is disabled if limit_raw < base_raw
    let is_disabled = limit_raw < base_raw;
    (base, limit, !is_disabled)
}
```
- Inclusive limit calculation `(limit_raw << 12) | 0xFFF`.
- Checked shift arithmetic preventing integer overflow on malformed inputs.
- Region 0 (`0x00000000`) verified active covering `0x00000000..0x00000FFF`.

---

### 2.3 DEF-17: Profile-Aware FLCOMP Decoding & Strict Frequency Typing

#### Defect Description
- Prior code defaulted reserved SPI clock frequencies silently to 20 MHz, concealing corrupted or tampered descriptors.
- In IFD v1, component densities occupy bits 2:0 and 5:3 (3 bits each, max 16 MB). In IFD v2, they occupy bits 3:0 and 7:4 (4 bits each, max 64 MB). Prior code read bit 3 on legacy descriptors, misinterpreting frequency bits as density.
- Clock frequency value `4` represents 50 MHz on IFD v1, but 30 MHz on IFD v2. Value `6` represents 17 MHz on Skylake+.

#### Remediation Implemented
1. Replaced raw integers with a strictly typed `SpiFrequency` enum:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpiFrequency {
    Mhz(u32),
    Reserved(u8),
    RepurposedEspi(u8),
}
```
2. Implemented `decode_spi_frequency` with profile-specific frequency mappings:
```rust
pub fn decode_spi_frequency(bits: u8, profile: IfdProfile) -> SpiFrequency {
    let val = bits & 0x07;
    match profile {
        IfdProfile::V1Legacy | IfdProfile::V1_5Intermediate => match val {
            0 => SpiFrequency::Mhz(20),
            1 => SpiFrequency::Mhz(33),
            2 => SpiFrequency::Mhz(48),
            4 => SpiFrequency::Mhz(50),
            _ => SpiFrequency::Reserved(val),
        },
        IfdProfile::V2Modern => match val {
            0 => SpiFrequency::Mhz(20),
            1 => SpiFrequency::Mhz(33),
            2 => SpiFrequency::Mhz(48),
            4 => SpiFrequency::Mhz(30),
            6 => SpiFrequency::Mhz(17),
            _ => SpiFrequency::Reserved(val),
        },
        IfdProfile::Unknown => match val {
            0 => SpiFrequency::Mhz(20),
            1 => SpiFrequency::Mhz(33),
            2 => SpiFrequency::Mhz(48),
            _ => SpiFrequency::Reserved(val),
        },
    }
}
```
3. Component density decoding respects 3-bit vs 4-bit fields and returns accurate MB sizes up to 64 MB.

---

### 2.4 DEF-18: Master Permission Bit Shifts & Decoupling from Runtime Locks

#### Defect Description
- Master permissions in `FLMSTRn` use different bit shifts across profiles:
  - IFD v1: Write mask shifted by 24, Read mask shifted by 16.
  - IFD v2: Write mask shifted by 20, Read mask shifted by 8 (or 4).
- The prior code claimed `host_flashing_locked = true/false` directly from `FLMSTR1`. In real hardware, BIOS flashing is frequently locked at runtime via PCH Protected Range registers (`PR0`..`PR4`), BIOS Lock Enable (`BLE`), or SMM BIOS Write Protect (`SMM_BWP`), none of which exist in the static descriptor. This gave users a false sense of security.

#### Remediation Implemented
1. Implemented `decode_master_rule_with_profile` with profile-aware shifts matching coreboot `ifdtool` reference values.
2. Renamed field to `host_descriptor_bios_write_allowed` to accurately reflect descriptor-level permissions.
3. Added `RuntimeChipsetLockStatus::Indeterminate` to Gate 2:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeChipsetLockStatus {
    Locked,
    Unlocked,
    Indeterminate,
}
```
Static descriptor parsing now explicitly marks runtime chipset locks as `Indeterminate`, preventing dangerous assumptions during in-system flashing workflows.

---

### 2.5 DEF-19: Programmer Compatibility Gate Tri-State Model

#### Defect Description
Gate 3 previously claimed `passed = true` based solely on the absence of static descriptor errors. External hardware SPI flashing depends on VCC voltage (1.8V vs 3.3V), hold/WP pin states, bus contention from an unpowered PCH, and chip ID matches. Claiming compatibility on binary data alone risked hardware damage.

#### Remediation Implemented
Strictly enforced tri-state status in `Gate3ProgrammerCompatibility`:
```rust
pub programmer_compatibility: ProgrammerCompatibilityStatus::Indeterminate,
pub passed: false, // Must remain false until hardware telemetry is supplied
```
A unit test `test_ifd_gate3_programmer_indeterminate_without_hardware` validates that static analysis never claims `passed = true` for external hardware programming.

---

## 3. Synthetic Fixture Builder Corrections

In `src-tauri/src/engine/testing/fixture_builder.rs`:
1. **16-Region Layout Collision**: When generating 16 regions starting at `FRBA = 0x40`, the region table occupies `0x40..0x80`. Previously, `FMBA` was placed at `0x60`, causing region 8 to overwrite `FMBA` with `0x00007FFF` (disabled region mask), wiping out master rules. Corrected by placing `FMBA = 0x80` (`0x08`) and `FCBA = 0xA0` (`0x0A`).
2. **`build_valid_ifd_v2_image`**: Added an explicit synthetic generator for modern 16-region IFD v2 images with valid Skylake+ `FLMAP2`, 15-bit address regions, 4-bit densities, and 12-master permission rules.
3. **Legacy FLMAP2 Initialization**: Initialized `FLMAP2` in legacy fixtures to `0x00000000` to prevent uninitialized memory from triggering modern heuristics.

---

## 4. Verification Evidence

All corrections have been validated against the full 42-test suite:

```text
running 42 tests
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flcomp_reserved_frequency_handling ... ok
test tests::engine_tests::test_ifd_differential_regression_against_coreboot_ifdtool ... ok
test tests::engine_tests::test_flreg_decoding_15bit_addressing ... ok
test tests::engine_tests::test_ifd_flreg_profile_aware_decoding_and_boundary_checks ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_ifd_v2_valid_image_parsing_and_profile_detection ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.25s
```
Zero regressions observed.
