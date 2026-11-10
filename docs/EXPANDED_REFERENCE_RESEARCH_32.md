# FriedChip+ — Expanded 32-Entry Reference Architecture & Implementation Pattern Research

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Deep Architectural Reference & Pattern Catalog  
**Status:** Complete Research Dossier — Awaiting Architecture Review  

---

## 1. Executive Summary & Research Methodology

Following the initial external repository inspection, this dossier expands the research scope into an exhaustive catalog of **32 distinct firmware architectures, container structures, hardware descriptors, and validation patterns** discovered across:
1. `platomav/BIOSUtilities`
2. `ALeXXBody/BIOSRecoverySuite`
3. `google/pawn`
4. `dreamwhite/bios-extraction-guide`

The objective is to establish an authoritative clean-room knowledge base that defines the exact binary headers, offset semantics, compression methods, checksum algorithms, and failure boundary conditions across the entire modern PC firmware servicing domain.

---

## 2. Comprehensive 32-Entry Pattern Catalog

```
                    ┌──────────────────────────────────────────────────┐
                    │     FriedChip+ 32-Entry Firmware Catalog         │
                    └──────────────────────────────────────────────────┘
                                             │
      ┌─────────────────────┬────────────────┴──────────────┬─────────────────────┐
      ▼                     ▼                               ▼                     ▼
┌──────────────┐     ┌──────────────┐                ┌──────────────┐      ┌──────────────┐
│ Part 1: OEM  │     │ Part 2: OEM  │                │ Part 3: SPI  │      │ Part 4: SIO  │
│  Containers  │     │ & Apple Pkgs │                │ Descriptors  │      │ & Controllers│
│ (Entries 1-6)│     │(Entries 7-18)│                │(Entries 19-26│      │(Entries 27-32│
└──────────────┘     └──────────────┘                └──────────────┘      └──────────────┘
```

---

### PART 1: OEM Firmware Update Containers (Entries 1–6)

#### 1. Dell PFS Update Container
- **Reference File:** `BIOSUtilities/biosutilities/dell_pfs_extract.py`
- **Detection Markers:**
  - Header Pattern: `\xEE\xAA\x76\x1B\xEC\xBB\x20\xF1\xE6\x51` (10 bytes) followed by Section Type byte (`0xAA` = Firmware, `0xBB` = Utilities) and 8-bit XOR checksum.
  - Footer Pattern: `\xEE\xAA\xEE\x8F\x49\x1B\xE8\xAE\x14\x37\x90` (11 bytes) followed by 8-bit XOR checksum.
- **Inner Volume Structure:**
  - `PFS.HDR.` (0x10 bytes): Tag (8 bytes), HeaderVersion (u32 LE), PayloadSize (u32 LE).
  - `PFS.FTR.` (0x10 bytes): PayloadSize (u32 LE), Negated CRC-32 Checksum (u32 LE), Tag (8 bytes).
  - Checksum Formula: `!crc32(payload, 0) == footer.checksum`.
- **Component Entry Structures:**
  - `DellPfsEntryBase`: GUID (16 bytes), HeaderVersion (u32: 1 or 2), VersionType ([u8; 4]), Version ([u16; 4]), Reserved (u64), DataSize (u32), DataSigSize (u32), DataMetSize (u32), DataMetSigSize (u32). Total 0x38 bytes.
  - `DellPfsEntryR1`: Adds 16 unknown bytes (total 0x48 bytes).
  - `DellPfsEntryR2`: Adds 32 unknown bytes (total 0x58 bytes).
- **Directory Table Resolution:**
  - Table GUID: `E0717CE3A9BB25824B9F0DC8FD041960` or `B033CB16EC9B45A14055F80E4D583FD3`.
  - Maps 16-byte component GUIDs to UTF-16LE friendly names (e.g. `System BIOS with BIOS Guard`, `Embedded Controller`, `Intel Management Engine`).

#### 2. Dell ThinOS PKG Container
- **Reference File:** `BIOSUtilities/biosutilities/dell_pfs_extract.py` (`_thinos_pkg_extract`)
- **Detection Marker:** `\x13\x55\x00` preceded by `0x72` (Classic RAW) or `0x73` (LVFS/FWUPD CAB).
- **Structure:**
  - 0x30-byte header.
  - Length at offset `0x10..0x14` (u32 LE).
  - MD5 hash at `0x20..0x30` (16 bytes).
  - Payload starts at `+0x30`, compressed with LZMA2 (`7zXZ`).
  - Contains TAR/CAB archives holding the nested Dell PFS container.

#### 3. Insyde iFlash Packaging (`$_IFLASH`)
- **Reference File:** `BIOSUtilities/biosutilities/insyde_ifd_extract.py`
- **Detection Marker:** ASCII `$_IFLASH` (8 bytes) at start or embedded.
- **Structure:**
  - 0x18-byte fixed header:
    - `0x00..0x08`: Signature `$_IFLASH`
    - `0x08..0x0C`: ImageTag (u32 LE)
    - `0x0C..0x10`: Reserved / Flags
    - `0x10..0x14`: TotalSize (u32 LE)
    - `0x14..0x18`: ImageSize (u32 LE)
  - Followed by image index table pointing to individual flash regions (BIOS, ME, EC).

#### 4. Insyde iFdPacker & 7z SFX Wrappers
- **Reference File:** `BIOSUtilities/biosutilities/insyde_ifd_extract.py`
- **Detection Marker:** `\x0D\x0A;!@InstallEnd@!\x0D\x0A` followed by 7-Zip signature `7z\xBC\xAF\x27` or LZMA stream `\x6E\xF4\x79\x5F\x4E`.
- **Extraction Behavior:**
  - Self-extracting PE executable with an embedded compressed archive overlay.
  - Contains platform flashing utilities (`H2OFFT-Wx64.exe`, `isflash.bin`, platform configuration INI).

#### 5. AMI PFAT / Intel BIOS Guard Package
- **Reference File:** `BIOSUtilities/biosutilities/ami_pfat_extract.py`
- **Detection Marker:** `_AMIPFAT.AMI_BIOS_GUARD_FLASH_CONFIGURATIONS`
- **Structure:**
  - Intel BIOS Guard Header (`IntelBiosGuardHeader`): Signature `_AMIPFAT`, ScriptSize, DataSize, PlatformID, BIOSSVN, ECSVN.
  - Script Module: BIOS Guard execution script containing cryptographic verification instructions for the PCH execution unit.
  - Data Module: Encrypted or encapsulated firmware volume blocks requiring verification against the platform key.

#### 6. AMI UCP / UAF / HPU Update Packages
- **Reference File:** `BIOSUtilities/biosutilities/ami_ucp_extract.py`
- **Detection Marker:** `@UAF` or `@HPU` enclosed by 12 bytes and `@` (`@UAF.{12}@`).
- **Structure:**
  - Module Tag: 4 bytes (`@UAF` or `@HPU`).
  - Module Size: u32 LE at `0x04..0x08`.
  - Header Length: u16 LE.
  - Encapsulated Payload: Uncompressed or compressed raw ROM image with embedded descriptor tables.

---

### PART 2: Vendor-Specific & Legacy Packaging (Entries 7–18)

#### 7. Phoenix TDK Package
- **Reference File:** `BIOSUtilities/biosutilities/phoenix_tdk_extract.py`
- **Detection Marker:** `$PACK\x00\x00\x00`
- **Structure:**
  - 0x14-byte header: `$PACK` signature, Size (u32 LE), ComponentCount (u32 LE).
  - Followed by component descriptors specifying physical flash target addresses and decompression algorithms (Deflate/LZSS).

#### 8. Panasonic SFX Package
- **Reference File:** `BIOSUtilities/biosutilities/panasonic_bios_extract.py`
- **Detection Marker:** `FjSfxBinay\xB2\xAC\xBC\xB9`
- **Structure:**
  - Proprietary SFX stub containing trailing binary payloads with XOR obfuscation.

#### 9. Fujitsu SFX Container
- **Reference File:** `BIOSUtilities/biosutilities/fujitsu_sfx_extract.py`
- **Detection Marker:** Fujitsu SFX signature sequences with bounded offset indexing.

#### 10. Fujitsu UPC Container
- **Reference File:** `BIOSUtilities/biosutilities/fujitsu_upc_extract.py`
- **Detection Marker:** `UPC\x00` header followed by partition tables.

#### 11. Toshiba COM / Self-Extractor
- **Reference File:** `BIOSUtilities/biosutilities/toshiba_com_extract.py`
- **Structure:**
  - DOS COM executable overlay holding raw ROM binaries concatenated with platform ID tables.

#### 12. Sony VAIO Package
- **Reference File:** `BIOSUtilities/biosutilities/vaio_package_extract.py`
- **Structure:**
  - Microsoft Cabinet (MSCF) or InstallShield multi-part payloads holding encrypted BIOS files (`.ROM` / `.WPH`).

#### 13. Apple ROM Version Metadata
- **Reference File:** `BIOSUtilities/biosutilities/apple_efi_id.py`
- **Detection Marker:** `Apple ROM Version\x0A  `
- **Structure:**
  - Model identifier, Board-ID, version, and release date stored in plain text at a fixed offset within the primary EFI firmware volume.

#### 14. Apple IM4P Container
- **Reference File:** `BIOSUtilities/biosutilities/apple_efi_im4p.py`
- **Detection Marker:** `\x16\x04IM4P\x16\x04mefi`
- **Structure:**
  - DER-encoded ASN.1 structure with payload type `mefi` (Mac EFI).
  - Contains compressed or encrypted Apple firmware payloads.

#### 15. Apple PBZX Stream
- **Reference File:** `BIOSUtilities/biosutilities/apple_efi_pbzx.py`
- **Detection Marker:** ASCII `pbzx`
- **Structure:**
  - Chunked container holding XZ-compressed payload streams extracted from macOS update payloads.

#### 16. Apple PKG / Payload Pipeline
- **Reference File:** `BIOSUtilities/biosutilities/apple_efi_pkg.py`
- **Structure:**
  - Multi-stage pipeline: XAR archive -> `Payload` file -> PBZX stream -> CPIO archive -> `EFIUpdaterApp.efi` and standalone ROM.

#### 17. Award BIOS LZH Archive
- **Reference File:** `BIOSUtilities/biosutilities/award_bios_extract.py`
- **Detection Marker:** `-lh[04567]-`
- **Structure:**
  - Legacy LHA/LZH compression container packing individual modules (System BIOS, VGA ROM, Setup).

#### 18. Portwell EFI Capsule
- **Reference File:** `BIOSUtilities/biosutilities/portwell_efi_extract.py`
- **Detection Marker:** `<U><U>`
- **Structure:**
  - Embedded industrial PC firmware volume encapsulated with proprietary vendor headers.

---

### PART 3: Platform Descriptors & Hardware Structures (Entries 19–26)

#### 19. Intel Flash Descriptor (IFD)
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/bios_analyzer.py` & `pawn`
- **Detection Marker:** `0x0FF0A55A` (`\x5A\xA5\xF0\x0F` Little-Endian) at offset `0x10`.
- **Address Arithmetic:**
  - FLREG 15-bit addressable page fields:
    - Base: `(val & 0x7FFF) << 12`
    - Limit: `(((val >> 16) & 0x7FFF) << 12) | 0xFFF`
  - Unmapped / Disabled condition: `limit < base`.
  - 15-bit field accommodates up to 128 MB flash capacity.
- **Regions (0..15):**
  - Region 0: Descriptor (4KB)
  - Region 1: BIOS / UEFI Primary Payload
  - Region 2: Intel ME / CSME / TXE
  - Region 3: Gigabit Ethernet (GbE)
  - Region 4: Platform Data Region (PDR)
  - Region 5: Device Expansion (DER)
  - Region 6: Secondary BIOS
  - Region 7: Microcode / PTT
  - Region 8: Embedded Controller (EC)
  - Region 9: Device Expansion 2
  - Region 10: Innovation Engine (IE)
  - Region 11: 10GbE
  - Region 12: Reserved / PTT

#### 20. Intel Management Engine / FPT Partition Table
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/me_details.py`
- **Detection Marker:** `$FPT` at start of ME region (offset `+0x10` or `+0x00`).
- **Structure:**
  - FPT Header: Signature `$FPT`, entry count, header length, checksum (sum of 32 bytes == 0 mod 256).
  - Partitions: `FTPR` (Fault Tolerant Partition Routing), `LOCL` (Local configuration), `ROMB` (ROM Bypass), `EFFS` (Extended Flash File System).
  - CPD Manifest: `$CPD` container holding `$MN2` / `$MAN` manifest with 4-part semantic version numbers.

#### 21. Intel ME State & HAP / AltMeDisable Straps
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/me_cleaner_lt.py`
- **Mechanism:**
  - Intel ME v11+ High Assurance Platform (HAP) bit: bit 16 of PCHSTRP0 (FPSBA + 0x00 or +0x28).
  - Pre-v11 AltMeDisable bit: bit 7 of PCHSTRP10.
  - Setting these bits instructs the PCH ME controller to halt after initializing the clocks and memory controller.

#### 22. UEFI Firmware Volume (FV)
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/uefi_patch.py`
- **Detection Marker:** `_FVH` at offset `0x28`.
- **Structure:**
  - Length: `0x48` bytes (base revision 2).
  - FvLength: u64 LE at `0x20..0x28`.
  - HeaderLength: u16 LE at `0x30..0x32`.
  - Checksum: 16-bit additive inverse checksum over header (`sum_16 mod 0x10000 == 0`).
  - BlockMap: Array of `NumBlocks: u32, BlockLength: u32` terminated by `0, 0`.

#### 23. UEFI Firmware File System (FFS)
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/bios_analyzer.py`
- **Structure:**
  - Files identified by 16-byte FileSystemGuid.
  - Sections: PE32 executables, TE executables, UI strings, version descriptors, compressed sections (LZMA, Tiano).

#### 24. SMBIOS Entry Point & Structures
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/smbios.py`
- **Detection Markers:** `_SM_` (32-bit EPS, length 0x1F) and `_SM3_` (64-bit EPS, length 0x18).
- **Structure:**
  - Intermediate anchor `_DMI_` at offset `0x10` with intermediate checksum.
  - Table Address: Physical base of SMBIOS structure stream.
  - Types:
    - Type 0: BIOS Information (vendor, version, release date).
    - Type 1: System Information (manufacturer, product, serial number, UUID).
    - Type 2: Baseboard Information (board manufacturer, asset tag).
    - Type 3: Chassis Information.
    - Type 17: Memory Device (locator, part number, bank).

#### 25. AMI NVAR Storage Format
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/nvar.py`
- **Detection Marker:** ASCII `NVAR`
- **Structure:**
  - 0x0A-byte header: Signature `NVAR`, RecordSize (u16 LE), NextRecord offset, Attributes byte.
  - Storage for NVRAM variables including setup passwords, boot entries, and secure boot keys.

#### 26. Apple SMC / EC Payload
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/apple_ec.py`
- **Structure:**
  - System Management Controller (SMC) key-value records (`#KEY`, `OSK0`, `OSK1`).

---

### PART 4: Embedded Controller (EC) Architectures (Entries 27–30)

#### 27. ITE Tech Embedded Controller
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/ec_patcher.py`
- **Target Chips:** `IT8586E`, `IT8587E`, `IT8987E`
- **Memory Footprint:** 128 KB (`0x20000`) or 256 KB (`0x40000`).
- **Layout & Signatures:**
  - Fixed vector table at `0x0000`.
  - Config block marker: `KBC_CONFIG_V1`.
  - Checksum: 8-bit XOR checksum over entire image stored at last byte (`image[len - 1]`).

#### 28. ENE Technology Embedded Controller
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/ec_version.py`
- **Target Chips:** `KB9012`, `KB9022`, `KB9028`
- **Memory Footprint:** 128 KB.
- **Architecture:** 8051-compatible core.
- **Extraction Characteristic:** Often packed directly inside HP and Lenovo update packages as standalone 128KB partitions.

#### 29. Microchip / SMSC Embedded Controller
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/ec_patcher.py`
- **Target Chips:** `MEC1609`, `MEC1633`, `MEC1653`, `MEC1663`
- **Memory Footprint:** 192 KB / 256 KB.
- **Boot Strategy:** Loads code from SPI flash into internal SRAM via eSPI or shared SPI bus.

#### 30. Nuvoton Embedded Controller
- **Reference File:** `BIOSRecoverySuite/bios_recovery/core/ec_patcher.py`
- **Target Chips:** `NPCE288`, `NPCE388`
- **Memory Footprint:** 128 KB / 256 KB internal flash.

---

### PART 5: Hardware Controllers & Permissions (Entries 31–32)

#### 31. Intel PCH Hardware Straps & Boot BIOS Straps (BBS)
- **Reference File:** `google/pawn/pawn/chipset_intel_8_series.cc` & `chipset_intel_pch.h`
- **Hardware Architecture:**
  - Root Complex Configuration Block (RCRB) and SPI BAR0 memory space.
  - General Control and Status (GCS) register: Boot BIOS Straps (BBS) determining whether the PCH boots from SPI or LPC.
  - BIOS Interface Lock Down (BILD) bit.
  - Protected Range Registers (`PR0..PR4`): Hardware write-protection boundaries configured by UEFI.

#### 32. Intel Flash Region Access Permissions (FRAP)
- **Reference File:** `google/pawn/pawn/chipset.cc`
- **Access Control Architecture:**
  - Flash Descriptor Master Access bitmasks:
    - Master 1: Host CPU / BIOS execution unit.
    - Master 2: Intel Management Engine (ME).
    - Master 3: Gigabit Ethernet (GbE).
  - FRAP register: Read and Write permission bitmasks for each master across Regions 0..4.
  - Explains why external SPI programmers succeed where host-based flashing utilities fail when regions are locked by descriptor permissions.

---

## 3. Cross-Repository Dependency & Pattern Analysis

From auditing these 32 entries, four architectural universalities emerge:

1. **The Layered Decapsulation Reality:**  
   Real-world firmware never presents itself as raw SPI. It is almost always a multi-layered onion:
   ```text
   Installer Executable (PE SFX) 
     └─► Vendor Container (Dell PFS / Insyde iFlash / AMI UCP)
           └─► Compression Layer (ZLIB / LZMA2 / LHA)
                 └─► Firmware Region / Partition (BIOS / ME / EC)
                       └─► Firmware Volume / File System (UEFI FV / FFS / FPT)
   ```
2. **The "Flash-Ready" Fallacy:**  
   An extracted System BIOS partition from Dell PFS (Entry 1) or HP Insyde (Entry 3) is **NOT** a flash-ready SPI image. It lacks the Flash Descriptor (Entry 19), ME region (Entry 20), GbE region, and alignment required by an external SPI chip.
3. **Common Primitive Requirement:**  
   Every single one of the 32 entries requires:
   - Safe bounded binary slicing with zero-copy semantics.
   - Endian-aware integer decoding (`u16`, `u32`, `u64`).
   - Checksum utilities (8-bit XOR, CRC-32, 16-bit word sum, SHA-256).
   - Provenance tracking (recording exact parent container, offset, length, and hash).
4. **Safety Boundaries:**  
   Bypasses (passwords in NVAR/EC, MDM locks) are strictly toxic from a legal/commercial standpoint and must be systematically excluded, while structural inspection and clean extraction are universally valuable.
