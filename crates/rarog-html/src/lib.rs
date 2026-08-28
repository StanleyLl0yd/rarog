use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseDiagnosticCode {
    MissingTagName,
    MismatchedEndTag,
    UnexpectedEndTag,
    UnclosedElement,
    UnterminatedTag,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: ParseDiagnosticCode,
    pub span: SourceSpan,
    pub message: String,
}

pub struct ParseOutput {
    pub document: Document,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    Closed,
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("streaming HTML input is already closed"),
        }
    }
}

impl Error for InputError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    InputNotClosed,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotClosed => formatter.write_str("streaming HTML input must be closed before parsing"),
        }
    }
}

impl Error for ParseError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamingInput {
    buffer: String,
    closed: bool,
}

impl StreamingInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &str) -> Result<(), InputError> {
        if self.closed {
            return Err(InputError::Closed);
        }
        self.buffer.push_str(chunk);
        Ok(())
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn complete(source: &str) -> Self {
        Self {
            buffer: source.to_owned(),
            closed: true,
        }
    }
}

pub fn parse(input: &str) -> Document {
    parse_with_diagnostics(input).document
}

pub fn parse_with_diagnostics(input: &str) -> ParseOutput {
    parse_stream(StreamingInput::complete(input)).expect("complete input is closed")
}

pub fn parse_stream(input: StreamingInput) -> Result<ParseOutput, ParseError> {
    if !input.closed {
        return Err(ParseError::InputNotClosed);
    }
    Ok(parse_buffer(&input.buffer))
}

#[derive(Clone, Debug)]
struct OpenElement {
    node: NodeId,
    tag_name: Option<String>,
    source_span: SourceSpan,
}

fn parse_buffer(input: &str) -> ParseOutput {
    let mut document = Document::new();
    let mut diagnostics = Vec::new();
    let mut stack = vec![OpenElement {
        node: document.root(),
        tag_name: None,
        source_span: SourceSpan::new(0, 0),
    }];
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(close_relative) = input[i..].find('>') else {
                diagnostics.push(diagnostic(
                    ParseDiagnosticCode::UnterminatedTag,
                    SourceSpan::new(i, input.len()),
                    "tag is not terminated before end of input",
                ));
                break;
            };
            let end = i + close_relative;
            let span = SourceSpan::new(i, end + 1);
            let raw = input[i + 1..end].trim();

            if raw.starts_with('!') {
                i = end + 1;
                continue;
            }

            if let Some(closing) = raw.strip_prefix('/') {
                let closing_tag = closing
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if closing_tag.is_empty() {
                    diagnostics.push(diagnostic(
                        ParseDiagnosticCode::MissingTagName,
                        span,
                        "end tag is missing a tag name",
                    ));
                } else if stack.len() == 1 {
                    diagnostics.push(diagnostic(
                        ParseDiagnosticCode::UnexpectedEndTag,
                        span,
                        format!("unexpected end tag </{closing_tag}>"),
                    ));
                } else {
                    let open_tag = stack
                        .last()
                        .and_then(|entry| entry.tag_name.as_deref())
                        .expect("non-root stack entries have tag names");
                    if open_tag != closing_tag {
                        diagnostics.push(diagnostic(
                            ParseDiagnosticCode::MismatchedEndTag,
                            span,
                            format!("end tag </{closing_tag}> closes <{open_tag}> in bootstrap parser"),
                        ));
                    }
                    stack.pop();
                }
            } else {
                let self_closing = raw.ends_with('/');
                let inner = raw.trim_end_matches('/').trim();
                let (tag, attributes) = parse_tag(inner);
                if tag.is_empty() {
                    diagnostics.push(diagnostic(
                        ParseDiagnosticCode::MissingTagName,
                        span,
                        "start tag is missing a tag name",
                    ));
                } else {
                    let parent = stack
                        .last()
                        .map(|entry| entry.node)
                        .expect("document stack is never empty");
                    let id = document
                        .append_new(
                            parent,
                            NodeKind::Element(ElementData::html(tag.clone()).with_attributes(attributes)),
                        )
                        .expect("bootstrap parser only appends to valid parents");
                    if !self_closing && !matches_void(document.node(id)) {
                        stack.push(OpenElement {
                            node: id,
                            tag_name: Some(tag),
                            source_span: span,
                        });
                    }
                }
            }
            i = end + 1;
        } else {
            let next = input[i..].find('<').map(|offset| i + offset).unwrap_or(bytes.len());
            let text = input[i..next]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                let parent = stack
                    .last()
                    .map(|entry| entry.node)
                    .expect("document stack is never empty");
                document
                    .append_new(parent, NodeKind::Text(text))
                    .expect("bootstrap parser only appends to valid parents");
            }
            i = next;
        }
    }

    for entry in stack.iter().skip(1).rev() {
        let tag_name = entry
            .tag_name
            .as_deref()
            .expect("non-root stack entries have tag names");
        diagnostics.push(diagnostic(
            ParseDiagnosticCode::UnclosedElement,
            entry.source_span,
            format!("element <{tag_name}> is not closed in bootstrap parser"),
        ));
    }

    debug_assert!(document.validate_invariants().is_ok());
    ParseOutput {
        document,
        diagnostics,
    }
}

fn diagnostic(
    code: ParseDiagnosticCode,
    span: SourceSpan,
    message: impl Into<String>,
) -> ParseDiagnostic {
    ParseDiagnostic {
        severity: DiagnosticSeverity::Error,
        code,
        span,
        message: message.into(),
    }
}

fn parse_tag(input: &str) -> (String, BTreeMap<String, String>) {
    let mut tag_end = input.len();
    for (index, character) in input.char_indices() {
        if character.is_whitespace() {
            tag_end = index;
            break;
        }
    }
    let tag = input[..tag_end].to_ascii_lowercase();
    let mut attributes = BTreeMap::new();
    let rest = input[tag_end..].trim();
    let mut cursor = 0usize;
    while cursor < rest.len() {
        while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= rest.len() {
            break;
        }
        let name_start = cursor;
        while cursor < rest.len() {
            let byte = rest.as_bytes()[cursor];
            if byte == b'=' || byte.is_ascii_whitespace() {
                break;
            }
            cursor += 1;
        }
        let name = rest[name_start..cursor].to_ascii_lowercase();
        while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let mut value = String::new();
        if cursor < rest.len() && rest.as_bytes()[cursor] == b'=' {
            cursor += 1;
            while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < rest.len()
                && (rest.as_bytes()[cursor] == b'"' || rest.as_bytes()[cursor] == b'\'')
            {
                let quote = rest.as_bytes()[cursor];
                cursor += 1;
                let start = cursor;
                while cursor < rest.len() && rest.as_bytes()[cursor] != quote {
                    cursor += 1;
                }
                value = rest[start..cursor].to_string();
                if cursor < rest.len() {
                    cursor += 1;
                }
            } else {
                let start = cursor;
                while cursor < rest.len() && !rest.as_bytes()[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                value = rest[start..cursor].to_string();
            }
        }
        if !name.is_empty() {
            attributes.insert(name, value);
        }
    }
    (tag, attributes)
}

fn matches_void(node: &rarog_dom::Node) -> bool {
    match &node.kind {
        NodeKind::Element(element) => matches!(
            element.tag_name.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SOURCE: &str = "<html><body><div id=\"x\">hello</div></body></html>";

    #[test]
    fn parsed_document_preserves_dom_invariants() {
        let output = parse_with_diagnostics(VALID_SOURCE);
        assert_eq!(output.document.validate_invariants(), Ok(()));
        assert!(output.document.generation() > 0);
        assert!(output.diagnostics.is_empty());
        let html = output.document.children(output.document.root())[0];
        let NodeKind::Element(element) = &output.document.node(html).kind else {
            panic!("expected html element");
        };
        assert_eq!(element.namespace, rarog_dom::Namespace::Html);
        assert_eq!(element.tag_name.as_str(), "html");
    }

    #[test]
    fn streaming_chunks_match_contiguous_input() {
        let mut input = StreamingInput::new();
        input.feed("<html><bo").unwrap();
        input.feed("dy><div id=\"x\">he").unwrap();
        input.feed("llo</div></body></html>").unwrap();
        input.close();

        let streamed = parse_stream(input).unwrap();
        let contiguous = parse_with_diagnostics(VALID_SOURCE);

        assert_eq!(streamed.document.snapshot(), contiguous.document.snapshot());
        assert_eq!(streamed.diagnostics, contiguous.diagnostics);
    }

    #[test]
    fn open_stream_is_rejected() {
        let mut input = StreamingInput::new();
        input.feed("<div>x</div>").unwrap();

        assert!(matches!(parse_stream(input), Err(ParseError::InputNotClosed)));
    }

    #[test]
    fn feed_after_close_is_rejected() {
        let mut input = StreamingInput::new();
        input.close();

        assert_eq!(input.feed("x"), Err(InputError::Closed));
    }

    #[test]
    fn unterminated_tag_has_deterministic_span() {
        let output = parse_with_diagnostics("<div");

        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(
            output.diagnostics[0],
            ParseDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: ParseDiagnosticCode::UnterminatedTag,
                span: SourceSpan::new(0, 4),
                message: "tag is not terminated before end of input".into(),
            }
        );
    }

    #[test]
    fn unexpected_end_tag_is_recoverable() {
        let output = parse_with_diagnostics("</div><p>x</p>");

        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(
            output.diagnostics[0].code,
            ParseDiagnosticCode::UnexpectedEndTag
        );
        assert_eq!(output.document.validate_invariants(), Ok(()));
        assert!(output.document.snapshot().contains("element:p[]"));
    }

    #[test]
    fn mismatched_end_tag_is_reported_without_breaking_dom_invariants() {
        let output = parse_with_diagnostics("<div><span>x</div>");

        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            vec![
                ParseDiagnosticCode::MismatchedEndTag,
                ParseDiagnosticCode::UnclosedElement,
            ]
        );
        assert_eq!(output.document.validate_invariants(), Ok(()));
    }
}
