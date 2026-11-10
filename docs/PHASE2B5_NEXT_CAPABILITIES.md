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

### Capability 1: Intel Management Engine (ME) Partition Analysis
- **Selected Component:** `MEAnalyzer`
- **Upstream Path:** `research/third_party/MEAnalyzer` (HEAD Commit: `d27ccdba0a`)
- **License:** BSD-2-Clause (Plato Mavropoulos)
- **Implementation Strategy:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (JSON protocol over stdin/stdout via adapter wrapper)
- **Technical Justification:**
  - Authoritative reference tool maintained by Plato Mavropoulos for Intel ME, TXE, and SPS firmware partitions.
  - Decodes the Flash Partition Table (FPT), partition headers (FTPR, FTUP, MFS), version numbers, chipsets, and Huffman dictionaries.
  - Native CLI (`MEA.py`) outputs terminal ASCII text; an adapter wrapper will translate native output into FriedChip+'s standard JSON protocol.
  - Out-of-process execution guarantees total memory safety, process isolation, and zero native Python dependency bloat in the Tauri core.

### Capability 2: AMD Platform Security Processor (PSP) Table Parsing
- **Selected Component:** `PSPTool`
- **Upstream Path:** `research/third_party/PSPTool` (HEAD Commit: `5055403889`)
- **License:** GPL-3.0 (Christian Werling)
- **Implementation Strategy:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (JSON protocol over stdin/stdout via adapter wrapper)
- **Technical Justification:**
  - The premier open-source tool for AMD firmware structures, covering Zen generations and earlier APU/CPUs.
  - Decodes AMD PSP directory structures (Directory Table 1 and 2), BIOS directories, entry types (SMU, ABL, PSP core binaries), and cryptographic public keys.
  - Elevating PSPTool establishes platform parity for AMD systems, complementing Intel IFD and ME analysis.
  - Executing as an isolated external worker process respects GPL-3.0 process boundaries and guarantees that parsing errors cannot compromise the host application.

### Capability 3: UEFI IFR (Internal Form Representation) Setup Extraction
- **Selected Component:** `IFRExtractor-RS`
- **Upstream Path:** `research/third_party/IFRExtractor-RS` (HEAD Commit: `b1b9b36c00`)
- **License:** BSD-2-Clause
- **Implementation Strategy:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (Executable Subprocess CLI Adapter)
- **Technical Justification:**
  - Upstream is packaged as a standalone binary CLI (`src/main.rs`, edition 2015, `nom 4.2.3`), rather than an exported Cargo library crate.
  - Extracts and decodes UEFI setup forms, hidden OEM debug menus, and UEFI variable mappings from IFR/HPK bytecode inside Setup DXE modules.
  - Executing via an external subprocess adapter (or rewriting cleanly into modern Rust) transforms FriedChip+ into a functional BIOS configuration explorer without invasive foreign code dependencies.

---

## 3. Rejected & Deferred Alternatives

| Candidate Upstream | Capability | Reason for Deferral or Rejection |
|--------------------|------------|----------------------------------|
| `uefisettings` | UEFI NVRAM runtime inspection | Deferred: Designed for Linux host `/sys/firmware/efi/efivars` and UEFI Shell boot services runtime; does not parse offline raw SPI flash NVRAM binary stores (NVAR/VSS) on Windows. |
| `ofrak` | Binary unpack / repack | Rejected: OFRAK Community License contains non-commercial restrictions. |
| `UEFI-Editor` | Web NVRAM editor | Rejected: Unlicensed repository; lacks open-source grant. |
| `chipsec` | Hardware register checks | Deferred: Phase 3 is locked; requires live kernel drivers (`/dev/mem`, WinRing0). |
| `flashrom` flashing | SPI flashing | Deferred: Physical hardware flashing is strictly locked until Phase 3. |
| `me_cleaner` | ME truncation | Deferred: Image modification and truncation carry high risk; read-only inspection first. |
