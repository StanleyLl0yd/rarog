mod standards;

use rarog_dom::Document;
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
    StandardsParseError,
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
            Self::InputNotClosed => {
                formatter.write_str("streaming HTML input must be closed before parsing")
            }
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
}

pub fn parse(input: &str) -> Document {
    parse_with_diagnostics(input).document
}

pub fn parse_with_diagnostics(input: &str) -> ParseOutput {
    let output = standards::parse(input);
    let diagnostics = output
        .errors
        .into_iter()
        .map(|(line, message)| ParseDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: ParseDiagnosticCode::StandardsParseError,
            span: line_span(input, line),
            message,
        })
        .collect();
    ParseOutput {
        document: output.document,
        diagnostics,
    }
}

pub fn parse_standards(input: &str) -> Document {
    parse(input)
}

pub fn parse_standards_with_diagnostics(input: &str) -> ParseOutput {
    parse_with_diagnostics(input)
}

pub fn parse_stream(input: StreamingInput) -> Result<ParseOutput, ParseError> {
    if !input.closed {
        return Err(ParseError::InputNotClosed);
    }
    Ok(parse_with_diagnostics(&input.buffer))
}

fn line_span(input: &str, line: u64) -> SourceSpan {
    if line == 0 {
        return SourceSpan::new(0, 0);
    }
    let target = line as usize;
    let mut current = 1usize;
    let mut start = 0usize;
    for (index, character) in input.char_indices() {
        if current == target && character == '\n' {
            return SourceSpan::new(start, index);
        }
        if character == '\n' {
            current += 1;
            start = index + character.len_utf8();
        }
    }
    if current == target {
        SourceSpan::new(start, input.len())
    } else {
        SourceSpan::new(input.len(), input.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::NodeKind;

    const VALID_SOURCE: &str = "<!doctype html><html><body><div id=\"x\">hello</div></body></html>";

    #[test]
    fn parsed_document_preserves_dom_invariants() {
        let output = parse_with_diagnostics(VALID_SOURCE);
        assert_eq!(output.document.validate_invariants(), Ok(()));
        assert!(output.document.generation() > 0);
        assert!(output.diagnostics.is_empty());
        let html = output.document.children(output.document.root()).unwrap()[0];
        let NodeKind::Element(element) = &output.document.node(html).unwrap().kind else {
            panic!("expected html element");
        };
        assert_eq!(element.namespace, rarog_dom::Namespace::Html);
        assert_eq!(element.tag_name.as_str(), "html");
    }

    #[test]
    fn streaming_chunks_match_contiguous_input() {
        let mut input = StreamingInput::new();
        input.feed("<!doctype html><html><bo").unwrap();
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

        assert!(matches!(
            parse_stream(input),
            Err(ParseError::InputNotClosed)
        ));
    }

    #[test]
    fn feed_after_close_is_rejected() {
        let mut input = StreamingInput::new();
        input.close();

        assert_eq!(input.feed("x"), Err(InputError::Closed));
    }

    #[test]
    fn canonical_and_standards_names_share_one_parser() {
        let canonical = parse_with_diagnostics("<table><tr><td>x</td></tr></table>");
        let standards = parse_standards_with_diagnostics("<table><tr><td>x</td></tr></table>");

        assert_eq!(canonical.document.snapshot(), standards.document.snapshot());
        assert_eq!(canonical.diagnostics, standards.diagnostics);
    }

    #[test]
    fn malformed_input_uses_standards_diagnostics_without_breaking_dom() {
        let output = parse_with_diagnostics("<div><span>x</div>");

        assert!(!output.diagnostics.is_empty());
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == ParseDiagnosticCode::StandardsParseError)
        );
        assert_eq!(output.document.validate_invariants(), Ok(()));
    }
}
