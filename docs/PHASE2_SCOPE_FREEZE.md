# FriedChip+ — Phase 2 Scope Freeze & Engineering Constraints Decree

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Scope Freeze Decree & Governance Specification  
**Milestone:** Phase 2 Implementation Audit & Scope Freeze  
**Effective Date:** November 2026  
**Auditor / Architect:** Antigravity (Advanced Agentic Coding)  

---

## 1. Scope Freeze Decree & Boundary Invariants

To maintain rigorous architectural discipline, prevent scope creep, and ensure absolute safety during firmware servicing, the Phase 2 implementation scope is formally **FROZEN** under the following non-negotiable constraints:

1. **Monolithic Engine Location:** The entire firmware engine is strictly confined to `src-tauri/src/engine/`.
2. **No Multi-Crate Cargo Workspace Refactor:** The application will remain a single Tauri Cargo project. No external crates (e.g. `crates/engine-binary`, `crates/engine-vendor`) will be created or extracted.
3. **No Dynamic Plugin Loading:** All firmware parsers, decoders, and extractors must be statically compiled into the Rust binary. No runtime DLL / shared-object plugin loading infrastructure will be introduced.
4. **No Premature Hardware Flashing:** No direct SPI hardware programming, USB communication drivers, or raw programmer protocols will be implemented in Phase 2.
5. **No Scope Expansion:** No additional manufacturer formats (Insyde, AMI, Phoenix, Apple) will be implemented until the Phase 2 core formats satisfy all three fixture tiers.

---

## 2. Hardware Roadmap Corrections

The earlier Phase 2 proposal included speculative assertions regarding hardware programmers and external tools. This document enacts the following mandatory corrections:

### A. Flashrom Integration Policy
- **Correction:** `flashrom` will **NOT** be packaged in the FriedChip+ desktop installer, and it is **NOT** a required runtime dependency.
- **Rationale:** Bundling `flashrom` introduces significant cross-compilation complexity on Windows (MinGW/MSYS dependencies, libusb, libftdi), exposes users to kernel driver risks (WinRing0 / direct port I/O hazards), and complicates commercial distribution licensing.
- **Status:** If external tool integration is explored in future phases, it will operate strictly via optional, user-configured external tool paths with explicit safety disclaimers.

### B. CH341A Rust Driver Policy
- **Correction:** The proposed "clean-room 400-line pure-Rust CH341A driver" is **REMOVED** from Phase 2 scope and deferred to a dedicated, independent hardware-engineering assessment in a future phase.
- **Rationale:** Promising hardware safety or a 400-line driver implementation without physical device testing, oscilloscope verification of SPI clock edges, and 1.8V vs 3.3V voltage level protection is irresponsible. Hardware drivers must be developed with actual physical hardware on bench fixtures, not through speculative software coding.

---

## 3. Licensing & Legal Governance Corrections

The legal analysis concerning third-party code and open-source licenses is formally amended as follows:

1. **Licensing Review Requirement:** All previous conclusions concerning the GPL distribution of external tools (e.g. Flashrom, UEFITool, Pawn) are classified as **engineering working notes requiring formal legal review**. FriedChip+ does not provide legal advice.
2. **Strict Clean-Room Architecture:**
   - Zero GPL/LGPL source code is copied or linked into the FriedChip+ codebase.
   - All parsers are implemented independently from open specifications, observable file behaviors, and synthetic reverse-engineered test fixtures.
   - Any future interaction with external CLI tools will occur strictly over standard out-of-process IPC boundaries (stdin/stdout/process execution), never via linked library binaries.

---

## 4. Phase 2 Frozen Scope: Exact Deliverables & Boundaries

Phase 2 implementation is strictly restricted to two reference formats, supported by core memory-safe primitives:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Phase 2 Frozen Workplan                         │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
       ┌────────────────────────────┴────────────────────────────┐
       ▼                                                         ▼
┌──────────────────────────────┐          ┌──────────────────────────────┐
│ Platform Layout Analysis     │          │ Vendor Update Decapsulation  │
│   [Intel Flash Descriptor]   │          │      [Dell PFS Extractor]    │
│ - Signature: 0x0FF0A55A      │          │ - Patterns: EEAA761B...      │
│ - 15-bit FLREG Page Math     │          │ - Pure-Rust Bounded ZLIB     │
│ - 3-Gate Validation Gates    │          │ - Inverted CRC-32 Validation │
│ - Overlap Collision Defense  │          │ - UTF-16LE Name Info Table   │
└──────────────────────────────┘          └──────────────────────────────┘
```

### In-Scope Deliverables
1. **Core Binary Primitives (`src-tauri/src/engine/binary/`):**
   - Memory-safe, bounded `BinaryReader`.
   - Cryptographic and firmware checksum suite (SHA-256, MD5, 8-bit XOR, IEEE CRC-32, Inverted CRC-32, 16-bit additive inverse).
   - Atomic verified write pipeline (`atomic_write_and_verify` with `.part.<uuid>` staging and read-back hash verification).
2. **Intel Flash Descriptor Engine (`src-tauri/src/engine/binary/ifd.rs`):**
   - Parse signature `0x0FF0A55A` at offset `0x10`.
   - Decode FLMAP0/1 region limits using 15-bit page arithmetic.
   - Implement Three-Gate validation (Structural, Platform, Programmer).
   - Reject corrupt signatures, region overlaps, and out-of-bounds regions.
3. **Dell PFS Update Extractor (`src-tauri/src/engine/vendor/dell_pfs.rs`):**
   - Detect Dell PFS section headers and `PFS.HDR.` volumes.
   - Pure-Rust bounded ZLIB decompression with 256MB anti-bomb threshold.
   - Validate 8-bit XOR header/footer checksums and inverted CRC-32 volume footers.
   - Resolve component names and versions from Name Information tables.
   - Extract raw components to workspace disk storage via verified atomic write.
   - Classify extracted BIOS partitions as `Unverified SPI Layout — Not Safe to Flash`.
4. **Desktop Workstation Integration:**
   - React `ExtractorsView.tsx` wired to real Tauri IPC commands (`extract_vendor_firmware`, `inspect_flash_descriptor`).
   - Drag-and-drop or native Windows file picker selection.
   - Automated registration of extracted partitions into the SQLite workspace inventory.

---

## 5. Explicitly Excluded Features (Not In Scope)

The following capabilities are **STRICTLY EXCLUDED** from Phase 2:

- **NO** hardware flasher drivers (CH341A, Dediprog, FTDI, Raspberry Pi).
- **NO** in-system host flashing or kernel drivers (WinRing0, `/dev/mem`).
- **NO** password removal, supervisor bypass, or credential cracking.
- **NO** anti-theft or MDM circumvention features.
- **NO** Insyde iFlash (`$_IFLASH`), AMI PFAT, Phoenix TDK, or Apple PKG parsers (deferred to Phase 3).
- **NO** full UEFI Firmware File System (FFS v2/v3) decompression or driver replacement.
- **NO** multi-crate workspace restructuring.

---

## 6. Verifiable Acceptance Criteria for Phase 2 Sign-Off

Phase 2 will be deemed complete and eligible for formal closure only when:

1. **Parser & Extractor Correctness:**
   - [x] IFD parser passes all synthetic unit and negative test cases.
   - [x] Dell PFS parser passes synthetic end-to-end extraction.
   - [x] Dell PFS parser successfully extracts official Dell Latitude 5490 reference binary without errors.
   - [ ] Automated reference differential test validates 100% byte-for-byte parity against golden outputs.
2. **Safety & Validation Invariants:**
   - [x] Original source files are strictly read-only and never modified.
   - [x] Output partitions are written via verified atomic staging with collision protection.
   - [x] Three-Gate validation model explicitly distinguishes parsing success from programmer readiness.
   - [x] Extracted OEM update partitions are never labeled as flash-ready.
3. **Desktop Workstation Integration:**
   - [x] Tauri IPC commands exposed and typed in TypeScript.
   - [x] Live extraction progress, section breakdown cards, and component tables visible in UI.
   - [x] Extracted files correctly registered in SQLite audit journal.

---

## 7. Next Actions & Standstill Agreement

In accordance with user instructions:
- **All firmware engine development remains paused.**
- **No unsupported `IMPLEMENTED` or `TESTED` claims will be made.**
- **No placeholder features will be presented as functional.**
- **The engineering team will wait for explicit user approval before beginning the next implementation sprint.**
