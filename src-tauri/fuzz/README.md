# FriedChip+ — Firmware Parser Fuzzing Suite

This directory contains standalone libFuzzer targets for stress-testing FriedChip+ binary parsers against arbitrary malformed, truncated, and adversarial byte sequences.

## Architecture & Isolation
The fuzzing crate is isolated in `src-tauri/fuzz/` and is **not** included as a workspace member in the root `Cargo.toml`. This ensures standard application builds (`cargo build`, `cargo test`, `tauri build`) remain 100% unaffected and lightning-fast without requiring nightly toolchains or LLVM libFuzzer dependencies on end-user/developer machines.

## Fuzz Targets

1. **`fuzz_ifd`**: Exercises `IntelFlashDescriptor::parse` against arbitrary binary streams (testing boundary collision detection, FLMAP decoding, and out-of-bounds guards).
2. **`fuzz_dell_pfs`**: Exercises `DellPfsParser::extract` against arbitrary compressed or uncompressed containers (testing bounded ZLIB decompression, inverted CRC-32 calculation, and section traversal).
3. **`fuzz_uefi_fv`**: Exercises `FirmwareVolume::find_volumes` against arbitrary byte arrays (testing `_FVH` signature discovery, block map parsing, and header checksums).

## Running Fuzz Targets

### On Linux or Windows Subsystem for Linux (WSL2) [Recommended]
```bash
# 1. Install cargo-fuzz
cargo install cargo-fuzz

# 2. Switch to nightly toolchain
rustup default nightly

# 3. Run target
cd src-tauri/fuzz
cargo fuzz run fuzz_ifd
cargo fuzz run fuzz_dell_pfs
cargo fuzz run fuzz_uefi_fv
```

### On Native Windows (MSVC)
Running native `cargo-fuzz` on Windows requires the Clang/LLVM compiler toolchain with AddressSanitizer (`asan`) support installed via Visual Studio Installer.
Alternatively, synthetic property and regression tests located in `src-tauri/src/tests/engine_tests.rs` cover all boundary conditions deterministically on all native platforms.
