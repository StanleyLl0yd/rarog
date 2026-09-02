from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()
old = '''        let shaped = run.shape_with_backend(&fallback, &FixedTextShaper::default());\n        assert_eq!(shaped.len(), 4);\n'''
new = '''        let shaped = run\n            .shape_with_backend(&fallback, &FixedTextShaper::default())\n            .unwrap();\n        assert_eq!(shaped.len(), 4);\n'''
if old not in text:
    raise SystemExit("missing shaping boundary test marker")
path.write_text(text.replace(old, new, 1))
