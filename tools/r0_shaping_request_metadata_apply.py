from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()

anchor = '''pub trait ShapingBackend {\n    fn shape_run(&self, text: &str, run: ShapingRun, face: &FontFace) -> ShapedRun;\n}\n'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct OpenTypeTag(u32);\n\nimpl OpenTypeTag {\n    pub const fn from_bytes(bytes: [u8; 4]) -> Self {\n        Self(u32::from_be_bytes(bytes))\n    }\n\n    pub const fn value(self) -> u32 {\n        self.0\n    }\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum ShapingScript {\n    Common,\n    Latin,\n    Cyrillic,\n    Hebrew,\n    Arabic,\n    Han,\n    Emoji,\n    Unknown,\n}\n\n#[derive(Clone, Debug, PartialEq, Eq)]\npub struct LanguageTag(String);\n\nimpl LanguageTag {\n    pub fn new(value: impl Into<String>) -> Self {\n        let value = value.into();\n        Self(if value.trim().is_empty() {\n            "und".into()\n        } else {\n            value.to_ascii_lowercase()\n        })\n    }\n\n    pub fn as_str(&self) -> &str {\n        &self.0\n    }\n}\n\nimpl Default for LanguageTag {\n    fn default() -> Self {\n        Self("und".into())\n    }\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct OpenTypeFeature {\n    pub tag: OpenTypeTag,\n    pub value: u32,\n}\n\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct VariationCoordinate {\n    pub axis: OpenTypeTag,\n    pub value: f32,\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct ShapingRequest {\n    pub run: ShapingRun,\n    pub script: ShapingScript,\n    pub language: LanguageTag,\n    pub features: Vec<OpenTypeFeature>,\n    pub variations: Vec<VariationCoordinate>,\n}\n\nimpl ShapingRequest {\n    pub fn bootstrap(text: &str, run: ShapingRun) -> Self {\n        Self {\n            run,\n            script: shaping_script_for_range(text, run.range),\n            language: LanguageTag::default(),\n            features: Vec::new(),\n            variations: Vec::new(),\n        }\n    }\n}\n\npub trait ShapingBackend {\n    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun;\n}\n'''
if anchor not in text:
    raise SystemExit('ShapingBackend anchor not found')
text = text.replace(anchor, insert, 1)

old_impl = '''impl ShapingBackend for FixedTextShaper {\n    fn shape_run(&self, text: &str, run: ShapingRun, face: &FontFace) -> ShapedRun {\n        let characters = text.chars().collect::<Vec<_>>();\n        let boundaries = grapheme_boundaries(text);\n'''
new_impl = '''impl ShapingBackend for FixedTextShaper {\n    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun {\n        let run = request.run;\n        let characters = text.chars().collect::<Vec<_>>();\n        let boundaries = grapheme_boundaries(text);\n'''
if old_impl not in text:
    raise SystemExit('FixedTextShaper backend impl anchor not found')
text = text.replace(old_impl, new_impl, 1)

old_method = '''    pub fn shape_with_backend<B: ShapingBackend>(\n        &self,\n        fallback: &FontFallbackChain,\n        backend: &B,\n    ) -> Vec<ShapedRun> {\n        self.shaping_runs()\n            .into_iter()\n            .filter_map(|run| {\n                fallback\n                    .face(run.face)\n                    .map(|face| backend.shape_run(&self.text, run, face))\n            })\n            .collect()\n    }\n'''
new_method = '''    pub fn shaping_requests(&self) -> Vec<ShapingRequest> {\n        self.shaping_runs()\n            .into_iter()\n            .map(|run| ShapingRequest::bootstrap(&self.text, run))\n            .collect()\n    }\n\n    pub fn shape_with_backend<B: ShapingBackend>(\n        &self,\n        fallback: &FontFallbackChain,\n        backend: &B,\n    ) -> Vec<ShapedRun> {\n        self.shaping_requests()\n            .into_iter()\n            .filter_map(|request| {\n                fallback\n                    .face(request.run.face)\n                    .map(|face| backend.shape_run(&self.text, &request, face))\n            })\n            .collect()\n    }\n'''
if old_method not in text:
    raise SystemExit('shape_with_backend anchor not found')
text = text.replace(old_method, new_method, 1)

script_anchor = '''pub fn paragraph_direction(text: &str) -> TextDirection {\n'''
script_fn = '''pub fn shaping_script_for_range(text: &str, range: TextRange) -> ShapingScript {\n    let characters = text.chars().collect::<Vec<_>>();\n    let Some(slice) = characters.get(range.start..range.end) else {\n        return ShapingScript::Unknown;\n    };\n    slice\n        .iter()\n        .copied()\n        .find_map(shaping_script_for_character)\n        .unwrap_or(ShapingScript::Common)\n}\n\nfn shaping_script_for_character(character: char) -> Option<ShapingScript> {\n    let code = character as u32;\n    if is_extended_pictographic(character) || is_regional_indicator(character) {\n        Some(ShapingScript::Emoji)\n    } else if matches!(code, 0x0041..=0x024f) {\n        Some(ShapingScript::Latin)\n    } else if matches!(code, 0x0400..=0x052f) {\n        Some(ShapingScript::Cyrillic)\n    } else if matches!(code, 0x0590..=0x05ff) {\n        Some(ShapingScript::Hebrew)\n    } else if matches!(code, 0x0600..=0x08ff | 0xfb50..=0xfdff | 0xfe70..=0xfefc) {\n        Some(ShapingScript::Arabic)\n    } else if matches!(code, 0x2e80..=0x9fff | 0xf900..=0xfaff) {\n        Some(ShapingScript::Han)\n    } else if is_common_font_character(character) || is_grapheme_extend(character) {\n        None\n    } else {\n        Some(ShapingScript::Unknown)\n    }\n}\n\n'''
if script_anchor not in text:
    raise SystemExit('paragraph_direction anchor not found')
text = text.replace(script_anchor, script_fn + script_anchor, 1)

# Adapt direct backend calls in existing tests.
text = text.replace('backend.shape_run(text, *run, fallback.face(run.face).unwrap())', 'backend.shape_run(text, &ShapingRequest::bootstrap(text, *run), fallback.face(run.face).unwrap())')
text = text.replace('FixedTextShaper::default().shape_run("ab", run, &face)', 'FixedTextShaper::default().shape_run("ab", &ShapingRequest::bootstrap("ab", run), &face)')
text = text.replace('FixedTextShaper::default().shape_run(text, run, fallback.face(run.face).unwrap())', 'FixedTextShaper::default().shape_run(text, &ShapingRequest::bootstrap(text, run), fallback.face(run.face).unwrap())')

module_end = text.rfind('\n}')
extra = r'''

    #[test]
    fn shaping_request_infers_script_without_changing_source_range() {
        let text = "abc Привет שלום مرحبا 世界 👩🏽\u{200d}💻";
        let fallback = FontFallbackChain::default();
        let run = TextRun::with_fallback(text.into(), &fallback);
        let requests = run.shaping_requests();
        assert!(requests.iter().any(|request| request.script == ShapingScript::Latin));
        assert!(requests.iter().any(|request| request.script == ShapingScript::Cyrillic));
        assert!(requests.iter().any(|request| request.script == ShapingScript::Hebrew));
        assert!(requests.iter().any(|request| request.script == ShapingScript::Arabic));
        assert!(requests.iter().any(|request| request.script == ShapingScript::Han));
        assert!(requests.iter().any(|request| request.script == ShapingScript::Emoji));
        assert!(requests.iter().all(|request| request.run.range.start < request.run.range.end));
    }

    #[test]
    fn language_tag_has_deterministic_und_default_and_normalization() {
        assert_eq!(LanguageTag::default().as_str(), "und");
        assert_eq!(LanguageTag::new("").as_str(), "und");
        assert_eq!(LanguageTag::new("RU-ru").as_str(), "ru-ru");
    }

    #[test]
    fn shaping_request_carries_features_and_variation_coordinates() {
        let run = ShapingRun {
            range: TextRange::new(0, 3),
            face: FontFaceId::new(0),
            level: BidiLevel::new(0),
        };
        let mut request = ShapingRequest::bootstrap("abc", run);
        request.language = LanguageTag::new("en");
        request.features.push(OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"liga"),
            value: 1,
        });
        request.variations.push(VariationCoordinate {
            axis: OpenTypeTag::from_bytes(*b"wght"),
            value: 650.0,
        });
        assert_eq!(request.script, ShapingScript::Latin);
        assert_eq!(request.language.as_str(), "en");
        assert_eq!(request.features[0].tag.value(), u32::from_be_bytes(*b"liga"));
        assert_eq!(request.variations[0].axis.value(), u32::from_be_bytes(*b"wght"));
    }

    #[test]
    fn backend_boundary_accepts_metadata_without_changing_bootstrap_geometry() {
        let fallback = FontFallbackChain::default();
        let run = shaping_runs("abc", &fallback)[0];
        let face = fallback.face(run.face).unwrap();
        let backend = FixedTextShaper::default();
        let baseline = backend.shape_run("abc", &ShapingRequest::bootstrap("abc", run), face);
        let mut configured = ShapingRequest::bootstrap("abc", run);
        configured.language = LanguageTag::new("en");
        configured.features.push(OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"kern"),
            value: 1,
        });
        configured.variations.push(VariationCoordinate {
            axis: OpenTypeTag::from_bytes(*b"wght"),
            value: 700.0,
        });
        let shaped = backend.shape_run("abc", &configured, face);
        assert_eq!(baseline, shaped);
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

arch = Path('docs/ARCHITECTURE.md')
arch_text = arch.read_text()
section = '''\n\n### Shaping request metadata\n\nR0 now carries backend-neutral shaping metadata in `ShapingRequest`. Every request preserves one resolved `ShapingRun` and adds script classification, a normalized language tag, OpenType feature settings, and variation-axis coordinates. Bootstrap requests infer script deterministically from the scalar-indexed source range and default language to `und`; feature and variation vectors are empty unless the caller explicitly configures them. The deterministic bootstrap backend accepts this metadata but intentionally ignores feature/variation semantics, leaving those decisions to a future OpenType implementation behind the same `ShapingBackend` boundary.\n'''
if '### Shaping request metadata' not in arch_text:
    arch.write_text(arch_text.rstrip() + section + '\n')

Path('docs/adr/0023-shaping-request-metadata.md').write_text('''# ADR-0023: Shaping request metadata\n\n## Status\n\nAccepted.\n\n## Context\n\nThe shaping backend boundary can already consume a resolved font face and bidi-safe shaping run, but a production OpenType implementation also needs script, language, feature settings, and variable-font coordinates. Encoding those as backend-specific parameters would leak shaping policy across the layout boundary.\n\n## Decision\n\nIntroduce `ShapingRequest`, which owns one existing `ShapingRun` plus `ShapingScript`, `LanguageTag`, a list of `OpenTypeFeature` settings, and `VariationCoordinate` values addressed by four-byte `OpenTypeTag`s. Bootstrap requests infer script deterministically from their source range, default language to `und`, and carry no features or variations. `ShapingBackend::shape_run` consumes the request rather than a bare run.\n\nThe R0 fixed backend accepts the complete request while intentionally ignoring OpenType behavior so current deterministic geometry does not change.\n\n## Consequences\n\nA production OpenType backend can receive the metadata it needs without changing bidi segmentation, font fallback, source mapping, line layout, fragmentation, or paint identity. Full Unicode Script data, BCP 47 validation/canonicalization, script/language inheritance from CSS/DOM, feature ranges, font-specific axis validation, and platform font discovery remain future work.\n''')
