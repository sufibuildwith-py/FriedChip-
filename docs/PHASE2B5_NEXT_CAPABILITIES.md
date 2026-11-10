# Phase 2B.5 — Next Capability Selection Assessment

**Project:** FriedChip+  
**Release:** Phase 2B.5  
**Scope:** Selection and Evidence-Backed Justification of the Next 3 Engineering Capabilities.

---

## 1. Selection Methodology

In accordance with the Mandatory Research-First Protocol, candidate capabilities were evaluated against five criteria:
1. **Upstream Source Maturity & Verification:** Cloned and audited in `research/third_party/`.
2. **License Compatibility:** Permissive open-source licenses (MIT, BSD, Apache) with zero commercial restrictions.
3. **Memory Safety & Language Alignment:** Preference for direct compiled Rust libraries, followed by clean isolated subprocess adapters.
4. **Architectural Value:** Complements existing Phase 2 foundations (IFD, Dell PFS, UEFI FV) without violating the Phase 3 physical hardware flashing lock.
5. **Zero Speculative Engineering:** Established authoritative algorithms with available differential golden test vectors.

---

## 2. The Next 3 Capabilities

### Capability 1: UEFI IFR (Internal Form Representation) Setup Extraction
- **Selected Component:** `IFRExtractor-RS`
- **Upstream Path:** `research/third_party/IFRExtractor-RS` (HEAD Commit: `b1b9b36c00`)
- **License:** BSD-2-Clause
- **Implementation Strategy:** `INTEGRATE_AS_LIBRARY` (Direct Cargo dependency)
- **Technical Justification:**
  - `IFRExtractor-RS` is implemented in pure, modern Rust without C bindings or unsafe foreign dependencies.
  - UEFI setup menus, hidden manufacturer debug options, and variable mappings are encoded in standard IFR/HPK bytecodes inside Setup DXE drivers.
  - Adding IFR decoding transforms FriedChip+ from a container carver into a functional BIOS configuration analyzer.
  - Memory-safe, zero subprocess latency, and perfectly matches the Tauri Rust backend.

### Capability 2: UEFI NVRAM Variable & Settings Inspection
- **Selected Component:** `uefisettings`
- **Upstream Path:** `research/third_party/uefisettings` (HEAD Commit: `3af9ff3c95`)
- **License:** BSD-3-Clause
- **Implementation Strategy:** `INTEGRATE_AS_LIBRARY` (Direct Cargo dependency)
- **Technical Justification:**
  - Written in pure Rust by the LinuxBoot team with clean data models and comprehensive unit tests.
  - Decodes VSS, NVAR, and authenticated UEFI variable stores embedded in NVRAM firmware volumes.
  - Allows security auditing of Secure Boot state (PK, KEK, db, dbx), boot order, and persistent vendor tokens.
  - Directly inspects byte slices already validated by FriedChip+'s `FirmwareVolume` module.

### Capability 3: Intel Management Engine (ME) Partition Analysis
- **Selected Component:** `MEAnalyzer`
- **Upstream Path:** `research/third_party/MEAnalyzer` (HEAD Commit: `d27ccdba0a`)
- **License:** BSD-2-Clause
- **Implementation Strategy:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (JSON protocol over stdin/stdout)
- **Technical Justification:**
  - Authoritative reference tool maintained by Plato Mavropoulos for Intel ME, TXE, and SPS firmware partitions.
  - Decodes the Flash Partition Table (FPT), partition headers (FTPR, FTUP, MFS), version numbers, chipsets, and Huffman dictionaries.
  - Because MEAnalyzer has a large historical firmware database and Python runtime, wrapping it via the Phase 2B.5 External Adapter runner guarantees total process isolation and memory safety without bloating the Tauri binary.

---

## 3. Rejected Alternatives for Next Milestone

| Candidate Upstream | Capability | Reason for Deferral or Rejection |
|--------------------|------------|----------------------------------|
| `ofrak` | Binary unpack / repack | Rejected: OFRAK Community License contains non-commercial restrictions. |
| `UEFI-Editor` | Web NVRAM editor | Rejected: Unlicensed repository; lacks open-source grant. |
| `chipsec` | Hardware register checks | Deferred: Phase 3 is locked; requires live kernel drivers (`/dev/mem`, WinRing0). |
| `flashrom` flashing | SPI flashing | Deferred: Physical hardware flashing is strictly locked until Phase 3. |
| `me_cleaner` | ME truncation | Deferred: Image modification and truncation carry high risk; read-only inspection first. |
