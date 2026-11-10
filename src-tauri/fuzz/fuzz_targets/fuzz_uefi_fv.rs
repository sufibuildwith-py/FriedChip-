#![no_main]
use libfuzzer_sys::fuzz_target;
use lbe_desktop_lib::engine::uefi::firmware_volume::FirmwareVolume;

fuzz_target!(|data: &[u8]| {
    // Fuzz UEFI Firmware Volume scanner: must never panic or enter infinite loops
    let _ = FirmwareVolume::find_volumes(data);
});
