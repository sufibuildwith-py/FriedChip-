# Phase 2B.5 — External Adapter Integration Architecture

**Project:** FriedChip+  
**Release:** Phase 2B.5 / Phase 2B.5.2  
**Scope:** Formal Sub-Process Adapter Contract, Communication Protocol, Isolation Architecture, and Licensing Boundaries for External Tools.

> [!IMPORTANT]
> **DESIGN SPECIFICATION ONLY — NOT YET IMPLEMENTED IN CODEBASE**  
> This document defines the target architectural contract and protocol specification for out-of-process tool execution in Phase 3. In strict compliance with the Phase 2 scope freeze and Phase 3 lock, **no subprocess runner, external adapter process, or child sandbox infrastructure has been implemented in the active codebase.**

---

## 1. Status & Implementation Boundaries (DEF-61, DEF-64)

To prevent ambiguity between architectural designs, working code, and legal assessments, FriedChip+ maintains an explicit status ledger using standardized status labels:
- `DESIGNED`: Architectural contract, schema, or specification defined in documentation.
- `IMPLEMENTED`: Working Rust code compiled into the application codebase.
- `TESTED`: Automated test suite exercising the feature with tangible evidence.
- `NOT_IMPLEMENTED`: Capability planned for future releases; absent from current codebase.
- `LEGAL_REVIEW_REQUIRED`: Intellectual property, copyleft boundary, or licensing question requiring independent legal review before productization.

### Status Matrix

| Architecture Layer | Current Status | Notes |
|--------------------|----------------|-------|
| **Adapter Protocol Schemas** | **DESIGNED** | Schemas, stdin/stdout protocol, and error formats documented herein. |
| **Subprocess Process Runner** | **NOT_IMPLEMENTED** | `src-tauri` contains zero `std::process::Command` adapter wrappers or process managers. |
| **Fault Boundary Separation** | **DESIGNED** | Process crash isolation is an intended architectural outcome, not currently executing. |
| **OS-Enforced Sandboxing** | **NOT_IMPLEMENTED** | Job Objects, AppContainer, and DACL restrictions are future requirements, not present code. |
| **Tested Process Isolation** | **NOT_IMPLEMENTED** | No sandboxing integration test suites exist in the current test tree. |
| **Real-World Adapter Execution** | **NOT_IMPLEMENTED** | Phase 3 is locked; external binaries are not executed by the application. |
| **GPL Process-Boundary Compliance** | **LEGAL_REVIEW_REQUIRED** | IPC separation does not automatically resolve copyleft obligations; legal review required. |

---

## 2. Principles of Architecture

FriedChip+ distinguishes strictly between:
1. **Core Native Engine:** Written in pure, memory-safe Rust inside `src-tauri/src/engine/`. Responsible for primary parsing, structural validation gates, byte-level integrity checks, database storage, and Tauri desktop UI events.
2. **External Adapters:** Out-of-tree worker executables written in other languages (Python, Go, C++) invoked strictly as isolated worker processes.

### Core Architecture Mandates
- **No In-Process Dynamic Plugin Marketplaces:** Third-party binary code or untrusted scripts are NEVER loaded dynamically into the main Tauri process address space (no `libloading`, no unsafe FFI shared libraries).
- **Subprocess Isolation:** Every external tool runs in an isolated OS process with restricted privileges.
- **Read-Only Input Guarantee:** The native engine only passes read-only file descriptors or immutable temp files to the adapter. Adapters cannot modify source firmware images.
- **Structured JSON Protocol:** Communication occurs exclusively over standard streams (`stdin` / `stdout`) using versioned JSON schemas.
- **Fail-Safe Process Supervisions:** Deterministic timeouts, resource limits, and strict exit code validation ensure crashes in an adapter cannot crash FriedChip+.

---

## 3. Adapter Protocol Specification

### 3.1 Standard Process Invocations
```text
+-----------------------------+                   +-------------------------------+
|                             |  stdin (JSON cmd) |                               |
|   FriedChip+ Native Engine  | ----------------> |  External Adapter Sub-Process |
|       (Rust / Tauri)        | <---------------- |     (Python / Go / Binary)    |
|                             |  stdout (JSON res)|                               |
+-----------------------------+                   +-------------------------------+
               |                                                  |
               +--- stderr redirected to debug logger ------------+
```

### 3.2 Input Command Schema (`stdin`)
The engine sends a single UTF-8 encoded JSON object over `stdin` followed by EOF (or newline):
```json
{
  "protocol_version": "1.0",
  "request_id": "req-a7f9-4b21-8201",
  "adapter_id": "me_analyzer",
  "operation": "analyze_fpt",
  "source_file_path": "/var/tmp/friedchip/input_immutable.bin",
  "expected_sha256": "3B8E...F41A",
  "timeout_seconds": 30,
  "options": {
    "extract_partitions": false,
    "detailed_hashes": true
  }
}
```

### 3.3 Output Result Schema (`stdout`)
The adapter must write a single UTF-8 encoded JSON response to `stdout`:
```json
{
  "protocol_version": "1.0",
  "request_id": "req-a7f9-4b21-8201",
  "adapter_id": "me_analyzer",
  "status": "SUCCESS",
  "exit_code": 0,
  "elapsed_ms": 248,
  "data": {
    "engine_type": "ME",
    "major_version": 11,
    "minor_version": 8,
    "hotfix_version": 92,
    "build_version": 4222,
    "partitions": [
      {
        "name": "FTPR",
        "offset": 4096,
        "size": 1048576,
        "sha256": "A1B2...C3D4"
      }
    ]
  },
  "errors": [],
  "warnings": []
}
```

### 3.4 Error Handling & Exit Codes
- **Exit Code 0:** Successful execution; `status` is `"SUCCESS"` and `data` contains parsed payload.
- **Exit Code 1:** Unrecoverable parse failure; `status` is `"PARSE_ERROR"` and `errors` contains diagnostic strings.
- **Exit Code 2:** Schema / Protocol invalid; invalid arguments or malformed request.
- **Timeout (SIGKILL / TerminateProcess):** Native engine enforces hard wall-clock timeout (default 30 seconds). The worker is killed, and the engine records `AdapterError::Timeout`.

---

## 4. Sandboxing & Operational Boundaries (DEF-61, DEF-64)

### 4.1 Fault Separation vs OS-Enforced Sandboxing
It is vital to distinguish between a **fault boundary** and a **security sandbox**:
- **Fault Boundary (Process Separation):** Spawning an external tool in a separate process isolates memory address spaces. If the worker encounters an unhandled panic, segmentation fault, or out-of-memory crash, the host FriedChip+ process remains running.
- **What Subprocess Spawning Does NOT Prevent:** Merely calling `std::process::Command` does **NOT** prevent:
  1. Arbitrary filesystem read or write access to the user's files.
  2. Outbound network socket creation or data exfiltration.
  3. Local privilege escalation or token inheritance.
  4. CPU or RAM exhaustion affecting the whole machine.
  5. Interacting with other processes owned by the user.
  6. Compromised or malicious child-process behavior.

### 4.2 Windows Subprocess Sandboxing Requirements (`NOT_IMPLEMENTED`)
In standard Rust on Windows, child processes inherit ambient user tokens and permissions. Standard process spawning provides zero security isolation.

For a true production-grade sandbox on Windows, future Phase 3 implementation must incorporate OS-enforced controls:
1. **Windows Job Objects (`CreateJobObjectW`, `SetInformationJobObject`):**
   - Configure `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so that child processes and any child processes they spawn terminate deterministically when the parent handle closes.
   - Configure memory limits (`JOB_OBJECT_LIMIT_JOB_MEMORY`) to prevent runaway allocations.
   - Restrict active process count to 1.
2. **Restricted Security Tokens & AppContainer:**
   - Drop administrative and ambient privileges using `CreateRestrictedToken` or `SaferComputeTokenFromLevel`.
   - Run child processes under Low Integrity Level (`SECURITY_MANDATORY_LOW_RID`) or AppContainer isolation where supported.
3. **Network Isolation:**
   - Deny network access via Windows Filtering Platform (WFP) rules or by stripping `SeNetworkLogonRight` and restricting socket creation privileges.
4. **Filesystem DACLs:**
   - Restrict write permissions strictly to the ephemeral temporary scratch directory (`tempfile::tempdir()`), preventing child writes to parent application binaries, configuration files, or user directories.

---

## 5. Licensing Boundaries & Legal Reality (DEF-64)

### 5.1 Subprocess IPC is NOT Automatic Legal Separation
An architectural boundary (such as IPC over standard input/output) does **not** automatically establish copyleft separation, license exemption, or legal compliance under the GNU General Public License (GPL). Legal precedents and copyright holders evaluate whether two programs form a combined work based on the intimacy of data exchange, functional coupling, and semantic dependencies, rather than purely the IPC mechanism.

Architectural isolation must never be presented as a settled legal conclusion. Every candidate tool requires independent legal review before redistribution or integration.

### 5.2 External Tool Licensing Ledger

| Tool Candidate | Repository Identity | Pinned Commit SHA | Actual License File & Version | Proposed Integration Type | Licensing Status |
|----------------|---------------------|-------------------|-------------------------------|---------------------------|------------------|
| **`MEAnalyzer`** | `https://github.com/platomav/MEAnalyzer.git` | `d27ccdba0a` | `LICENSE` (BSD-2-Clause with patent grant, Plato Mavropoulos) | External CLI Subprocess with JSON Wrapper | `LEGAL_REVIEW_REQUIRED` (Permissive license; low copyleft risk, attribution required) |
| **`PSPTool`** | `https://github.com/PSPReverse/PSPTool.git` | `5055403889` | `LICENSE` (GPL-3.0, Christian Werling) | External CLI Subprocess with JSON Wrapper | `LEGAL_REVIEW_REQUIRED` (Strict copyleft; IPC boundary alone does not guarantee exemption; distribution analysis required) |
| **`IFRExtractor-RS`** | `https://github.com/LongSoft/IFRExtractor-RS.git` | `b1b9b36c00` | `LICENSE.md` (BSD-2-Clause, LongSoft) | Standalone CLI Subprocess Adapter | `LEGAL_REVIEW_REQUIRED` (Permissive license; clean-room re-implementation preferred) |

