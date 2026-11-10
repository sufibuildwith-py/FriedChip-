# Phase 3A.1 · UEFI Analysis Implementation Roadmap & Recommendation (Corrected)

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Domain:** Implementation Milestones, Upstream Integration Sequencing & Risk Governance  
**Milestone:** Phase 3A.1 (Architecture Correction & Reuse-First Review)  
**Author / Engineering Lead:** Sufiyan Khan  
**Status:** COMPLETE STRATEGIC RECOMMENDATION  
**Scope Restriction:** Recommendation Only. Zero code changes in application source tree.

---

## 1. Executive Strategy: Proven Upstream Integration

The core directive of FriedChip+ is:

**Integrate proven components; do not recreate an entire firmware-analysis ecosystem.**

Instead of spending months implementing a native Rust parser that would inevitably struggle with edge cases and vendor quirks, FriedChip+ adopts a **Multi-Engine Upstream Integration Strategy**:
- `LongSoft/UEFITool` (`UEFIExtract` CLI) is our **primary extraction and parsing engine**. It embodies over a decade of production battle-testing across thousands of hardware platforms.
- `linuxboot/fiano` (`utk table --json`) is our **independent structured validation engine**.
- `LongSoft/IFRExtractor-RS` is our **primary IFR setup disassembler**.
- FriedChip+ focuses entirely on its core product value: hardened process orchestration, Windows Job Object security containment, unified NFOM normalization, cryptographic provenance, and rich UI exploration.

---

## 2. Revised Implementation Roadmap (Phase 3B)

```text
Phase 3B.1: Upstream Subprocess Orchestration & Initial NFOM Ingestion
  │   └── Job Object containment, UEFIExtract CLI adapter, report/dump normalization into NFOM
  ▼
Phase 3B.2: Differential Cross-Engine Validation Pipeline
  │   └── fiano utk integration, live differential comparator, volume count & GUID parity
  ▼
Phase 3B.3: Bulk Section Extraction & Decompression Artifact Management
  │   └── Atomic disk extraction, path traversal sanitization, lazy SHA-256 artifact catalog
  ▼
Phase 3B.4: IFR Setup Disassembly & Read-Only Variable Store Mapping
  │   └── IFRExtractor-RS adapter, setup question AST, relative VarOffset display (read-only)
  ▼
Phase 3B.5: Workstation UI Virtualization & Diagnostic Visualization
      └── React 19 virtualized tree explorer, structural anomaly badges, safe payload export
```

---

## 3. Recommended First Implementation Milestone: Phase 3B.1

### Title: Phase 3B.1 — Upstream Subprocess Orchestration & Initial NFOM Ingestion
**Assignment Type:** Upstream Component Integration & Normalization  
**Scope:** `src-tauri/src/engine/uefi/` (subprocess adapter, Job Object runner, and NFOM mapper).

### Specific Scope of Phase 3B.1:
1. **Upstream Engine Selection:**
   - Integrate `LongSoft/UEFITool` (`UEFIExtract` CLI, commit `de73942df908b9466c055e15108ac4e4c0b1688c`) as the primary extraction worker.
2. **Real Read-Only Operation Exercised:**
   - Invoke `UEFIExtract <image_path> report` or `dump` in a dedicated, isolated workspace directory.
   - Exercise genuine parsing of top-level and nested Firmware Volumes and FFS files without modifying source inputs.
3. **Normalized Ingestion (NFOM):**
   - Parse `UEFIExtract` report and output directory structure into the `FirmwareNode` (NFOM) hierarchy using fixed-width `u64` offsets and sizes.
   - Separate declared size, actual payload size, and alignment padding.
   - Tag nodes with `BackendEngine` provenance (`name: "UEFIExtract"`, `commit_sha: "de73942d"`).
4. **Differential Validation Setup:**
   - Execute `linuxboot/fiano` (`utk table --json`, commit `162b021e54496b89b2f9028f88234b2caec70a0c`) on the same image.
   - Verify that discovered Firmware Volume counts and byte offsets match identically between `UEFIExtract` and `utk`.
5. **Mandatory Security Controls Before Processing Untrusted Firmware:**
   - Enforce Windows Job Object limits on the worker process:
     * Memory ceiling: 512 MB (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`).
     * CPU execution timeout: 30 seconds (`TerminateJobObject` on deadline).
     * Automatic child termination on host close (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
   - Output path sanitization: Reject any extracted path attempting directory traversal (`../`).
6. **Integration Verification Tests:**
   - Execute differential validation tests on EDK2 OVMF and synthetic FV golden vectors.

### Strictly Excluded from Phase 3B.1:
- **NO** In-process C++ or Go FFI linking.
- **NO** Writing a clean-room native Rust FV/FFS parser.
- **NO** IFR setup form disassembly (Phase 3B.4).
- **NO** NVRAM variable modification or write-back logic.
- **NO** UI modifications in React or Tauri IPC commands (Phase 3B.5).
- **NO** Modification of the permanently frozen IFD parser.
- **NO** Physical SPI hardware flashing or kernel driver access.

---

## 4. Open Technical Risks & Mitigations

1. **Companion Binary Packaging on Windows:**
   - *Risk:* Deploying `UEFIExtract.exe` requires reliable distribution of a statically compiled Windows x64 binary.
   - *Mitigation:* `UEFIExtract` builds with standard MSVC `/MT` producing a 100% self-contained binary (~1.5 MB) with zero external DLL requirements.
2. **Windows Security Hardening Complexity:**
   - *Risk:* Windows Job Objects require direct Win32 API calls (`winapi` / `windows-sys` crate).
   - *Mitigation:* Encapsulate Job Object logic into a clean, dedicated runner module (`subprocess::job_runner`).
3. **Discrepancies in Upstream Volume Offset Reporting:**
   - *Risk:* Different upstream tools may report volume offsets relative to the BIOS region vs relative to the container root.
   - *Mitigation:* The NFOM normalization layer explicitly stores both `container_offset: u64` and `relative_offset: u64`.

---

## 5. Strict Project Boundaries

- **Intel Flash Descriptor (IFD) Permanently Frozen:** Zero modifications to `src-tauri/src/engine/binary/ifd.rs`.
- **Phase 3 Hardware Flashing Strictly Locked:** Physical SPI programmer hardware, FTDI drivers, and live flashing code remain strictly locked.
- **Documentation Integrity:** All upstream projects receive full BSD/MIT attribution in compliance with repository licenses.

---
EOF