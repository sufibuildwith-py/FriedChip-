//! Test-only parser path execution observer for mutation fuzzing and regression verification (DEF-44, DEF-50).
//!
//! Provides deterministic, zero-overhead execution path counters recorded at the exact
//! instruction sites inside parsers (IFD, Dell PFS, UEFI FV) without global mutable state
//! or production overhead.

use std::cell::Cell;

thread_local! {
    static OBSERVER_ENABLED: Cell<bool> = const { Cell::new(false) };

    // IFD path counters
    static IFD_SIG_ACCEPTED: Cell<usize> = const { Cell::new(0) };
    static IFD_SIG_REJECTED: Cell<usize> = const { Cell::new(0) };
    static IFD_GEOM_ACCEPTED: Cell<usize> = const { Cell::new(0) };
    static IFD_GEOM_REJECTED: Cell<usize> = const { Cell::new(0) };
    static IFD_PROF_DEC_REACHED: Cell<usize> = const { Cell::new(0) };
    static IFD_REG_TBL_ENTRY: Cell<usize> = const { Cell::new(0) };
    static IFD_REG_TBL_COMPLETED: Cell<usize> = const { Cell::new(0) };
    static IFD_REG_CONFLICT_REACHED: Cell<usize> = const { Cell::new(0) };

    // Dell PFS path counters (DEF-50: is_dell_pfs vs extract separated)
    static PFS_IS_DELL_PFS_MATCHED: Cell<usize> = const { Cell::new(0) };
    static PFS_IS_DELL_PFS_UNMATCHED: Cell<usize> = const { Cell::new(0) };
    static PFS_EXTRACT_ACCEPTED: Cell<usize> = const { Cell::new(0) };
    static PFS_EXTRACT_REJECTED: Cell<usize> = const { Cell::new(0) };
    static PFS_SECTION_TABLE_REACHED: Cell<usize> = const { Cell::new(0) };
    static PFS_GUID_NAME_REACHED: Cell<usize> = const { Cell::new(0) };
    static PFS_ZLIB_ATTEMPTED: Cell<usize> = const { Cell::new(0) };
    static PFS_ZLIB_SUCCEEDED: Cell<usize> = const { Cell::new(0) };
    static PFS_ZLIB_REJECTED: Cell<usize> = const { Cell::new(0) };
    static PFS_CRC_VERIFIED_MATCH: Cell<usize> = const { Cell::new(0) };
    static PFS_CRC_VERIFIED_MISMATCH: Cell<usize> = const { Cell::new(0) };
    static PFS_XOR_VERIFIED_MATCH: Cell<usize> = const { Cell::new(0) };
    static PFS_XOR_VERIFIED_MISMATCH: Cell<usize> = const { Cell::new(0) };

    // UEFI FV path counters (DEF-50: accounts for invalid position <0x28 and candidate rejection)
    static FV_FVH_DETECTED: Cell<usize> = const { Cell::new(0) };
    static FV_FVH_REJECTED: Cell<usize> = const { Cell::new(0) };
    static FV_HEADER_VALIDATED: Cell<usize> = const { Cell::new(0) };
    static FV_CHECKSUM_EVALUATED: Cell<usize> = const { Cell::new(0) };
    static FV_BLOCK_MAP_TRAVERSED: Cell<usize> = const { Cell::new(0) };
    static FV_SENTINEL_FOUND: Cell<usize> = const { Cell::new(0) };
    static FV_SENTINEL_MISSING: Cell<usize> = const { Cell::new(0) };
}

/// Snapshot of path execution counts collected by the observer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParserPathSnapshot {
    pub ifd_sig_accepted: usize,
    pub ifd_sig_rejected: usize,
    pub ifd_geom_accepted: usize,
    pub ifd_geom_rejected: usize,
    pub ifd_profile_decoder_reached: usize,
    pub ifd_region_table_entry: usize,
    pub ifd_region_table_completed: usize,
    pub ifd_region_conflict_reached: usize,

    pub pfs_is_dell_pfs_matched: usize,
    pub pfs_is_dell_pfs_unmatched: usize,
    pub pfs_extract_accepted: usize,
    pub pfs_extract_rejected: usize,
    pub pfs_section_table_reached: usize,
    pub pfs_guid_name_reached: usize,
    pub pfs_zlib_attempted: usize,
    pub pfs_zlib_succeeded: usize,
    pub pfs_zlib_rejected: usize,
    pub pfs_crc_verified_match: usize,
    pub pfs_crc_verified_mismatch: usize,
    pub pfs_xor_verified_match: usize,
    pub pfs_xor_verified_mismatch: usize,

    pub fv_fvh_detected: usize,
    pub fv_fvh_rejected: usize,
    pub fv_header_validated: usize,
    pub fv_checksum_evaluated: usize,
    pub fv_block_map_traversed: usize,
    pub fv_sentinel_found: usize,
    pub fv_sentinel_missing: usize,
}

impl ParserPathSnapshot {
    /// Backwards-compatibility alias for ifd_region_table_completed.
    pub fn ifd_region_table_decoded(&self) -> usize {
        self.ifd_region_table_completed
    }
}

/// Reset and enable parser instrumentation on the current thread.
pub fn enable_observer() {
    OBSERVER_ENABLED.with(|e| e.set(true));
    reset_observer();
}

/// Disable parser instrumentation on the current thread.
pub fn disable_observer() {
    OBSERVER_ENABLED.with(|e| e.set(false));
}

/// Reset all path counters to zero.
pub fn reset_observer() {
    IFD_SIG_ACCEPTED.with(|c| c.set(0));
    IFD_SIG_REJECTED.with(|c| c.set(0));
    IFD_GEOM_ACCEPTED.with(|c| c.set(0));
    IFD_GEOM_REJECTED.with(|c| c.set(0));
    IFD_PROF_DEC_REACHED.with(|c| c.set(0));
    IFD_REG_TBL_ENTRY.with(|c| c.set(0));
    IFD_REG_TBL_COMPLETED.with(|c| c.set(0));
    IFD_REG_CONFLICT_REACHED.with(|c| c.set(0));

    PFS_IS_DELL_PFS_MATCHED.with(|c| c.set(0));
    PFS_IS_DELL_PFS_UNMATCHED.with(|c| c.set(0));
    PFS_EXTRACT_ACCEPTED.with(|c| c.set(0));
    PFS_EXTRACT_REJECTED.with(|c| c.set(0));
    PFS_SECTION_TABLE_REACHED.with(|c| c.set(0));
    PFS_GUID_NAME_REACHED.with(|c| c.set(0));
    PFS_ZLIB_ATTEMPTED.with(|c| c.set(0));
    PFS_ZLIB_SUCCEEDED.with(|c| c.set(0));
    PFS_ZLIB_REJECTED.with(|c| c.set(0));
    PFS_CRC_VERIFIED_MATCH.with(|c| c.set(0));
    PFS_CRC_VERIFIED_MISMATCH.with(|c| c.set(0));
    PFS_XOR_VERIFIED_MATCH.with(|c| c.set(0));
    PFS_XOR_VERIFIED_MISMATCH.with(|c| c.set(0));

    FV_FVH_DETECTED.with(|c| c.set(0));
    FV_FVH_REJECTED.with(|c| c.set(0));
    FV_HEADER_VALIDATED.with(|c| c.set(0));
    FV_CHECKSUM_EVALUATED.with(|c| c.set(0));
    FV_BLOCK_MAP_TRAVERSED.with(|c| c.set(0));
    FV_SENTINEL_FOUND.with(|c| c.set(0));
    FV_SENTINEL_MISSING.with(|c| c.set(0));
}

/// Take a snapshot of the current path counts.
pub fn take_snapshot() -> ParserPathSnapshot {
    ParserPathSnapshot {
        ifd_sig_accepted: IFD_SIG_ACCEPTED.with(|c| c.get()),
        ifd_sig_rejected: IFD_SIG_REJECTED.with(|c| c.get()),
        ifd_geom_accepted: IFD_GEOM_ACCEPTED.with(|c| c.get()),
        ifd_geom_rejected: IFD_GEOM_REJECTED.with(|c| c.get()),
        ifd_profile_decoder_reached: IFD_PROF_DEC_REACHED.with(|c| c.get()),
        ifd_region_table_entry: IFD_REG_TBL_ENTRY.with(|c| c.get()),
        ifd_region_table_completed: IFD_REG_TBL_COMPLETED.with(|c| c.get()),
        ifd_region_conflict_reached: IFD_REG_CONFLICT_REACHED.with(|c| c.get()),

        pfs_is_dell_pfs_matched: PFS_IS_DELL_PFS_MATCHED.with(|c| c.get()),
        pfs_is_dell_pfs_unmatched: PFS_IS_DELL_PFS_UNMATCHED.with(|c| c.get()),
        pfs_extract_accepted: PFS_EXTRACT_ACCEPTED.with(|c| c.get()),
        pfs_extract_rejected: PFS_EXTRACT_REJECTED.with(|c| c.get()),
        pfs_section_table_reached: PFS_SECTION_TABLE_REACHED.with(|c| c.get()),
        pfs_guid_name_reached: PFS_GUID_NAME_REACHED.with(|c| c.get()),
        pfs_zlib_attempted: PFS_ZLIB_ATTEMPTED.with(|c| c.get()),
        pfs_zlib_succeeded: PFS_ZLIB_SUCCEEDED.with(|c| c.get()),
        pfs_zlib_rejected: PFS_ZLIB_REJECTED.with(|c| c.get()),
        pfs_crc_verified_match: PFS_CRC_VERIFIED_MATCH.with(|c| c.get()),
        pfs_crc_verified_mismatch: PFS_CRC_VERIFIED_MISMATCH.with(|c| c.get()),
        pfs_xor_verified_match: PFS_XOR_VERIFIED_MATCH.with(|c| c.get()),
        pfs_xor_verified_mismatch: PFS_XOR_VERIFIED_MISMATCH.with(|c| c.get()),

        fv_fvh_detected: FV_FVH_DETECTED.with(|c| c.get()),
        fv_fvh_rejected: FV_FVH_REJECTED.with(|c| c.get()),
        fv_header_validated: FV_HEADER_VALIDATED.with(|c| c.get()),
        fv_checksum_evaluated: FV_CHECKSUM_EVALUATED.with(|c| c.get()),
        fv_block_map_traversed: FV_BLOCK_MAP_TRAVERSED.with(|c| c.get()),
        fv_sentinel_found: FV_SENTINEL_FOUND.with(|c| c.get()),
        fv_sentinel_missing: FV_SENTINEL_MISSING.with(|c| c.get()),
    }
}

// -----------------------------------------------------------------------------
// Instrumentation Recording Callbacks
// -----------------------------------------------------------------------------

#[inline]
pub fn record_ifd_sig(valid: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if valid {
        IFD_SIG_ACCEPTED.with(|c| c.set(c.get() + 1));
    } else {
        IFD_SIG_REJECTED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_ifd_geom(valid: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if valid {
        IFD_GEOM_ACCEPTED.with(|c| c.set(c.get() + 1));
    } else {
        IFD_GEOM_REJECTED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_ifd_profile_decoder() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    IFD_PROF_DEC_REACHED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_ifd_region_table_entry() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    IFD_REG_TBL_ENTRY.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_ifd_region_table_completed() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    IFD_REG_TBL_COMPLETED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_ifd_region_conflict() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    IFD_REG_CONFLICT_REACHED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_pfs_is_dell_pfs(matched: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if matched {
        PFS_IS_DELL_PFS_MATCHED.with(|c| c.set(c.get() + 1));
    } else {
        PFS_IS_DELL_PFS_UNMATCHED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_pfs_extract(accepted: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if accepted {
        PFS_EXTRACT_ACCEPTED.with(|c| c.set(c.get() + 1));
    } else {
        PFS_EXTRACT_REJECTED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_pfs_section_table() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    PFS_SECTION_TABLE_REACHED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_pfs_guid_name() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    PFS_GUID_NAME_REACHED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_pfs_zlib_attempt() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    PFS_ZLIB_ATTEMPTED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_pfs_zlib_result(success: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if success {
        PFS_ZLIB_SUCCEEDED.with(|c| c.set(c.get() + 1));
    } else {
        PFS_ZLIB_REJECTED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_pfs_crc(matched: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if matched {
        PFS_CRC_VERIFIED_MATCH.with(|c| c.set(c.get() + 1));
    } else {
        PFS_CRC_VERIFIED_MISMATCH.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_pfs_xor(matched: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if matched {
        PFS_XOR_VERIFIED_MATCH.with(|c| c.set(c.get() + 1));
    } else {
        PFS_XOR_VERIFIED_MISMATCH.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_fv_fvh(found: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if found {
        FV_FVH_DETECTED.with(|c| c.set(c.get() + 1));
    } else {
        FV_FVH_REJECTED.with(|c| c.set(c.get() + 1));
    }
}

#[inline]
pub fn record_fv_header_validated() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    FV_HEADER_VALIDATED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_fv_checksum_evaluated() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    FV_CHECKSUM_EVALUATED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_fv_block_map_traversed() {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    FV_BLOCK_MAP_TRAVERSED.with(|c| c.set(c.get() + 1));
}

#[inline]
pub fn record_fv_sentinel(found: bool) {
    if !OBSERVER_ENABLED.with(|e| e.get()) { return; }
    if found {
        FV_SENTINEL_FOUND.with(|c| c.set(c.get() + 1));
    } else {
        FV_SENTINEL_MISSING.with(|c| c.set(c.get() + 1));
    }
}
