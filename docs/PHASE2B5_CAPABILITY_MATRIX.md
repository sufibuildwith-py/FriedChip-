# Phase 2B.5 — Capability Mapping Matrix

**Project:** FriedChip+  
**Release:** Phase 2B.5  
**Scope:** Mapping 17 Core Firmware Capabilities to Upstream Repositories and FriedChip+ Product Modules.

---

## 1. Capability to Repository and Product Module Mapping

| # | Firmware Capability | Upstream Source Repositories | Primary Upstream File / Struct Reference | FriedChip+ Product Module | Integration Mechanism | Status in Phase 2B.5 |
|---|---------------------|------------------------------|------------------------------------------|---------------------------|-----------------------|----------------------|
| 1 | **IFD Structural Validation & Gate 1** | `coreboot/util/ifdtool`, `flashrom` | `ifdtool.c:check_signature()`, `ich_descriptors.c` | `engine::binary::ifd` | Native Rust Clean-Room | Implemented & Verified |
| 2 | **IFD Platform Profile Classification** | `flashrom`, `coreboot/util/ifdtool` | `ich_descriptors.c:guess_ich_chipset()`, `ich_descriptors.h` | `engine::binary::ifd` | Native Rust Clean-Room | Implemented & Verified |
| 3 | **IFD Master Access Rules (FLMSTR)** | `flashrom`, `coreboot/util/ifdtool` | `ich_descriptors.h:struct ich_desc_master` | `engine::binary::ifd` | Native Rust Clean-Room | Implemented & Verified |
| 4 | **IFD Component Densities & Frequencies** | `flashrom`, `coreboot/util/ifdtool` | `ich_descriptors.h:struct ich_desc_component` | `engine::binary::ifd` | Native Rust Clean-Room | Implemented & Verified |
| 5 | **IFD Region Physical Slicing & Bounds** | `coreboot/util/ifdtool`, `flashrom` | `ifdtool.c:parse_regions()`, `ich_descriptors.h:FLREG` | `engine::binary::ifd`, `engine::binary::region` | Native Rust Clean-Room | Implemented & Verified |
| 6 | **Dell PFS Update Container Extraction** | `BIOSUtilities` | `Dell_PFS_Extract.py:parse_pfs()` | `engine::vendor::dell_pfs` | Native Rust Clean-Room | Implemented & Verified |
| 7 | **Insyde IFWI Package Deconstruction** | `BIOSUtilities` | `Insyde_IFWI_Extract.py` | `engine::vendor::insyde` (Planned) | External Adapter | Documented for Phase 3 |
| 8 | **AMI / Phoenix Capsule Parsing** | `BIOSUtilities`, `UEFITool` | `UEFITool/uefipareser.cpp:parseCapsule()` | `engine::vendor::capsule` (Planned) | Native Rust Clean-Room | Documented for Phase 3 |
| 9 | **UEFI Firmware Volume Header Detection** | `edk2/BaseTools`, `UEFITool` | `PiFirmwareVolume.h`, `UEFITool/basetypes.h` | `engine::uefi::firmware_volume` | Native Rust Clean-Room | Implemented & Verified |
| 10 | **UEFI FV 16-Bit Complement Checksum** | `edk2/BaseTools`, `fiano/utk` | `BaseTools/Source/C/Common/CommonLib.c:CalculateChecksum16()` | `engine::binary::checksums`, `engine::uefi::firmware_volume` | Native Rust Clean-Room | Implemented & Verified |
| 11 | **UEFI Block Map Sentinel Validation** | `edk2/BaseTools` | `PiFirmwareVolume.h:EFI_FV_BLOCK_MAP_ENTRY` | `engine::uefi::firmware_volume` | Native Rust Clean-Room | Implemented & Verified |
| 12 | **UEFI FFS File Traversal & GUID Table** | `UEFITool`, `fiano/utk` | `UEFITool/ffsparser.cpp`, `fiano/pkg/uefi/ffs.go` | `engine::uefi::ffs` (Planned) | Native Rust Clean-Room | Candidate for Phase 3 |
| 13 | **UEFI Section Decompression (Tiano/LZMA)** | `UEFITool`, `edk2/BaseTools` | `UEFITool/decompress.cpp`, `BaseTools/Source/C/LzmaCompress` | `engine::uefi::compression` (Planned) | Rust Crate (`lzma-rs`, `flate2`) | Candidate for Phase 3 |
| 14 | **UEFI IFR Form Extraction & Decode** | `IFRExtractor-RS` | `src/ifrextractor.rs`, `src/hpk.rs` | `engine::uefi::ifr` (Planned) | Direct Library (`IFRExtractor-RS`) | Top Candidate Phase 3 |
| 15 | **UEFI NVRAM Variable Storage Parsing** | `uefisettings` | `src/nvram.rs`, `src/variable.rs` | `engine::uefi::nvram` (Planned) | Direct Library (`uefisettings`) | Top Candidate Phase 3 |
| 16 | **Intel ME Partition Table Analysis** | `MEAnalyzer` | `MEA.py:parse_fpt()` | `engine::intel::me` (Planned) | External Adapter (Subprocess) | Top Candidate Phase 3 |
| 17 | **AMD PSP Directory Table Parsing** | `PSPTool` | `psptool/entry.py`, `psptool/directory.py` | `engine::amd::psp` (Planned) | External Adapter (Subprocess) | Candidate for Phase 3 |

---

## 2. Capability Architecture Summary

- **Phase 2 Baseline (Current Release):**
  - Capabilities 1, 2, 3, 4, 5 (Intel Flash Descriptor Complete Parsing & Gates 1–3)
  - Capability 6 (Dell PFS Update Container Extraction & Verification)
  - Capabilities 9, 10, 11 (UEFI Firmware Volume Header Detection, Checksum, Sentinel Verification)
- **Phase 3 Candidates (Recommended for Next Milestone):**
  - Capability 14: UEFI IFR Opcode / Form Extraction via `IFRExtractor-RS` (Library)
  - Capability 15: UEFI NVRAM Setting Parsing via `uefisettings` (Library)
  - Capability 16: Intel ME Partition Analysis via `MEAnalyzer` (External Adapter)
