# Feature-Tier & Module Capability Matrix

## 1. Tier Architecture & Inheritance Model

LBE++ utilizes a cumulative, tiered licensing and capability model. Our clean-room engineering plan adopts this established 5-tier taxonomy to organize module development:

```
[Basic] ────► [Black] ────► [Red] ────► [Green] ────► [Blue]
```

- **Basic:** Core extraction (Universal, Temp-dir, Vendor: Dell, HP, Toshiba, Samsung, Sony, Acer, Asus), basic EC extraction, basic DMI data viewer, basic hex inspection, CRC calculators, BIOS splitting.
- **Black:** Basic tier + automated ME/CSME analysis and cleaning tools, extended dump repairs, advanced security dump cleaners.
- **Red:** Black tier + professional repair workflows, multi-region image rebuilding, advanced manufacturer-specific patch routines.
- **Green:** Red tier + specialized manufacturer workflows (Lenovo 4th–8th gen BIOS patchers, Lenovo ID generators, Dell 8FC8/advanced service routines, extended EC microcontrollers).
- **Blue:** Green tier + advanced platform tools, comprehensive multi-vendor repair kits, batch automation, and specialized workstation diagnostics.

---

## 2. Comprehensive Feature Inventory

### Evidence Taxonomy:
- **DOCUMENTED**: Explicitly described in supplied LBE++ documentation or manuals.
- **OBSERVED**: Confirmed through actual application workflows or recorded tool execution.
- **ASSUMPTION**: Inferred from tooling taxonomy; requires empirical verification and sample dumps.
- **IMPLEMENTED**: Independently implemented in this codebase.
- **TESTED**: Passed defined test suite with known fixtures.
- **EXCLUDED**: Outside permitted project scope due to ethical or legal boundaries.

| Tier | Category | Feature / Tool Name | Evidence Status | Safety Classification | Target Phase | Implementation Scope & Notes |
|---|---|---|---|---|---|---|
| **Basic** | Core Extraction | Universal BIOS Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Unpack standard container formats (ZIP, 7z, Inno, self-extracting EXEs). |
| **Basic** | Core Extraction | Temp Directory Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Passive filesystem notifications (`ReadDirectoryChangesW`) + periodic reconciliation; session-scoped; zero process injection. |
| **Basic** | Vendor Extract | Dell Extractor (HDR / PFS) | **DOCUMENTED** | Supported Servicing | Phase 6 | Unpack Dell client update binaries into raw BIOS and EC components. Candidate: Dell Latitude 5490. |
| **Basic** | Vendor Extract | HP Extractor & Decryptor | **DOCUMENTED** | Supported Servicing | Phase 6 | Unpack HP Insyde/Phoenix update payloads and decrypt encrypted updates. Candidate: HP EliteBook 840 G5. |
| **Basic** | Vendor Extract | Lenovo Extractor (FL1 / FL2 / Inno) | **DOCUMENTED** | Supported Servicing | Phase 6 | Decompress Lenovo update wrappers; parse FL1/FL2 payload sections into separate BIOS and EC images. Candidate: Lenovo ThinkPad T480. |
| **Basic** | Vendor Extract | Toshiba Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Extract BIOS and EC regions from Toshiba package formats. |
| **Basic** | Vendor Extract | Samsung Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Extract and unpack Samsung proprietary update packages. |
| **Basic** | Vendor Extract | Sony Vaio Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Unpack Sony Vaio compressed update packages into raw firmware images. |
| **Basic** | Vendor Extract | Acer / Gateway Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Extract raw SPI payloads from Acer Insyde update executables. |
| **Basic** | Vendor Extract | Asus CAP Stripper | **DOCUMENTED** | Supported Servicing | Phase 6 | Strip 2048-byte / vendor capsule header to produce raw SPI flash image. |
| **Basic** | Vendor Extract | Fujitsu Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Extract BIOS image payloads from Fujitsu system updates. |
| **Basic** | Vendor Extract | Panasonic Toughbook Extractor | **DOCUMENTED** | Supported Servicing | Phase 6 | Unpack Panasonic proprietary package formats. |
| **Basic** | EC Extraction | EC Firmware Splitter & Slicer | **DOCUMENTED** | Supported Servicing | Phase 5 | Architecture-specific profile validation (ITE, ENE, Nuvoton, MEC); yields `UNCLASSIFIED` on boundary uncertainty. |
| **Basic** | DMI / Identity | DMI Data Viewer | **DOCUMENTED** | Supported Servicing | Phase 7 | Inspect SMBIOS tables (Type 1, 2, 3 records) independent of physical storage discovery. |
| **Basic** | DMI / Identity | Windows OEM Key (MSDM) Extractor | **DOCUMENTED** | Supported Servicing | Phase 7 | Locate and extract ACPI MSDM digital license key. |
| **Basic** | Utilities | Virtual Hex Inspector | **DOCUMENTED** | Supported Servicing | Phase 2 | 64MB+ smooth virtualization, offset jumping, data inspectors. |
| **Basic** | Utilities | Checksum & Hash Utility | **DOCUMENTED** | Supported Servicing | Phase 2 | SHA-256 (primary), MD5 (legacy reference), CRC32. |
| **Basic** | Utilities | BIOS Splitter & Merger | **DOCUMENTED** | Supported Servicing | Phase 2 | Split unified dumps into discrete chip sizes (e.g., 16MB + 8MB = 24MB). |
| **Basic** | Utilities | Binary Diff Visualizer | **DOCUMENTED** | Supported Servicing | Phase 2 | Visual comparison of two firmware revisions highlighting modified byte spans. |
| **Black** | Intel ME | Intel ME/CSME Analyzer | **DOCUMENTED** | Supported Servicing | Phase 4 | Parse FPT, manifest, major/minor version, SKU, FitC build, initialization state. |
| **Black** | Intel ME | Auto ME Clean & Region Injector | **DOCUMENTED** | Supported Servicing | Phase 4 | Replace corrupted ME region with clean repository image after generation/SKU/stepping compatibility check. |
| **Black** | Security | Security Dump Cleaner (Generic) | **DOCUMENTED** | Permitted Alternative | Phase 7 | Permitted: Reset corrupted NVRAM / VSS state. Prohibited: Lock bypass. |
| **Black** | Security | Apple EFI / FSC Variable Cleaner | **DOCUMENTED** | Permitted Alternative | Phase 7 | Permitted: Recalculate FSC CRC and NVRAM variables. Prohibited: MDM bypass. |
| **Red** | BIOS Repair | Automated Image Reconstructor | **DOCUMENTED** | Supported Servicing | Phase 4 | Reconstruct damaged UEFI volumes using clean factory payload. |
| **Red** | BIOS Repair | Image Injector & Module Replacer | **DOCUMENTED** | Supported Servicing | Phase 3 | Replace specific FFS modules / microcode while preserving volume layout. |
| **Red** | Vendor Repair | HP FSYS Block CRC Recalculator | **DOCUMENTED** | Supported Servicing | Phase 7 | Recalculate and update HP FSYS block checksums after authorized DMI edit. |
| **Green** | Vendor Repair | Lenovo BIOS Patcher (4th–8th Gen) | **DOCUMENTED** | Permitted Alternative | Phase 7 | Permitted: Supervisor recovery via documented service mode. |
| **Green** | DMI / Identity | Lenovo ID & Asset Generator | **DOCUMENTED** | Supported Servicing | Phase 7 | Re-serialize replaced motherboard with original system serial/UUID using vendor NVRAM profile. |
| **Green** | EC Extraction | Advanced EC Microcontrollers | **DOCUMENTED** | Supported Servicing | Phase 5 | Specialized architecture profiles for ITE (IT8586), ENE (KB9012), Nuvoton, MEC. |
| **Green** | Security | Dell 8FC8 Service Assistant | **DOCUMENTED** | Permitted Alternative | Phase 7 | Permitted: Factory service tag reset. Prohibited: Password cracking. |
| **Blue** | Diagnostics | Comprehensive Hardware Diagnostic | **DOCUMENTED** | Supported Servicing | Phase 8 | Full diagnostic report: IFD permissions, ME health, DMI integrity, programmer compatibility. |
| **Blue** | Automation | Batch Firmware Processor | **DOCUMENTED** | Supported Servicing | Phase 8 | Batch validation, hashing, and unpacking across firmware corpora. |
| **Platform** | Ecosystem | Local Firmware Database | **DOCUMENTED** | Supported Servicing | Phase 8 | Offline database of clean ME regions, EC binaries, and reference profiles. |
| **Platform** | Ecosystem | Diagnostic Tutorials & Reference | **DOCUMENTED** | Supported Servicing | Phase 8 | Integrated offline repair guides and pinout references. |

---

## 3. Restricted & Excluded Features Policy

Where legacy tools advertise security bypasses, our clean-room workstation implements strictly legitimate servicing alternatives:

| Requested / Legacy Feature | Status | Reason for Restriction | Permitted Legitimate Alternative |
|---|---|---|---|
| **Apple iCloud / FMM Unlock** | **EXCLUDED** | Circumvents platform activation locks and anti-theft controls. Violates ethical boundaries. | Display NVRAM variable status and Apple FSC block integrity reporting only. |
| **Enterprise MDM / DEP Bypass** | **EXCLUDED** | Circumvents corporate ownership and device management controls. | Report enrollment state without modification; permit authorized factory BIOS re-flashing. |
| **Computrace / Absolute Lock Removal** | **EXCLUDED** | Defeats asset tracking and anti-theft management firmware flags. | Audit and display Absolute persistence agent status in the UEFI volume report. |
| **Brute-Force Password Generators** | **EXCLUDED** | Unauthorized credential extraction and challenge bypass. | Support authorized service tag re-serialization and factory master clear workflows where documented by OEM. |

---

## 4. Unresolved Product-Tier Questions

- **"Blue++" Tier Status:** Certain technician forum threads and commercial listings refer to a "Blue++" label. However, official EgyfixLab documentation and manuals catalog only Basic, Black, Red, Green, and Blue tiers (often bundling top-tier additions under "LBE MasterKit"). Until concrete documentation demonstrates that "Blue++" is a separate, distinct tier with unique functional modules, it is recorded as an unresolved commercial variation rather than an architectural tier.
