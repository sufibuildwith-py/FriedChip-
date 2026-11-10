# Laptop Firmware Engineering Workstation (LBE++ Clean-Room Alternative)

An independently engineered, professional Windows desktop workstation for laptop firmware analysis, extraction, repair, and integrity verification.

---

## 1. Project Overview

This project provides an offline-first, professional desktop alternative to legacy BIOS servicing tools (such as LBE++ by EgyfixLab). It is designed specifically for hardware repair technicians, board-level repair specialists, and firmware engineers.

### Core Capabilities:
- **BIOS Extraction & Unpacking:** Unpack vendor update packages (Dell PFS/HDR, HP Insyde, Lenovo FL1/FL2, Asus CAP) into programmer-ready SPI binaries.
- **Intel CSME / TXE Health & Clean ME:** Manifest inspection, health diagnostics, and automated clean ME injection.
- **Embedded Controller (EC) Extraction:** Locate and slice standalone EC binaries for ITE, ENE, Nuvoton, and SMSC microcontrollers.
- **Serialization & DMI Preservation:** Transfer original Service Tags, Serial Numbers, UUIDs, and Windows OEM product keys across images without damaging SMBIOS integrity.
- **Firmware Inspection Utilities:** Hardware-bench Virtual Hex Viewer, byte-level diff visualizer, entropy analysis, and cryptographic hashing.
- **Traceability & Audit:** Append-only SQLite operation journal with pre- and post-operation validation.

---

## 2. Architecture & Tech Stack

| Component | Technology | Rationale |
|---|---|---|
| **Desktop Shell** | Tauri 2 | Native Windows integration, secure sandbox, minimal memory footprint, Rust backend |
| **Frontend** | React 19 + TypeScript + Vite | High-density benchtop UI, reactive state, virtualized hex viewing |
| **Firmware Engine** | Pure Rust | Memory-safe binary slicing, high performance, deterministic cryptographic hashing |
| **Local Database** | SQLite | Offline-first project management, audit journals, reference database |
| **Binary Storage** | Managed Filesystem | Prevents SQLite BLOB bloat; enables direct programmer read/write |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full architectural specifications.

---

## 3. Clean-Room & Safety Boundaries

- **Strict Clean-Room Protocol:** Zero proprietary source code, decompiled pseudocode, or copyrighted assets. All algorithms are implemented from open firmware specifications and observable file behaviors.
- **Authorized Servicing Only:** Built for legitimate firmware repair, corrupted ME recovery, and hardware servicing. Anti-theft bypass, MDM circumvention, and credential cracking are strictly out of scope.
- **Non-Destructive Invariant:** Original firmware inputs are never overwritten in place.

See [docs/CLEAN_ROOM_SAFETY.md](docs/CLEAN_ROOM_SAFETY.md) for complete compliance guidelines.

---

## 4. Current Status: Phase 1 Complete (Desktop Application Foundation)

Phase 1 desktop workstation foundation is **complete, built, and verified**.

### Deliverables:
- [x] Initialized Git repository and clean ignore configuration.
- [x] Defined System Architecture & Data Model ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)).
- [x] Established Clean-Room and Safety Protocol ([docs/CLEAN_ROOM_SAFETY.md](docs/CLEAN_ROOM_SAFETY.md)).
- [x] Compiled Feature-Parity & Discovery Matrix ([docs/PHASE0_DISCOVERY_MATRIX.md](docs/PHASE0_DISCOVERY_MATRIX.md)).
- [x] Defined Feature-Tier Matrix & Module Capability Breakdown ([docs/FEATURE_TIER_MATRIX.md](docs/FEATURE_TIER_MATRIX.md)).
- [x] Detailed Multi-Phase Roadmap ([docs/ROADMAP.md](docs/ROADMAP.md)).
- [x] Firmware Sample & Evidence Submission Guide ([docs/SAMPLE_SUBMISSION_GUIDE.md](docs/SAMPLE_SUBMISSION_GUIDE.md)).
- [x] Tauri 2 + React 19 + TypeScript + Vite Desktop Application Shell.
- [x] SQLite persistence engine with versioned schema migrations (`workstation.db` with WAL mode).
- [x] Safe read-only firmware file import workflow with streaming SHA-256 and signature analysis.
- [x] Structured operation history and execution logging.
- [x] High-resolution desktop screenshots ([docs/screenshots/](docs/screenshots/)).
- [x] 8/8 Automated backend unit tests passing with 0 failures.

### Candidate Reference Devices (Phase 6 Public Extraction Baseline):
- **Dell Latitude 5490** (PFS/HDR container)
- **HP EliteBook 840 G5** (Insyde encrypted payload)
- **Lenovo ThinkPad T480** (InnoSetup / FL1 / FL2 payload)

### Build & Test Commands:
```powershell
# Run frontend build
npm run build

# Run backend unit test suite
cd src-tauri; cargo test

# Build release desktop executable
npm run tauri build -- --no-bundle
```
3. **Phase 1 Kickoff:** Initialize the Tauri 2 + React 19 skeleton and SQLite metadata journal.
