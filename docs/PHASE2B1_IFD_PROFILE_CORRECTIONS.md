# Phase 2B.1 — Intel Flash Descriptor (IFD) Profile & Decoding Corrections

**Document ID:** `SPEC-IFD-2B1-002`  
**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** SPECIFICATION COMPLETE & CODE IMPLEMENTED  
**Scope:** IFD Profile Resolution, Register Bitfield Correctness & Safety Semantics (DEF-25, DEF-26, DEF-27, DEF-28, DEF-29)  

---

## 1. Overview & Core Engineering Problem

The Intel Flash Descriptor (IFD) defines the physical organization, component characteristics, and master access permissions of the SPI flash memory on Intel platforms. Historically, Intel has updated the descriptor across generations:
- **IFD v1 (Legacy):** Introduced with ICH8 through Lynx Point (Series 8/9). Characterized by 12-bit FLREG addressing (4KB granularity, up to 16MB), 3-bit component density, legacy FLMSTR shifts (bits 31:24 write, 23:16 read, 15:0 Requester ID), and an explicit Number of Regions field (`FLMAP0[26:24]`).
- **IFD v1.5 (Intermediate / Server):** Used specifically on Intel C610 series (Wellsburg, Haswell/Broadwell-E server/workstation PCH). Uses 12-bit FLREG addressing, 4-bit component density, legacy master shifts, and 50 MHz read clocks.
- **IFD v2 (Modern):** Introduced with 100-Series PCH (Skylake) and continuing through modern Raptor Lake / Meteor Lake. Characterized by 15-bit FLREG addressing (up to 128MB), 4-bit component density, modern FLMSTR shifts (bits 31:20 write, 19:8 read, bits 7:0 zero/dropped Requester ID), support for 17 MHz fast-read clocks, and **the deprecation/removal of the NR field in FLMAP0**.

Prior to Phase 2B.1, several critical engineering defects were identified in FriedChip+'s IFD decoding engine:
1. **DEF-25:** Profile detection relied on an ad-hoc address heuristic (`ICCRIBA >= 0x31`) and decoded a 4-bit NR field instead of a 3-bit field, while incorrectly attempting to read NR on modern descriptors where Intel dropped the field.
2. **DEF-26:** Descriptors with an `Unknown` profile performed speculative legacy decoding, risking false positive SPI clock and master permission inferences.
3. **DEF-27:** `V1_5Intermediate` profile was incorrectly mapped to mobile low-power chipsets (Lynx Point-LP / Wildcat Point-LP) in conflict with upstream coreboot evidence.
4. **DEF-28:** Static descriptor write access (`FLMSTR1`) was conflated with live chipset flashing locks (`host_flashing_locked`).
5. **DEF-29:** Gate 3 (external programmer compatibility) was marked `Incompatible` when Gate 1 (structural integrity) failed, conflating file corruption with programmer capability.

---

## 2. DEF-25: Profile Detection & Geometry-Based Region Derivation

### 2.1 Removal of Heuristic `ICCRIBA >= 0x31` Marker
Previous iterations used the offset of the Integrated Clock Controller Root Complex Information Base Address (`ICCRIBA`) in FLMAP2 as a marker for IFD v2. However, inspection of upstream coreboot (`util/ifdtool/ifdtool.c`) and Intel PCH datasheets confirmed that `ICCRIBA` is specific to platforms with an integrated clock controller (starting with Cougar Point / Panther Point) and does not cleanly distinguish IFD v1 from v2 across all platform variants. 

`ICCRIBA >= 0x31` has been completely eliminated as an IFD generation classifier.

### 2.2 FLMAP0 Number of Regions (`NR`) 3-Bit Isolation
Per Intel PCH External Design Specifications (EDS), `FLMAP0` register bitfields are defined as:
- Bits `7:0`: Flash Component Base Address (`FCBA`), bits [11:4] of byte offset.
- Bits `9:8`: Number of Components (`NC`), 0 = 1 component, 1 = 2 components.
- Bits `15:10`: Reserved.
- Bits `23:16`: Flash Region Base Address (`FRBA`), bits [11:4] of byte offset.
- Bits `26:24`: Number of Regions (`NR`), 3-bit value. Value $N$ indicates $N + 1$ regions (e.g., value 4 = 5 regions).
- Bits `31:27`: Reserved.

Prior code applied a 4-bit mask (`0x0F`), allowing reserved bit 27 to leak into region count calculations. In Phase 2B.1, the mask was corrected to strictly 3 bits:
```rust
let nr_bits = ((flmap0 >> 24) & 0x07) as usize;
let legacy_nr_count = nr_bits.saturating_add(1);
```
A dedicated regression test (`test_ifd_flmap0_nr_reserved_bit27_isolation`) verifies that setting bit 27 has zero effect on region counting.

### 2.3 Geometry-Derived Region Count (`max_regions_from_geometry`)
Starting with Skylake 100-Series PCH, Intel designated bits 24 onwards of `FLMAP0` as **Reserved**. The `NR` field was dropped. In modern descriptors, reading `FLMAP0[26:24]` produces zeros or undefined bits.

To determine the maximum number of regions supported by the descriptor without reading the dropped `NR` field, FriedChip+ adopted the coreboot `max_regions_from_fdbar` algorithm (`research/third_party/coreboot/util/ifdtool/ifdtool.c` lines 452–474):
1. Extract section base addresses from `FLMAP0` and `FLMAP1`:
   - `FCBA = (FLMAP0 & 0xFF) << 4`
   - `FMBA = (FLMAP1 & 0xFF) << 4`
   - `FRBA = ((FLMAP0 >> 16) & 0xFF) << 4`
   - `FPSBA = ((FLMAP1 >> 16) & 0xFF) << 4`
   - `FLUMAP1_OFFSET = 0x0EFC` (3,836 bytes; 4,096 - 260)
2. Sort the array of 5 boundary offsets.
3. Locate `FRBA` in the sorted array and find the immediately succeeding offset (`next`).
4. Calculate available table space: `avail = next - frba`.
5. Each region entry requires 4 bytes (`FLREGn`). Therefore:
   $$\text{max\_regions} = \min\left(\left\lfloor\frac{\text{avail}}{4}\right\rfloor, 16\right)$$

```rust
pub fn max_regions_from_geometry(fcba: usize, fmba: usize, frba: usize, fpsba: usize) -> usize {
    const FLUMAP1_OFFSET: usize = 0x0EFC; // 4096 - 260
    const MAX_REGIONS: usize = 16;

    let mut sorted = [fcba, fmba, frba, fpsba, FLUMAP1_OFFSET];
    sorted.sort_unstable();

    for i in 0..sorted.len().saturating_sub(1) {
        if sorted[i] == frba {
            let next = sorted[i + 1];
            if next > frba {
                let avail = next - frba;
                return (avail / 4).min(MAX_REGIONS);
            }
        }
    }
    0
}
```

### 2.4 Profile Confidence & Provenance Model
Profile detection now outputs a structured `ProfileResolution` struct containing:
- `profile`: `IfdProfile` (`V1Legacy`, `V1_5Intermediate`, `V2Modern`, `Unknown`).
- `confidence`: `ProfileConfidence` (`Confirmed`, `Probable`, `Unknown`).
- `source`: `ProfileSource` (`AutoDetected`, `UserOverride`).
- `max_regions`: Geometry- or register-bounded region capacity.

Criteria:
- `Confirmed`: Multiple convergent indicators (e.g., modern FLMSTR bit layout + 17 MHz fast-read clock + ISL >= 0x30; or legacy FLMSTR layout + valid legacy NR + ISL <= 21).
- `Probable`: Single strong indicator without conflicting features.
- `Unknown`: Insufficient or contradictory indicators. Speculative decoding is disabled.
- `UserOverride`: Explicit caller override via CLI or API. Always assigned `ProfileConfidence::Confirmed` and `ProfileSource::UserOverride`.

---

## 3. DEF-26: Safe Unknown Profile Handling

When a firmware binary contains an Intel Flash Descriptor that cannot be definitively classified, speculative decoding introduces catastrophic risks (e.g., miscalculating SPI flash size, corrupting flash offsets, or assuming permissions that allow bricking).

In Phase 2B.1, all decoding functions implement strict safe fallbacks for `IfdProfile::Unknown`:

1. **`decode_flreg_with_profile(val, IfdProfile::Unknown)`**:
   Returns `(0, 0, false)`. Never guesses 12-bit vs 15-bit address shifts. Region is marked disabled.
2. **`decode_spi_frequency(val, IfdProfile::Unknown)`**:
   Returns `SpiFrequency::Reserved(val)` (0 MHz). Never infers 20/33/50 MHz clock frequencies.
3. **`decode_flcomp_with_profile(..., IfdProfile::Unknown)`**:
   Preserves raw 32-bit `flcomp` register for forensic analysis, but sets `density_bytes = 0`, `density_mb = 0.0`, and `read_clock_frequency_mhz = 0`.
4. **`decode_master_rule_with_profile(..., IfdProfile::Unknown)`**:
   Returns `read_mask = 0`, `write_mask = 0`, `bios_readable = false`, and `bios_writable = false`.

Verified by unit test: `test_ifd_unknown_profile_safe_fallback_no_speculative_decoding`.

---

## 4. DEF-27: IFD V1.5 Wellsburg Evidence & Scope

Independent review revealed that earlier documentation loosely suggested `V1.5` might apply to low-power mobile chipsets such as Lynx Point-LP and Wildcat Point-LP.

Inspection of upstream coreboot source code (`research/third_party/coreboot/util/ifdtool/ifdtool.c` lines 344–348) reveals:
```c
chipset = ifd2_platform_to_chipset(platform);
if (chipset == CHIPSET_8_SERIES_WELLSBURG)
    ifd_version = IFD_VERSION_1_5;
else
    ifd_version = IFD_VERSION_2;
```
Coreboot strictly confines `IFD_VERSION_1_5` to **Intel 8-Series Wellsburg (C610)** enterprise/server chipset. Mobile low-power chipsets (Lynx Point-LP / Wildcat Point-LP) use IFD v1 with low-power descriptor straps, not IFD v1.5.

Technical characteristics of Wellsburg C610 IFD:
- Addressing: 12-bit FLREG addressing (same as IFD v1).
- Density: 4-bit component density (similar to IFD v2).
- Master rules: Legacy master shifts (bits 31:24 write, 23:16 read).
- Read frequency: 50 MHz read clock.

Because no authentic Wellsburg C610 dump is currently registered in `fixtures/realworld/`, `IfdProfile::V1_5Intermediate` remains designated as **Provisional / Unresolved** in FriedChip+. It will not be assigned by heuristic guessing without an authentic hardware dump and verified regression test.

---

## 5. DEF-28: Runtime Chipset Lock Semantics Remediation

Static firmware analysis of an Intel Flash Descriptor inspects `FLMSTR1` (Host CPU / BIOS Master Access Permissions). In `FLMSTR1`:
- Bit 24 (Legacy) or Bit 20 (Modern) controls whether the Host CPU is granted write access to Region 0 (Flash Descriptor).

When this bit is `0`, the PCH SPI controller blocks the Host CPU from issuing write or erase cycles to the Flash Descriptor region. However, this is an **offline descriptor permission**, NOT a runtime chipset lock.

Runtime flashing locks are enforced dynamically by system software during boot via PCI configuration space registers:
- **BIOS_CNTL (BC)**: Bit 0 `BIOSWE` (BIOS Write Enable), Bit 1 `BLE` (BIOS Lock Enable), Bit 5 `SMM_BWP` (SMM BIOS Write Protection).
- **PR0–PR4 (Protected Range Registers)**: SPI controller registers programmed by SMM to lock specific physical address ranges.

Previous FriedChip+ code exported a boolean field named `host_flashing_locked`. This was misleading because an engineer or technician could assume the platform cannot be flashed via software, when in reality `SMM_BWP` or `PR0` could be disabled, or conversely, `FLMSTR1` could allow write while runtime `SMM_BWP` blocks it.

Remediation:
1. Removed `host_flashing_locked` from `Gate3ProgrammerCompatibility`.
2. Added `host_descriptor_write_disabled`: accurately describes the static register state (`!host_descriptor_bios_write_allowed`).
3. Set `runtime_lock_status` strictly to `RuntimeChipsetLockStatus::Indeterminate` with explanatory notes explaining that live PCI configuration space interrogation (via kernel driver or CHIPSEC) is required to determine runtime lock status.

---

## 6. DEF-29: Gate 3 Programmer Compatibility Decoupling

In FriedChip+'s multi-gate validation model:
- **Gate 1 (Structural Integrity):** Validates descriptor magic signature (`0x0FF0A55A`), table boundaries, component counts, and image dimensions.
- **Gate 2 (Platform Compatibility):** Evaluates profile detection, chipset generation, and strap indicators.
- **Gate 3 (Programmer Compatibility):** Evaluates physical SPI programmer capability (e.g., external 3.3V/1.8V hardware flashing tool, total flash density, chip geometry).

Prior code contained logic where a Gate 1 structural failure automatically caused Gate 3 to be marked `Incompatible`. This was conceptually erroneous: a corrupted image file on disk does not mean an external hardware programmer cannot flash the physical SPI chip. Furthermore, without a live physical programmer connected (or hardware telemetry provided), Gate 3 cannot claim `Compatible` or `Incompatible`.

Remediation:
1. Gate 3 status is decoupled from Gate 1 structural validation.
2. Without live physical hardware evidence (`hardware_evidence_present == false`), Gate 3 status is unconditionally `ProgrammerCompatibilityStatus::Indeterminate`.
3. Renamed capacity fields from `detected_chip_size_bytes` to `descriptor_inferred_flash_capacity_bytes` and `descriptor_inferred_flash_capacity_mb` (with serde aliases `detected_chip_size_bytes` for backward compatibility) to make explicit that capacity is inferred from the descriptor's FCBA, not detected from physical JEDEC ID telemetry.
