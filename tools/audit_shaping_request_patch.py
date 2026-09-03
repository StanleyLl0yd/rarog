from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
s = path.read_text()
old = '''    for run in runs.iter().copied() {
        let mut request_start = run.range.start;
        let mut current_script = None;

        for window in boundaries.windows(2) {
            let cluster_start = window[0];
            let cluster_end = window[1];
            if cluster_start < run.range.start || cluster_end > run.range.end {
                continue;
            }

            let cluster_script = shaping_script_for_characters(
'''
new = '''    for run in runs.iter().copied() {
        let mut request_start = run.range.start;
        let mut current_script = None;
        let first_boundary = boundaries.partition_point(|boundary| *boundary < run.range.start);
        let after_last_boundary =
            boundaries.partition_point(|boundary| *boundary <= run.range.end);

        for window in boundaries[first_boundary..after_last_boundary].windows(2) {
            let cluster_start = window[0];
            let cluster_end = window[1];
            let cluster_script = shaping_script_for_characters(
'''
if s.count(old) != 1:
    raise SystemExit("shaping request loop anchor mismatch")
s = s.replace(old, new, 1)
path.write_text(s)
