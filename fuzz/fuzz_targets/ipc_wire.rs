#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_ipc::{IpcLimits, decode_wire_frame};

const LIMITS: IpcLimits = IpcLimits {
    max_message_bytes: 4 * 1024,
    max_queued_messages: 8,
    max_queued_bytes: 8 * 1024,
};

fuzz_target!(|data: &[u8]| {
    let _ = decode_wire_frame(data, LIMITS);
});
