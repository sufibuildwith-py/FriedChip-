# FriedChip+ — Intel Flash Descriptor Register Audit (DEF-09, DEF-10, DEF-11)

**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE — SPECIFICATION-ALIGNED  
**Target Milestone:** Phase 2A.1 Descriptor Remediation  
**Authority References:**
1. Intel 6 Series Chipset and Intel C200 Series Chipset SPI Programming Guide (Document 290744-001)
2. Intel 100 Series / 200 Series Chipset PCH Datasheet Vol 1 & 2
3. Libreboot `ich9utils/src/descriptor/descriptor.h`
4. Coreboot `util/ifdtool/ifdtool.c` (`decode_flmstr`, `IFD_VERSION_2`)
5. Flashrom `ich_descriptors.h` & `ich_descriptors.c`

---

## 1. Executive Summary & Root Cause Analysis

During Phase 2A audit review, three significant register decoding defects were identified in the Intel Flash Descriptor parser (`src-tauri/src/engine/binary/ifd.rs`):

- **DEF-09 (FLMAP Inversion):** The Flash Map 0 (`FLMAP0`) and Flash Map 1 (`FLMAP1`) registers were erroneously inverted in field offsets and bitfields. The parser extracted region count (`NR`) and component count (`NC`) from inverted offsets and treated `FLMAP1` as containing component information.
- **DEF-10 (FLCOMP Bitfield Misalignment):** The Component 0 density and SPI clock configuration register (`FLCOMP`) read Fast Read Support from bit 13 and Fast Read Clock Frequency from bits 14 onward. Under Intel PCH specifications, Fast Read Support is located at bit 20, Fast Read Clock Frequency occupies bits 23:21, Standard Read Clock Frequency occupies bits 19:17, and Component 1/2 densities occupy bits 2:0 and 5:3.
- **DEF-11 (Master Section & Permission Conflation):** The parser treated Flash Master permissions as a single MMIO register (`FRAP`), whereas the binary Flash Descriptor stored on SPI NOR flash contains a dedicated Flash Master Section (`FMBA`) composed of distinct 32-bit registers (`FLMSTR1` for Host CPU, `FLMSTR2` for Intel ME, and `FLMSTR3` for GbE). Additionally, the parser did not account for bit-shift differences between IFD v1 (ICH8..Broadwell) and IFD v2 (Skylake..Raptor Lake+).

This document establishes the authoritative bitfield specifications used to rectify `src-tauri/src/engine/binary/ifd.rs`.

---

## 2. Flash Map Registers Specification Audit (DEF-09)

The Flash Descriptor starts with a 16-byte Signature Block at offset `0x00..0x0F` (magic value `0x0FF0A55A` at offset `0x10`). The Flash Map registers begin immediately after the signature block at offset `0x14`.

### 2.1 FLMAP0 — Flash Map 0 (Offset `0x14`)

`FLMAP0` provides pointers to the Flash Component Base Address (`FCBA`) and the Flash Region Base Address (`FRBA`), along with component count and region count masks.

| Bit Range | Field Name | Description | Mask | Shift | Address Translation |
|---|---|---|---|---|---|
| `7:0` | `FCBA` | Flash Component Base Address | `0x000000FF` | `0` | Base byte offset = `FCBA << 4` |
| `9:8` | `NC` | Number of Components | `0x00000300` | `8` | Number of Flash chips = `NC + 1` (0=1 chip, 1=2 chips) |
| `15:10` | Reserved | Reserved for future Intel expansion | `0x0000FC00` | `10` | Must be 0 |
| `23:16` | `FRBA` | Flash Region Base Address | `0x00FF0000` | `16` | Base byte offset = `FRBA << 4` |
| `27:24` | `NR` | Number of Regions | `0x0F000000` | `24` | Number of regions supported = `NR + 1` |
| `31:28` | Reserved | Reserved for future Intel expansion | `0xF0000000` | `28` | Must be 0 |

**Defect Contrast:**
- *Prior Defective Code:* Extracted `NR` from bits 7:0 of `FLMAP0`, and searched for `NC`/`FCBA` inside `FLMAP1`.
- *Remediated Code:* Strictly decodes `FCBA = (flmap0 & 0xFF) << 4`, `NC = ((flmap0 >> 8) & 0x03) + 1`, `FRBA = ((flmap0 >> 16) & 0xFF) << 4`, and `NR = ((flmap0 >> 24) & 0x0F) + 1`.

---

### 2.2 FLMAP1 — Flash Map 1 (Offset `0x18`)

`FLMAP1` provides pointers to the Flash Master Base Address (`FMBA`) and Flash PCH Strap Base Address (`FPSBA` or `FISBA`).

| Bit Range | Field Name | Description | Mask | Shift | Address Translation |
|---|---|---|---|---|---|
| `7:0` | `FMBA` | Flash Master Base Address | `0x000000FF` | `0` | Base byte offset = `FMBA << 4` |
| `10:8` | `NM` | Number of Masters | `0x00000700` | `8` | Number of bus masters = `NM + 1` |
| `15:11` | Reserved | Reserved | `0x0000F800` | `11` | Must be 0 |
| `23:16` | `FISBA` / `FPSBA` | Flash PCH / Internal Strap Base Address | `0x00FF0000` | `16` | Base byte offset = `FISBA << 4` |
| `31:24` | `ISL` / `PSL` | Internal Straps Length | `0xFF000000` | `24` | Length in Dwords |

---

### 2.3 FLMAP2 — Flash Map 2 (Offset `0x1C`)

`FLMAP2` provides pointers to the Flash Master Secondary / Processor Strap Base Address (`FMSBA`).

| Bit Range | Field Name | Description | Mask | Shift | Address Translation |
|---|---|---|---|---|---|
| `7:0` | `FMSBA` | Processor Strap Base Address | `0x000000FF` | `0` | Base byte offset = `FMSBA << 4` |
| `15:8` | `MSL` | Processor Straps Length | `0x0000FF00` | `8` | Length in Dwords |
| `31:16` | Reserved | Reserved | `0xFFFF0000` | `16` | Must be 0 |

---

## 3. Flash Component Register Specification Audit (DEF-10)

The Component Block starts at offset `FCBA = (FLMAP0[7:0] << 4)`. The primary register is `FLCOMP` (Flash Components, Dword 0 at `FCBA + 0x00`).

### 3.1 FLCOMP Register Bitfield Definitions

| Bit Range | Field Name | Description | Mask | Shift | Decoded Interpretation |
|---|---|---|---|---|---|
| `2:0` | `COMP1_DENSITY` | Component 1 Flash Density | `0x00000007` | `0` | `0=512KB`, `1=1MB`, `2=2MB`, `3=4MB`, `4=8MB`, `5=16MB`, `6=32MB`, `7=64MB` |
| `5:3` | `COMP2_DENSITY` | Component 2 Flash Density | `0x00000038` | `3` | Valid if `NC >= 2`. Same exponential mapping as Comp 1. |
| `16:6` | Reserved | Reserved (includes dual-output configuration) | `0x0001FFC0` | `6` | Reserved |
| `19:17` | `FREQ_READ` | SPI Standard Read Clock Frequency | `0x000E0000` | `17` | `0=20MHz`, `1=33MHz`, `2=48MHz`, `4=50MHz` |
| `20` | `FAST_READ_SUPP` | Fast Read Support | `0x00100000` | `20` | `1 = Enabled`, `0 = Disabled` |
| `23:21` | `FREQ_FAST_READ` | Fast Read Clock Frequency | `0x00E00000` | `21` | Valid if bit 20=1. `0=20MHz`, `1=33MHz`, `2=48MHz`, `4=50MHz` |
| `26:24` | `FREQ_WRITE` | Fast Write / Erase Clock Frequency | `0x07000000` | `24` | `0=20MHz`, `1=33MHz`, `2=48MHz`, `4=50MHz` |
| `29:27` | `FREQ_READ_ID` | Read ID & Status Clock Frequency | `0x38000000` | `27` | `0=20MHz`, `1=33MHz`, `2=48MHz`, `4=50MHz` |
| `31:30` | Reserved | Reserved | `0xC0000000` | `30` | Reserved |

### 3.2 Clock Frequency Mapping Table (`decode_spi_frequency_mhz`)

```text
Bits Value | Decoded Clock Frequency
-----------|------------------------
0b000 (0)  | 20 MHz
0b001 (1)  | 33 MHz
0b010 (2)  | 48 MHz
0b100 (4)  | 50 MHz
Others     | 0 MHz (Reserved / Unknown)
```

---

## 4. Flash Master Section & Permissions Specification Audit (DEF-11)

The Flash Master Section starts at `FMBA = (FLMAP1[7:0] << 4)`. It contains separate 32-bit registers for each bus master capable of initiating transactions on the SPI bus through the PCH internal memory-mapped SPI controller.

### 4.1 Master Allocation by Index

- Master 1 (`FMBA + 0x00`): **`FLMSTR1`** — Host CPU / BIOS
- Master 2 (`FMBA + 0x04`): **`FLMSTR2`** — Intel ME (Management Engine) / CSME
- Master 3 (`FMBA + 0x08`): **`FLMSTR3`** — Gigabit Ethernet (GbE) controller

### 4.2 Generation-Specific Bit Shifts: IFD v1 vs IFD v2

Intel altered the Flash Master register bit allocations between IFD v1 (up to 8 flash regions) and IFD v2 (supporting up to 16 flash regions for Skylake, Kaby Lake, Coffee Lake, Tiger Lake, Alder Lake, etc.):

#### IFD v1.0 Layout (Legacy: ICH8 through Wildcat Point / Broadwell)
- `15:0`: **Requester ID** (PCI Requester ID of the master, e.g. `0x0000` for Host CPU)
- `23:16`: **Read Access Mask** (Bit $n = 1 \implies$ Master has Read access to Flash Region $n$, for $n \in 0..7$)
- `31:24`: **Write Access Mask** (Bit $n = 1 \implies$ Master has Write access to Flash Region $n$, for $n \in 0..7$)

```text
Read Mask Shift:  16  (Mask: 0xFF)
Write Mask Shift: 24  (Mask: 0xFF)
```

#### IFD v2.0 Layout (Modern: Skylake 100-series through 700-series)
- `19:8`: **Read Access Mask** (Bit $n = 1 \implies$ Master has Read access to Flash Region $n$, for $n \in 0..11$)
- `31:20`: **Write Access Mask** (Bit $n = 1 \implies$ Master has Write access to Flash Region $n$, for $n \in 0..11$)
- `3:0`: **Extended Read Access Mask** (Regions 12..15 in Cannon Lake+)
- `7:4`: **Extended Write Access Mask** (Regions 12..15 in Cannon Lake+)

```text
Read Mask Shift:  8   (Mask: 0xFFF)
Write Mask Shift: 20  (Mask: 0xFFF)
```

### 4.3 Standard Intel Flash Regions
- Region 0: Flash Descriptor
- Region 1: BIOS
- Region 2: Intel Management Engine (ME)
- Region 3: Gigabit Ethernet (GbE)
- Region 4: Platform Data Region (PDR)
- Region 5: Device Expansion (DER) / EC Firmware
- Region 6: Secondary BIOS
- Region 7: Reserved / Microcode
- Region 8: Embedded Controller (EC)
- Region 9: Device Expansion 2

### 4.4 Hardware Access Reality vs Descriptor Permissions

> [!IMPORTANT]
> Descriptor master permissions do NOT guarantee that a master can write to a flash region, NOR do they restrict external physical SPI programmers.
> 1. **External SPI Hardware Programmers:** External flash programmers (CH341A, Dediprog, Raspberry Pi SPI) interface directly with the SPI NOR flash chip pins (`MOSI`, `MISO`, `SCK`, `CS#`). The PCH SPI controller is completely unpowered or held in reset; therefore, **all PCH master permissions are bypassed**.
> 2. **Hardware Protected Range Registers (`PR0..PR4`):** Even if `FLMSTR1` gives Host CPU write permission to Region 1 (BIOS), runtime MMIO registers inside the PCH SPIBAR (specifically `PR0` through `PR4` and `BIOS_CNTL.SMM_BWP`) can hardware-lock write operations to memory ranges within the BIOS region.
> 3. **FriedChip+ Engineering Enforcement:** The parser output now clearly separates descriptor permissions from in-circuit hardware writeability, and Gate 3 programmer compatibility returns `ProgrammerCompatibilityStatus::Indeterminate` when physical hardware evidence is absent.

---

## 5. Verification Matrix Summary

| Verification Category | Implementation Vector | Specification Test Target | Outcome |
|---|---|---|---|
| `FLMAP0` Offset & Fields | `ifd.rs::IntelFlashDescriptor::parse` | `FCBA=0x80`, `NC=1`, `FRBA=0x40`, `NR=5` | **VERIFIED** |
| `FLMAP1` Offset & Fields | `ifd.rs::IntelFlashDescriptor::parse` | `FMBA=0x60`, `NM=3`, `FISBA=0x100`, `ISL=2` | **VERIFIED** |
| `FLCOMP` Densities & Clocks | `ifd.rs::decode_flcomp` | `0x08900005` (16MB, 50MHz Fast Read, 33MHz Read ID) | **VERIFIED** |
| Dual-Chip Density | `ifd.rs::decode_flcomp` | `0x0000002B` (4MB Comp1, 16MB Comp2) | **VERIFIED** |
| IFD v1 Master Rules | `ifd.rs::decode_master_rule` | `0x02170000` (Requester ID `0x0000`, R: `0x17`, W: `0x02`) | **VERIFIED** |
| IFD v2 Master Rules | `ifd.rs::decode_master_rule` | `0x00200300` (Modern PCH 100-series bit shifts) | **VERIFIED** |
