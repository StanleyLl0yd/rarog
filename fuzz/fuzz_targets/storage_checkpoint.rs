#![no_main]

use libfuzzer_sys::fuzz_target;
use rarog_process::ProcessTopology;
use rarog_storage::{
    StorageCheckpointLimits, StorageLimits, StorageProcessState, restore_storage_checkpoint,
};

const CHECKPOINT_LIMIT: usize = 1024 * 1024;

fuzz_target!(|data: &[u8]| {
    let Ok(mut topology) = ProcessTopology::try_new(1) else {
        return;
    };
    let Ok(assignment) = topology.ensure_storage_process() else {
        return;
    };
    let limits = StorageLimits {
        max_origins: 16,
        max_entries_per_origin: 32,
        max_key_bytes: 1024,
        max_value_bytes: 64 * 1024,
        max_origin_bytes: 256 * 1024,
        max_total_bytes: 512 * 1024,
    };
    let Ok(mut storage) = StorageProcessState::try_new(assignment.process(), limits) else {
        return;
    };
    let checkpoint_limits = StorageCheckpointLimits {
        max_checkpoint_bytes: CHECKPOINT_LIMIT,
    };
    let _ = restore_storage_checkpoint(&mut storage, data, checkpoint_limits);
});
