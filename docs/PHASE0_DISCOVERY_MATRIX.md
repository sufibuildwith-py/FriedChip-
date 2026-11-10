# Phase 0: Feature Parity & Discovery Matrix

## 1. Evidence Taxonomy & Verification Baseline

> [!IMPORTANT]
> **Source Evidence vs. Implementation Verification:**
> In accordance with clean-room standards, a capability is **never** classified as verified or implemented simply because it is described in documentation, is technically feasible, or exists in an external tool.
>
> Implementation and verification statuses are strictly reserved for code and tests present within this repository.

### Status Definitions:
- **`DOCUMENTED`**: Explicitly documented in supplied LBE++ product documentation, manuals, or official guides.
- **`OBSERVED`**: Confirmed through observed execution of an actual application workflow (e.g. video capture, bench log).
- **`ASSUMPTION`**: Inferred from domain conventions or partial evidence; requires empirical validation.
- **`IMPLEMENTED`**: Independently written in this codebase (currently none, pending Phase 1+).
- **`TESTED`**: Successfully passed automated test suite with synthetic fixtures or verified dumps (currently none).
- **`EXCLUDED`**: Explicitly excluded from implementation due to clean-room safety and ethical boundaries.

---

## 2. Feature-Parity Matrix

| Feature / Domain | Tier | Evidence Status | Implementation Status | Input Artefact Type | Expected Output Artefact | Acceptance & Precondition Criteria | Target Phase |
|---|---|---|---|---|---|---|---|
| **Intel Flash Descriptor (IFD) Parser** | Basic | **DOCUMENTED** | Not Implemented | Complete SPI Image | Decoded Region Map & Master Permissions | Match `5A A5 F0 0F` at offset `0x10`; parse FLMAP0/1/2; report region offsets & access masks; reject non-IFD dumps cleanly. | Phase 3 |
| **Universal BIOS Extractor** | Basic | **DOCUMENTED** | Not Implemented | Vendor Update Payload | Raw BIOS Region / SPI Image | Extract compressed payloads from standard containers (InnoSetup, Wise, 7z, SFX) without host execution. | Phase 6 |
| **Temp Directory Extractor** | Basic | **DOCUMENTED** | Not Implemented | Active OEM Flasher | Intercepted Update Payload | Passive notifications (`ReadDirectoryChangesW`) + reconciliation loop; session-scoped; zero process injection. | Phase 6 |
| **Dell Extractor (PFS / HDR / ZPE)** | Basic | **DOCUMENTED** | Not Implemented | Dell Update Package (`.exe` / `.hdr`) | BIOS Region & EC Slice | Parse Dell PFS directory structures and extract discrete BIOS, EC, and microcode sections. | Phase 6 |
| **HP Insyde Extractor & Decryptor** | Basic | **DOCUMENTED** | Not Implemented | HP Update Executable / Encrypted Bin | Raw BIOS Region / Complete SPI Image | Identify Insyde packaging, decrypt encrypted firmware blocks using standard keys, strip packaging padding. | Phase 6 |
| **Lenovo Extractor (FL1 / FL2 / Inno)** | Basic | **DOCUMENTED** | Not Implemented | Lenovo Package / `.FL1` / `.FL2` | BIOS Region & EC Slice | Decompress Lenovo update wrappers; parse FL1/FL2 payload sections into separate BIOS and EC images. | Phase 6 |
| **Asus Capsule Stripper** | Basic | **DOCUMENTED** | Not Implemented | Asus Capsule (`.CAP`) | Raw SPI Image | Strip 2048-byte / vendor security header; verify remaining binary matches exact flash chip capacity. | Phase 6 |
| **Toshiba / Samsung / Sony Extractors** | Basic | **DOCUMENTED** | Not Implemented | OEM Update Package | BIOS Region & EC Slice | Parse vendor-specific self-extracting formats into clean flashable binaries. | Phase 6 |
| **Acer / Gateway Extractor** | Basic | **DOCUMENTED** | Not Implemented | Acer Executable Payload | Raw BIOS Region | Parse Insyde-wrapped installer executable and extract raw flash binary. | Phase 6 |
| **Fujitsu & Panasonic Extractors** | Basic | **DOCUMENTED** | Not Implemented | OEM Update Installer | BIOS Region & EC Slice | Decompress vendor update payload structures. | Phase 6 |
| **EC Firmware Splitter & Profiler** | Basic | **DOCUMENTED** | Not Implemented | Vendor Update / Combined SPI Dump | Programmer-Compatible EC Image | Architecture-specific profiles (ITE, ENE, Nuvoton, MEC); yields `UNCLASSIFIED` on boundary uncertainty. | Phase 5 |
| **Virtual Hex Inspector** | Basic | **DOCUMENTED** | Not Implemented | Any Firmware Binary | Interactive Canvas View | Zero-latency navigation across 64MB+ files; synchronized ASCII view; byte offset inspection. | Phase 2 |
| **Cryptographic Integrity Engine** | Basic | **DOCUMENTED** | Not Implemented | Any Firmware Binary | Checksum Report | Compute primary SHA-256 and legacy MD5/CRC32; streaming calculation for large files. | Phase 2 |
| **Binary Diff Visualizer** | Basic | **DOCUMENTED** | Not Implemented | Two Firmware Binaries | Side-by-Side Diff Map | Accurately highlight every modified, inserted, and deleted byte span between two revisions. | Phase 2 |
| **BIOS Splitter & Merger** | Basic | **DOCUMENTED** | Not Implemented | Complete SPI Image / Multi-Chip Dumps | Split SPI Binaries | Split combined image into discrete physical chip files (e.g. 16MB + 8MB = 24MB) and recombine cleanly. | Phase 2 |
| **SMBIOS Structure Parser** | Basic | **DOCUMENTED** | Not Implemented | Decoded BIOS Region / NVRAM | Decoded SMBIOS Table Record | Decode Type 1, 2, 3 records; strictly decoupled from persistent physical storage discovery. | Phase 7 |
| **DMI Storage Discovery & Preservation** | Basic | **DOCUMENTED** | Not Implemented | Complete SPI Image / BIOS Region | Reconstructed Image with Donor DMI | Vendor-specific profiles (Lenovo NVRAM, HP FSYS, Dell factory blocks); recalculates container CRCs. | Phase 7 |
| **Windows OEM Key (MSDM) Extractor** | Basic | **DOCUMENTED** | Not Implemented | Complete SPI Image / BIOS Region | Extracted 25-Char Product Key | Parse ACPI table directory; locate `MSDM` signature; extract and validate 25-character digital product key. | Phase 7 |
| **Intel ME/CSME Manifest & Health Analyzer** | Black | **DOCUMENTED** | Not Implemented | Complete SPI Image / ME Region | ME Diagnostic Report | Parse FPT table; detect ME major generation (7–16), SKU (Consumer/Corporate), FitC build, file system health. | Phase 4 |
| **Auto ME Clean & Region Injector** | Black | **DOCUMENTED** | Not Implemented | Corrupted SPI Dump + Clean ME Region | Reconstructed SPI Image | Enforce ME compatibility (Gen, SKU, Stepping); transfer FIT config settings; synthesize valid SPI image. | Phase 4 |
| **VSS / NVRAM Variable Cleaner** | Black | **DOCUMENTED** | Not Implemented | BIOS Region / Complete SPI Image | Reconstructed SPI Image | Locate `$VSS` / authenticated variable stores; reset corrupt variables while preserving platform identity. | Phase 7 |
| **Automated Image Reconstructor** | Red | **DOCUMENTED** | Not Implemented | Damaged Dump + Clean Factory Payload | Reconstructed SPI Image | Rebuild broken UEFI volume tables by merging damaged dump identity with pristine factory code. | Phase 4 |
| **FFS Module Replacer / Injector** | Red | **DOCUMENTED** | Not Implemented | BIOS Region + Replacement FFS Module | Reconstructed BIOS Region | Replace specific GUID-matched FFS module or microcode update while preserving volume checksums. | Phase 3 |
| **HP FSYS Block CRC Recalculator** | Red | **DOCUMENTED** | Not Implemented | Modified HP SPI Image | Reconstructed SPI Image | Recalculate and update proprietary HP FSYS block CRC checksums after authorized identity repair. | Phase 7 |
| **Lenovo Service Mode Assistant (4th–8th Gen)** | Green | **DOCUMENTED** | Not Implemented | Lenovo BIOS Dump | Reconstructed SPI Image | Permitted service mode patch for authorized motherboard servicing; non-destructive validation. | Phase 7 |
| **Lenovo ID & Serial Generator** | Green | **DOCUMENTED** | Not Implemented | Replacement Motherboard Dump | Reconstructed SPI Image | Inject original machine serial number, UUID, and model ID into blank or replacement Lenovo boards. | Phase 7 |
| **Advanced Microcontroller EC Profiles** | Green | **DOCUMENTED** | Not Implemented | Combined Capsule / EC Binary | Programmer-Compatible EC Image | Dedicated architecture profiles for ITE (IT8586E/IT8587E), ENE (KB9012/KB9022), Nuvoton, MEC. | Phase 5 |
| **Dell 8FC8 Service Tag Assistant** | Green | **DOCUMENTED** | Not Implemented | Dell Service Dump | Reconstructed SPI Image | Authorized factory service tag reset and verification for motherboard replacements. | Phase 7 |
| **Comprehensive Diagnostic Workstation** | Blue | **DOCUMENTED** | Not Implemented | Complete SPI Image | Workstation Health Report | Full audit of IFD permissions, ME health, UEFI volume integrity, and DMI serialization consistency. | Phase 8 |
| **Batch Firmware Processor** | Blue | **DOCUMENTED** | Not Implemented | Directory of Firmware Binaries | Batch Diagnostic & Unpacking Log | Concurrent execution of validation, unpacking, and hashing across multiple firmware files. | Phase 8 |
| **Local Reference Firmware Library** | Platform | **DOCUMENTED** | Not Implemented | Local Filesystem Database | Indexed Repository Metadata | Local SQLite repository indexing known-clean ME regions, stock EC binaries, and board profiles. | Phase 8 |
| **Apple iCloud / FMM Unlock** | Security | **EXCLUDED** | Excluded | N/A | None | **Excluded per clean-room safety guidelines.** Circumvents platform anti-theft controls. | N/A |
| **Enterprise MDM / DEP Bypass** | Security | **EXCLUDED** | Excluded | N/A | None | **Excluded per clean-room safety guidelines.** Circumvents corporate ownership management. | N/A |
| **Computrace Lock Removal** | Security | **EXCLUDED** | Excluded | N/A | None | **Excluded per clean-room safety guidelines.** Defeats enterprise tracking/anti-theft persistence. | N/A |

---

## 3. Candidate Reference Devices & Public Test Packages

To establish reproducible test baselines without committing proprietary OEM binaries to version control, the following public candidate packages are selected:

| Candidate Model | Manufacturer | Target Package Version | Official Download Portal / Source | Download Date | Known SHA-256 (Installer) | Expected Extraction Artefacts |
|---|---|---|---|---|---|---|
| **Latitude 5490** | Dell | 1.14.0 (or latest reproducible) | Dell Official Support Portal (`dell.com/support`) | Phase 0 Baseline | *To be recorded on download* | Raw SPI Flash Image (16MB or 32MB) + discrete EC payload from PFS container. |
| **EliteBook 840 G5** | HP | 01.12.00 (or latest reproducible) | HP Official Customer Support (`support.hp.com`) | Phase 0 Baseline | *To be recorded on download* | Decrypted Insyde BIOS payload (16MB/32MB) + separate EC image. |
| **ThinkPad T480** | Lenovo | N24ET63W / 1.38 | Lenovo Support Portal (`pcsupport.lenovo.com`) | Phase 0 Baseline | *To be recorded on download* | Uncompressed FL1 (BIOS region) + FL2 (EC image) from Inno installer. |

*Storage Policy:* Binary files remain in local, git-ignored storage (`firmware-samples/public-updates/`). Only the hash manifests and extraction recipes are tracked in Git.

---

## 4. Testing Requirements & Acceptance Gates

Every implementation task must satisfy rigorous acceptance gates before code is merged:

### 4.1 Synthetic Test Fixtures
- Binary parsers (`engine-binary`, `engine-uefi`, `engine-me`, `engine-ec`) must be validated against generated synthetic byte streams with deterministic properties:
  - Valid IFD fixture: Exact 16-byte header, `5A A5 F0 0F` signature, mapped regions.
  - Invalid signature fixture: Signature bit-flips, non-Intel formats, AMD layouts.
  - Truncated fixture: Files cut short at region boundaries.
  - Corrupt volume fixture: Invalid FFS header lengths and bad volume checksums.
  - Ambiguous EC fixture: Slices with uncertain vectors; must verify that engine returns `UNCLASSIFIED`.

### 4.2 Public Manufacturer Package Testing
- Initial extraction tests must use non-confidential, publicly downloadable manufacturer update executables from official support portals.
- Expected outputs must be verified against known-good SHA-256 hashes.

### 4.3 Non-Destructive Invariant & Error Testing
- Every automated test must verify that the original input file remains byte-for-byte identical after processing.
- Mandatory test cases for all operations:
  1. **Malformed Input:** Garbage data fed into parser returns typed error, does not panic or crash.
  2. **Truncated Input:** Incomplete file triggers `UnexpectedEof` error with byte position.
  3. **Platform Mismatch:** Attempting ME replacement across different major generations returns `IncompatiblePlatformError`.
  4. **Job Cancellation:** Async jobs interrupted mid-execution clean up temporary scratch files and commit nothing to the output database.
