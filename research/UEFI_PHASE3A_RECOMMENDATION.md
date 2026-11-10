# Phase 3A · UEFI Analysis Implementation Roadmap & Recommendation

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** Implementation Milestones, Dependency Sequencing & Risk Management  
**Milestone:** Phase 3A (Upstream Component Intelligence & Architecture Design)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE STRATEGIC RECOMMENDATION  
**Scope Restriction:** Recommendation Only. Zero code changes in application source tree.

---

## 1. Executive Research Synthesis

Phase 3A has conducted an in-depth source code audit of 14 upstream firmware repositories (`UEFITool`, `fiano`, `uefi-firmware-parser`, `edk2`, `IFRExtractor-RS`, `BIOSUtilities`, `binwalk`, `unblob`, `ofrak`, `Universal-IFR-Extractor`, `efiSeek`, `efiXplorer`, `fwhunt-scan`, `UEFI-Editor`).

### Key Strategic Findings:
1. **Reuse vs. Reinvention:** Reimplementing low-level FFS or IFR parsers from scratch without referencing proven upstream code is a recipe for silent failure. `LongSoft/UEFITool` and `LongSoft/IFRExtractor-RS` contain over a decade of real-world hardware quirk handling. We adopt their algorithms and AST structures directly into a clean-room Rust architecture.
2. **Licensing Protection:** We strictly reject GPL-2.0/GPL-3.0 components (`Universal-IFR-Extractor`, `BIOSUtilities`, `fwhunt-scan`) from production builds. Permissive BSD-2-Clause (`UEFITool`, `IFRExtractor-RS`), BSD-3-Clause (`fiano`), and MIT (`uefi-firmware-parser`) projects provide 100% of the needed functionality without legal risk.
3. **Memory Safety & Windows Simplicity:** Direct cgo/FFI linking of Go or C++ engines into Tauri is rejected due to Windows SEH conflicts and memory corruption risks. The core pipeline must be native Rust, complemented by an isolated `UEFIExtract` CLI subprocess adapter for bulk decomposition and differential testing.
4. **Permanent IFD Freeze:** The Intel Flash Descriptor subsystem remains permanently frozen. The UEFI analysis pipeline sits on top of the IFD BIOS region output without modifying any descriptor decoding logic.

---

## 2. Phased Implementation Roadmap (Phase 3B)

To maintain disciplined engineering velocity and Karpathy simplicity guidelines, Phase 3B is broken down into five focused, verifiable milestones:

```text
Phase 3B.1: Firmware Volume Traversal & Invariant Validation
  │   └── Header validation, 16-bit word checksum, block maps, alignment, FV discovery
  ▼
Phase 3B.2: Firmware File System (FFS) Traversal
  │   └── FFSv2 / FFSv3 extended sizes, file states, GUID resolution, alignment padding
  ▼
Phase 3B.3: Section Traversal & Decompression Engine
  │   └── Standard & extended sections, Tiano/EFI decompression, LZMA integration, bomb limits
  ▼
Phase 3B.4: PE32 / TE Executable Inspection & IFR Setup Analysis
  │   └── Non-executing PE/TE metadata, IFR opcode disassembly, VarStore offset mapping
  ▼
Phase 3B.5: Normalized Object Model & Workstation UI Integration
      └── Complete tree explorer in React 19, diagnostic warnings, atomic file extraction
```

---

## 3. First Implementation Assignment Proposal: Phase 3B.1

### Title: Phase 3B.1 — Firmware Volume Traversal & Invariant Validation
**Assignment Type:** Surgical Implementation & Unit Test Verification  
**Subsystem:** `src-tauri/src/engine/uefi/firmware_volume.rs`

### Objective:
Implement a robust, clean-room native Rust Firmware Volume (FV) parser adhering to UEFI PI Specification Volume 3. It will scan the BIOS region identified by the IFD parser (or standalone binary dumps) and discover all valid Firmware Volumes.

### Specific Scope of Phase 3B.1:
1. **Parse `EFI_FIRMWARE_VOLUME_HEADER`:**
   - Signature validation: `_FVH` (`0x4856465F`) at offset `+0x28`.
   - FileSystemGuid identification (FFSv2: `8C8CE578-...`, FFSv3: `54737666-...`).
   - Declared `FvLength` and `HeaderLength`.
   - 16-bit inverted word checksum verification across `HeaderLength`.
2. **Block Map Interpretation:**
   - Parse `{ NumBlocks: u32, BlockLength: u32 }` until the `{ 0, 0 }` sentinel.
   - Assert invariant: $\sum (\text{NumBlocks} \times \text{BlockLength}) == \text{FvLength}$.
   - Detect and reject missing sentinel or block map overrun.
3. **Autonomous Volume Discovery:**
   - Scan container on 4KB / 64KB alignment boundaries for valid `_FVH` signatures.
   - Enforce physical container boundary limits.
4. **Negative Test Suite & Synthetic Fixtures:**
   - Add deterministic synthetic fixtures in `fixture_builder.rs` (valid FV, corrupt checksum, missing sentinel, truncated header).
   - Add unit tests verifying exact error propagation.

### Explicitly Excluded from Phase 3B.1 (Deferred to Later Milestones):
- **NO** FFS file parsing or file payload extraction (Phase 3B.2).
- **NO** Section parsing or decompression (Phase 3B.3).
- **NO** PE32/TE or IFR analysis (Phase 3B.4).
- **NO** GUI modifications or Tauri IPC updates (Phase 3B.5).
- **NO** Subprocess execution or external executable calls.
- **NO** SPI flashing, hardware drivers, or live flashing code.

---

## 4. Open Risks & Unresolved Technical Questions

1. **Non-Standard Vendor FV Formats:**
   - *Risk:* Apple and older Phoenix/Insyde platforms occasionally format headers with non-standard alignment or omit the block map sentinel.
   - *Mitigation:* The parser must support a relaxed diagnostic mode that flags a warning rather than panicking, falling back to declared `FvLength`.
2. **FFSv3 64-bit Extended Length in Wild Images:**
   - *Risk:* FFSv3 images with 64-bit lengths are rare in client PCs but common in multi-socket enterprise server firmware.
   - *Mitigation:* Unit tests must validate arithmetic boundary checks using synthetic 64-bit headers.
3. **Subprocess Companion Packaging on Windows:**
   - *Risk:* Bundling `UEFIExtract.exe` requires cross-compilation or pre-built static binaries for Windows x64.
   - *Mitigation:* The native Rust engine is the primary path. The companion binary is purely an optional enhancement.

---

## 5. Strict Project Boundaries

- **Intel Flash Descriptor (IFD) Permanently Frozen:** Zero modifications to `src-tauri/src/engine/binary/ifd.rs`.
- **Phase 3 Hardware Flashing Strictly Locked:** No hardware SPI programmer code, no FTDI/CH341 drivers, no live kernel drivers, and no PFAT decryption.

**Phase 3A is complete. We recommend proceeding to Phase 3B.1 upon authorization.**

---
EOF
