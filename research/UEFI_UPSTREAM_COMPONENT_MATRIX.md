# Phase 3A.1 · UEFI Upstream Component Capability Matrix (Corrected)

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** UEFI PI Specification Capabilities, Invariant Validation & Upstream Reuse  
**Milestone:** Phase 3A.1 (Architecture Correction & Reuse-First Review)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE RESEARCH DELIVERABLE  
**Scope Restriction:** Research and Architecture Design Only. Zero code changes in application source tree.

---

## 1. Evaluation Methodology & Capability Taxonomy

This matrix evaluates upstream firmware projects across the six core structural capabilities defined by the **UEFI Platform Initialization (PI) Specification 1.10** and the **UEFI Specification 2.10**:

1. **Capability A — Firmware Volume (FV):** Header validation, 16-bit inverted word checksums, block map decomposition, alignment, erased polarity, extent calculation, candidate scanning vs verified discovery, nested volumes.
2. **Capability B — Firmware File System (FFS):** FFSv2 vs FFSv3 (64-bit `ExtendedSize` via `FFS_ATTRIB_LARGE_FILE`), header 8-byte alignment vs data alignment (1B to 16MB), file states, header checksum (with `State` and `FileChecksum` as zero) and data checksum (`0xAA` vs valid sum), GUID resolution.
3. **Capability C — Firmware Sections:** Common section headers (24-bit size), extended section headers (32-bit size when `Size == 0xFFFFFF`), encapsulation sections (`0x01` Compression, `0x02` GUID-defined, `0x17` FV Image), leaf sections (`0x10` PE32, `0x12` TE, `0x14` Version, `0x15` UI, `0x19` Raw), recursion bounds.
4. **Capability D — Decompression:** EFI 1.1, Tiano, LZMA (7-Zip and EDK II variants), Brotli, bounded memory buffers, decompression bomb protection.
5. **Capability E — PE32 / TE Executable Analysis:** Non-executing PE32/PE32+ headers, Terse Executable (TE) stripped header mathematical reconstruction, subsystem classification, machine architecture, entry point RVA.
6. **Capability F — IFR / BIOS Setup Analysis:** Internal Form Representation opcode disassembly, formsets, forms, questions, default values, expressions, VarStore ID mapping with strictly relative `VarOffset` (non-physical).

---

## 2. Upstream Capability Comparison Matrix

| Component | Capability A: FV | Capability B: FFS | Capability C: Sections | Capability D: Decompression | Capability E: PE32 / TE | Capability F: IFR / Setup | Primary License | Upstream Integration Role |
|---|:---:|:---:|:---:|:---:|:---:|:---:|---|---|
| **LongSoft/UEFITool** (`de73942d`) | **Full** (PI 1.0-1.10, quirks) | **Full** (FFSv2/v3, states) | **Full** (Standard & Ext) | **Full** (EFI, Tiano, LZMA, Brotli) | **Full** (PE32/TE, sections) | **Partial** (NVRAM stores, no IFR AST) | BSD-2-Clause | **PRIMARY_EXTRACTION_ENGINE** (`UEFIExtract` CLI Subprocess Adapter) |
| **linuxboot/fiano** (`162b021e`) | **Full** (PI 1.0-1.10, blockmaps) | **Full** (FFSv2/v3, JSON) | **Full** (Standard & Ext) | **Partial** (External/glzma) | **Basic** (PE32 identification) | **None** | BSD-3-Clause | **STRUCTURED_VALIDATION_ENGINE** (`utk table --json` Subprocess Adapter) |
| **LongSoft/IFRExtractor-RS** (`b1b9b36c`) | **None** | **None** | **None** | **None** | **None** | **Full** (UEFI 2.1..2.8+ AST, VarStores) | BSD-2-Clause | **PRIMARY_IFR_ANALYZER** (Adapted AST / CLI Subprocess) |
| **tianocore/edk2** (`6bee033c`) | **Normative Reference** | **Normative Reference** | **Normative Reference** | **Normative Reference** (`Decompress.c`) | **Normative Reference** | **Normative Reference** (VFR/IFR) | BSD-2-Clause-Patent | **NORMATIVE_SPECIFICATION_STANDARD** |
| **theopolis/uefi-firmware-parser** (`e9c8cb48`) | **Full** (Broad OEM support) | **Full** (FFSv2/v3) | **Full** (Standard & Ext) | **Full** (Tiano, LZMA C ext) | **Full** (PE32/TE inspection) | **None** | MIT | **OFFLINE_TEST_ORACLE_ONLY** (Differential Test Comparator) |
| **platomav/BIOSUtilities** (`f799425af`) | **Vendor Specific** | **None** | **None** | **Vendor Specific** | **None** | **None** | GPL-3.0 | **OFFLINE_TEST_ORACLE_ONLY** (Quarantined Reference for Dell PFS) |
| **binarly-io/fwhunt-scan** (`6fc71159`) | **Basic** | **Basic** | **Basic** | **None** | **Full** (TE arithmetic) | **None** | GPL-3.0 | **OFFLINE_RESEARCH_REFERENCE** (TE Parser Arithmetic Reference) |
| **DSecurity/efiSeek** (`573f4b9b`) | **None** | **None** | **None** | **None** | **Full** (Protocol GUIDs) | **None** | Apache-2.0 | **STATIC_DATA_REFERENCE** (Protocol GUID Catalog) |
| **ReFirmLabs/binwalk** (`26713972`) | **Heuristic** | **Heuristic** | **None** | **Partial** | **Heuristic** | **None** | MIT | **OFFLINE_RESEARCH_REFERENCE** (Carving Signatures) |
| **onekey-sec/unblob** (`61663627`) | **Heuristic** | **Basic** | **Basic** | **Bounded** | **None** | **None** | Apache-2.0 | **OFFLINE_RESEARCH_REFERENCE** (Decompression Safety Bounds) |
| **redballoonsecurity/ofrak** (`428ea757`) | **Full** | **Full** | **Full** | **Full** | **Full** | **None** | OFRAK Community 1.1 | **STRICTLY_REJECTED** (Proprietary Non-Commercial Restriction) |
| **donovan6000/Universal-IFR-Extractor** (`8f556811`) | **None** | **None** | **None** | **None** | **None** | **Partial** (UEFI 2.1..2.3 only) | GPL-3.0 | **STRICTLY_REJECTED** (GPL-3.0; Superseded by IFRExtractor-RS) |

---

## 3. Capability Deep-Dive Evaluations & Corrected Specifications

### 3.1 Capability A: Firmware Volume (FV) Discovery & Traversal
* **Normative Authority:** UEFI PI Specification 1.10 Volume 3, Chapter 3.
* **Structural Invariants:**
  - Signature magic at byte offset `+0x28`: `_FVH` (`0x4856465F`).
  - `HeaderLength` must be $\ge 0x38$ (base header size) plus the size of the block map entries.
  - 16-bit inverted zero-sum word checksum across `HeaderLength`:
    $$\sum_{i=0}^{\frac{\text{HeaderLength}}{2} - 1} \text{Word}_i \equiv 0 \pmod{2^{16}}$$
  - Block Map: Array of `EFI_FV_BLOCK_MAP_ENTRY` (`{ NumBlocks: UINT32, Length: UINT32 }`) terminating strictly with sentinel `{ 0, 0 }`.
  - Invariant: $\sum (\text{NumBlocks} \times \text{Length}) == \text{FvLength}$.
  - Erased Polarity: Determined by `EFI_FVB2_ERASE_POLARITY` attribute bit (1 = `0xFF`, 0 = `0x00`).
* **Corrected Discovery Principles (Candidate Scanning vs Verified Discovery):**
  - **No Mandatory Fixed Alignment:** Firmware Volumes are **not** universally located at 4 KiB or 64 KiB alignment. While top-level volumes in SPI flash are often block-aligned, nested volumes inside sections (e.g. `EFI_SECTION_FIRMWARE_VOLUME_IMAGE`) or vendor update capsules can begin at non-aligned byte offsets (such as `+0x04` or `+0x18` following section headers).
  - **Candidate Scanning:** Scanning for `_FVH` at candidate `offset + 0x28` is a heuristic candidate search that produces false positives (binary code, strings, text documents containing `_FVH`).
  - **Verified Discovery Filter:** A candidate is confirmed as a valid Firmware Volume only if it passes all six verification gates:
    1. Signature `_FVH` matches at `candidate_offset + 0x28`.
    2. `HeaderLength >= 0x38` and does not exceed container bounds.
    3. `FileSystemGuid` matches known specification GUIDs (`EFI_FIRMWARE_FILE_SYSTEM2_GUID`, `EFI_FIRMWARE_FILE_SYSTEM3_GUID`, or vendor NVRAM filesystem GUIDs).
    4. Block map terminates with `{0, 0}` and sum of blocks exactly equals `FvLength`.
    5. Inverted 16-bit word checksum over `HeaderLength` evaluates to zero.
    6. Physical extent containment: `candidate_offset + FvLength <= container_size`.
* **Upstream Integration Decision:**
  - **Primary Engine:** `LongSoft/UEFITool` (`UEFIExtract`). Its C++ engine (`common/ffsparser.cpp`) handles corrupted block maps, relaxed alignment, and vendor quirks reliably.
  - **Validation Engine:** `linuxboot/fiano` (`utk table --json`).

---

### 3.2 Capability B: Firmware File System (FFS) Traversal
* **Normative Authority:** UEFI PI Specification 1.10 Volume 3, Chapter 3.2.
* **Structural Invariants:**
  - Standard Header: `EFI_FFS_FILE_HEADER` (24 bytes). Contains 16-byte `Name` GUID, `IntegrityCheck` (Header checksum and File data checksum), `Type` (`0x01` RAW to `0x0B` SMM_CORE), `Attributes` (`UINT8`), and 3-byte `Size` (`UINT8[3]`).
* **Corrected FFSv3 Extended Size Specification:**
  - **Critical Correction:** `Size == 0xFFFFFF` is **not** the discriminator for FFSv3 file headers! (That rule applies only to section headers).
  - In PI Specification 1.10 §3.2.3, when `FFS_ATTRIB_LARGE_FILE` (`0x01`) is set in `Attributes`, the 24-bit `Size[3]` field in `EFI_FFS_FILE_HEADER` **must be set to zero** (`{0x00, 0x00, 0x00}`).
  - The actual 64-bit size is stored in `EFI_FFS_FILE_HEADER2.ExtendedSize` (`UINT64`), which immediately follows the standard header.
  - Filesystem GUID Distinction:
    * `EFI_FIRMWARE_FILE_SYSTEM2_GUID`: `8C8CE578-8A3D-4F1C-9935-896185C32DD3` (standard FFS2, 24-bit size, max 16 MiB).
    * `EFI_FIRMWARE_FILE_SYSTEM3_GUID`: `54744AE0-DC3A-450F-B44E-3D49E1804FCA` (FFS3, supports `EFI_FFS_FILE_HEADER2` with 64-bit `ExtendedSize`).
* **Corrected FFS Alignment Specification:**
  - **Header Alignment:** Every FFS file header **must always be aligned on an 8-byte boundary** relative to the base of the firmware volume.
  - **Data Alignment:** The file data (immediately following the header) must satisfy the alignment requested by `Attributes` bits [3:5] (`FFS_ATTRIB_DATA_ALIGNMENT`):
    * `0b000` (0): 1-byte alignment
    * `0b001` (1): 16-byte alignment
    * `0b010` (2): 128-byte alignment
    * `0b011` (3): 512-byte alignment
    * `0b100` (4): 1 KiB alignment
    * `0b101` (5): 4 KiB alignment
    * `0b110` (6): 32 KiB alignment
    * `0b111` (7): 64 KiB alignment
    * In FFS3, `FFS_ATTRIB_DATA_ALIGNMENT_2` (`0x02` in `Attributes2` / bit 1) extends alignments from 128 KiB up to 16 MiB.
  - Traversal must separate header boundary progression (8 bytes) from data padding insertion.
* **Corrected FFS Checksum Algorithm:**
  - **Header Checksum:** An 8-bit sum over all bytes in `EFI_FFS_FILE_HEADER` (or `EFI_FFS_FILE_HEADER2`).
    * **Crucial Rule:** The `State` field (`UINT8`) and the `IntegrityCheck.Checksum.File` field (`UINT8`) **must be treated as `0x00`** during both calculation and verification of the header checksum.
    * The sum of all header bytes (with `State = 0` and `FileChecksum = 0`) modulo 256 must equal zero:
      $$\sum_{\text{header bytes}} b \equiv 0 \pmod{256}$$
    * Reason: `State` bits transition from `1` to `0` in physical flash as states advance, so `State` is excluded from checksumming.
  - **File Data Checksum:**
    * If `FFS_ATTRIB_CHECKSUM` (`0x40`) is set in `Attributes`, `IntegrityCheck.Checksum.File` must equal the 8-bit sum across all file data bytes (sum modulo 256 is 0).
    * If `FFS_ATTRIB_CHECKSUM` is **not** set, `IntegrityCheck.Checksum.File` **must be set to `0xAA`** (`FFS_FIXED_CHECKSUM`).
* **Upstream Integration Decision:**
  - **Primary Engine:** `UEFIExtract` (handles state transitions, deleted file parsing, corrupted padding recovery).
  - **Validation Engine:** `fiano` (`pkg/uefi/file.go`).

---

### 3.3 Capability C: Firmware Section Traversal
* **Normative Authority:** UEFI PI Specification 1.10 Volume 3, Chapter 3.3.
* **Corrected Section Types (PI Specification 1.10):**
  - `EFI_SECTION_COMPRESSION` = `0x01` (Encapsulation: uncompressed payload)
  - `EFI_SECTION_GUID_DEFINED` = `0x02` (Encapsulation: GUID-defined vendor processing)
  - `EFI_SECTION_DISPOSABLE` = `0x03` (Encapsulation: disposable during boot)
  - `EFI_SECTION_PE32` = `0x10` (Leaf: PE32 / PE32+ executable image)
  - `EFI_SECTION_PIC` = `0x11` (Leaf: Position Independent Code)
  - `EFI_SECTION_TE` = `0x12` (Leaf: Terse Executable image)
  - `EFI_SECTION_DXE_DEPEX` = `0x13` (Leaf: DXE dependency expression)
  - `EFI_SECTION_VERSION` = `0x14` (Leaf: build version string)
  - `EFI_SECTION_USER_INTERFACE` = `0x15` (Leaf: human-readable UTF-16 Unicode file title)
  - `EFI_SECTION_COMPATIBILITY16` = `0x16` (Leaf: 16-bit legacy CSM option ROM / payload)
  - `EFI_SECTION_FIRMWARE_VOLUME_IMAGE` = `0x17` (Encapsulation: nested Firmware Volume)
  - `EFI_SECTION_FREEFORM_SUBTYPE_GUID` = `0x18` (Leaf: GUID-identified raw blob or HII package)
  - `EFI_SECTION_RAW` = `0x19` (Leaf: unformatted binary data)
  - `EFI_SECTION_PEI_DEPEX` = `0x1B` (Leaf: PEI dependency expression)
  - `EFI_SECTION_MM_DEPEX` = `0x1C` (Leaf: SMM / MM dependency expression)
* **Section Header Layouts:**
  - Standard Section Header: `EFI_COMMON_SECTION_HEADER` (4 bytes: 3-byte `Size`, 1-byte `Type`).
  - Extended Section Header: In section headers (unlike FFSv3 file headers), if `Size[3] == 0xFFFFFF`, the true size is stored in `EFI_COMMON_SECTION_HEADER2.ExtendedSize` (`UINT32`).
* **Upstream Integration Decision:**
  - Handled directly by `UEFIExtract` and `utk`.

---

### 3.4 Capability D: Decompression Engine
* **Supported Algorithms:**
  - Standard EFI 1.1 Compression (`0x01`): LZSS + standard Huffman coding.
  - Tiano Compression (`0x02`): Modified Huffman coding and LZSS window (TianoCore standard).
  - LZMA Compression (`EE4E5898-3914-4259-9D6E-DC7BD79403CF`): 7-Zip LZMA stream.
  - Brotli Compression (`3D532050-5CDA-4FD0-879E-0F7F630D5D1B`): RFC 7932.
* **Upstream Integration Decision:**
  - **Primary Extraction Engine:** `UEFIExtract` natively executes all four decompressors in C++ inside a process-isolated Windows Job Object.
  - **Normative Reference:** `edk2/BaseTools/Source/C/Common/Decompress.c`.

---

### 3.5 Capability E: PE32 / TE Executable Inspection
* **Normative Authority:** Microsoft PE/COFF Specification & UEFI PI Specification Volume 1 (TE Format).
* **Key Metadata Inspection (Zero Execution):**
  - Machine Architecture: x86 (`0x014C`), x86_64 (`0x8664`), AArch64 (`0xAA64`), ARM Thumb (`0x01C2`).
  - Subsystem: `EFI_APPLICATION = 10`, `EFI_BOOT_SERVICE_DRIVER = 11`, `EFI_RUNTIME_DRIVER = 12`.
  - Entry Point RVA, Image Base, and Section Headers (`.text`, `.data`, `.rdata`, `.reloc`).
  - **Terse Executable (TE) Reconstruction:** Strips DOS stub and COFF header; true virtual addresses and entry point offsets require mathematical reconstruction using `StrippedSize - sizeof(EFI_TE_IMAGE_HEADER)`.
* **Upstream Integration Decision:**
  - Traversal and extraction delegated to `UEFIExtract`.
  - Rust metadata extraction uses established open-source parser crates (`pelite` / `goblin`) or reference algorithms from `fwhunt-scan` (`uefi_te.py`).

---

### 3.6 Capability F: IFR / BIOS Setup Disassembly
* **Normative Authority:** UEFI Specification 2.10 Chapters 32 & 33 (HII and IFR).
* **Corrected IFR Offset Principles:**
  - **Critical Rule:** In an IFR question (`EFI_IFR_ONE_OF`, `EFI_IFR_NUMERIC`, `EFI_IFR_CHECKBOX`), the `VarOffset` field represents a relative byte offset **within the storage buffer of the associated variable store** (`VarStoreId`).
  - **Not a Physical Flash Address:** `VarOffset` is **not** an absolute SPI flash offset and **not** a physical NVRAM address.
  - IFR bytecode does **not** establish where a variable resides in physical SPI flash.
  - Physical flash variable mapping requires independent NVRAM engine parsing: matching the variable store's `Name` and `Guid` against the active variable database (UEFI Fault Tolerant / VSS / EVSA / NVAR) in physical flash.
* **Upstream Integration Decision:**
  - **Primary Engine:** `LongSoft/IFRExtractor-RS` (`b1b9b36c`). Clean BSD-2-Clause license, full AST coverage for modern UEFI 2.8+ forms.
  - Adapted as a read-only setup viewer; automated write-back or NVRAM modification is strictly excluded.

---

## 4. Upstream Component Reuse Recommendations Summary

| Subsystem Component | Selected Upstream Component | License | Proposed Reuse Mode | Strategic Justification |
|---|---|---|---|---|
| **Firmware Volume & FFS Traversal** | `LongSoft/UEFITool` (`UEFIExtract`) | BSD-2-Clause | **Subprocess Adapter (Windows Job Object)** | Over 10 years of maturity; handles corrupted block maps, non-standard alignment, and OEM quirks flawlessly. |
| **Structured Tree Verification** | `linuxboot/fiano` (`utk table --json`) | BSD-3-Clause | **Subprocess Adapter (Differential Oracle)** | Native JSON output allows instantaneous cross-engine differential validation of volume trees and GUIDs. |
| **Decompression Service** | `LongSoft/UEFITool` (`UEFIExtract`) | BSD-2-Clause | **Subprocess Adapter (Hardened Process)** | In-tree EFI 1.1, Tiano, LZMA, and Brotli decoders executed with zero-copy isolation. |
| **IFR Setup AST Disassembler** | `LongSoft/IFRExtractor-RS` | BSD-2-Clause | **Adapted Rust Module / CLI Subprocess** | Written in Rust; complete AST for modern setup forms and questions; permissive BSD license. |
| **Normative Bitfield Specs** | `tianocore/edk2` (`BaseTools`) | BSD-2-Clause-Patent | **Specification Standard** | Canonical reference for header structs, checksums, and compression math. |
| **Offline Test Oracles** | `theopolis/uefi-firmware-parser`, `BIOSUtilities` | MIT / GPL-3.0 | **Offline Test Oracles Only** | Broad OEM fixture validation during development; strictly excluded from production binaries. |

---
EOF