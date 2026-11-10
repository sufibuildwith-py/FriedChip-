# FriedChip+ — Firmware Format Support Discovery
## Technical Format Catalog, Header Layouts & Detection Specifications

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author:** Antigravity (Advanced Agentic Coding)  
**Lead Architect & Creator:** Sufiyan Khan  
**Classification:** Engineering Specification & Binary Structure Catalog  
**Status:** Complete Format Discovery  

---

## 1. Overview & Detection Hierarchy

FriedChip+ adheres to a strict multi-tiered format detection hierarchy. Filename extensions (e.g. `.bin`, `.rom`, `.exe`, `.cap`) are treated merely as external metadata and **never** establish format identity. Binary payloads are classified through deterministic magic bytes, structural checksums, and boundary validation.

```
Incoming Firmware Binary / Update Payload
   │
   ├── [Offset 0x00 / 0x10 == 0x0FF0A55A?] ───> Intel Flash Descriptor (SPI Image)
   │
   ├── [Matches PFS.HDR. / PFS.FTR. / ZLIB markers?] ──> Dell PFS Container
   │
   ├── [Starts with \x13\x55\x00...7zXZ?] ───> Dell ThinOS PKG
   │
   ├── [Contains $_IFLASH or ;!@InstallEnd@!?] ───> Insyde iFlash / iFdPacker
   │
   ├── [Contains _AMIPFAT or @(UAF|HPU)?] ───> AMI PFAT (BIOS Guard) / AMI UCP
   │
   ├── [Contains $PACK\x00\x00\x00?] ───> Phoenix TDK
   │
   ├── [Contains FjSfxBinay\xB2\xAC... (CAB ^ 0xFF)?] ──> Fujitsu Obfuscated SFX
   │
   ├── [Contains _FVH at offset 0x28 within 0x48 header?] ──> UEFI Firmware Volume
   │
   ├── [Contains SMBIOS _SM_ / _SM3_ and passes walk()?] ──> SMBIOS Structure Table
   │
   └── [None of the above verified] ───> UNCLASSIFIED / UNKNOWN
```

---

## 2. Exhaustive Format Specifications

### 2.1 Dell PFS Update Container

#### Detection Patterns & Signatures
- **ZLIB Section Header Marker:**
  ```text
  \xEE\xAA\x76\x1B\xEC\xBB\x20\xF1\xE6\x51 <Type: 1B> <Checksum: 1B> \x78\x9C
  ```
  - Section Type `0xAA`: Firmware payload (PFS image).
  - Section Type `0xBB`: Flash utilities (PFS, PE executable, or 7z archive).
  - Checksum: 8-bit XOR checksum of the 10 preceding header bytes.
- **ZLIB Section Footer Marker:**
  ```text
  \xEE\xAA\xEE\x8F\x49\x1B\xE8\xAE\x14\x37\x90 <Checksum: 1B>
  ```
- **Inner PFS Header Signature:** `PFS.HDR.` (ASCII 8 bytes: `0x50 0x46 0x53 0x2E 0x48 0x44 0x52 0x2E`)
- **Inner PFS Footer Signature:** `PFS.FTR.` (ASCII 8 bytes: `0x50 0x46 0x53 0x2E 0x46 0x54 0x52 0x2E`)

#### Binary Header Layouts (Little-Endian)
```text
DellPfsHeader (0x10 bytes):
  0x00 - 0x07 : Tag               [8 bytes ASCII: "PFS.HDR."]
  0x08 - 0x0B : HeaderVersion     [u32: usually 1 or 2]
  0x0C - 0x0F : PayloadSize       [u32: size of following payload in bytes]

DellPfsFooter (0x10 bytes):
  0x00 - 0x03 : PayloadSize       [u32: matches Header PayloadSize]
  0x04 - 0x07 : Checksum          [u32: bitwise NOT of CRC-32 with init vector 0]
  0x08 - 0x0F : Tag               [8 bytes ASCII: "PFS.FTR."]

DellPfsEntryBase (0x38 bytes):
  0x00 - 0x0F : GUID              [16 bytes UUID, Little-Endian]
  0x10 - 0x13 : HeaderVersion     [u32: 1 = R1, 2 = R2]
  0x14 - 0x17 : VersionType       [4 x u8]
  0x18 - 0x1F : Version           [4 x u16]
  0x20 - 0x27 : Reserved          [u64]
  0x28 - 0x2B : DataSize          [u32: component payload size]
  0x2C - 0x2F : DataSigSize       [u32: signature block size]
  0x30 - 0x33 : DataMetSize       [u32: component metadata block size]
  0x34 - 0x37 : DataMetSigSize    [u32: metadata signature size]

DellPfsEntryR1 (Total 0x48 bytes):
  0x38 - 0x47 : UnknownR1         [4 x u32]

DellPfsEntryR2 (Total 0x58 bytes):
  0x38 - 0x57 : UnknownR2         [8 x u32]
```

#### Extraction Behavior & Checksum Formula
- **Decompression:** Payload stream decompresses via standard DEFLATE / ZLIB (`zlib::decompress`).
- **Checksum Verification:**
  ```rust
  fn verify_pfs_footer(payload: &[u8], expected_checksum: u32) -> bool {
      // Inverted CRC-32 with initial accumulator 0
      let mut crc = !0u32;
      for &b in payload {
          crc = crc32_table[((crc ^ (b as u32)) & 0xFF) as usize] ^ (crc >> 8);
      }
      let final_checksum = !crc;
      final_checksum == expected_checksum
  }
  ```
- **Payload Partitions:** Yields individual sub-components: System BIOS (`.bin`), Intel ME (`.bin`), EC (`.bin`), Microcode (`.bin`), Diagnostics (`.bin`).

---

### 2.2 Dell ThinOS PKG Package

#### Detection Pattern & Header
- **Signature:** `\x13\x55\x00` preceded by `0x72` (RAW mode) or `0x73` (CAB mode).
- **Embedded Stream:** Located at offset +0x30, containing a 7z XZ stream (`7zXZ`).
- **Size Field:** `u32` at offset +0x10 specifies the length of the compressed stream.

---

### 2.3 Insyde iFlash & iFdPacker SFX

#### Detection Patterns
- **iFlash Marker:** `$_IFLASH` (ASCII 8 bytes: `0x24 0x5F 0x49 0x46 0x4C 0x41 0x53 0x48`)
- **iFdPacker SFX Marker:** `\x0D\x0A;!@InstallEnd@!\x0D\x0A(7z\xBC\xAF\x27|\x6E\xF4\x79\x5F\x4E)`

#### Binary Header Layout (Little-Endian)
```text
IflashHeader (0x18 bytes):
  0x00 - 0x07 : Signature         [8 bytes: "$_IFLASH"]
  0x08 - 0x0F : ImageTag          [8 bytes ASCII, e.g. "BIOSIMG_", "EC_IMG__"]
  0x10 - 0x13 : TotalSize         [u32: total length from header end]
  0x14 - 0x17 : ImageSize         [u32: unpadded image length from header end]
```
- **Padding:** `TotalSize - ImageSize`.

#### Standard Partition Tags
- `BIOSIMG`: Raw UEFI BIOS Region image (`.bin`).
- `EC_IMG`: Embedded Controller binary (`.bin`).
- `ME_IMG`: Intel Management Engine region binary (`.bin`).
- `INI_IMG`: OEM platform configuration (`platform.ini`).
- `PDT_IMG`: Platform Descriptor Table.
- `ISH_IMG`: Integrated Sensor Hub firmware.
- `TBT_IMG`: Thunderbolt controller firmware.

#### Password-Protected SFX
- Insyde iFdPacker 7z SFX wrappers use the static password:
  `Y`t~i!L@i#t$U%h^s7A*l(f)E-d=y+S_n?i`

---

### 2.4 AMI UCP (Universal Capsule Package) & PFAT (BIOS Guard)

#### Detection Patterns
- **AMI UCP:** `@(UAF|HPU).{12}@`
- **AMI PFAT:** `_AMIPFAT.AMI_BIOS_GUARD_FLASH_CONFIGURATIONS`

#### Binary Header Layouts
```text
UafHeader (0x10 bytes):
  0x00 - 0x03 : ModuleTag         [4 bytes ASCII, e.g. "@UAF"]
  0x04 - 0x07 : ModuleSize        [u32: total size of UAF block]
  0x08 - 0x09 : Checksum          [u16: additive 16-bit word sum]
  0x0A - 0x0B : Unknown           [2 x u8]
  0x0C - 0x0F : Reserved          [4 x u8]

IntelBiosGuardHeader (0x30 bytes):
  0x00 - 0x01 : BGVerMajor        [u16: BIOS Guard major version]
  0x02 - 0x03 : BGVerMinor        [u16: BIOS Guard minor version]
  0x04 - 0x13 : PlatformID        [16 bytes OEM GUID]
  0x14 - 0x17 : Attributes        [u32: execution control flags]
  0x18 - 0x1B : ScriptVersion     [2 x u16]
  0x1C - 0x1F : ScriptSize        [u32: size of BG execution bytecode script]
  0x20 - 0x23 : DataSize          [u32: length of following update payload]
  0x24 - 0x27 : BIOSSVN           [u32: BIOS Security Version Number]
  0x28 - 0x2B : ECSVN             [u32: EC Security Version Number]
  0x2C - 0x2F : VendorInfo        [u32]
```

---

### 2.5 Phoenix TDK Package

#### Detection Pattern & Header
- **Signature:** `$PACK\x00\x00\x00`
- **Header Layout (0x10 bytes):**
  - `0x00 - 0x07`: `Tag` (8 bytes: `"$PACK\x00\x00\x00"`)
  - `0x08 - 0x0B`: `Size` (`u32`)
  - `0x0C - 0x0F`: `Count` (`u32`: number of embedded component entries)

---

### 2.6 Fujitsu Obfuscated SFX

#### Detection Pattern & Header
- **Signature:** `FjSfxBinay\xB2\xAC\xBC\xB9\xFF\xFF\xFF\xFF`
- **De-obfuscation Rule:**
  - The actual Microsoft Cabinet (`MSCF`) archive begins at offset `match_start + 0x0A`.
  - The entire CAB payload is obfuscated by applying bitwise `XOR 0xFF` across all bytes.
  - Applying `byte ^ 0xFF` restores the standard valid Microsoft Cabinet archive.

---

### 2.7 Intel Flash Descriptor (IFD) & Regions

#### Detection & Signatures
- **Descriptor Signature:** `0x0FF0A55A` (Little-Endian bytes: `\x5A\xA5\xF0\x0F`).
- **Signature Offsets:** Validated at offset `0x0000` or `0x0010`.
- **Descriptor Size:** Exactly 4,096 bytes (0x1000, 1 page) in FLREG0.

#### FLREG Register Decoding
Each Flash Region Register (`FLREG0` through `FLREG4` / `FLREG8`) encodes region base and limit in 4 KB blocks:
```rust
pub fn decode_flreg(val: u32) -> Option<(u32, u32)> {
    let base_raw = val & 0x7FFF;
    let limit_raw = (val >> 16) & 0x7FFF;
    
    // An unused/unmapped region has limit < base (often base = 0x7FFF, limit = 0)
    if limit_raw < base_raw {
        return None;
    }
    
    let base = base_raw << 12;
    let limit = (limit_raw << 12) | 0xFFF;
    Some((base, limit))
}
```
- **Standard Regions:**
  - `Region 0`: Flash Descriptor (0x0000..0x0FFF)
  - `Region 1`: BIOS Region (UEFI firmware volumes)
  - `Region 2`: Intel Management Engine (ME) / CSME / TXE
  - `Region 3`: Gigabit Ethernet (GbE) NVRAM
  - `Region 4`: Platform Data Region (PDR)
  - `Region 8`: Embedded Controller (EC) Region (when integrated on SPI)

---

### 2.8 UEFI Firmware Volume (`_FVH`)

#### Detection & Header Structure
- **Signature:** `_FVH` located at byte offset `0x28` of the volume header.
- **Header Structure (`EFI_FIRMWARE_VOLUME_HEADER`, 0x48 bytes):**
  - `0x00 - 0x0F`: Zero Vector (16 bytes zeroes)
  - `0x10 - 0x1F`: FileSystemGuid (16 bytes EFI GUID)
  - `0x20 - 0x27`: FvLength (`u64`: volume capacity in bytes)
  - `0x28 - 0x2B`: Signature (4 bytes ASCII: `_FVH`)
  - `0x2C - 0x2F`: Attributes (`u32`)
  - `0x30 - 0x31`: HeaderLength (`u16`: usually 0x48 or 0x58 with extended headers)
  - `0x32 - 0x33`: Checksum (`u16`: 16-bit word additive inverse sum)
  - `0x34 - 0x35`: ExtHeaderOffset (`u16`)
  - `0x36 - 0x36`: Reserved (`u8`)
  - `0x37 - 0x37`: Revision (`u8`: 2)
  - `0x38 - 0x47`: BlockMap entries (array of `{ NumBlocks: u32, Length: u32 }`, terminated by zeroes)

#### Integrity Verification
Summing the entire header as an array of unsigned 16-bit words must equal `0x0000`:
```rust
pub fn verify_fv_checksum(header_bytes: &[u8]) -> bool {
    if header_bytes.len() < 0x34 { return false; }
    let mut sum = 0u16;
    for chunk in header_bytes.chunks_exact(2) {
        sum = sum.wrapping_add(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    sum == 0
}
```

---

### 2.9 SMBIOS Structure Table

#### Entry Points
- **SMBIOS 2.x:** `_SM_` (0x1F bytes entry structure) with anchor string `_DMI_` at +0x10.
- **SMBIOS 3.x:** `_SM3_` (0x18 bytes 64-bit entry structure).

#### Structure Validation Rules
1. Every structure begins with a 4-byte header: `Type: u8`, `Length: u8`, `Handle: u16`.
2. Header is followed by formatted fixed fields of length `Length - 4`.
3. Fixed fields are followed by an unformatted string pool of null-terminated ASCII strings.
4. The entire structure terminates with a double NUL (`\x00\x00`).
5. A table is rejected if strings contain non-printable control characters (< 0x20, excluding standard whitespace).
6. A valid table must terminate with structure `Type 127` (`END_OF_TABLE`).

---

### 2.10 Intel ME / CSME Manifests

#### Detection Signatures
- **Manifest Tags:** `$MN2` (Intel ME 2.x through CSME 16.x) or `$MAN` (Intel ME 1.x).
- **Tag Location:** Offset `base + 0x1C` within the manifest structure.
- **Version Fields:** Located at `base + 0x24` (`tag + 0x08`):
  - 4 x `u16` Little-Endian: `Major`, `Minor`, `Hotfix`, `Build`.
- **Engine Classification:**
  - The RSA public key is located at `base + 0x80` with length `0x104` bytes.
  - An MD5 hash of this 260-byte key matching known TXE signatures identifies the engine as Intel TXE rather than standard ME/CSME.

---

## 3. Format Readiness & Implementation Matrix

| Format | Status in Inspected Repositories | FriedChip+ Implementation Feasibility | Recommended Engine Crate |
|---|---|---|---|
| **Dell PFS** | Extensively documented (`dell_pfs_extract.py`) | High (Pure byte slices + zlib) | `engine-vendor` |
| **Dell ThinOS PKG** | Documented (`dell_pfs_extract.py`) | Medium (Requires LZMA2 / XZ decompression) | `engine-vendor` |
| **Insyde iFlash** | Extensively documented (`insyde_ifd_extract.py`) | High (Direct 0x18 header parsing) | `engine-vendor` |
| **Insyde iFdPacker SFX** | Documented with static password | Medium (Requires 7z stream extraction) | `engine-vendor` |
| **AMI UCP / UAF** | Documented (`ami_ucp_extract.py`) | Medium (UAF chunk parser) | `engine-vendor` |
| **AMI PFAT / BIOS Guard** | Documented (`ami_pfat_extract.py`) | High (Header parsing; script execution excluded) | `engine-vendor` |
| **Phoenix TDK** | Documented (`phoenix_tdk_extract.py`) | Medium (PE resource traversal) | `engine-vendor` |
| **Fujitsu XOR SFX** | Documented (`fujitsu_sfx_extract.py`) | High (Trivial `XOR 0xFF` transform) | `engine-vendor` |
| **Intel Flash Descriptor** | Documented (`bios_analyzer.py`, `pawn`) | Very High (Standard FLREG math) | `engine-binary` |
| **UEFI Volume (`_FVH`)** | Documented (`bios_analyzer.py`, `fixture_builder`) | High (Checksum + block mapping) | `engine-uefi` |
| **SMBIOS / DMI** | Documented (`smbios.py`) | High (Safe structural iterator) | `engine-dmi` |
| **Intel ME Manifest** | Documented (`me_manifest.py`) | High (`$MN2` tag search + version tuple) | `engine-me` |

---
*Format support discovery complete. All binary structures, offsets, and checksum algorithms documented for independent implementation.*
