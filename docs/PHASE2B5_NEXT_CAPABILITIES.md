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

## 2. The Next 3 Capabilities (DESIGNED — Phase 3 Candidates)

### Capability 1: Intel Management Engine (ME) Partition Analysis
- **Selected Component:** `MEAnalyzer`
- **Repository Identity:** `https://github.com/platomav/MEAnalyzer.git`
- **Local Path:** `research/third_party/MEAnalyzer`
- **Pinned Commit SHA:** `d27ccdba0a4b706c4b14d84c62f275e7a9b19d5c`
- **Verified License:** BSD-2-Clause with patent grant (Plato Mavropoulos, `LICENSE`)
- **Integration Mechanism:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (JSON protocol over stdin/stdout via adapter wrapper script)
- **Implementation Status:** `DESIGNED` (Adapter runner is `NOT_IMPLEMENTED` in Phase 2)
- **Legal Review Status:** `LEGAL_REVIEW_REQUIRED` (Permissive license with patent clause; low copyleft risk, attribution required)
- **Technical Justification:**
  - Authoritative reference tool maintained by Plato Mavropoulos for Intel ME, TXE, and SPS firmware partitions.
  - Decodes the Flash Partition Table (FPT), partition headers (FTPR, FTUP, MFS), version numbers, chipsets, and Huffman dictionaries.
  - Native CLI (`MEA.py`) outputs terminal ASCII text; an adapter wrapper will translate native output into FriedChip+'s standard JSON protocol.
  - Out-of-process execution provides a memory fault boundary preventing Python crashes from destabilizing the Tauri host process, avoiding heavy native Python runtime embedding. Full security isolation requires OS sandboxing controls (Job Objects, AppContainer) scheduled for future implementation.

### Capability 2: AMD Platform Security Processor (PSP) Table Parsing
- **Selected Component:** `PSPTool`
- **Repository Identity:** `https://github.com/PSPReverse/PSPTool.git`
- **Local Path:** `research/third_party/PSPTool`
- **Pinned Commit SHA:** `50554038891dff1d3c03ee7ecf7194f1c1a93176`
- **Verified License:** GPL-3.0 (Christian Werling, `LICENSE`)
- **Integration Mechanism:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (JSON protocol over stdin/stdout via adapter wrapper script)
- **Implementation Status:** `DESIGNED` (Adapter runner is `NOT_IMPLEMENTED` in Phase 2)
- **Legal Review Status:** `LEGAL_REVIEW_REQUIRED` (Strict copyleft; IPC process boundaries do not automatically establish GPL exemption; formal legal analysis required before binary redistribution)
- **Technical Justification:**
  - The premier open-source tool for AMD firmware structures, covering Zen generations and earlier APU/CPUs.
  - Decodes AMD PSP directory structures (Directory Table 1 and 2), BIOS directories, entry types (SMU, ABL, PSP core binaries), and cryptographic public keys.
  - Elevating PSPTool establishes platform parity for AMD systems, complementing Intel IFD and ME analysis.
  - Spawning as a separate process provides fault containment against unhandled parsing exceptions. Architectural separation is an isolation boundary but must not be treated as a legal determination of GPL compliance.

### Capability 3: UEFI IFR (Internal Form Representation) Setup Extraction
- **Selected Component:** `IFRExtractor-RS`
- **Repository Identity:** `https://github.com/LongSoft/IFRExtractor-RS.git`
- **Local Path:** `research/third_party/IFRExtractor-RS`
- **Pinned Commit SHA:** `b1b9b36c000e1e97d1dfd9a4b373400a4ceb52bf`
- **Verified License:** BSD-2-Clause (LongSoft, `LICENSE.md`)
- **Integration Mechanism:** `INTEGRATE_AS_EXTERNAL_ADAPTER` (Standalone CLI Subprocess Adapter)
- **Implementation Status:** `DESIGNED` (Adapter runner is `NOT_IMPLEMENTED` in Phase 2; clean-room Rust rewrite evaluated for Phase 3)
- **Legal Review Status:** `LEGAL_REVIEW_REQUIRED` (Permissive BSD-2-Clause license; minimal legal complexity, copyright attribution required)
- **Technical Justification:**
  - Upstream is packaged as a standalone binary CLI (`src/main.rs`, edition 2015, `nom 4.2.3`), rather than an exported Cargo library crate, and has zero unit tests.
  - Extracts and decodes UEFI setup forms, hidden OEM debug menus, and UEFI variable mappings from IFR/HPK bytecode inside Setup DXE modules.
  - Executing via an external subprocess adapter (or re-implementing natively using modern nom/winnow combinators) transforms FriedChip+ into a functional BIOS configuration explorer without foreign code entanglement.

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
