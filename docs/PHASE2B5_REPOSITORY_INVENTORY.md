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
| 2 | `biosforge` | `535309e407` | `master` | `https://github.com/LongSoft/BIOSForge.git` | 24 | Yes (`LICENSE.md`) | BSD-2-Clause | Clean checkout (Python) |
| 3 | `BIOSUtilities` | `f799425af9` | `main` | `https://github.com/platomav/BIOSUtilities.git` | 38 | Yes (`LICENSE`) | BSD-2-Clause | Clean checkout (Python) |
| 4 | `bios_extract` | `0a7bc1d717` | `main` | `https://github.com/anton-a-k/bios_extract.git` | 38 | Yes (`LICENSE`) | GPL-2.0 | Clean checkout (C/Python) |
| 5 | `chipsec` | `f1935b9bf2` | `chipsec2` | `https://github.com/chipsec/chipsec.git` | 761 | Yes (`COPYING`) | GPL-2.0 | Clean checkout (Python/C) |
| 6 | `coreboot` | `66e704b166` | `main` | `https://review.coreboot.org/coreboot.git` | 24,338 | Yes (`COPYING`) | GPL-2.0 | Clean checkout (C) |
| 7 | `edk2` | `6bee033c69` | `master` | `https://github.com/tianocore/edk2.git` | 10,294 | Yes (`License.txt`) | BSD-2-Clause-Patent | Clean checkout (C) |
| 8 | `efiSeek` | `573f4b9b5b` | `master` | `https://github.com/ynadji/efiSeek.git` | 30 | Yes (`LICENSE`) | Apache-2.0 | Clean checkout (Java Ghidra) |
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
| 19 | `PSPTool` | `5055403889` | `master` | `https://github.com/cwerling/ghidra-amdfw.git` | 39 | Yes (`LICENSE`) | GPL-3.0 | Clean checkout (Python) |
| 20 | `UEFI-Editor` | `44e8c9ba4a` | `master` | `https://github.com/BoringBoredom/UEFI-Editor.git` | 50 | No | Unlicensed / Public GitHub | Clean checkout (TypeScript/React) |
| 21 | `uefi-firmware-parser` | `e9c8cb4858` | `main` | `https://github.com/theopolis/uefi-firmware-parser.git` | 82 | Yes (`LICENSE`) | BSD-2-Clause | Clean checkout (Python/C) |
| 22 | `uefisettings` | `3af9ff3c95` | `main` | `https://github.com/linuxboot/uefisettings.git` | 109 | Yes (`LICENSE`) | BSD-3-Clause | Clean checkout (Rust) |
| 23 | `UEFITool` | `de73942df9` | `new_engine` | `https://github.com/LongSoft/UEFITool.git` | 380 | Yes (`LICENSE.md`) | BSD-2-Clause | Clean checkout (C++/C) |
| 24 | `unblob` | `61663627c3` | `main` | `https://github.com/onekey-sec/unblob.git` | 187 | Yes (`LICENSE`) | MIT | Git unstaged collision on Windows (see Section 3) |
| 25 | `Universal-IFR-Extractor` | `8f55681108` | `master` | `https://github.com/LongSoft/Universal-IFR-Extractor.git` | 13 | Yes (`LICENSE.md`) | GPL-3.0 | Clean checkout (C++) |

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
