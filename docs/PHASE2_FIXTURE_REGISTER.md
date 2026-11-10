# FriedChip+ — Phase 2 Fixture Strategy & Asset Register

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Test Asset Register & Verification Methodology  
**Milestone:** Phase 2 Implementation Audit & Scope Freeze  
**Audit Date:** November 2026  
**Auditor:** Antigravity (Advanced Agentic Coding)  

---

## 1. Executive Summary & Fixture Methodology

To eliminate ambiguity between simulated unit tests and real-world commercial reliability, FriedChip+ formally establishes a **Three-Class Fixture Strategy**:

```
                       Three-Tier Fixture Hierarchy
 ┌────────────────────────────────────────────────────────────────────────┐
 │ Tier 3: REAL_WORLD (Authentic OEM executables & hardware dumps)       │
 │   - Highest fidelity, zero synthetic bias, tests real-world edge cases │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Tier 2: REFERENCE (Golden differential outputs from verified tools)   │
 │   - Validates parser output matches audited Python/C++ implementations │
 └───────────────────────────────────┬────────────────────────────────────┘
                                     │
 ┌───────────────────────────────────▼────────────────────────────────────┐
 │ Tier 1: SYNTHETIC (In-memory deterministic test buffers)               │
 │   - Rapid CI execution, negative boundary injection, zero licensing risk│
 └────────────────────────────────────────────────────────────────────────┘
```

> **Mandatory Quality Invariant:**  
> **A firmware format or extractor must NEVER be described as "verified" or "production-ready" simply because its Tier 1 (Synthetic) fixtures pass.** Complete format verification requires verified execution across all three fixture tiers.

---

## 2. Three Fixture Classes Defined

### Class 1: SYNTHETIC
- **Definition:** Deterministic, programmatically constructed byte buffers generated in-memory by test utilities (e.g. `SyntheticFixtureBuilder`).
- **Purpose:** Verifies basic structural math, validates bounds checking, tests negative cases (corrupted signatures, bit flips, overlapping regions, integer overflows), and executes rapidly in continuous integration pipelines without external binary files.
- **Inherent Limitation:** Susceptible to developer confirmation bias. Validates only the assumptions shared between the builder and the parser.

### Class 2: REFERENCE
- **Definition:** Golden output datasets, intermediate extracted structures, and byte-for-byte validation hashes derived from executing established, authoritative reference implementations (e.g. `platomav/BIOSUtilities` or `ALeXXBody/BIOSRecoverySuite`) on identical input sets.
- **Purpose:** Proves differential equivalence. Guarantees that FriedChip+'s clean-room Rust implementation produces identical partitions, hashes, and offsets as the reference implementations without sharing their code.
- **Inherent Limitation:** Requires maintaining reference tool environments and verified golden JSON/binary output snapshots.

### Class 3: REAL_WORLD
- **Definition:** Authentic, byte-exact binary packages obtained directly from manufacturer support CDNs (e.g. Dell support portal, HP software repository) or read directly from motherboard SPI NOR chips using external hardware programmers (CH341A, Dediprog SF100, Raspberry Pi SPI).
- **Purpose:** The ultimate standard of engineering ground truth. Tests real-world compiler variations, legacy padding idiosyncrasies, undocumented vendor flags, multi-section update payloads, and authentic platform configurations.
- **Inherent Limitation:** Cannot be committed directly to Git repositories if proprietary OEM licensing or multi-hundred-megabyte file sizes conflict with repository policies. Must be managed via managed test runners or local fixture caches.

---

## 3. Format-by-Format Fixture Register

The table below catalogs every Phase 2 firmware format, recording which fixture classes currently exist, which have been executed, and which remain missing.

| Format Identifier | Description | SYNTHETIC Status | REFERENCE Status | REAL_WORLD Status | Verification Level |
| :--- | :--- | :---: | :---: | :---: | :--- |
| **FMT-IFD** | Intel Flash Descriptor (0x0FF0A55A) | **EXISTS / TESTED** | **MISSING** | **MISSING** | **Tier 1 Only** (Synthetic Verified) |
| **FMT-DELL-PFS** | Dell PFS Update Package (R1 & R2) | **EXISTS / TESTED** | **MISSING** | **EXISTS / TESTED** | **Tier 1 + Tier 3** (Ref Differential Missing) |
| **FMT-UEFI-FV** | UEFI Firmware Volume (_FVH) | **EXISTS / TESTED** | **MISSING** | **MISSING** | **Tier 1 Only** (Scaffolding Verified) |
| **FMT-INSYDE-IFLASH** | Insyde iFlash Packaging (`$_IFLASH`) | **MISSING** | **MISSING** | **MISSING** | **Unimplemented / Frozen** |
| **FMT-AMI-PFAT** | AMI PFAT / BIOS Guard (`_AMIPFAT`) | **MISSING** | **MISSING** | **MISSING** | **Unimplemented / Frozen** |
| **FMT-DELL-THINOS** | Dell ThinOS PKG (LZMA2 / 7zXZ) | **MISSING** | **MISSING** | **MISSING** | **Unimplemented / Frozen** |

---

## 4. Detailed Audit of Existing Fixture Assets

### 4.1 Asset: Intel Flash Descriptor (IFD)
- **Synthetic Fixtures (`SyntheticFixtureBuilder`):**
  - `build_valid_ifd_image(16MB)`: Generates valid FLMAP0/1, 5 regions (Descriptor, BIOS, ME, GbE, PlatformData), and deterministic fill patterns.
  - `build_ifd_with_disabled_region()`: Sets Region 1 register to `0x00007FFF` (`limit < base`).
  - `build_ifd_with_overlapping_regions()`: Creates 4KB collision between Region 0 and Region 2.
  - `build_ifd_with_out_of_bounds_region()`: Sets 16MB limit in a 64KB image.
  - `build_ifd_with_corrupt_signature()`: Injects `0xDEADBEEF` at offset `0x10`.
- **Reference Fixtures:** **MISSING**. No golden output comparisons against `bios_analyzer.py` or `pawn` exist.
- **Real-World Fixtures:** **MISSING**. No standalone raw 8MB/16MB/32MB motherboard SPI dumps have been added to the test suite.

### 4.2 Asset: Dell PFS Update Container
- **Synthetic Fixtures (`SyntheticFixtureBuilder`):**
  - `build_synthetic_dell_pfs()`: Assembles multi-section container with ZLIB stream, 8-bit XOR header/footer, Name Info directory table (UTF-16LE), and 3 component payloads.
  - `build_corrupt_xor_dell_pfs()`: Inverts 8-bit XOR checksum byte.
  - `build_corrupt_crc_dell_pfs()`: Corrupts inverted IEEE 802.3 CRC-32 preceding `PFS.FTR.`.
- **Reference Fixtures:** **MISSING**. No automated diff comparing FriedChip+ extracted `.bin` files against `dell_pfs_extract.py` output.
- **Real-World Fixtures:**
  - **Asset:** `Latitude_5X90_1.41.0.exe` (Official Dell Latitude 5490 / 5590 BIOS update package).
  - **Source:** Dell open download catalog (`downloads.dell.com`).
  - **File Size:** ~35.4 MB.
  - **Current Location:** Transient local scratch directory (`C:/Users/xtrar/.../scratch/Latitude_5X90_1.41.0.exe`).
  - **Execution Result:** Successfully extracted 14 component payloads including System BIOS with BIOS Guard, Intel ME, and Embedded Controller.
  - **Defect Noted (DEF-01):** Hardcoded absolute path causes test to skip silently in non-local environments.

### 4.3 Asset: UEFI Firmware Volume (FV)
- **Synthetic Fixtures (`SyntheticFixtureBuilder`):**
  - `build_valid_firmware_volume(128KB)`: Builds 0x48-byte header with `_FVH` signature, single block map, and 16-bit additive inverse checksum.
- **Reference Fixtures:** **MISSING**.
- **Real-World Fixtures:** **MISSING**. No raw UEFI volumes sliced from production motherboards.

---

## 5. Action Plan for Missing Fixtures

Before any format can be elevated from `PARTIALLY IMPLEMENTED` to `FULLY VERIFIED`, the following fixtures must be registered and automated:

```text
Action 1: Relocate Real-World Dell Latitude fixture into standard fixture cache path.
          -> verify: cargo test fails cleanly with actionable message if fixture is absent.

Action 2: Create Reference Golden Output JSON for Dell Latitude 5490 using upstream `dell_pfs_extract.py`.
          -> verify: Differential test asserts 100% byte-for-byte identity of all 14 extracted partitions.

Action 3: Ingest a verified 16MB Intel SPI flash dump (e.g. Lenovo T480 / Dell 5490 SOIC-8 dump).
          -> verify: IFD parser extracts identical region boundaries and hashes as Intel FIT tool.
```
