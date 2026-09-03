from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    '''        let new_styles = if stylesheet_sources_changed {
            StyleSet::for_document(&self.document)
        } else {
            self.styles.clone()
        };
        validate_style_limits(&new_styles, self.limits)?;
''',
    '''        let mut rebuilt_styles = stylesheet_sources_changed
            .then(|| StyleSet::for_document(&self.document));
        let new_styles = rebuilt_styles.as_ref().unwrap_or(&self.styles);
        validate_style_limits(new_styles, self.limits)?;
''',
    "borrow unchanged styles",
)
text = replace_once(
    text,
    "            self.full_rebuild(new_styles);\n",
    '''            let styles = rebuilt_styles
                .take()
                .unwrap_or_else(|| self.styles.clone());
            self.full_rebuild(styles);
''',
    "full rebuild owns styles only on fallback",
)
text = text.replace(
    "            self.styles = new_styles;\n",
    '''            if let Some(styles) = rebuilt_styles.take() {
                self.styles = styles;
            }
''',
)
if text.count("self.styles = new_styles") != 0:
    raise SystemExit("unconverted new_styles assignment remains")
text = replace_once(
    text,
    "    let document = rarog_html::parse_standards(source);\n",
    "    let document = rarog_html::parse(source);\n",
    "canonical HTML parser entry point",
)
path.write_text(text)
