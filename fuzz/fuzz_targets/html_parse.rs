#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let output = rarog_html::parse_with_diagnostics(source);
    assert_eq!(output.document.validate_invariants(), Ok(()));
});
