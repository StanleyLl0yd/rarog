from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text()
old = '''        let mut style_candidates = BTreeSet::new();
        let mut requires_full_rebuild = false;
'''
new = '''        let mut style_candidates = self
            .dirty
            .entries()
            .iter()
            .filter_map(|(node, flags)| flags.style.then_some(*node))
            .collect::<BTreeSet<_>>();
        let mut requires_full_rebuild = false;
'''
if old not in text:
    raise SystemExit("missing style candidate marker")
path.write_text(text.replace(old, new, 1))
