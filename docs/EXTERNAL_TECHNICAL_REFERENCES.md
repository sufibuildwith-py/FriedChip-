# FriedChip+ — External Firmware Reference Engineering Audit
## Technical Reference Dossier & Source Inspection Report

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author:** Antigravity (Advanced Agentic Coding)  
**Lead Architect & Creator:** Sufiyan Khan  
**Classification:** Research & Technical Reference Specification  
**Status:** Complete Audit (Phase 2 Architectural Reference)  

---

## 1. Executive Summary & Audit Scope

This document provides a technical audit of four public reference firmware repositories to guide the independent, clean-room implementation of the **FriedChip+** Rust firmware engine (`engine-binary`, `engine-uefi`, `engine-vendor`).

In accordance with strict clean-room engineering standards:
1. **Zero Source Code Copying:** No source code, binary patches, proprietary firmware blobs, or assets are copied into FriedChip+.
2. **Behavioral & Structural Inspection Only:** Code is inspected solely to identify binary layout specifications, mathematical checksums, header offsets, decompression algorithms, and failure boundary conditions.
3. **No Security Circumvention:** All password bypasses, security-defeat scripts, and unauthorized credential tools in the inspected repositories are documented as excluded from product scope.
4. **No Kernel Drivers / Live Host Memory Access:** Windows kernel drivers, physical memory mapping (`/dev/mem`, WinRing0), and live host tampering are rejected in favor of safe, non-destructive, offline file analysis.

---

## 2. Repository Audits

### 2.1 Repository A: `platomav/BIOSUtilities`

- **Repository:** `https://github.com/platomav/BIOSUtilities`
- **Inspected Commit SHA:** `f799425af93264b62a2f3c3cc5bbabf8a2226114`
- **Primary Language:** Python 3 (with `ctypes`, `re`, `lzma`, `zlib`)
- **License:** BSD 2-Clause + Patent License (Copyright © 2019–2025 Plato Mavropoulos)

#### Inspected Source Files & Technical Findings

##### 1. Dell PFS & ThinOS PKG Extractor
- **Exact File:** `biosutilities/dell_pfs_extract.py` (lines 40–360), `biosutilities/common/patterns.py` (lines 35–45)
- **Relevant Structures & Functions:**
  - `DellPfsHeader`: `Tag` (`PFS.HDR.`), `HeaderVersion` (`UINT32`), `PayloadSize` (`UINT32`). Total 0x10 bytes.
  - `DellPfsFooter`: `PayloadSize` (`UINT32`), `Checksum` (`UINT32`, negated CRC-32 with initial vector 0), `Tag` (`PFS.FTR.`). Total 0x10 bytes.
  - `DellPfsEntryBase`: `GUID` (16 bytes, Little-Endian `UINT32 * 4`), `HeaderVersion` (`UINT32`, 1 or 2), `VersionType` (`UINT8 * 4`), `Version` (`UINT16 * 4`), `Reserved` (`UINT64`), `DataSize` (`UINT32`), `DataSigSize` (`UINT32`), `DataMetSize` (`UINT32`), `DataMetSigSize` (`UINT32`). Total 0x38 bytes.
  - `DellPfsEntryR1`: `DellPfsEntryBase` + `Unknown` (`UINT32 * 4`, total 0x48 bytes).
  - `DellPfsEntryR2`: `DellPfsEntryBase` + `Unknown` (`UINT32 * 8`, total 0x58 bytes).
  - `DellPfsPfatMetadata`: `Address` (`UINT32`), `Unknown0` (`UINT32`), `Offset` (`UINT32`), `DataSize` (`UINT32`), `Unknown1` (`UINT32`), `Unknown2` (`UINT32`), `Unknown3` (`UINT8`). Total 0x19 bytes.
  - `_is_pfs_hdr`: Pattern `\xEE\xAA\x76\x1B\xEC\xBB\x20\xF1\xE6\x51.\x78\x9C` (ZLIB stream header with 8-bit XOR checksum byte).
  - `_is_pfs_ftr`: Pattern `\xEE\xAA\xEE\x8F\x49\x1B\xE8\xAE\x14\x37\x90` followed by 8-bit XOR checksum byte.
  - `_is_pfs_pkg`: ThinOS PKG header starting with `\x13\x55\x00` at offset -1 (`0x72` RAW or `0x73` CAB), with length at +0x10 and payload at +0x30 compressed with LZMA2 (`7zXZ`).
- **What It Establishes:**
  - Modern Dell update executables wrap one or more ZLIB-compressed sections demarcated by signature sequences `EEAA761B...` and `EEAAEE8F...`.
  - Sections of type `AA` contain the raw PFS container; type `BB` contains utility binaries.
  - The inner PFS container is a sequential table of firmware entries (System BIOS, Intel ME, Embedded Controller, Microcode, TPM, Diagnostics) indexed by GUID and metadata headers.
  - Footer verification requires bitwise NOT of standard IEEE 802.3 CRC-32 with vector 0 (`~crc32(payload, 0)`).
- **What Remains Unverified:**
  - Heuristic naming of entries when metadata strings are truncated or absent in older Dell R1 revisions.
  - Re-assembly rules for flashing extracted components back into a flash programmer image.
- **FriedChip+ Implementation Plan:**
  - Implement clean-room zero-copy binary parser in `crates/engine-vendor/src/dell_pfs.rs` using Rust `nom` or standard byte slices.
  - Implement pure Rust decompressor using `flate2` (ZLIB) and `xz2` / `lzma-rs` (ThinOS PKG).

##### 2. Insyde iFlash & iFdPacker Extractor
- **Exact File:** `biosutilities/insyde_ifd_extract.py` (lines 25–185)
- **Relevant Structures & Functions:**
  - `IflashHeader`: `Signature` (8 bytes, `$_IFLASH`), `ImageTag` (8 bytes ASCII), `TotalSize` (`UINT32`), `ImageSize` (`UINT32`). Total 0x18 bytes.
  - Padding calculation: `TotalSize - ImageSize`.
  - Known image tags (`INS_IFL_IMG`): `BIOSIMG` (BIOS-UEFI), `EC_IMG` (Embedded Controller), `ME_IMG` (Management Engine), `INI_IMG` (platform.ini), `IOM_IMG`, `ISH_IMG`, `PDT_IMG`, `TBT_IMG`, `BIOSCER` (Certificate).
  - `PAT_INSYDE_SFX`: Pattern `\x0D\x0A;!@InstallEnd@!\x0D\x0A(7z\xBC\xAF\x27|\x6E\xF4\x79\x5F\x4E)` matching 7z SFX stub.
  - Known static password for iFdPacker encrypted SFX: `Y`t~i!L@i#t$U%h^s7A*l(f)E-d=y+S_n?i`.
- **What It Establishes:**
  - Insyde packages contain raw flash images concatenated behind 24-byte headers prefixed with `$_IFLASH`.
  - Many OEM vendor executables package an encrypted 7-Zip SFX using a known universal OEM password.
- **What Remains Unverified:**
  - Newer Insyde H2OFFT packages using proprietary AES-128 CBC payload encryption without the legacy SFX password.
- **FriedChip+ Implementation Plan:**
  - Implement `engine-vendor/src/insyde_iflash.rs` parsing all `$_IFLASH` structures and extracting constituent component partitions without spawning subshell processes.

##### 3. AMI UCP & PFAT / Intel BIOS Guard
- **Exact Files:** `biosutilities/ami_ucp_extract.py` (lines 30–120), `biosutilities/ami_pfat_extract.py` (lines 25–110)
- **Relevant Structures:**
  - `UafHeader`: `ModuleTag` (`CHAR * 4`), `ModuleSize` (`UINT32`), `Checksum` (`UINT16`), `Unknown0` (`UINT8`), `Unknown1` (`UINT8`), `Reserved` (`UINT8 * 4`).
  - `AmiBiosGuardHeader`: `Size` (`UINT32`), `Checksum` (`UINT32`), `Tag` (`CHAR * 8`, `_AMIPFAT`), `Flags` (`UINT8`).
  - `IntelBiosGuardHeader`: `BGVerMajor`/`Minor` (`UINT16`), `PlatformID` (`UINT8 * 16`), `Attributes` (`UINT32`), `ScriptVer` (`UINT16 * 2`), `ScriptSize` (`UINT32`), `DataSize` (`UINT32`), `BIOSSVN` (`UINT32`), `ECSVN` (`UINT32`), `VendorInfo` (`UINT32`).
- **What It Establishes:**
  - AMI UCP update packages wrap UEFI payloads inside Universal AMI Firmware (UAF) blocks.
  - Intel BIOS Guard (PFAT) wraps firmware blocks in signed scripts verified by CPU microcode before execution.

##### 4. Phoenix TDK, Panasonic, Fujitsu, Toshiba COM, and VAIO
- **Phoenix TDK (`phoenix_tdk_extract.py`):** Header `Tag` (`$PACK\x00\x00\x00`), `Size` (`UINT32`), `Count` (`UINT32`). Entries contain nested PE stubs and LZMA/RAW compressed ROMs.
- **Panasonic (`panasonic_bios_extract.py`):** Encapsulated within PE resource sections using Microsoft LZNT1 compression (decompressed via `dissect.util.compression.lznt1`).
- **Fujitsu SFX (`fujitsu_sfx_extract.py`):** Embedded Microsoft Cabinet (`MSCF`) archive obfuscated by applying bitwise `XOR 0xFF` starting at offset +0x0A after the `FjSfxBinay` marker.
- **Toshiba COM (`toshiba_com_extract.py`):** 16-bit DOS COM executables matching `\x00{2}[\x00-\x02]BIOS`. Relies on external utility `comextract.exe`.
- **VAIO (`vaio_package_extract.py`):** INI configuration tagged `[Setting]` and Cabinet archives extracting update components.

---

### 2.2 Repository B: `ALeXXBody/BIOSRecoverySuite`

- **Repository:** `https://github.com/ALeXXBody/BIOSRecoverySuite`
- **Inspected Commit SHA:** `86fdb1486cb39b2543392f5b8afbb5c2b6c4e5a6`
- **Primary Language:** Python 3 (clean architecture, dataclasses, pytest suite)
- **License:** MIT License (Copyright © 2026 ALeXXBody)

#### Inspected Source Files & Technical Findings

##### 1. Unified BIOS Analyzer Architecture
- **Exact File:** `bios_recovery/core/bios_analyzer.py` (lines 15–280)
- **Relevant Classes & Functions:**
  - `BiosType` Enum: `INTEL_IFD`, `AMI_APTIO`, `PHOENIX`, `AWARD`, `INSYDE`, `DELL_PFS`, `APPLE_EFI`, `COREBOOT`, `AMI_CAPSULE`, `UEFI_FV`, `UNKNOWN`.
  - `RegionType` Enum: `FLASH_DESCRIPTOR`, `BIOS`, `INTEL_ME`, `TXE`, `GBE`, `PLATFORM_DATA`, `EC`, `NVRAM`, `FIRMWARE_VOLUME`.
  - `RegionInfo` Dataclass: `region_type`, `offset`, `size`, `name`, `version`, `details`.
  - `BiosAnalysisResult`: Aggregates cryptographic hashes (MD5, SHA-256), detected `bios_type`, parsed regions, detected ME/EC versions, issues, and warnings.
  - `decode_flreg(value: int) -> tuple[int, int]`:
    ```python
    base = (value & 0x7FFF) << 12
    limit = (((value >> 16) & 0x7FFF) << 12) | 0xFFF
    return base, limit
    ```
    Uses 15-bit addressable page masks supporting modern 128 MB flash capacities, avoiding the legacy 13-bit / 32 MB limit.
  - `_detect_type()`: Performs multi-stage signature matching (`0x0FF0A55A` at offset 0x00 or 0x10, `$FPT`, `_FVH`, `PFS`, `_AMI`), falling back to structural search for UEFI Firmware Volumes before marking as `UNKNOWN`.

##### 2. SMBIOS & DMI Separation
- **Exact Files:** `bios_recovery/core/smbios.py` (lines 1–110), `bios_recovery/core/dmi_extractor.py` (lines 1–90)
- **Relevant Functions & Logic:**
  - `walk(data, start, end)`: Traverses well-formed SMBIOS structures, validating length headers (`slen >= 4`), checking printable ASCII string sets terminating with double NUL (`\x00\x00`), and stopping at `END_OF_TABLE` (Type 127).
  - Explicit rule: Rejects random byte occurrences of `_SM_` or `_DMI_` that fail structural validation or lack a valid System Information structure (Type 1).
  - Separates logical SMBIOS structure extraction from physical persistent NVRAM/DMI storage locations.

##### 3. Intel ME Manifest & EC Firmware Detection
- **Exact Files:** `bios_recovery/core/me_manifest.py` (lines 15–85), `bios_recovery/platforms/ec/ec_version.py` (lines 15–65)
- **Relevant Logic:**
  - `iter_manifest_versions()`: Searches for manifest tags (`$MN2` for ME 2+ / CSME; `$MAN` for ME 1.x) at offset `base + 0x1C`, decoding 4x `UINT16` (major, minor, hotfix, build) at `base + 0x24`.
  - Distinguishes Intel ME vs TXE by taking the MD5 hash of the 0x104-byte RSA public key at `base + 0x80` against `TXE_PUBKEY_MD5`.
  - `scan_ec_firmware_version()`: Scans strictly within isolated EC regions for OEM version tokens (e.g. ThinkPad regex `(?<![A-Z0-9])[A-Z][0-9]{1,2}[A-Z]{1,3}[0-9]{2}[A-Z](?![A-Z0-9])`), preferring the rightmost occurrence and returning "not found" rather than guessing.

##### 4. Source Safety & Verified-Write Mechanisms
- **Exact Files:** `bios_recovery/core/safety.py` (lines 1–55), `bios_recovery/core/verified_write.py` (lines 1–25)
- **Relevant Functions:**
  - `ensure_distinct(input_path, output_path)`: Resolves real filesystem paths via `os.path.realpath` and `os.path.samefile`, preventing accidental in-place overwriting of source dumps.
  - `write_file_atomic(path, data)`: Writes output to `.tmp` file in destination directory, flushes buffer, executes `os.fsync()`, syncs file permissions, and invokes atomic replace (`os.replace`).
  - `write_verified(path, data)`: Executes atomic write, immediately re-reads complete on-disk file, and verifies byte-for-byte identity with `data`, raising `OSError` on mismatch.

##### 5. Synthetic Fixture Architecture
- **Exact File:** `tests/fixture_builder.py` (lines 1–110)
- **Design Pattern:** Constructs synthetic, structurally valid binary fixtures in memory (synthesizing valid `_FVH` volume headers with calculated 16-bit inverted checksums, SMBIOS type 0/1/2/3 tables, and FLREG descriptors) for deterministic end-to-end regression testing without committing copyrighted OEM dumps.

##### 6. Excluded Scope Audit
- `bios_recovery/core/dmi_editor.py` (password bypass logic), `tests/test_master_passwords.py`, `bios_recovery/platforms/intel/hp_unlocker.py`: These modules implement password removal, cryptographic master password generation, and security patch injections.
- **FriedChip+ Safety Invariant:** These capabilities are strictly excluded from FriedChip+.

---

### 2.3 Repository C: `google/pawn`

- **Repository:** `https://github.com/google/pawn`
- **Inspected Commit SHA:** `45b7b3865c3d4f2aa6e0defebb477dacb8b8a3b9`
- **Primary Language:** C++17 (Abseil C++ library, CMake)
- **License:** Apache License 2.0 (Copyright © 2014–2026 Google LLC)

#### Inspected Source Files & Technical Findings

##### 1. Live SPI Acquisition Strategy
- **Exact Files:** `pawn/pawn.cc` (lines 35–180), `pawn/physical_memory.cc` (lines 30–75), `pawn/chipset.cc` (lines 40–120)
- **Relevant Functions & Mechanisms:**
  - Target: Live execution on host Intel system to dump the active SPI flash chip without an external programmer.
  - PCI LPC Interface: Opens PCI configuration space via `Pci::Create()`, enabling x86 ring-3 I/O port read privileges (`iopl(3)`).
  - Chipset Detection: Queries PCI `VendorId` (0x8086 for Intel) and `DeviceId` to identify chipset generation (ICH8, ICH9, ICH10, 6-series through 9-series).
  - RCBA Mapping: Reads Root Complex Base Address (RCBA) register from PCI configuration, opening `/dev/mem` with `O_RDWR` and mapping 16 KiB via `mmap()` to access the Chipset Configuration Space.
  - Hardware Sequencing: Disables software sequencing and issues read requests directly through the SPI Controller Hardware Sequencing registers (`FADDR`, `FDATA0..15`, `HSFS`, `HSFC`).
- **Hardware & Kernel Constraints:**
  - Requires Linux root privileges (`uid == 0`).
  - Fails on modern Linux kernels compiled with `CONFIG_IO_STRICT_DEVMEM=y` unless booted with kernel parameter `iomem=relaxed`.
  - Limited to legacy Intel PCH generations (ICH8 through 9-series Cougar Point/Wildcat Point). Modern Intel Skylake through Raptor Lake PCHs replaced RCBA with Sideband Router (P2SB / PCR) memory mapping, rendering Pawn’s RCBA access obsolete on modern platforms.
  - Fails if the chipset has locked hardware configuration via `FLOCKDN=1` or if SPI Protected Range registers (`PR0..PR4`) enforce read protection.
- **FriedChip+ Architectural Decision:**
  - FriedChip+ will **NOT** implement live memory dumping, `/dev/mem` access, kernel drivers (such as WinRing0), or undocumented Windows memory handles.
  - Live acquisition is fragile, triggers endpoint security/antivirus flags, and is inherently unsafe for commercial firmware servicing.
  - FriedChip+ operates purely as an offline analysis engine consuming verified files created by external SPI programmers or official OEM update installers.

---

### 2.4 Repository D: `dreamwhite/bios-extraction-guide`

- **Repository:** `https://github.com/dreamwhite/bios-extraction-guide`
- **Inspected Commit SHA:** `85ed036293fe25964737f8cb0fcb958034fa6434`
- **Primary Language:** Markdown documentation, shell snippets, Python references
- **License:** Unlicensed (Standard GitHub Terms of Service; no open-source license provided)

#### Inspected Workflows & Technical Findings

##### 1. OEM Unpacking Procedures
- **Exact Files:** `Dell/README.md`, `Lenovo/README.md`
- **Workflows Documented:**
  - Dell: Execution of Plato Mavropoulos’ `Dell_PFS_Extract.py` against official update executables to generate individual partition payloads (`System BIOS`, `Intel ME`, etc.).
  - Lenovo: Execution of `innoextract -e <installer.exe>` to extract the `app/` subfolder containing `.fl1`, `.fl2`, or `.fb` files. In cases with `.fb` files, extraction via 7-Zip (`7z x <bios>.fb`) yields the uncompressed `.reloc` payload.

##### 2. UEFI Variable & IFR Analysis Workflows
- **Exact Files:** `README.md` (lines 25–95), `utils/ifrextract`
- **Workflows Documented:**
  - UEFITool is used to search for Unicode strings (`"CFG Lock"`, `"DVMT Pre-Allocated"`) inside the BIOS region.
  - The matching PE32 image section within the Setup driver is extracted as a `.bin` payload.
  - `IFRExtractor` / `IFRExtractor-RS` parses the UEFI Internal Form Representation (IFR) opcodes, producing a human-readable text file mapping variable names to their VarStore GUID, VarStore ID, and variable byte offset (e.g. `VarOffset: 0x5C`).

##### 3. Inspection vs Modification Boundary
- **Runtime Modification Tools:** The guide documents using EFI utility shells (`setup_var.efi`, `modGRUBShell.efi`, `RU.efi`) to write directly to non-volatile UEFI variables, flipping bits to disable MSR 0xE2 lock (`CFG Lock`).
- **FriedChip+ Safety Invariant:**
  - The guide’s extraction techniques (InnoExtract, Dell PFS parsing, UEFITool volume traversal, IFR disassembly) are legitimate inspection workflows suitable for implementation in FriedChip+.
  - Automated modification of security variables (such as arbitrary runtime NVRAM bit flipping) is outside the scope of safe servicing and will not be automated by FriedChip+.

---

## 3. Comparative Summary Matrix

| Capability / Dimension | `BIOSUtilities` | `BIOSRecoverySuite` | `google/pawn` | `bios-extraction-guide` | FriedChip+ Design Target |
|---|---|---|---|---|---|
| **Primary Domain** | OEM Update Unpackers | Firmware Analysis & Recovery | Live Intel SPI Dump | Hackintosh / UEFI Tweaking | Commercial Firmware Workstation |
| **Language & Engine** | Python 3 + CTypes | Python 3 + PyTest | C++17 + Abseil | Markdown + External Tools | Pure Rust (`engine-*`) + Tauri 2 |
| **Dell PFS Support** | Complete (Header, Entries, CRC) | Partial (via parser wrapper) | None | Wraps BIOSUtilities | Native Rust Parser (Phase 2 candidate) |
| **Insyde iFlash Support** | Complete (`$_IFLASH` & SFX) | Partial | None | None | Native Rust Parser (Phase 2 candidate) |
| **Intel Flash Descriptor** | Pattern regex check | Complete FLREG (15-bit pages) | Register-level read | None | Native Rust Parser (`0x0FF0A55A`) |
| **SMBIOS / DMI** | None | Structural validation & walk | None | None | Strict Structural Parser |
| **Hardware Access** | None (Filesystem only) | None (Filesystem only) | Privileged `/dev/mem` | UEFI Shell tools | None (Pure Offline File Analysis) |
| **Source Safety** | Output directory creation | `ensure_distinct` & `write_verified` | Overwrite prompt | Manual warnings | Pre-hash check, Atomic Partials, No In-place |
| **Synthetic Fixtures** | None | Full (`fixture_builder.py`) | Unit bits test | None | Rust In-Memory Synthetic Generator |
| **License** | BSD-2-Clause + Patent | MIT | Apache-2.0 | Unlicensed | Proprietary (FriedChip+) |

---
*Research and architectural dossier complete. No application source code or firmware algorithms implemented.*
