#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_webidl::{StandardsWebIdlFrontend, parse_with};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let _ = parse_with(&StandardsWebIdlFrontend, source);
});
