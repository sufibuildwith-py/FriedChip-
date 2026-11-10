# FriedChip+ — Mandatory Research-First Engineering Protocol

**Product:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** MANDATORY GOVERNANCE PROTOCOL — ACTIVE & ENFORCED  
**Scope:** All future implementations, corrections, refactors, parsers, decoders, validators, extractors, security assessments, and test suites.

---

## 1. Core Principle

> **FriedChip+ should build on established firmware engineering, not repeatedly reinvent it. Research first. Compare. Validate. Reuse responsibly. Then implement.**

Every engineering task begins with empirical source research in `research/` and `research/third_party/`. Speculation, unverified assumptions, and reinventing existing, tested algorithms are strictly prohibited.

---

## 2. Research Before Implementation

Before creating new functionality or modifying existing implementation logic:

1. **Inspect `research/` Directory:**
   - Review architectural designs in `research/architecture/`.
   - Review domain findings in `research/domains/`.
   - Review empirical evidence and benchmarks in `research/evidence/`.
   - Review licenses in `research/licenses/`.
   - Read the existing research reports and adoption/comparison matrices (`MASTER_COMPONENT_ADOPTION_MATRIX.md`, `MASTER_REPOSITORY_REGISTER.md`, `EXECUTIVE_FINDINGS.md`).

2. **Inspect Upstream Source Code in `research/third_party/`:**
   - Inspect actual source code, tests, documentation, commit history, and license files.
   - Do NOT rely exclusively on README descriptions, repository star counts, or previous AI-generated summaries.

3. **Identify Reusable Implementations:**
   - Trace existing algorithms, binary data structures, validation strategies, checksum routines, and test fixtures.

4. **Targeted Authoritative Web Research:**
   - Cross-check findings against authoritative industry specifications (Intel SPI Programming Guides, UEFI Specifications, PCI-SIG, Jedec, vendor whitepapers).

---

## 3. Reuse Before Reinvention

For every implementation task, explicitly investigate whether an existing upstream project already solves the problem.

Prioritize established reference projects present in `research/third_party/`:
- **UEFITool / UEFIExtract:** Canonical UEFI volume, FFS, and section parsing; GUID databases.
- **BIOSUtilities:** Dell PFS, Insyde iFlash, AMI UCP/PFAT, Phoenix TDK extraction.
- **Fiano:** LinuxBoot Go-based UEFI and CBFS structures.
- **coreboot / ifdtool:** Intel Flash Descriptor decoding, region table calculation, master access masks.
- **ME Analyzer:** Intel ME/CSME/TXE manifest decoding and partition tables.
- **PSPTool:** AMD Platform Security Processor directory parsing and entry extraction.
- **CHIPSEC:** Static hardware security assessments, Protected Range (PR) register checks.
- **Binwalk / Unblob:** Binary signature scanning and recursive container extraction architectures.
- **uefi-firmware-parser:** Python UEFI volume decomposition patterns.
- **IFRExtractor-RS:** Fast, memory-safe UEFI 2.8+ IFR opcode disassembly.

Do not introduce new parsers, binary layouts, algorithms, or heuristics when a proven upstream implementation exists. Reuse compatible components when technically and legally appropriate. When direct reuse is unsuitable (e.g. language barrier or copyleft constraints), independently implement the concept using the upstream project's documented behaviour and authoritative specifications.

---

## 4. License and Provenance Requirements

Before adopting or adapting any code:

1. **License Verification:**
   - Identify the repository license and file-level licensing/notices.
   - Check transitive dependencies and their licenses.
2. **Provenance Documentation:**
   - Record upstream repository URL, exact commit SHA, source file paths, and technical reason for adoption.
   - Preserve required copyright, patent, and attribution notices.
3. **Copyleft & Clean-Room Boundaries:**
   - Never copy GPL-licensed code into the proprietary Rust workstation core (`src-tauri/src/engine/`).
   - Implement clean-room functional equivalents derived purely from documented specifications and abstract algorithms.
   - Permissive licenses (MIT, BSD-2-Clause, Apache 2.0) may be directly integrated where technically appropriate.

---

## 5. Mandatory Research Assessment Template

Before any implementation code is written, a research assessment must be produced and verified:

| Field | Required Information |
|---|---|
| **Problem** | Exact engineering problem being solved |
| **Upstream references** | Cloned repositories, source paths, exact commit SHAs |
| **Web references** | Specifications, vendor datasheets, and authoritative standards |
| **Existing solutions** | Relevant algorithms, data structures, and test cases |
| **Reuse decision** | `REUSE`, `ADAPT_CONCEPT`, `INDEPENDENT_IMPLEMENTATION`, or `REJECT` |
| **Limitations** | Known upstream gaps, edge cases, and architectural risks |
| **License review** | License compatibility, attribution requirements, clean-room boundary |
| **Proposed design** | Evidence-backed, minimal implementation approach |

If research reveals that an existing design is incorrect or suboptimal, revise the design before writing code.

---

## 6. Evidence and Honesty

- **No False Inspection:** Never claim source inspection without actually opening and reading the source file and line ranges.
- **No Unverified Claims:** Never claim an upstream component is reusable without checking its implementation and license.
- **No Invented Specifications:** Never invent register layouts, bit positions, magic numbers, or compatibility assumptions.
- **Distinguish Facts from Inferences:** Clearly distinguish documented behaviour from observed empirical behaviour and working hypotheses.
- **Explicit Uncertainties:** Record unresolved uncertainties immediately; prefer `INDETERMINATE` over unsupported positive assertions.
- **Differential Verification:** Implement differential regression tests against trusted upstream tools whenever possible.

---

## 7. Research Availability Protocol & Diagnostics

If `research/third_party/` contains incomplete clones, placeholders, failed downloads, missing files, or corrupted checkouts:

1. Diagnose the problem immediately (check remotes, branches, commit SHAs, file counts).
2. Fetch or recover the missing material before proceeding.
3. Do not silently substitute README-only research.
4. Report unverified items explicitly.

### Verified Upstream Repository Inventory (`research/third_party/`)

The following 25 upstream repositories are fully cloned, verified, and active in `research/third_party/`:

| Repository Name | Tracked Commit SHA | License File | Upstream Remote URL | Source File Count |
|---|---|---|---|---|
| **binwalk** | `2671397` | `LICENSE` (GPL-2.0) | `https://github.com/ReFirmLabs/binwalk` | 302 |
| **biosforge** | `535309e` | `LICENSE` (BSD-2-Clause) | `https://github.com/GPU-Solution/biosforge` | 24 |
| **BIOSUtilities** | `f799425` | `LICENSE` (BSD-2-Clause-Patent) | `https://github.com/platomav/BIOSUtilities` | 38 |
| **bios_extract** | `0a7bc1d` | `COPYING` (GPL-2.0) | `https://github.com/coreboot/bios_extract` | 38 |
| **chipsec** | `f1935b9` | `LICENSE` (GPL-2.0) | `https://github.com/chipsec/chipsec` | 761 |
| **coreboot** | `66e704b1` | `LICENSES` (GPL-2.0) | `https://github.com/coreboot/coreboot` | 24,338 |
| **edk2** | `6bee033` | `License-History.txt` (BSD-2-Clause-Patent) | `https://github.com/tianocore/edk2` | 10,294 |
| **efiSeek** | `573f4b9` | `LICENSE` (GPL-3.0) | `https://github.com/DSecurity/efiSeek` | 30 |
| **efiXplorer** | `110a8d9` | `LICENSE` (MIT) | `https://github.com/binarly-io/efiXplorer` | 60 |
| **EMBA** | `adad91b` | `licenses` (GPL-3.0) | `https://github.com/e-m-b-a/emba` | 1,276 |
| **fiano** | `162b021` | `LICENSE` (BSD-3-Clause) | `https://github.com/linuxboot/fiano` | 313 |
| **flashrom** | `35c679c` | `COPYING.rst` (GPL-2.0) | `https://github.com/flashrom/flashrom` | 474 |
| **fwhunt-scan** | `6fc7115` | `LICENSE` (Apache-2.0) | `https://github.com/binarly-io/fwhunt-scan` | 28 |
| **fwupd** | `868cd5b` | `COPYING` (LGPL-2.1) | `https://github.com/fwupd/fwupd` | 4,970 |
| **IFRExtractor-RS** | `b1b9b36` | `LICENSE` (LGPL-3.0) | `https://github.com/LongSoft/IFRExtractor-RS` | 10 |
| **MEAnalyzer** | `d27ccdb` | `LICENSE` (BSD-2-Clause-Patent) | `https://github.com/platomav/MEAnalyzer` | 7 |
| **me_cleaner** | `41eba87` | `COPYING` (GPL-3.0) | `https://github.com/corna/me_cleaner` | 6 |
| **ofrak** | `428ea75` | `LICENSE` (OFRAK License / Apache-2.0) | `https://github.com/redballoonsecurity/ofrak` | 1,071 |
| **PSPTool** | `5055403` | `LICENSE` (GPL-3.0) | `https://github.com/PSPReverse/PSPTool` | 39 |
| **UEFI-Editor** | `44e8c9b` | None (Unlicensed / Custom rewrite) | `https://github.com/BoringBoredom/UEFI-Editor` | 50 |
| **uefi-firmware-parser** | `e9c8cb4` | `LICENSE` (BSD-2-Clause) | `https://github.com/theopolis/uefi-firmware-parser` | 82 |
| **uefisettings** | `3af9ff3` | `LICENSE` (BSD-3-Clause) | `https://github.com/linuxboot/uefisettings` | 109 |
| **UEFITool** | `de73942` | `LICENSE.md` (BSD-2-Clause) | `https://github.com/LongSoft/UEFITool` | 380 |
| **unblob** | `6166362` | `LICENSE` (MIT) | `https://github.com/onekey-sec/unblob` | 187 |
| **Universal-IFR-Extractor** | `8f55681` | `LICENSE` (GPL-3.0) | `https://github.com/donovan6000/Universal-IFR-Extractor` | 13 |

---

## 8. Implementation Gate

No implementation code may be committed or merged until:
1. The research assessment is completed with upstream citations and commit SHAs.
2. The architectural decision (`REUSE`, `ADAPT`, or `INDEPENDENT_IMPLEMENTATION`) is established.
3. The clean-room boundary and license compatibility are verified.
4. Success criteria and differential tests are formulated.
5. The final report documents what was reused, what was adapted, what was rejected, remaining gaps, and test evidence.
