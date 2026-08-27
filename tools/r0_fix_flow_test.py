from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text()
old_source = '''        let source = "<div id=\\"before\\" style=\\"height:5px\\"></div><div id=\\"outer\\" style=\\"padding:2px\\"><div id=\\"target\\" style=\\"height:20px\\"></div></div><div id=\\"after\\" style=\\"height:10px\\"></div>";\n        let expected_source = "<div id=\\"before\\" style=\\"height:5px\\"></div><div id=\\"outer\\" style=\\"padding:2px\\"><div id=\\"target\\" style=\\"height:32px\\"></div></div><div id=\\"after\\" style=\\"height:10px\\"></div>";'''
new_source = '''        let source = "<div id=\\"before\\" style=\\"height:5px;background:#eeeeee\\"></div><div id=\\"outer\\" style=\\"padding:2px;background:#112233\\"><div id=\\"target\\" style=\\"height:20px\\"></div></div><div id=\\"after\\" style=\\"height:10px;background:#445566\\"></div>";\n        let expected_source = "<div id=\\"before\\" style=\\"height:5px;background:#eeeeee\\"></div><div id=\\"outer\\" style=\\"padding:2px;background:#112233\\"><div id=\\"target\\" style=\\"height:32px\\"></div></div><div id=\\"after\\" style=\\"height:10px;background:#445566\\"></div>";'''
if old_source not in text:
    raise SystemExit("flow test source marker not found")
path.write_text(text.replace(old_source, new_source, 1))
