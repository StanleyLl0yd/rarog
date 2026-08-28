#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_engine::{RenderOptions, RenderSession};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let mut options = RenderOptions::default();
    options.viewport.width = 64.0;
    options.viewport.height = 64.0;
    let _ = RenderSession::new(source, options);
});
