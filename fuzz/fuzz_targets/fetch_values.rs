#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_fetch::{FetchMethod, Header, HeaderList};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let _ = FetchMethod::try_new(source);
    let _ = Header::try_new(source, "value");
    let _ = Header::try_new("x-fuzz", source);

    if let Ok(mut headers) = HeaderList::try_new(8, 1024) {
        for part in source.split('\n').take(8) {
            let _ = headers.append("x-fuzz", part);
        }
        let _ = headers.remove("x-fuzz");
    }
});
