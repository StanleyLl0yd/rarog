#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_css::{StyleSource, Stylesheet};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let stylesheet = Stylesheet::parse(StyleSource::author(1, "fuzz"), source);
    let _ = stylesheet.rules.len();
});
