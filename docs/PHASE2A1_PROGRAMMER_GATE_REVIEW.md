# FriedChip+ — Programmer Compatibility Gate Architectural Review (DEF-12)

**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE — TRI-STATE ARCHITECTURE IMPLEMENTED  
**Target Milestone:** Phase 2A.1 Safety Architecture  
**Affected Module:** `src-tauri/src/engine/binary/ifd.rs`

---

## 1. Executive Summary & Problem Statement

Prior to Phase 2A.1, the Intel Flash Descriptor parser returned a speculative boolean compatibility assertion (`gate3_programmer.passed = true`) whenever the descriptor binary was structurally valid and the image size matched the sum of component capacities.

This design was identified in DEF-12 as unsafe firmware engineering practice:
> **Static binary analysis of an Intel Flash Descriptor CANNOT certify that an external hardware programmer (such as CH341A, Dediprog, RT809F, or a Raspberry Pi) can successfully program the physical chip in-circuit.**

Certifying programmer compatibility from a file image alone risks catastrophic hardware damage (e.g. attempting to flash a 1.8V chip with a 3.3V programmer without a level shifter) or mysterious write failures (due to motherboard back-powering, bus contention, or write-protect pin pull-downs).

---

## 2. Tri-State Compatibility Architecture

To prevent false confidence and guide technicians safely, Gate 3 was refactored into a rigorous tri-state status model (`ProgrammerCompatibilityStatus`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammerCompatibilityStatus {
    /// Confirmed compatible: verified against physical chip ID (JEDEC), operating voltage, and bus hold-off.
    Compatible,
    /// Confirmed incompatible: image size smaller than descriptor geometry, or structural damage detected.
    Incompatible,
    /// Indeterminate: static binary analysis is valid, but required physical hardware evidence is absent.
    Indeterminate,
}
```

### 2.1 Behavior Matrix

| Static Binary State | Physical Hardware Evidence Attached | Gate 3 Status | Gate 3 Passed Boolean |
|---|---|---|---|
| Structurally Corrupt / Truncated | Any | `Incompatible` | `false` |
| Image Size < Component Density | Any | `Incompatible` | `false` |
| Valid Descriptor Binary | **None** (Static Analysis Only) | **`Indeterminate`** | **`false`** |
| Valid Descriptor Binary | Confirmed JEDEC ID + Match Voltage + Isolated Hold-Off | `Compatible` | `true` |

---

## 3. Physical Hardware Parameters Required for Certification

A physical programming operation can only transition from `Indeterminate` to `Compatible` when the workstation receives verified hardware telemetry from a hardware probe or programmer device:

### 3.1 1. Flash Package & Pinout Verification
- **Package Types:** SOIC-8 (150mil / 208mil), SOIC-16, WSON-8 (6x5mm / 8x6mm), BGA-24.
- **Pinout Mapping:** Standard SPI pinout (`CS#`, `SO/IO1`, `WP#/IO2`, `GND`, `SI/IO0`, `SCK`, `HOLD#/RESET#/IO3`, `VCC`). In SOIC-16, pin 1 is typically `HOLD#/IO3` while `CS#` is pin 7. Using an 8-pin test clip on an unverified package causes ground shorts.

### 3.2 2. JEDEC Identification Interrogation (`RDID` / `9Fh`)
- Binary analysis indicates what capacity the descriptor expects (e.g. 16 MB via `FLCOMP`).
- However, physical programming requires querying the actual silicon via SPI command `0x9F` (`RDID`), returning:
  - **Manufacturer ID:** e.g., `0xEF` (Winbond), `0xC2` (Macronix), `0x1F` (Adesto/Atmel), `0x20` (Micron), `0xC8` (GigaDevice).
  - **Memory Type:** e.g., `0x40` (W25Q series).
  - **Capacity Code:** e.g., `0x18` (128 Mbit / 16 MB).
- If the hardware returns `0x000000` or `0xFFFFFF`, the clip has poor contact, bus contention exists, or the chip is unpowered.

### 3.3 3. Operating Voltage Verification (1.8V vs 3.3V)
- Applying 3.3V to a 1.8V SPI NOR flash chip (common on Skylake and newer mobile chipsets, e.g. Winbond `W25Q128FW` or Macronix `MX25U12835F`) will permanently degrade or destroy the silicon within seconds.
- Without reading the physical chip part marking or querying an authorized database using the JEDEC ID, operating voltage is strictly `None`.

### 3.4 4. In-Circuit Hold-Off & Motherboard Back-Powering
- When flashing in-circuit (without desoldering the chip), powering pin 8 (`VCC`) with 3.3V often back-powers the PCH, EC, and 3.3V standby power rail on the motherboard.
- If the PCH partially powers up, its SPI pins will actively drive the SPI bus, creating bus contention with the external programmer and corrupting the write operation.
- Safe in-circuit programming requires:
  - Asserting the chipset reset line (`PLTRST#` or `RSMRST#`) to hold PCH SPI pins in high-impedance (tri-state).
  - Monitoring programmer current draw to detect excessive power sinking into motherboard rails.

---

## 4. Verification Constraints Enforced by FriedChip+

When Gate 3 is evaluated during static analysis, the parser automatically attaches explicit engineering warnings and verification constraints to the `IfdAnalysisReport`:

```rust
let mut programmer_constraints = Vec::new();
programmer_constraints.push(format!(
    "Required SPI NOR flash capacity: {:.2} MB (0x{:X} bytes)",
    detected_capacity as f64 / (1024.0 * 1024.0),
    detected_capacity
));
programmer_constraints.push(
    "External programmer compatibility cannot be positively asserted from binary descriptor analysis alone.".to_string()
);
programmer_constraints.push(
    "Requires physical hardware interrogation: flash chip package pinout, JEDEC ID (RDID), operating voltage (1.8V vs 3.3V), and in-circuit hold-off/back-powering characteristics.".to_string()
);
programmer_constraints.push(
    "Host-based access permissions are distinct from external SPI programmer compatibility; external programmers bypass PCH master permission masks.".to_string()
);
programmer_constraints.push(
    "Descriptor master permissions do not guarantee host or in-circuit writeability: hardware Protected Range registers (PR0..PR4) and BIOS_CNTL lock bits enforce independent hardware protections.".to_string()
);
```

### 4.1 Automated Test Verification

Unit test `test_ifd_gate3_programmer_indeterminate_without_hardware` validates that:
```rust
assert!(report.gate1_structural.passed);
assert_eq!(report.gate3_programmer.status, ProgrammerCompatibilityStatus::Indeterminate);
assert!(!report.gate3_programmer.passed);
assert!(!report.gate3_programmer.hardware_evidence_present);
assert!(report.gate3_programmer.required_voltage_volts.is_none());
```

This ensures that FriedChip+ maintains commercial-grade engineering rigor: it never claims a flash image is "ready to flash" until physical verification is accomplished.
