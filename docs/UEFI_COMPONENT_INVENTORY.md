# Phase 3A.1 · UEFI Upstream Component Inventory (Corrected)

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** UEFI PI Structure Analysis, Firmware Volumes, FFS, Sections & BIOS Setup  
**Milestone:** Phase 3A.1 (Architecture Correction & Reuse-First Review)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE RESEARCH DELIVERABLE  
**Scope Restriction:** Research and Architecture Design Only. Zero code changes in application source tree.

---

## 1. Executive Summary & Product Philosophy

**FriedChip+ should integrate proven components, not recreate an entire firmware-analysis ecosystem.**

The analogy is building one excellent workstation from the best parts of many existing machines—not manufacturing every component ourselves. Decades of engineering effort have already been invested by mature open-source projects (`UEFITool`, `fiano`, `IFRExtractor-RS`, `edk2`) to solve the Byzantine edge cases, non-standard vendor encapsulations, and hardware-specific quirks of UEFI firmware.

This inventory audits 14 candidate upstream repositories locally cloned under `research/third_party/`. Every repository has been inspected directly on disk for authentic commits, tracked files, licenses, build feasibility on Windows, CLI stability, and suitability for production integration versus offline test verification.

---

## 2. Upstream Repository Register

| Component | Pinned Commit SHA | Branch | License | Primary Language | Tracked Files | Upstream Role in FriedChip+ |
|---|---|---|---|---|---|---|
| **UEFITool** (`LongSoft/UEFITool`) | `de73942df908b9466c055e15108ac4e4c0b1688c` | `new_engine` | BSD-2-Clause | C++14 / C++17 | 380 | **PRIMARY_EXTRACTION_ENGINE** (`UEFIExtract` CLI Subprocess Adapter) |
| **fiano** (`linuxboot/fiano`) | `162b021e54496b89b2f9028f88234b2caec70a0c` | `main` | BSD-3-Clause | Go 1.20+ | 313 | **STRUCTURED_VALIDATION_ENGINE** (`utk table --json` Subprocess Adapter) |
| **IFRExtractor-RS** (`LongSoft/IFRExtractor-RS`) | `b1b9b36c00e3209ff0b1d2a76f10cb17e339473d` | `main` | BSD-2-Clause | Rust 2015 | 10 | **PRIMARY_IFR_ANALYZER** (Adapted AST / CLI Subprocess) |
| **edk2** (`tianocore/edk2`) | `6bee033c69e2c4050cb65c378d282b8f23ca0a36` | `master` | BSD-2-Clause-Patent | C / EDK II Build | 10,308 | **NORMATIVE_SPECIFICATION_STANDARD** (Reference Headers & Decompress.c) |
| **uefi-firmware-parser** (`theopolis/uefi-firmware-parser`) | `e9c8cb48585f988a52dc8720d44d8bf6295cf621` | `main` | MIT (bundled BSD/PD) | Python 3 / C | 82 | **OFFLINE_TEST_ORACLE_ONLY** (Differential Test Comparator) |
| **BIOSUtilities** (`platomav/BIOSUtilities`) | `f799425af93264b62a2f3c3cc5bbabf8a2226114` | `main` | GPL-3.0 | Python 3 | 38 | **OFFLINE_TEST_ORACLE_ONLY** (Quarantined Reference for Dell PFS) |
| **binwalk** (`ReFirmLabs/binwalk`) | `26713972e3f9f52dc37d4a421c4554b0bd9e82ef` | `master` | MIT | Python 3 / C | 302 | **OFFLINE_RESEARCH_REFERENCE** (Heuristic Carving Signatures) |
| **unblob** (`onekey-sec/unblob`) | `61663627c3a656752044c3864424eefd79784255` | `main` | Apache-2.0 | Python 3.10+ / Rust | 3,447 | **OFFLINE_RESEARCH_REFERENCE** (Decompression Safety Bounds) |
| **ofrak** (`redballoonsecurity/ofrak`) | `428ea75712ddfc80c6a52b8a6e46b12a4a7fa7ee` | `master` | OFRAK Community 1.1 | Python 3 / C++ | 1,071 | **STRICTLY_REJECTED** (Proprietary Non-Commercial Restriction) |
| **Universal-IFR-Extractor** (`donovan6000/Universal-IFR-Extractor`) | `8f556811080ded90c88d325210dfe44f01bda5f1` | `master` | GPL-3.0 | C++ | 13 | **STRICTLY_REJECTED** (GPL-3.0; Superseded by IFRExtractor-RS) |
| **efiSeek** (`DSecurity/efiSeek`) | `573f4b9b5ba2731cdecafc6594d0fd0570e28fff` | `master` | Apache-2.0 | Java (Ghidra) | 30 | **STATIC_DATA_REFERENCE** (Protocol GUID Catalog) |
| **efiXplorer** (`binarly-io/efiXplorer`) | `110a8d9086848da3c3798a3bff3c4c101a79986a` | `master` | GPL-3.0 | C++ (IDA Pro) | 63 | **OFFLINE_RESEARCH_REFERENCE** (GPL-3.0 Quarantined) |
| **fwhunt-scan** (`binarly-io/fwhunt-scan`) | `6fc7115959e4825199882f5b397a0d7c34cbbfd1` | `master` | GPL-3.0 | Python 3 | 25 | **OFFLINE_RESEARCH_REFERENCE** (TE Parser Arithmetic Reference) |
| **UEFI-Editor** (`BoringBoredom/UEFI-Editor`) | `44e8c9ba4a05dbd6fc4b1c247ed17948b6e58f8d` | `master` | Unlicensed | TypeScript / React | 42 | **UI_LAYOUT_REFERENCE_ONLY** (No Code Reuse) |

---

## 3. Detailed Component Technical Audits

### 3.1 LongSoft/UEFITool (New Engine)
* **Local Path:** `research/third_party/UEFITool`
* **Remote Origin:** `https://github.com/LongSoft/UEFITool`
* **Inspected Commit:** `de73942df908b9466c055e15108ac4e4c0b1688c` (branch `new_engine`)
* **License:** BSD-2-Clause (`LICENSE.md`, Copyright (c) 2015, Nikolaj Schlej). Fully permissive and safe for distribution.
* **Core Capabilities:**
  - **Read-Only Structural Parsing:** Mature, battle-tested C++ recursive parser for UEFI PI Firmware Volumes, FFS files, and nested section hierarchies.
  - **Traversals:** Exhaustive traversal of standard and FFSv3 extended volumes, file states, GUID identification, alignment padding, and deleted/corrupted file recovery.
  - **Decompression Handlers:** In-tree implementations of EFI 1.1, Tiano, LZMA SDK, and Brotli.
  - **Extraction Output:** Standalone `UEFIExtract` CLI executable decomposes firmware images into deterministic filesystem directory structures (`dump/`) containing extracted raw payloads, uncompressed sections, and structural dump files (`dump.txt`).
* **Windows Build Feasibility:**
  - `UEFIExtract` is a pure C++ CLI utility without Qt dependencies.
  - Compiles natively on Windows x64 using MSVC (`cl.exe`) or CMake (`cmake -B build && cmake --build build --config Release`).
  - Statically links C runtime (`/MT`); produces a standalone executable with zero external runtime or DLL dependencies.
* **CLI Output Stability & Machine-Readable Options:**
  - Invocation modes: `UEFIExtract image.bin dump` (extracts entire structure tree and payloads to disk) and `UEFIExtract image.bin report` (produces structural summary).
  - The generated directory tree mirrors the exact volume-file-section hierarchy, enabling deterministic ingestion into FriedChip+'s Normalized Firmware Object Model (NFOM).
* **Known Vendor-Format Limitations:**
  - While it handles almost all standard and OEM FFS variants (Intel, AMD, Apple, AMI, Phoenix, Insyde), proprietary outer update wrappers (e.g. Dell encrypted PFS capsules, Insyde signed wrappers) must be de-encapsulated before `UEFIExtract` can parse the inner BIOS volumes.
* **Role in FriedChip+:** **Primary Analysis & Bulk Extraction Backend**. Wrapped in a hardened Windows Job Object subprocess adapter.

---

### 3.2 linuxboot/fiano
* **Local Path:** `research/third_party/fiano`
* **Remote Origin:** `https://github.com/linuxboot/fiano`
* **Inspected Commit:** `162b021e54496b89b2f9028f88234b2caec70a0c` (branch `main`)
* **License:** BSD-3-Clause (`LICENSE`, Copyright (c) 2018-2019 LinuxBoot Authors). Fully permissive.
* **Core Capabilities:**
  - **FV, FFS and Section Support:** Implements clean, specification-compliant parsing of UEFI PI Firmware Volumes, Block Maps, FFS headers, and section types in Go.
  - **JSON Output Quality:** The `utk` tool provides a native JSON visitor (`pkg/visitors/json.go`) invoked via `utk image.bin table --json`. This produces a clean, machine-readable JSON object tree directly over `stdout`.
  - **Extraction Capabilities:** `utk image.bin dump <GUID>` extracts specific files or volumes.
* **Windows Build Feasibility:**
  - Go features flawless cross-compilation: `GOOS=windows GOARCH=amd64 go build -o utk.exe ./cmds/utk`.
  - Produces a single static `.exe` (~8-12 MB) with zero external runtime dependencies.
* **Read-Only vs Modification Command Separation:**
  - Fiano contains both read-only query commands (`table`, `find`, `dump`, `json`) and dangerous image mutation commands (`replace`, `insert`, `remove`).
  - **Strict Architectural Boundary:** FriedChip+'s subprocess adapter strictly executes only read-only commands (`table --json`, `dump`). The mutation commands are completely disabled/uncalled, ensuring image immutability.
* **Dependencies & Unsafe Implementation Details:**
  - Uses Go standard library + `golang.org/x/tools`. Minimal attack surface. Decompression of legacy Tiano/EFI sections relies on external helper logic or basic fallback.
* **Role in FriedChip+:** **Secondary Structured Validation Engine & Test Oracle**. Provides instant JSON trees for differential cross-checking against `UEFIExtract`.

---

### 3.3 LongSoft/IFRExtractor-RS
* **Local Path:** `research/third_party/IFRExtractor-RS`
* **Remote Origin:** `https://github.com/LongSoft/IFRExtractor-RS`
* **Inspected Commit:** `b1b9b36c00e3209ff0b1d2a76f10cb17e339473d` (branch `main`)
* **License:** BSD-2-Clause (`LICENSE`, Copyright (c) 2018, LongSoft). Safe for reuse with attribution.
* **Core Capabilities:**
  - **AST and Opcode Coverage:** Implements complete Abstract Syntax Tree (AST) representations for UEFI 2.1 through UEFI 2.8+ Internal Form Representation (IFR) opcodes (`EFI_IFR_FORM_SET_OP`, `EFI_IFR_ONE_OF_OP`, `EFI_IFR_CHECKBOX_OP`, `EFI_IFR_NUMERIC_OP`, `EFI_IFR_VARSTORE_OP`).
  - Covers legacy Framework IFR opcodes alongside modern UEFI HII packages.
* **Adaptation Effort & Missing Library Interface:**
  - Packaged upstream as a CLI binary (`src/main.rs`) using `nom = "4.2.3"`.
  - Lacks a standalone `src/lib.rs` library export.
  - **Integration Strategy:** Either wrap the compiled executable as a CLI subprocess emitting structured text/JSON or adapt its AST and parsing routines into a clean-room Rust module (`engine::uefi::ifr`) while updating `nom` to modern version and maintaining BSD-2-Clause copyright notices.
* **Unsupported or Incomplete Opcodes:**
  - Some vendor-proprietary setup extensions (e.g., custom OEM string tokens or non-standard AMI setup opcodes) are decoded as generic raw opcodes without crashing.
* **Role in FriedChip+:** **Primary IFR Setup Analyzer**. Strictly read-only; maps question IDs to NVRAM `VarStoreId` and relative `VarOffset`. Zero automated write-back.

---

### 3.4 theopolis/uefi-firmware-parser
* **Local Path:** `research/third_party/uefi-firmware-parser`
* **Remote Origin:** `https://github.com/theopolis/uefi-firmware-parser`
* **Inspected Commit:** `e9c8cb48585f988a52dc8720d44d8bf6295cf621` (branch `main`)
* **License:** MIT (`LICENSE`, Copyright (c) 2014 Teddy Reed, Hector Martin).
* **Vendor-Format Breadth:** Exceptional heuristic coverage for unpacking legacy vendor containers (Apple, Dell, HP, Insyde capsules).
* **Runtime Requirements & Packaging Feasibility:**
  - Requires Python 3 runtime and native C compiler to build `EfiCompressor.c`.
  - Bundling a complete Python runtime and native C extension into a standalone Windows desktop app introduces bloat (50MB+), fragile DLL dependencies, and slow startup times.
* **Role in FriedChip+:** **Offline Development Test Oracle Only**. Used in CI and automated differential testing scripts to verify parser fidelity across diverse OEM dumps. Not distributed in production binaries.

---

### 3.5 tianocore/edk2
* **Local Path:** `research/third_party/edk2`
* **Remote Origin:** `https://github.com/tianocore/edk2`
* **Inspected Commit:** `6bee033c69e2c4050cb65c378d282b8f23ca0a36` (branch `master`)
* **License:** BSD-2-Clause-Patent (`License.txt`).
* **Normative Structures & Compression References:**
  - Authoritative reference headers: `MdePkg/Include/Pi/PiFirmwareVolume.h`, `PiFirmwareFile.h`.
  - Authoritative reference decompression implementation: `BaseTools/Source/C/Common/Decompress.c` (EFI 1.1 and Tiano algorithms).
* **License and Redistribution Obligations:**
  - BSD-2-Clause-Patent allows full reuse of specifications, header definitions, and algorithms provided standard copyright and patent grant notices are retained.
* **Role in FriedChip+:** **Normative Specification Standard**.

---

### 3.6 Other Upstream Repositories Evaluated

* **BIOSUtilities (`platomav/BIOSUtilities`, `f799425af9`, GPL-3.0):**
  - Collection of Python extraction scripts for Dell PFS, HP, Insyde, and Phoenix capsules.
  - **Quarantine Policy:** GPL-3.0 copyleft. Strictly isolated from production builds. Retained only as an offline development reference for vendor outer capsule unpacking algorithms.
* **binwalk (`ReFirmLabs/binwalk`, `26713972e3`, MIT):**
  - Python-based firmware carving tool. Useful as an offline reference for signature patterns when scanning corrupted or non-standard firmware containers.
* **unblob (`onekey-sec/unblob`, `61663627c3`, Apache-2.0):**
  - Modern carving and extraction suite. Serves as reference for safe resource bounding (decompression bombs, maximum nesting depth).
* **OFRAK (`redballoonsecurity/ofrak`, `428ea75712`, OFRAK Community 1.1):**
  - Proprietary non-commercial educational license. **STRICTLY REJECTED**.
* **Universal-IFR-Extractor (`donovan6000/Universal-IFR-Extractor`, `8f55681108`, GPL-3.0):**
  - Legacy C++ IFR tool. **STRICTLY REJECTED** (GPL-3.0; superseded by BSD-2-Clause `IFRExtractor-RS`).
* **efiSeek (`DSecurity/efiSeek`, `573f4b9b5b`, Apache-2.0):**
  - Ghidra extension. Static reference dataset for known UEFI Protocol GUIDs.
* **efiXplorer (`binarly-io/efiXplorer`, `110a8d9086`, GPL-3.0):**
  - IDA Pro C++ plugin. Security heuristics reference; excluded from production due to GPL-3.0.
* **fwhunt-scan (`binarly-io/fwhunt-scan`, `6fc7115959`, GPL-3.0):**
  - Python security scanner. Clean reference for Terse Executable (TE) stripped header arithmetic (`uefi_te.py`).
* **UEFI-Editor (`BoringBoredom/UEFI-Editor`, `44e8c9ba4a`, Unlicensed):**
  - Web UI for setup forms. Visual inspection only for UI layout concepts.
