# FriedChip+ — Intel Flash Descriptor Code Corrections & Golden Tests (DEF-09, DEF-10, DEF-11)

**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** IMPLEMENTED & TESTED  
**Target Milestone:** Phase 2A.1 Descriptor Remediation  
**Modules Modified:**
- `src-tauri/src/engine/binary/ifd.rs`
- `src-tauri/src/engine/testing/fixture_builder.rs`
- `src-tauri/src/tests/engine_tests.rs`

---

## 1. Overview of Code Modifications

In accordance with the Intel SPI Programming Guide and reference audit findings, the Intel Flash Descriptor parser, synthetic fixture builder, and integration test suite were updated with surgical precision.

### 1.1 Summary of Architectural Changes

1. **`engine/binary/ifd.rs`:**
   - Corrected FLMAP0 reading: `FCBA = (flmap0 & 0xFF) << 4`, `NC = ((flmap0 >> 8) & 0x03) + 1`, `FRBA = ((flmap0 >> 16) & 0xFF) << 4`, `NR = ((flmap0 >> 24) & 0x0F) + 1`.
   - Corrected FLMAP1 reading: `FMBA = (flmap1 & 0xFF) << 4`, `NM = ((flmap1 >> 8) & 0x07) + 1`, `FISBA = ((flmap1 >> 16) & 0xFF) << 4`, `ISL = ((flmap1 >> 24) & 0xFF)`.
   - Corrected FLMAP2 reading: `FMSBA = (flmap2 & 0xFF) << 4`, `MSL = ((flmap2 >> 8) & 0xFF)`.
   - Implemented `decode_spi_frequency_mhz(bits)` helper supporting standard Intel frequencies (20 MHz, 33 MHz, 48 MHz, 50 MHz).
   - Re-aligned `decode_flcomp` to exact Intel specifications: Component 1 density (bits 2:0), Component 2 density (bits 5:3), Read clock (bits 19:17), Fast Read support (bit 20), Fast Read clock (bits 23:21), Write clock (bits 26:24), Read ID clock (bits 29:27).
   - Replaced monolithic FRAP decoder with modular `decode_master_rule` supporting discrete `FLMSTR1`, `FLMSTR2`, and `FLMSTR3` registers with generation-aware bit shifts (`rd_shift = 16, wr_shift = 24` for IFD v1; `rd_shift = 8, wr_shift = 20` for IFD v2).
   - Upgraded Gate 3 Programmer Compatibility to tri-state `ProgrammerCompatibilityStatus::Indeterminate` when binary analysis lacks physical hardware evidence.

2. **`engine/testing/fixture_builder.rs`:**
   - Updated `build_valid_ifd_image` to write spec-compliant register values:
     - `FLMAP0` at offset `0x14`: `0x04040008` (FRBA=0x40, NR=5, NC=1, FCBA=0x80)
     - `FLMAP1` at offset `0x18`: `0x02100206` (ISL=2, FISBA=0x100, NM=3, FMBA=0x60)
     - `FLCOMP` at offset `0x80`: `0x09900005` (16MB density, 20MHz read, fast read supported, 50MHz fast read, 33MHz write, 33MHz read ID)
     - `FLMSTR1` at offset `0x60`: `0x02170000` (Host CPU: Read Descriptor, BIOS, ME, PlatData; Write BIOS)
     - `FLMSTR2` at offset `0x64`: `0x04050000` (Intel ME: Read Descriptor, ME; Write ME)
     - `FLMSTR3` at offset `0x68`: `0x08080000` (GbE: Read GbE; Write GbE)

3. **`tests/engine_tests.rs`:**
   - Added unit test `test_ifd_flcomp_known_raw_register_values` verifying `0x08900005` and dual-chip `0x0000002B`.
   - Added unit test `test_ifd_flmstr_known_raw_register_values` verifying IFD v1 `0x02170000`, full-access `0xFFFFFFFF`, and IFD v2 `0x00200300`.
   - Added unit test `test_ifd_gate3_programmer_indeterminate_without_hardware`.
   - Annotated real-world tests with `#[ignore = "..."]` to enforce clean CI test semantics.

---

## 2. Derivation of Golden Test Vectors

To prevent circular test validation (where a buggy fixture builder validates a buggy parser), test vectors were derived independently from first principles and published specifications:

### Vector 1: Golden `FLCOMP` Register `0x08900005`
- **Specification Source:** Intel 6-Series SPI Programming Guide, Section 4.2.1
- **Target Configuration:**
  - Single SPI NOR chip, capacity 16 MB
  - Standard Read Frequency: 20 MHz
  - Fast Read Enabled: True
  - Fast Read Frequency: 50 MHz
  - Write / Erase Frequency: 20 MHz
  - Read ID / Status Frequency: 33 MHz
- **Bitwise Derivation:**
  $$\begin{aligned}
  \text{Bits 2:0} &= 5\ (\text{Binary } 101_2) \implies 512\text{ KB} \times 2^5 = 16\text{ MB} & (\text{Value: } \mathtt{0x00000005}) \\
  \text{Bits 19:17} &= 0\ (\text{Binary } 000_2) \implies 20\text{ MHz Standard Read} & (\text{Value: } \mathtt{0x00000000}) \\
  \text{Bit 20} &= 1\ (\text{Binary } 1_2) \implies \text{Fast Read Supported} & (\text{Value: } \mathtt{0x00100000}) \\
  \text{Bits 23:21} &= 4\ (\text{Binary } 100_2) \implies 50\text{ MHz Fast Read} & (\text{Value: } \mathtt{0x00800000}) \\
  \text{Bits 26:24} &= 0\ (\text{Binary } 000_2) \implies 20\text{ MHz Write} & (\text{Value: } \mathtt{0x00000000}) \\
  \text{Bits 29:27} &= 1\ (\text{Binary } 001_2) \implies 33\text{ MHz Read ID} & (\text{Value: } \mathtt{0x08000000})
  \end{aligned}$$
- **Sum Total:**
  $$\mathtt{0x00000005} \mid \mathtt{0x00100000} \mid \mathtt{0x00800000} \mid \mathtt{0x08000000} = \mathbf{0x08900005}$$

### Vector 2: Golden `FLCOMP` Dual-Chip Register `0x0000002B`
- **Target Configuration:**
  - Component 1: 4 MB ($512\text{ KB} \times 2^3$, bits 2:0 = 3)
  - Component 2: 16 MB ($512\text{ KB} \times 2^5$, bits 5:3 = 5)
  - Fast Read: Disabled (bit 20 = 0)
- **Bitwise Derivation:**
  $$\begin{aligned}
  \text{Bits 2:0} &= 3\ (\mathtt{0x03}) \\
  \text{Bits 5:3} &= 5\ (\mathtt{0x05} \ll 3 = \mathtt{0x28}) \\
  \text{Total} &= \mathtt{0x03} \mid \mathtt{0x28} = \mathbf{0x0000002B}
  \end{aligned}$$

### Vector 3: Golden `FLMSTR1` IFD v1 Register `0x02170000`
- **Specification Source:** Coreboot `ifdtool.c`, Libreboot `ich9utils`
- **Target Configuration:**
  - Master: Host CPU / BIOS on legacy PCH (ICH8 through Haswell)
  - Requester ID: `0x0000` (bits 15:0)
  - Read Access Mask: `0x17` (bits 23:16) $\implies$ Regions 0 (Descriptor), 1 (BIOS), 2 (ME), 4 (Platform Data)
  - Write Access Mask: `0x02` (bits 31:24) $\implies$ Region 1 (BIOS only)
- **Bitwise Derivation:**
  $$\mathtt{0x0000} \mid (\mathtt{0x17} \ll 16) \mid (\mathtt{0x02} \ll 24) = \mathbf{0x02170000}$$

### Vector 4: Golden `FLMSTR1` IFD v2 Register `0x00200300`
- **Specification Source:** Intel 100-Series PCH SPI Programming Guide
- **Target Configuration:**
  - Master: Host CPU / BIOS on modern PCH (Skylake / Sunrise Point+)
  - Read Access Mask: `0x03` $\implies$ Regions 0 (Descriptor) and 1 (BIOS) readable
  - Write Access Mask: `0x02` $\implies$ Region 1 (BIOS) writable
- **Bitwise Derivation (IFD v2 Shifts):**
  $$\begin{aligned}
  \text{Read Mask (bits 19:8)} &= \mathtt{0x03} \ll 8 = \mathtt{0x00000300} \\
  \text{Write Mask (bits 31:20)} &= \mathtt{0x02} \ll 20 = \mathtt{0x00200000} \\
  \text{Total} &= \mathtt{0x00000300} \mid \mathtt{0x00200000} = \mathbf{0x00200300}
  \end{aligned}$$

---

## 3. Test Execution and Verification Proof

Both default testing and full integration testing passed with zero warnings or errors.

### 3.1 Default Test Suite Run
```bash
$ cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
running 38 tests
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_fcba_decoding_component_density ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
test tests::engine_tests::test_ifd_valid_image_parsing_and_gates ... ok
test tests::engine_tests::test_ifd_gate3_programmer_indeterminate_without_hardware ... ok
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ignored, Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/
[FUZZ EVIDENCE] In-process bounded fuzzing completed: 3 x 100000 (300,000 total iterations) in 16.36s with 0 crashes or hangs.
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 36 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 16.36s
```

### 3.2 Full Suite Run with Ignored Tests Included
```bash
$ cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored --nocapture
running 38 tests
test tests::engine_tests::test_ifd_flcomp_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_flmstr_known_raw_register_values ... ok
test tests::engine_tests::test_ifd_master_permissions_decoding ... ok
[PASS] test_dell_pfs_differential_regression_against_biosutilities: All 9 exact matches and 3 documented deltas verified.
test tests::engine_tests::test_dell_pfs_differential_regression_against_biosutilities ... ok
[PASS] test_real_world_dell_latitude_5x90_if_available: Successfully extracted 14 components from verified Dell package.
test tests::engine_tests::test_real_world_dell_latitude_5x90_if_available ... ok
[FUZZ EVIDENCE] In-process bounded fuzzing completed: 3 x 100000 (300,000 total iterations) in 16.82s with 0 crashes or hangs.
test tests::engine_tests::test_in_process_bounded_parser_stress_fuzzing ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.82s
```
