# FriedChip+ — Validation Model & Artifact Architecture Gaps

**Project:** FriedChip+ Clean-Room Firmware Engineering Workstation  
**Creator / Product Credit:** Sufiyan Khan  
**Classification:** Architecture Specification & Validation Gap Analysis  
**Milestone:** Phase 2 Implementation Audit & Scope Freeze  
**Audit Date:** November 2026  
**Auditor:** Antigravity (Advanced Agentic Coding)  

---

## 1. Executive Summary & Core Principle

In firmware engineering, confusing container unpackability with physical flash safety is a catastrophic hazard. Technicians frequently assume that if an extractor successfully outputs a `.bin` file named `System BIOS`, that file is complete, bootable, and safe to flash directly to an SPI chip using an external programmer. In reality, OEM update partitions almost always lack the Flash Descriptor, Intel ME, GbE region, and board-specific padding required for hardware initialization.

> **Foundational Safety Principle:**  
> **Successful parsing or extraction does NEVER imply bootability or flash safety.**  
> Every parsing and extraction operation in FriedChip+ must strictly decouple structural integrity from hardware flashability through a formalized **Three-Gate Validation Model**.

---

## 2. The Formal Three-Gate Validation Model

```
                       Three-Gate Validation Pipeline
┌─────────────────────────────────────────────────────────────────────────┐
│                       GATE 1: STRUCTURAL VALIDITY                       │
│  - Magic signatures present and valid?                                  │
│  - Internal tables, headers, and payload bounds within file limits?     │
│  - Checksums (CRC-32, XOR-8, FV-16) mathematically verify?             │
│  - No overlapping partitions or negative size overflows?               │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ Passed
┌────────────────────────────────────▼────────────────────────────────────┐
│                    GATE 2: PLATFORM COMPATIBILITY                       │
│  - Compatible with target PCH chipset family & CPU generation?          │
│  - Intel ME / CSME major version aligned with PCH silicon stepping?     │
│  - Embedded Controller (EC) architecture matches physical SIO chip?     │
│  - Board ID / Model ID matches system hardware profile?                 │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ Passed
┌────────────────────────────────────▼────────────────────────────────────┐
│                   GATE 3: PROGRAMMER COMPATIBILITY                      │
│  - Exact byte-for-byte capacity match for target SPI flash chip?        │
│  - Complete image with Descriptor, ME, BIOS, and GbE present?           │
│  - Host flashing permissions (FRAP/PR) vs External SPI programmer?      │
│  - Read-back verification and pinout voltage constraints satisfied?    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Gate 1: Structural Validity
- **Question Answered:** *"Is this file internally uncorrupted and syntactically well-formed according to its format specification?"*
- **Criteria:** Correct magic signatures, valid header and footer checksums, non-overlapping partition boundaries, absence of truncation, and intact compression streams.
- **Verdict:** If Gate 1 fails, processing halts immediately. No downstream operations may consume structurally malformed data.

### Gate 2: Platform Compatibility
- **Question Answered:** *"Is this firmware structurally compatible with the targeted motherboard silicon and controller hardware?"*
- **Criteria:** CPU generation match, Intel PCH chipset compatibility (e.g. 100/200/300 Series), ME major version alignment, Embedded Controller architecture (ITE vs ENE vs Nuvoton vs Microchip), and OEM Board-ID verification.
- **Verdict:** A failure indicates that even if flashed cleanly, the system will experience boot-hangs, boot-loops, or 30-minute thermal shutdowns.

### Gate 3: Programmer Compatibility
- **Question Answered:** *"Can this exact binary file be written to a physical SPI NOR flash chip using an external programmer (e.g. CH341A / Dediprog) or in-system flasher without bricking the device?"*
- **Criteria:**
  1. Exact byte capacity match for commercial SPI flash chips (e.g. exactly 8,388,608 bytes for 8MB / 64Mb chips, 16,777,216 bytes for 16MB / 128Mb chips).
  2. Complete image presence: Must contain Region 0 (Flash Descriptor) at offset `0x0000`, valid FLREG region bounds, and correctly positioned boot vector at `0xFFFFFFF0`.
  3. Distinction between Host-Based Flashing and External SPI Flashing:
     - *Host Flashing:* Constrained by Intel Flash Descriptor Master Access Permissions (FRAP) and Protected Range registers (PR0..PR4).
     - *External SPI Programming:* Bypasses host permissions entirely, but requires proper handling of hardware Write Protect (`/WP`) pins and target voltage selection (1.8V vs 3.3V).
- **Verdict:** If Gate 3 fails, the UI must explicitly disallow flashing and label the artifact:  
  **`STATUS: UNVERIFIED SPI LAYOUT — NOT SAFE TO FLASH`**

---

## 3. Universal Firmware Artifact Model Specification

To prevent vendor lock-in and eliminate ad-hoc structs across parsers, the engine requires a unified `FirmwareArtifact` representation. Format-specific metadata must be extensible without contaminating the core domain model with vendor-specific fields.

```rust
pub struct FirmwareArtifact {
    /// Unique identifier for this artifact within the workspace.
    pub id: String,
    /// Reference to parent artifact ID (None for root imported files).
    pub parent_id: Option<String>,
    /// Human-readable component name (e.g. "System BIOS", "Intel ME Region").
    pub name: String,
    /// Detected firmware format category.
    pub format: ArtifactFormat,
    /// Byte offset within parent container where this component begins.
    pub source_offset: usize,
    /// Extracted payload size in bytes.
    pub raw_size: usize,
    /// Cryptographic hashes for immutability and provenance tracking.
    pub sha256: String,
    pub md5: Option<String>,
    /// Path on disk if materialized via verified atomic write.
    pub file_path: Option<String>,
    /// Three-Gate Validation assessments.
    pub gate1_structural: Gate1Result,
    pub gate2_platform: Gate2Result,
    pub gate3_programmer: Gate3Result,
    /// Human-readable warnings, cautions, and operational issues.
    pub warnings: Vec<String>,
    /// Extensible format-specific metadata dictionary.
    pub vendor_metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Timestamp of extraction or generation (ISO 8601).
    pub timestamp: String,
}
```

---

## 4. Current Implementation Gaps & Discrepancies

Comparing the current codebase against the target validation and artifact model reveals the following concrete architectural gaps:

| Component | Target Requirement | Current Implementation State | Severity |
| :--- | :--- | :--- | :---: |
| **Artifact Model** | Shared `FirmwareArtifact` struct with provenance and parent-child tracking. | Disparate, ad-hoc structs: `DellPfsExtractedComponent` in `dell_pfs.rs` vs `FlashRegion` in `region.rs`. | **High** |
| **PFS Flashing Readiness** | Extracted BIOS partitions must be explicitly evaluated for Gate 3 Programmer Compatibility. | Assigns a textual string `Dell Extracted BIOS Payload (Unverified SPI Layout)`, but lacks structured Gate 3 boolean checks. | **Medium** |
| **IFD FRAP Permissions** | Read and decode Master Access Permissions (FMBA / FRAP register). | Hardcodes `host_flashing_locked: true` as a static assumption rather than evaluating read/write bitmasks. | **Medium** |
| **Provenance Tracking** | Artifacts must track parent container file ID, source byte offset, and extraction depth. | `DellPfsExtractedComponent` tracks section name and depth, but does not record the parent database file UUID or container offset. | **Medium** |
| **Metadata Extensibility** | Shared extensible JSON map for format-specific details (e.g. PFS GUIDs, IFD FLREG values). | Format details are hardcoded into specific struct fields, preventing unified UI rendering in Workspace tables. | **Low** |

---

## 5. Remediation Roadmap for Subsequent Sprints

1. **Sprint 1 (Post-Audit):** Refactor `src-tauri/src/models/` to introduce `FirmwareArtifact` and `ValidationGateResults`.
2. **Sprint 2:** Update `ifd.rs` and `dell_pfs.rs` to output unified `FirmwareArtifact` instances.
3. **Sprint 3:** Surface Gate 1, Gate 2, and Gate 3 badges directly in the React `WorkspaceView` and `ExtractorsView`.
