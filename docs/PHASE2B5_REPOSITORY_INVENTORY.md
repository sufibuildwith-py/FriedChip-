# Phase 2B.5 — Third-Party Repository Inventory

**Project:** FriedChip+  
**Release:** Phase 2B.5  
**Audit Date:** 2026-11-10  
**Repository Count:** 25 Cloned Repositories Under `research/third_party/`

---

## 1. Executive Summary

In compliance with the FriedChip+ Mandatory Research-First Engineering Protocol, all 25 cloned repositories residing in `research/third_party/` were inventoried, checked against live Git metadata (HEAD commit SHA, active branch, remote origin, tracked file count), and audited for license files and local checkout integrity.

All 25 directories are confirmed genuine Git checkouts. One platform-specific filesystem defect was identified: `unblob` contains uppercase/lowercase file collisions on Windows NTFS filesystems within its test output fixtures, which is documented below.

---

## 2. Complete Repository Inventory Table

| # | Repository Name | HEAD Commit SHA | Branch | Primary Remote URL | Tracked Source Files | License File Present | Verified License Type | Local Status |
|---|-----------------|-----------------|--------|--------------------|----------------------|----------------------|-----------------------|--------------|
| 1 | `binwalk` | `26713972e3` | `master` | `https://github.com/ReFirmLabs/binwalk.git` | 302 | Yes (`LICENSE`) | MIT | Clean checkout (Rust v3) |
| 2 | `biosforge` | `535309e407` | `master` | `https://github.com/GPU-Solution/biosforge.git` | 24 | Yes (`LICENSE.md`) | BSD-2-Clause | Clean checkout (Python) |
| 3 | `BIOSUtilities` | `f799425af9` | `main` | `https://github.com/platomav/BIOSUtilities.git` | 38 | Yes (`LICENSE`) | BSD-2-Clause | Clean checkout (Python) |
| 4 | `bios_extract` | `0a7bc1d717` | `main` | `https://github.com/coreboot/bios_extract.git` | 38 | Yes (`LICENSE`) | GPL-2.0 | Clean checkout (C/Python) |
| 5 | `chipsec` | `f1935b9bf2` | `chipsec2` | `https://github.com/chipsec/chipsec.git` | 761 | Yes (`COPYING`) | GPL-2.0 | Clean checkout (Python/C) |
| 6 | `coreboot` | `66e704b166` | `main` | `https://review.coreboot.org/coreboot.git` | 24,338 | Yes (`COPYING`) | GPL-2.0 | Clean checkout (C) |
| 7 | `edk2` | `6bee033c69` | `master` | `https://github.com/tianocore/edk2.git` | 10,294 | Yes (`License.txt`) | BSD-2-Clause-Patent | Clean checkout (C) |
| 8 | `efiSeek` | `573f4b9b5b` | `master` | `https://github.com/DSecurity/efiSeek.git` | 30 | Yes (`LICENSE`) | Apache-2.0 | Clean checkout (Java Ghidra) |
| 9 | `efiXplorer` | `110a8d9086` | `master` | `https://github.com/binarly-io/efiXplorer.git` | 60 | Yes (`LICENSE`) | GPL-3.0 | Clean checkout (C++ IDA) |
| 10 | `EMBA` | `adad91bfae` | `master` | `https://github.com/e-m-b-a/emba.git` | 1,276 | Yes (`LICENSE`) | GPL-3.0 | Clean checkout (Shell/JSON) |
| 11 | `fiano` | `162b021e54` | `main` | `https://github.com/linuxboot/fiano.git` | 313 | Yes (`LICENSE`) | BSD-3-Clause | Clean checkout (Go) |
| 12 | `flashrom` | `35c679c7ec` | `main` | `https://github.com/flashrom/flashrom.git` | 474 | Yes (`COPYING`) | GPL-2.0 | Clean checkout (C) |
| 13 | `fwhunt-scan` | `6fc7115959` | `master` | `https://github.com/binarly-io/fwhunt-scan.git` | 28 | Yes (`LICENSE`) | GPL-3.0 | Clean checkout (Python) |
| 14 | `fwupd` | `868cd5b74b` | `main` | `https://github.com/fwupd/fwupd.git` | 4,970 | Yes (`COPYING`) | LGPL-2.1+ | Clean checkout (C) |
| 15 | `IFRExtractor-RS` | `b1b9b36c00` | `main` | `https://github.com/LongSoft/IFRExtractor-RS.git` | 10 | Yes (`LICENSE.md`) | BSD-2-Clause | Clean checkout (Rust) |
| 16 | `MEAnalyzer` | `d27ccdba0a` | `master` | `https://github.com/platomav/MEAnalyzer.git` | 7 | Yes (`LICENSE`) | BSD-2-Clause | Clean checkout (Python) |
| 17 | `me_cleaner` | `41eba87c85` | `master` | `https://github.com/corna/me_cleaner.git` | 6 | Yes (`COPYING`) | GPL-3.0 | Clean checkout (Python) |
| 18 | `ofrak` | `428ea75712` | `master` | `https://github.com/redballoonsecurity/ofrak.git` | 1,071 | Yes (`LICENSE`) | OFRAK Community License | Clean checkout (Python) |
| 19 | `PSPTool` | `5055403889` | `master` | `https://github.com/PSPReverse/PSPTool.git` | 39 | Yes (`LICENSE`) | GPL-3.0 | Clean checkout (Python) |
| 20 | `UEFI-Editor` | `44e8c9ba4a` | `master` | `https://github.com/BoringBoredom/UEFI-Editor.git` | 50 | No | Unlicensed / Public GitHub | Clean checkout (TypeScript/React) |
| 21 | `uefi-firmware-parser` | `e9c8cb4858` | `main` | `https://github.com/theopolis/uefi-firmware-parser.git` | 82 | Yes (`LICENSE`) | BSD-2-Clause | Clean checkout (Python/C) |
| 22 | `uefisettings` | `3af9ff3c95` | `main` | `https://github.com/linuxboot/uefisettings.git` | 109 | Yes (`LICENSE`) | BSD-3-Clause | Clean checkout (Rust) |
| 23 | `UEFITool` | `de73942df9` | `new_engine` | `https://github.com/LongSoft/UEFITool.git` | 380 | Yes (`LICENSE.md`) | BSD-2-Clause | Clean checkout (C++/C) |
| 24 | `unblob` | `61663627c3` | `main` | `https://github.com/onekey-sec/unblob.git` | 187 | Yes (`LICENSE`) | MIT | Git unstaged collision on Windows (see Section 3) |
| 25 | `Universal-IFR-Extractor` | `8f55681108` | `master` | `https://github.com/donovan6000/Universal-IFR-Extractor.git` | 13 | Yes (`LICENSE.md`) | GPL-3.0 | Clean checkout (C++) |

---

## 3. Platform & Checkout Anomalies

### 3.1 Windows NTFS Case-Sensitivity Collision in `unblob`
The repository `research/third_party/unblob` contains tracked fixture directories with identical names differing only by case sensitivity:
- `tests/files/suffixes/__outputs__/chunks/_carve_extract`
- `tests/files/suffixes/__outputs__/chunks/_carve_extract_dir`

On case-insensitive filesystems (standard Windows NTFS without case-sensitivity directory flags), Git cannot track both variants simultaneously in the working tree without unstaged modifications or checkout collisions. 
- **Assessment:** This anomaly affects only `unblob`'s upstream automated test suite execution on Windows hosts.
- **Remediation Policy:** Do not force-reclone or modify the repository working tree. `unblob` is classified for Linux-containerized adapter integration only.

### 3.2 Unlicensed Repository: `UEFI-Editor`
`UEFI-Editor` (`research/third_party/UEFI-Editor`) lacks an explicit open-source license file in its repository root. Per FriedChip+ Clean-Room and Intellectual Property policy, this repository is classified as `RESEARCH_ONLY` / `REJECT_FOR_PRODUCT` and zero source code or algorithms may be copied into FriedChip+.

---

## 4. Repository Identity Reconciliation (DEF-58)

During independent review, five repository references in research tracking documentation were identified where the primary canonical upstream differed in naming from secondary forks or working mirrors. In accordance with DEF-58, all five repositories were inspected directly in their local Git clones (`git remote get-url origin`, commit SHAs, file authors, and license files). No clones were deleted or overwritten; their exact provenance is documented below:

1. **`biosforge` (GPU-Solution/biosforge vs LongSoft/BIOSForge):**
   - **Local Clone Remote:** `https://github.com/GPU-Solution/biosforge.git`
   - **HEAD Commit SHA:** `535309e40733857e2a9b40f28784fc2aa1e1e442`
   - **Authors / Attribution:** Lucas Efler / Misio (`GPU-Solution`).
   - **Reconciliation:** The local clone is the primary upstream authored by Lucas Efler. `LongSoft/BIOSForge` is a related community fork/mirror. The local clone retains full BSD-2-Clause licensing and 24 tracked Python files.

2. **`bios_extract` (coreboot/bios_extract vs anton-a-k/bios_extract):**
   - **Local Clone Remote:** `https://github.com/coreboot/bios_extract.git`
   - **HEAD Commit SHA:** `0a7bc1d717281c7f66a7b212fefc2a93175c0aa1`
   - **Authors / Attribution:** Nicholas Chin, coreboot project (`coreboot`).
   - **Reconciliation:** The local clone is the official coreboot project repository under the `coreboot` GitHub organization, not the personal fork `anton-a-k/bios_extract`. The local clone retains full GPL-2.0 licensing and 38 tracked C/Python files.

3. **`efiSeek` (DSecurity/efiSeek vs ynadji/efiSeek):**
   - **Local Clone Remote:** `https://github.com/DSecurity/efiSeek.git`
   - **HEAD Commit SHA:** `573f4b9b5b2bb5a7d6eec4e1f727c95e927cb0f6`
   - **Authors / Attribution:** Evgeny Rasskazov, DSec / Digital Security (`DSecurity`).
   - **Reconciliation:** The local clone is the primary authoritative repository authored by Evgeny Rasskazov. `ynadji/efiSeek` is an individual contributor fork. The local clone contains the full Ghidra analyzer suite under Apache-2.0.

4. **`PSPTool` (PSPReverse/PSPTool vs cwerling/ghidra-amdfw):**
   - **Local Clone Remote:** `https://github.com/PSPReverse/PSPTool.git`
   - **HEAD Commit SHA:** `50554038891dff1d3c03ee7ecf7194f1c1a93176`
   - **Authors / Attribution:** Christian Werling (`cwerling@posteo.de`, `PSPReverse`).
   - **Reconciliation:** The local clone is the primary standalone Python library and CLI tool `PSPTool` hosted under `PSPReverse`. `cwerling/ghidra-amdfw` is a separate Ghidra integration repository by the same author. The local clone contains 39 Python files implementing AMD PSP directory parsing under GPL-3.0.

5. **`Universal-IFR-Extractor` (donovan6000/Universal-IFR-Extractor vs LongSoft/Universal-IFR-Extractor):**
   - **Local Clone Remote:** `https://github.com/donovan6000/Universal-IFR-Extractor.git`
   - **HEAD Commit SHA:** `8f55681108c9d1df52bfae6b206775db2ce5d676`
   - **Authors / Attribution:** Donovan Cudney (`donovan6000`).
   - **Reconciliation:** The local clone is the original authoritative C++ implementation authored by Donovan Cudney. `LongSoft/Universal-IFR-Extractor` is LongSoft's subsequent fork. The local clone contains 13 C++ source files under GPL-3.0.

