#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_url::WebUrl;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let _ = WebUrl::parse(source);
    let base = WebUrl::parse("https://example.test/a/b/").expect("static fuzz base is valid");
    let _ = WebUrl::resolve(&base, source);
});
