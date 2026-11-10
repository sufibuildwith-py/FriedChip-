# Development Roadmap & Milestone Criteria

## Engineering Milestone Overview

```
Phase 0 ──► Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6 ──► Phase 7 ──► Phase 8 ──► Phase 9
Baseline     Foundation   Binary     IFD/UEFI    Intel ME    EC Slicer   Vendors     DMI/Asset   Workstation Packaging
Research     Desktop      Engine     Parser      Engine      Engine      Extractors  Repair      Library     Release
```

---

## Phase 0: Product Discovery & Clean-Room Baseline (Reviewed & Ready for Implementation Planning)
- **Status:** Complete & Approved for Implementation Planning.
- **Scope & Deliverables:**
  - Formulate clean-room safety protocols, data protection policies, and copyright exclusion rules.
  - Reconcile feature tier taxonomy across Basic, Black, Red, Green, and Blue tiers.
  - Formulate 3-Tier Validation Architecture: Structural Validation, Platform Compatibility, and Programmer Compatibility Assessment.
  - Establish candidate reference packages for Dell Latitude 5490, HP EliteBook 840 G5, and Lenovo ThinkPad T480.
  - Isolate unverified tier variations (Blue++) as unresolved commercial questions.
- **Milestone Exit Gate:**
  - All formal documentation approved.
  - Developer establishes local `firmware-samples/` directory and installs the Rust toolchain (`rustup`).

---

## Phase 1: Desktop Application Foundation (Complete & Verified)
- **Status:** Complete & Passed 100% of Acceptance Tests.
- **Scope & Deliverables:**
  - Initialized Tauri 2 desktop shell with React 19, TypeScript, and Vite.
  - Implemented bundled SQLite database engine with versioned migrations (`001_initial_schema.sql`).
  - Created 9 workstation navigation destinations with dark graphite engineering UI aesthetic.
  - Implemented safe read-only firmware ingestion workflow (streaming SHA-256 + legacy MD5, structural signature detection).
  - Implemented append-only operation audit journal and execution logs in SQLite WAL mode.
  - Passed 8 comprehensive automated unit tests covering hashing, persistence, format validation, duplicate rejection, non-destructive source preservation, and workspace recovery across restarts.
- **Milestone Exit Gate:**
  - Production release binary built at `src-tauri/target/release/lbe-desktop.exe` (10.5 MB).
  - All 8 unit tests passing (0 failures).
  - UI screenshots captured and verified.

---

## Phase 2: Binary Engine & Workstation Utilities (`engine-binary`)
- **Scope:**
  - Implement pure Rust binary engine: memory-safe buffer slicing, pattern searching (Boyer-Moore), entropy analysis.
  - Implement high-performance Virtual Hex Viewer (virtualized canvas rendering, 60fps scrolling on 64MB+ files).
  - Implement visual Binary Diff visualizer showing byte disparities between two images.
  - Implement BIOS Splitter / Merger utility for multi-chip board configurations.
- **Definition of Done:**
  - Virtual Hex Viewer renders synthetic 64MB files with zero UI freezing.
  - Binary diff accurately identifies 100% of inserted/modified/deleted byte spans against synthetic diff fixtures.
  - Unit tests cover edge cases: empty files, odd-length buffers, out-of-bounds slice requests, and cancelled operations.

---

## Phase 3: Intel Flash Descriptor (IFD) & UEFI Engine (`engine-uefi`)
- **Scope:**
  - Implement Intel Flash Descriptor parser verifying standard signature `0x0FF0A55A` (`5A A5 F0 0F` at offset `0x10`).
  - Parse Descriptor Map tables (FLMAP0/1/2): Flash Components, Flash Regions (BIOS, ME, GbE, PDR), Flash Master permissions.
  - Implement UEFI Firmware Volume (FV) and Firmware File System (FFS) header scanner and GUID lookup.
  - Differentiate external SPI programming constraints from host-based flashing permissions.
  - Surface visual Region Map in UI displaying discrete partitions.
- **Definition of Done:**
  - Correctly parses valid IFD images and cleanly rejects non-IFD/AMD/corrupt images with typed `InvalidDescriptorSignature` errors.
  - Unit tests verify synthetic valid, corrupted, truncated, and non-Intel binary fixtures.

---

## Phase 4: Intel ME / CSME / TXE Engine (`engine-me`)
- **Scope:**
  - Implement parser for Flash Partition Table (FPT) and ME/CSME manifests (CSME 7 through 16).
  - Extract ME major generation, SKU (Consumer, Corporate, Slim, Lite), FitC build version, and health state.
  - Implement Auto ME Clean pipeline enforcing strict pre-flight compatibility (Generation, SKU, Stepping match) and preserving system FIT configuration parameters.
  - Implement Automated Image Reconstructor merging factory code with preserved identity.
- **Definition of Done:**
  - Correctly identifies ME parameters from real and synthetic dumps.
  - Clean ME injection succeeds only when compatibility checks pass; rejects mismatched SKUs or major generations with clear diagnostics.
  - Three-tier validation confirms reconstructed output meets structural and programmer compatibility gates.

---

## Phase 5: Embedded Controller (EC) Engine (`engine-ec`)
- **Scope:**
  - Implement architecture-specific EC validation profiles for ITE (IT8586E/IT8587E), ENE (KB9012/KB9022), Nuvoton (NPCE), and Microchip (MEC).
  - Enforce microcontroller microcode validation: inspect vector tables, jump addresses, and padding byte patterns.
  - Implement fail-safe classification: ambiguous or non-standard extractions output `UNCLASSIFIED`.
- **Definition of Done:**
  - Accurately identifies and slices valid EC code from candidate packages and multi-region dumps.
  - Rejects truncated or misaligned slices; flags extracted EC files as "Programmer-Compatible" only when architecture profile checks pass.

---

## Phase 6: Manufacturer-Specific Firmware Extractors (`engine-vendor`)
- **Scope:**
  - **Dell:** PFS and HDR container unpackers for Client BIOS update packages (tested with Latitude 5490 candidate).
  - **HP:** Insyde payload extractor and decryptor for encrypted update packages (tested with EliteBook 840 G5 candidate).
  - **Lenovo:** Decompression of Inno/7-Zip wrappers and parsing of FL1/FL2 payload files (tested with ThinkPad T480 candidate).
  - **Asus / Acer / Toshiba / Samsung / Sony / Fujitsu / Panasonic:** Vendor-specific unpackers and CAP header strippers.
  - **Temp Directory Extractor:** Passive filesystem notifications (`ReadDirectoryChangesW`) with periodic reconciliation loop; session-scoped; zero process injection.
- **Definition of Done:**
  - Automated tests unpack public update executables for each supported vendor and produce byte-verified BIOS/EC images matching expected SHA-256 hashes.
  - Gracefully reports unsupported or legacy vendor formats without application crashes.

---

## Phase 7: DMI & Serialization Preservation (`engine-dmi`)
- **Scope:**
  - Decouple SMBIOS table record decoding (Type 1, 2, 3) from persistent physical firmware storage discovery.
  - Implement manufacturer storage profiles: UEFI NVRAM stores (`$VSS`, Authenticated Variables), HP FSYS blocks, Dell Service Tag blocks, Lenovo asset blocks.
  - Extract and display Service Tag, Serial Number, UUID, Board Model, and Windows OEM product key (MSDM).
  - Implement DMI transfer pipeline: transfer donor serialization into target clean image and recalculate structure-specific CRCs/checksums.
  - Implement permitted service mode / factory serialization assistants (Lenovo, Dell).
- **Definition of Done:**
  - Donor DMI values correctly injected into clean target without corrupting surrounding NVRAM or SMBIOS checksums.
  - MSDM extractor recovers exact 25-character digital product keys.

---

## Phase 8: Advanced Workstation Utilities & Firmware Library
- **Scope:**
  - Local offline firmware library: indexing known-clean ME repositories, EC binaries, and board profiles.
  - Batch firmware processor for automated bulk validation, unpacking, and hashing.
  - Comprehensive Diagnostic Workstation: generates exportable diagnostic reports (HTML/PDF) detailing IFD permissions, ME health, UEFI volume hierarchy, and programmer compatibility.
- **Definition of Done:**
  - Batch processing successfully processes firmware directories concurrently.
  - Diagnostic report generates complete hardware audit matching technician bench requirements.

---

## Phase 9: Verification, Packaging & Production Release
- **Scope:**
  - Comprehensive regression testing across full test corpus (synthetic fixtures, public packages, bench pairs).
  - MSI installer generation with Tauri bundler.
  - Final documentation, user manual, and offline troubleshooting guide.
- **Definition of Done:**
  - 100% test pass rate across all normal, malformed, and failure test suites.
  - Standalone Windows installer installs, runs completely offline, and passes all security checks.
