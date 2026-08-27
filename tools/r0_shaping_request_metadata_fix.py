from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()
old = '''    pub fn shaping_requests(&self) -> Vec<ShapingRequest> {\n        self.shaping_runs()\n            .into_iter()\n            .map(|run| ShapingRequest::bootstrap(&self.text, run))\n            .collect()\n    }\n'''
new = '''    pub fn shaping_requests(&self) -> Vec<ShapingRequest> {\n        shaping_requests_for_runs(&self.text, &self.shaping_runs())\n    }\n'''
if old not in text:
    raise SystemExit('shaping_requests method anchor not found')
text = text.replace(old, new, 1)

anchor = '''pub fn shaping_script_for_range(text: &str, range: TextRange) -> ShapingScript {\n'''
helper = '''fn shaping_requests_for_runs(text: &str, runs: &[ShapingRun]) -> Vec<ShapingRequest> {\n    let boundaries = grapheme_boundaries(text);\n    let mut requests = Vec::new();\n\n    for run in runs.iter().copied() {\n        let mut request_start = run.range.start;\n        let mut current_script = None;\n\n        for window in boundaries.windows(2) {\n            let cluster_start = window[0];\n            let cluster_end = window[1];\n            if cluster_start < run.range.start || cluster_end > run.range.end {\n                continue;\n            }\n\n            let cluster_script = shaping_script_for_range(\n                text,\n                TextRange::new(cluster_start, cluster_end),\n            );\n            if matches!(cluster_script, ShapingScript::Common) {\n                continue;\n            }\n\n            match current_script {\n                Some(script) if script != cluster_script => {\n                    let request_run = ShapingRun {\n                        range: TextRange::new(request_start, cluster_start),\n                        face: run.face,\n                        level: run.level,\n                    };\n                    let mut request = ShapingRequest::bootstrap(text, request_run);\n                    request.script = script;\n                    requests.push(request);\n                    request_start = cluster_start;\n                    current_script = Some(cluster_script);\n                }\n                None => current_script = Some(cluster_script),\n                Some(_) => {}\n            }\n        }\n\n        if request_start < run.range.end {\n            let request_run = ShapingRun {\n                range: TextRange::new(request_start, run.range.end),\n                face: run.face,\n                level: run.level,\n            };\n            let mut request = ShapingRequest::bootstrap(text, request_run);\n            if let Some(script) = current_script {\n                request.script = script;\n            }\n            requests.push(request);\n        }\n    }\n\n    requests\n}\n\n'''
if anchor not in text:
    raise SystemExit('script helper anchor not found')
text = text.replace(anchor, helper + anchor, 1)
path.write_text(text)

adr = Path('docs/adr/0023-shaping-request-metadata.md')
adr_text = adr.read_text()
adr_text = adr_text.replace(
    'Bootstrap requests infer script deterministically from their source range, default language to `und`, and carry no features or variations.',
    'Bootstrap requests split existing bidi×font shaping runs again at grapheme-safe script changes, infer script deterministically for each resulting source range, default language to `und`, and carry no features or variations.'
)
adr.write_text(adr_text)
