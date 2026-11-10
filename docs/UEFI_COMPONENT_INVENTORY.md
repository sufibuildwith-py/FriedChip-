# Phase 3A · UEFI Upstream Component Inventory

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** UEFI PI Structure Analysis, Firmware Volumes, FFS, Sections & BIOS Setup  
**Milestone:** Phase 3A (Upstream Component Intelligence & Architecture Design)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE RESEARCH DELIVERABLE  
**Scope Restriction:** Research and Architecture Design Only. Zero code changes in application source tree.

---

## 1. Executive Summary

This inventory audits the 14 upstream repositories locally cloned under `research/third_party/` relevant to UEFI structure analysis, firmware volumes, file systems, section compression, executable inspection, and BIOS setup IFR disassembly.

Every repository has been inspected directly on disk for:
- Authentic remote URL, active branch, and pinned commit SHA.
- Tracked file count and working tree cleanliness.
- Actual license files, license versions, and attribution requirements.
- Core source directories, build requirements, and runtime dependencies.
- Windows compatibility and offline air-gapped usability.
- Usability role (Native Library, CLI Subprocess Adapter, Differential Test Oracle, Reference Only, or Rejected).

---

## 2. Upstream Repository Register

| Component | Pinned Commit SHA | Branch | License | Primary Language | Tracked Files | Suitability Role |
|---|---|---|---|---|---|---|
| **UEFITool** (`LongSoft/UEFITool`) | `de73942df908b9466c055e15108ac4e4c0b1688c` | `new_engine` | BSD-2-Clause | C++14 / C++17 | 380 | **REUSE_WITH_ADAPTER** (CLI) / **REFERENCE_ONLY** |
| **fiano** (`linuxboot/fiano`) | `162b021e54496b89b2f9028f88234b2caec70a0c` | `main` | BSD-3-Clause | Go 1.20+ | 313 | **REUSE_WITH_ADAPTER** (CLI) / **REFERENCE_ONLY** |
| **uefi-firmware-parser** (`theopolis/uefi-firmware-parser`) | `e9c8cb48585f988a52dc8720d44d8bf6295cf621` | `main` | MIT (bundled BSD/PublicDomain) | Python 3 / C | 82 | **ISOLATED_SUBPROCESS** (Test Oracle) / **REFERENCE_ONLY** |
| **edk2** (`tianocore/edk2`) | `6bee033c69e2c4050cb65c378d282b8f23ca0a36` | `master` | BSD-2-Clause-Patent | C / EDK II Build | 10,308 | **REFERENCE_ONLY** (Normative Specification Standard) |
| **BIOSUtilities** (`platomav/BIOSUtilities`) | `f799425af93264b62a2f3c3cc5bbabf8a2226114` | `main` | GPL-3.0 | Python 3 | 38 | **ISOLATED_SUBPROCESS** (Test Oracle Only) |
| **binwalk** (`ReFirmLabs/binwalk`) | `26713972e3f9f52dc37d4a421c4554b0bd9e82ef` | `master` | MIT | Python 3 / C | 302 | **REFERENCE_ONLY** (Carving Signatures) |
| **unblob** (`onekey-sec/unblob`) | `61663627c3a656752044c3864424eefd79784255` | `main` | Apache-2.0 | Python 3.10+ / Rust | 3,447 | **REFERENCE_ONLY** (Safety Boundaries & Carving) |
| **ofrak** (`redballoonsecurity/ofrak`) | `428ea75712ddfc80c6a52b8a6e46b12a4a7fa7ee` | `master` | OFRAK Community 1.1 | Python 3 / C++ | 1,071 | **REJECTED** (Proprietary / Commercial Restriction) |
| **IFRExtractor-RS** (`LongSoft/IFRExtractor-RS`) | `b1b9b36c00e3209ff0b1d2a76f10cb17e339473d` | `main` | BSD-2-Clause | Rust 2015 | 10 | **REUSE_WITH_ADAPTER** / **REFERENCE_ONLY** (Adapt to Rust) |
| **Universal-IFR-Extractor** (`donovan6000/Universal-IFR-Extractor`) | `8f556811080ded90c88d325210dfe44f01bda5f1` | `master` | GPL-3.0 | C++ | 13 | **REJECTED** (Superseded by IFRExtractor-RS; GPL-3.0) |
| **efiSeek** (`DSecurity/efiSeek`) | `573f4b9b5ba2731cdecafc6594d0fd0570e28fff` | `master` | Apache-2.0 | Java (Ghidra) | 30 | **REFERENCE_ONLY** (Protocol GUID Catalog) |
| **efiXplorer** (`binarly-io/efiXplorer`) | `110a8d9086848da3c3798a3bff3c4c101a79986a` | `master` | GPL-3.0 | C++ (IDA Pro) | 63 | **REFERENCE_ONLY** (Security Heuristics; GPL-3.0) |
| **fwhunt-scan** (`binarly-io/fwhunt-scan`) | `6fc7115959e4825199882f5b397a0d7c34cbbfd1` | `master` | GPL-3.0 | Python 3 | 25 | **REFERENCE_ONLY** (TE Parser & Rules; GPL-3.0) |
| **UEFI-Editor** (`BoringBoredom/UEFI-Editor`) | `44e8c9ba4a05dbd6fc4b1c247ed17948b6e58f8d` | `master` | Unlicensed / All Rights Reserved | TypeScript / React 19 | 42 | **REFERENCE_ONLY** (UI Layout Ideas Only) |

---

## 3. Detailed Component Technical Audits

### 3.1 LongSoft/UEFITool (New Engine)
- **Local Path:** `research/third_party/UEFITool`
- **Remote Origin:** `https://github.com/LongSoft/UEFITool`
- **Inspected Commit:** `de73942df908b9466c055e15108ac4e4c0b1688c` (branch `new_engine`)
- **License:** BSD-2-Clause (`LICENSE.md`, Copyright (c) 2015, Nikolaj Schlej).
- **Architecture & Build:** CMake / Meson. C++14/C++17 codebase. The engine is cleanly decoupled into `common/` (parsers and models) and frontends (`UEFITool` Qt GUI, `UEFIExtract` CLI, `UEFIFind` CLI).
- **Core Modules:**
  * `common/ffsparser.h` & `ffsparser.cpp`: Comprehensive recursive walker for UEFI PI Firmware Volumes, FFS files, and nested sections.
  * `common/ffs.h` & `ffs.cpp`: Struct definitions for `EFI_FIRMWARE_VOLUME_HEADER`, `EFI_FFS_FILE_HEADER`, `EFI_FFS_FILE_HEADER2` (FFSv3 64-bit size), and section types.
  * `common/Tiano/`: Tiano and EFI decompressor implementation.
  * `common/LZMA/`: LZMA SDK wrapper.
  * `common/brotli/`: Brotli decompressor integration.
  * `common/nvramparser.cpp`: Parses Intel VSS, AMI NVAR, and Phoenix EVSA variable stores.
  * `UEFIExtract/uefiextract_main.cpp`: CLI tool decomposing images into directory trees and emitting a structured `dump.txt`.
- **Windows Feasibility:** Fully native Windows MSVC build with zero POSIX-only dependencies. Builds clean standalone executable `UEFIExtract.exe`.
- **Strengths:** Industry gold standard for UEFI structure analysis. Unmatched handling of non-standard vendor FFS quirks, corrupted block maps, and FFSv3 extensions.
- **Weaknesses:** Written in C++; in-process binding via C++/CLI or Rust FFI introduces memory unsafety and build complexity. Best consumed as a CLI subprocess adapter or clean-room native Rust reference.

---

### 3.2 linuxboot/fiano
- **Local Path:** `research/third_party/fiano`
- **Remote Origin:** `https://github.com/linuxboot/fiano`
- **Inspected Commit:** `162b021e54496b89b2f9028f88234b2caec70a0c` (branch `main`)
- **License:** BSD-3-Clause (`LICENSE`, Copyright (c) 2018-2019 LinuxBoot Authors).
- **Architecture & Build:** Go module (`go.mod`, Go 1.20+). Clean modular design separating `pkg/uefi/` (models) and `cmds/` (CLIs).
- **Core Modules:**
  * `pkg/uefi/firmwarevolume.go`: Header validation, 16-bit word checksum verification, block map traversal, extent calculations.
  * `pkg/uefi/file.go`: FFS file header decoding, state byte analysis, GUID resolution, alignment padding.
  * `pkg/uefi/section.go`: Section headers, compression sections, GUID-defined sections.
  * `pkg/visitors/json.go`: JSON serialization visitor that traverses the entire firmware object tree and outputs structured JSON to standard output.
  * `cmds/utk/`: "UEFI ToolKit" CLI tool supporting commands: `utk bios.bin json`, `utk bios.bin table`, `utk bios.bin extract`.
- **Windows Feasibility:** Go cross-compiles cleanly to native Windows x64 (`GOOS=windows go build ./cmds/utk`). Single static binary with zero external DLL dependencies.
- **Strengths:** First-class JSON output schema! Highly readable, well-tested against modern server and client firmware. Permissive BSD-3-Clause license.
- **Weaknesses:** Embedding Go into Rust in-process via cgo is brittle, slow, and conflicts with Windows SEH and Tauri threading. Best integrated via CLI subprocess adapter (`utk json`) emitting to stdout.

---

### 3.3 theopolis/uefi-firmware-parser
- **Local Path:** `research/third_party/uefi-firmware-parser`
- **Remote Origin:** `https://github.com/theopolis/uefi-firmware-parser`
- **Inspected Commit:** `e9c8cb48585f988a52dc8720d44d8bf6295cf621` (branch `main`)
- **License:** MIT (`LICENSE`, Copyright (c) 2014 Teddy Reed, Hector Martin). Bundles Intel EDK II Tiano/EFI C compressor (BSD-2-Clause) and 7-Zip LZMA SDK (Public Domain).
- **Architecture & Build:** Python package (`setup.py`, `pyproject.toml`). Requires Python 3 and a C compiler (MSVC on Windows) to build native C extension `uefi_firmware/compression/EfiCompressor.c`.
- **Core Modules:**
  * `uefi_firmware/uefi.py`: Object-oriented Python models for `FirmwareVolume`, `FirmwareFile`, and `FirmwareSection`.
  * `uefi_firmware/guids/`: Comprehensive vendor GUID databases (`efiguids.py`, `efiguids_ami.py`, `efiguids_dell.py`, `efiguids_lenovo.py`).
  * `bin/uefi-firmware-parser`: CLI tool decomposing images and exporting JSON metadata (`-j`).
- **Windows Feasibility:** Requires Python 3 interpreter and pre-compiled native extension wheel on Windows.
- **Strengths:** Broadest format coverage across vendor update wrappers (Dell, HP, Apple, Insyde, Capsule). Extensive test fixtures in `tests/`.
- **Weaknesses:** Python runtime overhead. Not suitable for production bundling in a lightweight Tauri desktop app. Invaluable as an external development test oracle for differential regression testing.

---

### 3.4 tianocore/edk2
- **Local Path:** `research/third_party/edk2`
- **Remote Origin:** `https://github.com/tianocore/edk2`
- **Inspected Commit:** `6bee033c69e2c4050cb65c378d282b8f23ca0a36` (branch `master`)
- **License:** BSD-2-Clause-Patent (`License.txt`).
- **Architecture & Build:** The normative reference implementation of the UEFI and PI specifications. Massive C codebase.
- **Core Modules:**
  * `MdePkg/Include/Pi/PiFirmwareVolume.h`: Canonical C declarations for `EFI_FIRMWARE_VOLUME_HEADER`, `EFI_FV_BLOCK_MAP_ENTRY`, attributes, and file system GUIDs.
  * `MdePkg/Include/Pi/PiFirmwareFile.h`: Canonical declarations for `EFI_FFS_FILE_HEADER`, `EFI_FFS_FILE_HEADER2`, file types, state bits, and section types.
  * `BaseTools/Source/C/Common/Decompress.c`: Normative C implementation of the EFI 1.1 and Tiano decompression algorithms.
- **Suitability:** Pure normative reference for bitfield correctness, invariants, and clean-room Rust implementations.

---

### 3.5 LongSoft/IFRExtractor-RS
- **Local Path:** `research/third_party/IFRExtractor-RS`
- **Remote Origin:** `https://github.com/LongSoft/IFRExtractor-RS`
- **Inspected Commit:** `b1b9b36c00e3209ff0b1d2a76f10cb17e339473d` (branch `main`)
- **License:** BSD-2-Clause (`LICENSE`, Copyright (c) 2018, LongSoft).
- **Architecture & Build:** Pure Rust 2015 binary crate (`Cargo.toml`). Dependencies: `nom = "4.2.3"`, `sha256 = "1.6.0"`.
- **Core Modules:**
  * `src/main.rs`: Full AST representation of IFR opcodes, formsets, forms, questions (OneOf, Checkbox, Numeric, String), default values, and expressions.
  * `src/uefi_parser.rs`: Opcode parser for modern UEFI 2.1 through UEFI 2.8+ specifications.
  * `src/framework_parser.rs`: Opcode parser for legacy Intel Framework IFR opcodes.
- **Windows Feasibility:** 100% native Rust, compiles natively on Windows x64 with `cargo build`.
- **Strengths:** Written in Rust! Highly permissive BSD-2-Clause license (unlike older GPL-3.0 C++ extractors). Covers modern UEFI 2.8+ opcodes.
- **Weaknesses:** Packaged as an executable binary without `src/lib.rs`; uses legacy `nom 4.2.3`.
- **Integration Strategy:** We can adapt its parsing logic and AST definitions into a modern clean-room Rust module (`engine::uefi::ifr`) or build it as a standalone helper CLI binary with zero GPL copyleft risk.

---

### 3.6 Other Evaluated Tools
- **BIOSUtilities (`platomav/BIOSUtilities`, `f799425af9`, GPL-3.0):** Python extraction scripts. Dedicated differential testing oracle for vendor wrappers (Dell PFS).
- **binwalk (`ReFirmLabs/binwalk`, `26713972e3`, MIT):** Reference for heuristic file-carving signatures for non-contiguous or corrupted volumes.
- **unblob (`onekey-sec/unblob`, `61663627c3`, Apache-2.0):** Modern Python/Rust carving framework. Reference for bounded decompression limits and recursion exhaustion protection.
- **OFRAK (`redballoonsecurity/ofrak`, `428ea75712`, OFRAK Community License 1.1):** Proprietary, non-commercial educational license. **REJECTED** for redistribution or embedding.
- **Universal-IFR-Extractor (`donovan6000/Universal-IFR-Extractor`, `8f55681108`, GPL-3.0):** C++, unmaintained since 2014, GPL-3.0 copyleft. **REJECTED** in favor of BSD-2-Clause `IFRExtractor-RS`.
- **efiSeek (`DSecurity/efiSeek`, `573f4b9b5b`, Apache-2.0):** Java Ghidra extension. Reference data for UEFI Protocol GUID catalogs.
- **efiXplorer (`binarly-io/efiXplorer`, `110a8d9086`, GPL-3.0):** IDA Pro C++ plugin. Security heuristics reference; cannot be distributed due to GPL-3.0.
- **fwhunt-scan (`binarly-io/fwhunt-scan`, `6fc7115959`, GPL-3.0):** Python security scanner. Provides clean reference parser for Terse Executable (`uefi_te.py`).
- **UEFI-Editor (`BoringBoredom/UEFI-Editor`, `44e8c9ba4a`, Unlicensed):** Web-based React UI. Visual reference for displaying IFR forms and menus in React.

---
EOF
