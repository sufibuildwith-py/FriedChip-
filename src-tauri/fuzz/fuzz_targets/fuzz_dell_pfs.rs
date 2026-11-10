#![no_main]
use libfuzzer_sys::fuzz_target;
use lbe_desktop_lib::engine::vendor::dell_pfs::DellPfsParser;

fuzz_target!(|data: &[u8]| {
    // Fuzz Dell PFS parser: bounded memory, must never panic or enter infinite loops
    if DellPfsParser::is_dell_pfs(data) {
        let _ = DellPfsParser::extract(data, None, "fuzz_target");
    }
});
