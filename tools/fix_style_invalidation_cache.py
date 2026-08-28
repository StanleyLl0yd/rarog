from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text()
old = '''        document.set_attribute(node, "class", "hot").unwrap();
        dirty.capture(&document);
'''
new = '''        document.set_attribute(node, "class", "hot").unwrap();
        let styles = StyleSet::for_document(&document);
        dirty.capture(&document, &styles);
'''
if old not in text:
    raise SystemExit("missing dirty-state test marker")
path.write_text(text.replace(old, new, 1))
