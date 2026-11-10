#![no_main]
use libfuzzer_sys::fuzz_target;
use lbe_desktop_lib::engine::binary::ifd::IntelFlashDescriptor;

fuzz_target!(|data: &[u8]| {
    // Fuzz Intel Flash Descriptor parser: must never panic on arbitrary byte inputs
    let _ = IntelFlashDescriptor::parse(data);
});
