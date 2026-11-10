# Clean-Room Provenance, Safety Boundaries & Data Protection

## 1. Clean-Room Provenance & Development Boundary

This project is an **independent, clean-room re-engineering** of laptop firmware servicing and extraction workflows.

### Strict Development Rules:
- **No Proprietary Code or Disassembly:** No binary decompilations, disassemblies, or reverse-engineered source trees of LBE++ or related OEM utilities may be consulted, stored, or reviewed during development.
- **Specification-Driven Implementation:** Architecture and algorithms are derived from open hardware specifications (Intel Platform Innovation Framework for UEFI, Intel SPI programming guides, UEFI PI 1.x/2.x specifications, PCI/ACPI standards) and public format documentation.
- **Independent Asset Creation:** No proprietary trademarks, UI artwork, icons, or strings are copied. All UI designs and codebases are entirely novel.
- **Separation of Evidence and Implementation:** A feature is never marked `IMPLEMENTED` or `TESTED` until written in this repository and passed by an automated test suite.
- **Proprietary Binaries Exclusion:** Proprietary OEM update packages and hardware firmware dumps are strictly excluded from version control (`.gitignore`). Only reproducible metadata, download sources, and cryptographic hash manifests are committed.

---

## 2. Ethical Boundaries & Permitted Alternatives

Firmware servicing tools must distinguish legitimate hardware repair from unauthorized security bypasses:

### Supported Authorized Servicing:
- **Corrupted SPI Flash Recovery:** Restoring unbootable laptops resulting from failed BIOS updates or defective flash memory ICs.
- **Intel CSME / TXE Health Restoration:** Cleaning corrupted Intel ME regions (fixing 30-minute thermal/watchdog shutdowns, erratic fan speed, missing NVMe/SATA detection) using clean ME repository regions after strict compatibility verification.
- **Factory Update Extraction:** Unpacking manufacturer update packages (`.exe`, `.hdr`, `.fd`, `.cap`, `.FL1`) to obtain raw SPI and EC images for hardware chip programmers.
- **Embedded Controller (EC) Extraction:** Architecture-specific profile validation for ITE, ENE, Nuvoton, and Microchip microcontrollers; outputs `UNCLASSIFIED` on boundary ambiguity.
- **Serialization & DMI Preservation:** Transferring legitimate machine serial numbers, UUIDs, and Windows OEM license keys (MSDM) from damaged donor dumps to clean replacement dumps during authorized motherboard or flash IC swaps using manufacturer-specific storage profiles.
- **Firmware Verification & Auditing:** Comprehensive three-tier validation (structural, platform compatibility, and programmer compatibility) to evaluate hardware constraints.

### Prohibited Actions & Permitted Alternatives:

| Prohibited Capability | Reason for Exclusion | Permitted Legitimate Alternative |
|---|---|---|
| **Apple iCloud / FMM Unlock** | Defeats platform activation locks and stolen-device safeguards. | Non-destructive NVRAM inspection and FSC variable store integrity diagnostics. |
| **Enterprise MDM / DEP Bypass** | Defeats corporate device enrollment and ownership controls. | Enrollment status auditing; standard authorized BIOS re-flashing. |
| **Computrace Lock Removal** | Defeats enterprise tracking and anti-theft persistence agents. | Identification and reporting of persistence agent status within UEFI volume map. |
| **Brute-Force Credential Cracking** | Bypasses authentication controls without authorization. | Support authorized factory master clear procedures where documented by OEM. |

---

## 3. Data Protection, Customer Privacy & Sanitization

1. **Non-Destructive Invariant:**
   - The application **never** modifies or overwrites the original input firmware file in place.
   - All input files are opened with strict read-only permissions (`std::fs::OpenOptions::new().read(true)`).
   - All output images are written to discrete, version-stamped files in designated project scratch spaces.
2. **Customer Data Sanitization Policy:**
   - Never request or commit customer-sensitive firmware dumps containing personal user credentials, enterprise cryptographic keys, or corporate asset tags.
   - Initial test suites must rely on synthetic test fixtures and publicly downloadable OEM update packages.
   - Any hardware donor dumps submitted for edge-case diagnostics must be sanitized and authorized.
3. **Primary Integrity Hashing:**
   - **SHA-256** is the primary cryptographic integrity check across the application.
   - MD5 is computed solely as a secondary hash for backwards compatibility with legacy bench databases.
4. **Three-Tier Validation Architecture:**
   - No image is designated "Programmer-Compatible" without satisfying:
     1. Structural Validation (headers, magic bytes, volume lengths, CRCs).
     2. Platform Compatibility Validation (ME generation, SKU, chipset, DMI structure).
     3. Programmer Compatibility Assessment (exact power-of-2 size matching flash IC capacity, voltage constraints, external SPI vs host-based flasher rules).
