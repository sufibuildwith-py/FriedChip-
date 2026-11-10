# Phase 3A · UEFI Upstream Component Capability Matrix

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** UEFI PI Specification Capabilities & Upstream Component Evaluation  
**Milestone:** Phase 3A (Upstream Component Intelligence & Architecture Design)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE RESEARCH DELIVERABLE  
**Scope Restriction:** Research and Architecture Design Only. Zero code changes in application source tree.

---

## 1. Evaluation Methodology & Capability Taxonomy

This matrix evaluates upstream firmware projects across the six core structural capabilities required for comprehensive UEFI firmware analysis in FriedChip+:

1. **Capability A — Firmware Volume (FV):** Header validation, word checksums, block map decomposition, alignment, erased polarity, extent calculation, nested volumes.
2. **Capability B — Firmware File System (FFS):** FFSv2 and FFSv3 (64-bit extended size), file states, header/data checksums, GUID identification, alignment padding, invalid file recovery.
3. **Capability C — Firmware Sections:** Common section headers, extended 32-bit section headers, compression encapsulation, GUID-defined sections, nested section hierarchies.
4. **Capability D — Decompression:** EFI 1.1, Tiano, LZMA (7-Zip and EDK II variants), Brotli, bounded memory buffers, decompression bomb protection.
5. **Capability E — PE32 / TE Executable Analysis:** PE32/PE32+ headers, Terse Executable (TE) headers, section table bounds, machine architecture, entry point metadata.
6. **Capability F — IFR / BIOS Setup Analysis:** Internal Form Representation opcode disassembly, formsets, forms, questions, default values, expressions, VarStore ID mapping.

---

## 2. Upstream Capability Comparison Matrix

| Component | Capability A: FV | Capability B: FFS | Capability C: Sections | Capability D: Decompression | Capability E: PE32 / TE | Capability F: IFR / Setup | Primary License | Proposed Reuse Mode |
|---|:---:|:---:|:---:|:---:|:---:|:---:|---|---|
| **LongSoft/UEFITool** (`de73942d`) | **Full** (PI 1.0-1.8, quirks) | **Full** (FFSv2/v3, states) | **Full** (Standard & Ext) | **Full** (EFI, Tiano, LZMA, Brotli) | **Full** (PE32/TE, sections) | **Partial** (NVRAM stores, no IFR AST) | BSD-2-Clause | **REUSE_WITH_ADAPTER** (CLI) / **REFERENCE_ONLY** |
| **linuxboot/fiano** (`162b021e`) | **Full** (PI 1.0-1.7, blockmaps) | **Full** (FFSv2/v3, JSON) | **Full** (Standard & Ext) | **Partial** (glzma, Tiano via external) | **Basic** (PE32 identification) | **None** | BSD-3-Clause | **REUSE_WITH_ADAPTER** (CLI) / **REFERENCE_ONLY** |
| **theopolis/uefi-firmware-parser** (`e9c8cb48`) | **Full** (Broad OEM support) | **Full** (FFSv2/v3) | **Full** (Standard & Ext) | **Full** (C extension: Tiano, LZMA) | **Full** (PE32/TE inspection) | **None** | MIT | **ISOLATED_SUBPROCESS** (Test Oracle) |
| **tianocore/edk2** (`6bee033c`) | **Normative Reference** | **Normative Reference** | **Normative Reference** | **Normative Reference** (C) | **Normative Reference** | **Normative Reference** (VFR/IFR) | BSD-2-Clause-Patent | **REFERENCE_ONLY** |
| **LongSoft/IFRExtractor-RS** (`b1b9b36c`) | **None** | **None** | **None** | **None** | **None** | **Full** (UEFI 2.1..2.8+ AST, VarStores) | BSD-2-Clause | **REUSE_WITH_ADAPTER** / **REFERENCE_ONLY** (Rust) |
| **donovan6000/Universal-IFR-Extractor** (`8f556811`) | **None** | **None** | **None** | **None** | **None** | **Partial** (UEFI 2.1..2.3 only) | GPL-3.0 | **REJECTED** (Superseded by IFRExtractor-RS) |
| **binarly-io/fwhunt-scan** (`6fc71159`) | **Basic** (Uses uefi-extractor) | **Basic** (FFS parsing) | **Basic** (Section parser) | **None** (Delegates) | **Full** (Detailed TE/PE analyzer) | **None** | GPL-3.0 | **REFERENCE_ONLY** (TE Parser) |
| **DSecurity/efiSeek** (`573f4b9b`) | **None** | **None** | **None** | **None** | **Full** (Protocol GUIDs in PE32) | **None** | Apache-2.0 | **REFERENCE_ONLY** (GUID Catalog) |
| **ReFirmLabs/binwalk** (`26713972`) | **Heuristic** (Magic carving) | **Heuristic** (File carving) | **None** | **Partial** (External tools) | **Heuristic** (PE headers) | **None** | MIT | **REFERENCE_ONLY** (Signatures) |
| **onekey-sec/unblob** (`61663627`) | **Heuristic** (Chunk carving) | **Basic** (FFS extraction) | **Basic** | **Bounded** (Zlib, LZMA) | **None** | **None** | Apache-2.0 | **REFERENCE_ONLY** (Safety limits) |
| **redballoonsecurity/ofrak** (`428ea757`) | **Full** (Python AST) | **Full** (Python AST) | **Full** | **Full** (Python adapters) | **Full** | **None** | OFRAK Community 1.1 | **REJECTED** (Proprietary restrictions) |

---

## 3. Capability Deep-Dive Evaluations

### 3.1 Capability A: Firmware Volume (FV) Traversal
- **Normative Specification:** UEFI Platform Initialization (PI) Specification Volume 3, Chapter 3.
- **Structural Invariants:**
  * Signature magic at `+0x28`: `_FVH` (`0x4856465F`).
  * 16-bit word checksum across `HeaderLength`: $\sum_{i=0}^{\text{HeaderLength}/2 - 1} \text{Word}_i \equiv 0 \pmod{2^{16}}$.
  * Block Map: Array of `{ NumBlocks: uint32, BlockLength: uint32 }` terminating in `{ 0, 0 }`. Total block count $\times$ block length must equal `FvLength`.
  * Erased Polarity: Determined by `EFI_FVB2_ERASE_POLARITY` attribute bit (1 = `0xFF`, 0 = `0x00`).
- **Best Upstream Implementations:**
  1. **UEFITool (`common/ffsparser.cpp`):** Exceptional edge-case handling. Correctly detects non-contiguous volumes, handles corrupted block maps with fallback to declared length, checks FFSv2 vs FFSv3 FileSystemGUIDs.
  2. **fiano (`pkg/uefi/firmwarevolume.go`):** Clean, idiomatic parsing of block maps and checksum validation. Emits structured JSON.
- **Recommended Strategy:**
  * **Primary in-process implementation:** Clean-room native Rust parser in `engine::uefi::firmware_volume` modeled after EDK II specs and UEFITool logic.
  * **Secondary validation oracle:** `UEFIExtract` and `utk json` executed in differential tests.

---

### 3.2 Capability B: Firmware File System (FFS) Traversal
- **Normative Specification:** UEFI PI Specification Volume 3, Chapter 3.2.
- **Structural Invariants:**
  * File Header: `EFI_FFS_FILE_HEADER` (24 bytes). Contains 16-byte `Name` GUID, `IntegrityCheck` (Header checksum and Data checksum), `Type` (`0x01` RAW to `0x0B` SMM_CORE), `Attributes`, and 3-byte `Size`.
  * FFSv3 Extended Size: If 3-byte `Size == 0xFFFFFF`, an extended 64-bit `Size` follows in `EFI_FFS_FILE_HEADER2`.
  * File States: Multi-phase state byte tracking flash write states (`HEADER_CONSTRUCTION` -> `HEADER_VALID` -> `DATA_VALID` -> `MARKED_FOR_UPDATE` -> `DELETED`).
  * Alignment: `Attributes` bits [3:5] define power-of-two byte alignment (1 byte to 64KB).
- **Best Upstream Implementations:**
  1. **UEFITool (`common/ffsparser.cpp:parseFfsFile`):** Full state tracking, handles deleted files, recovers orphaned files in corrupted padding regions.
  2. **fiano (`pkg/uefi/file.go`):** Clean separation of file types, attributes, and alignment padding calculations.
- **Recommended Strategy:**
  * Clean-room native Rust implementation in `engine::uefi::ffs`. Reference UEFITool for deleted file heuristics and FFSv3 64-bit extensions.

---

### 3.3 Capability C: Firmware Sections
- **Normative Specification:** UEFI PI Specification Volume 3, Chapter 3.3.
- **Structural Invariants:**
  * Header: `EFI_COMMON_SECTION_HEADER` (4 bytes: 3-byte `Size`, 1-byte `Type`).
  * Extended Header: If 3-byte `Size == 0xFFFFFF`, an extended 32-bit `Size` follows in `EFI_COMMON_SECTION_HEADER2`.
  * Section Types:
    - Encapsulation: `0x14` COMPRESSION, `0x15` GUID_DEFINED, `0x17` FIRMWARE_VOLUME_IMAGE.
    - Leaf: `0x10` PE32, `0x11` PIC, `0x12` TE, `0x18` RAW, `0x19` USER_INTERFACE (Unicode string), `0x1B` VERSION.
- **Best Upstream Implementations:**
  1. **UEFITool (`common/ffsparser.cpp:parseSection`):** Handles recursive descent of nested compression sections and nested firmware volumes cleanly. Preserves unknown sections without aborting.
  2. **uefi-firmware-parser (`uefi_firmware/uefi.py`):** Recursive descent with rich metadata logging.
- **Recommended Strategy:**
  * Clean-room native Rust recursive parser in `engine::uefi::section`. Enforce strict recursion depth limits ($\le 16$) to prevent stack overflow on adversarial inputs.

---

### 3.4 Capability D: Decompression Handlers
- **Algorithms:**
  * **EFI 1.1 / Standard Compression (`0x01`):** LZSS + Huffman coding.
  * **Tiano Compression (`0x02`):** Modified Huffman parameters and LZSS window.
  * **LZMA Compression (`GUID: EE4E5898-3914-4259-9D6E-DC7BD79403CF`):** Standard 7-Zip LZMA stream.
  * **Brotli Compression (`GUID: 3D532050-5CDA-4FD0-879E-0F7F630D5D1B`):** RFC 7932.
- **Security & Reliability Vulnerabilities:**
  * Decompression bombs (e.g., small 1KB stream expanding to 4GB of zeros).
  * Integer overflow on 32-bit unpacked size fields.
  * Invalid Huffman tables causing infinite loops or out-of-bounds array reads.
- **Best Upstream Implementations:**
  1. **EDK II (`BaseTools/Source/C/Common/Decompress.c`):** Normative reference for Tiano/EFI.
  2. **UEFITool (`common/Tiano/TianoDecompress.c`):** Clean standalone C implementation of Tiano.
  3. **Rust Ecosystem (`lzma-rs`, `brotli`):** Pure Rust, memory-safe decoders for LZMA and Brotli.
- **Recommended Strategy:**
  * Implement clean-room pure Rust Tiano/EFI decompressor (`engine::uefi::decompression::tiano`) based on EDK II specification.
  * Integrate pure Rust `lzma-rs` for LZMA decompression.
  * Enforce hard decompression limits: maximum unpacked size capped at 128 MB per section; maximum expansion ratio capped at 50:1.

---

### 3.5 Capability E: PE32 / TE Executable Inspection
- **Normative Specification:** Microsoft PE/COFF Specification & UEFI PI Specification Volume 1 (TE Format).
- **Key Inspection Metadata:**
  * Machine Architecture (x86 32-bit `0x014C`, x86_64 `0x8664`, AArch64 `0xAA64`, ARM `0x01C2`).
  * Subsystem (`EFI_APPLICATION = 10`, `EFI_BOOT_SERVICE_DRIVER = 11`, `EFI_RUNTIME_DRIVER = 12`).
  * Entry Point RVA and Image Base.
  * Section Headers (`.text`, `.data`, `.rdata`, `.reloc`).
  * TE Stripped Header Reconstruction: TE removes DOS stub and COFF header; image base and code offset must be adjusted by `StrippedSize - sizeof(EFI_TE_IMAGE_HEADER)`.
- **Best Upstream Implementations:**
  1. **UEFITool (`common/peimage.cpp`):** Robust parsing of both PE32 and TE images, extracts subsystem, machine, and section tables.
  2. **binarly-io/fwhunt-scan (`fwhunt_scan/uefi_te.py`):** Cleanest mathematical description of TE header stripping and virtual address mapping.
  3. **Rust Ecosystem (`pelite` / `goblin`):** Zero-copy, panic-free Rust PE parsers.
- **Recommended Strategy:**
  * Clean-room native Rust parser in `engine::uefi::pe_te`. Use pure Rust arithmetic to validate headers and compute entry point offsets without executing code.

---

### 3.6 Capability F: IFR / BIOS Setup Disassembly
- **Normative Specification:** UEFI Specification Chapter 32 & 33 (HII and IFR).
- **Core Requirements:**
  * Parse HII Package Lists extracted from Freeform/Raw sections.
  * Decode IFR opcodes: `EFI_IFR_FORM_SET_OP` (`0x0E`), `EFI_IFR_FORM_OP` (`0x01`), `EFI_IFR_ONE_OF_OP` (`0x05`), `EFI_IFR_CHECKBOX_OP` (`0x06`), `EFI_IFR_NUMERIC_OP` (`0x07`), `EFI_IFR_VARSTORE_OP` (`0x26`).
  * Map Question IDs to NVRAM `VarStoreId` and byte offsets (`VarOffset`).
  * Build an interactive menu tree showing default values, options, and hidden setup questions (e.g. Overclocking, CFG Lock, Above 4G Decoding).
- **Best Upstream Implementations:**
  1. **LongSoft/IFRExtractor-RS (`src/uefi_parser.rs`, `src/main.rs`):** Modern Rust AST, covers UEFI 2.1 through UEFI 2.8+ opcodes. BSD-2-Clause.
  2. **Universal-IFR-Extractor:** C++, GPL-3.0, unmaintained. Superseded by IFRExtractor-RS.
- **Recommended Strategy:**
  * Adapt `IFRExtractor-RS` AST and opcode parsing logic into a clean-room Rust module (`engine::uefi::ifr`) or integrate it as an isolated CLI subprocess adapter emitting JSON.
  * Strictly enforce read-only offline inspection: display settings, options, and NVRAM variable offsets, with **zero automated NVRAM patching or modification**.

---

## 4. Reuse & Integration Recommendations Summary

| Subsystem Component | Primary Upstream Reference | Proposed Integration Mode | Justification |
|---|---|---|---|
| **Firmware Volume Walker** | `LongSoft/UEFITool`, `edk2` | **Clean-Room Native Rust** | Core engine loop; requires zero-allocation performance, tight error propagation, and memory safety. |
| **FFS & Section Parser** | `LongSoft/UEFITool`, `linuxboot/fiano` | **Clean-Room Native Rust** | Essential for populating FriedChip+'s Normalized Object Model. |
| **EFI / Tiano Decompressor** | `edk2/Decompress.c`, `UEFITool/Tiano` | **Clean-Room Native Rust** | Bounded Huffman/LZSS decoder in pure Rust eliminates C/C++ memory corruption risks. |
| **LZMA Decompressor** | `lzma-rs` (Rust crate) | **REUSE_DIRECTLY (Rust Crate)** | Mature, memory-safe, pure Rust LZMA decoder. |
| **Full Image Decomposer & Benchmark** | `LongSoft/UEFITool` (`UEFIExtract`) | **REUSE_WITH_ADAPTER (Isolated Subprocess)** | Gold standard CLI backend for bulk file dumping and differential regression testing. |
| **Tree JSON Serializer** | `linuxboot/fiano` (`utk json`) | **ISOLATED_SUBPROCESS (Test Oracle)** | Provides independent verification of volume hierarchies and GUID lists. |
| **PE32 / TE Inspector** | `fwhunt-scan/uefi_te.py`, `pelite` | **Clean-Room Native Rust** | Lightweight metadata parser (architecture, entry point, sections). |
| **IFR / Setup Disassembler** | `LongSoft/IFRExtractor-RS` | **REUSE_WITH_ADAPTER / Adapted Rust Module** | BSD-2-Clause, written in Rust, complete UEFI 2.8+ opcode AST. |

---
EOF
