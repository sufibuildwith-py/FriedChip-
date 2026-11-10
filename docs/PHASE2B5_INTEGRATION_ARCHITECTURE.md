# Phase 2B.5 — External Adapter Integration Architecture

**Project:** FriedChip+  
**Release:** Phase 2B.5  
**Scope:** Formal Sub-Process Adapter Contract, Communication Protocol, and Isolation Architecture for External Tools.

---

## 1. Principles of Architecture

FriedChip+ distinguishes strictly between:
1. **Core Native Engine:** Written in pure, memory-safe Rust inside `src-tauri/src/engine/`. Responsible for primary parsing, structural validation gates, byte-level integrity checks, database storage, and Tauri desktop UI events.
2. **External Adapters:** Out-of-tree worker executables written in other languages (Python, Go, C++) invoked strictly as sandboxed worker processes.

### Core Architecture Mandates
- **No In-Process Dynamic Plugin Marketplaces:** Third-party binary code or untrusted scripts are NEVER loaded dynamically into the main Tauri process address space (no `libloading`, no unsafe FFI shared libraries).
- **Subprocess Isolation:** Every external tool runs in an isolated OS process with restricted privileges.
- **Read-Only Input Guarantee:** The native engine only passes read-only file descriptors or immutable temp files to the adapter. Adapters cannot modify source firmware images.
- **Structured JSON Protocol:** Communication occurs exclusively over standard streams (`stdin` / `stdout`) using versioned JSON schemas.
- **Fail-Safe Process Supervisions:** Deterministic timeouts, resource limits, and strict exit code validation ensure crashes in an adapter cannot crash FriedChip+.

---

## 2. Adapter Protocol Specification

### 2.1 Standard Process Invocations
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

### 2.2 Input Command Schema (`stdin`)
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

### 2.3 Output Result Schema (`stdout`)
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

### 2.4 Error Handling & Exit Codes
- **Exit Code 0:** Successful execution; `status` is `"SUCCESS"` and `data` contains parsed payload.
- **Exit Code 1:** Unrecoverable parse failure; `status` is `"PARSE_ERROR"` and `errors` contains diagnostic strings.
- **Exit Code 2:** Schema / Protocol invalid; invalid arguments or malformed request.
- **Timeout (SIGKILL / TerminateProcess):** Native engine enforces hard wall-clock timeout (default 30 seconds). The worker is killed, and the engine records `AdapterError::Timeout`.

---

## 3. Sandboxing & Operational Boundaries

1. **Working Directory:** Adapters execute within an isolated ephemeral temporary directory created per request (`tempfile::tempdir()`).
2. **Network Isolation:** Adapters run without outbound network access. No remote telemetry or cloud calls are authorized.
3. **Environment Isolation:** Adapters inherit only an explicit, sanitized whitelist of environment variables (`PATH`, `SYSTEMROOT`, `TEMP`).
4. **Clean-Up:** On process termination (normal or abnormal), temporary scratch directories are unlinked and destroyed immediately.
