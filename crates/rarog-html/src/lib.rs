mod standards;

use rarog_dom::Document;

pub const DEFAULT_MAX_STREAMING_INPUT_BYTES: usize = 16 * 1024 * 1024;
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
    InvalidLimit,
    LimitExceeded { bytes: usize, limit: usize },
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("streaming HTML input is already closed"),
            Self::InvalidLimit => {
                formatter.write_str("streaming HTML input limit must be non-zero")
            }
            Self::LimitExceeded { bytes, limit } => {
                write!(
                    formatter,
                    "streaming HTML input would contain {bytes} bytes; limit is {limit}"
                )
            }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingInput {
    buffer: String,
    closed: bool,
    max_bytes: usize,
}

impl Default for StreamingInput {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            closed: false,
            max_bytes: DEFAULT_MAX_STREAMING_INPUT_BYTES,
        }
    }
}

impl StreamingInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_with_max_bytes(max_bytes: usize) -> Result<Self, InputError> {
        if max_bytes == 0 {
            return Err(InputError::InvalidLimit);
        }
        Ok(Self {
            buffer: String::new(),
            closed: false,
            max_bytes,
        })
    }

    pub const fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn feed(&mut self, chunk: &str) -> Result<(), InputError> {
        if self.closed {
            return Err(InputError::Closed);
        }
        let bytes =
            self.buffer
                .len()
                .checked_add(chunk.len())
                .ok_or(InputError::LimitExceeded {
                    bytes: usize::MAX,
                    limit: self.max_bytes,
                })?;
        if bytes > self.max_bytes {
            return Err(InputError::LimitExceeded {
                bytes,
                limit: self.max_bytes,
            });
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
    let diagnostics = if output.errors.is_empty() {
        Vec::new()
    } else {
        let line_starts = line_starts(input);
        output
            .errors
            .into_iter()
            .map(|(line, message)| ParseDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: ParseDiagnosticCode::StandardsParseError,
                span: line_span(input, &line_starts, line),
                message,
            })
            .collect()
    };
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

fn line_starts(input: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(input.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

fn line_span(input: &str, line_starts: &[usize], line: u64) -> SourceSpan {
    if line == 0 {
        return SourceSpan::new(0, 0);
    }
    let Ok(index) = usize::try_from(line - 1) else {
        return SourceSpan::new(input.len(), input.len());
    };
    let Some(&start) = line_starts.get(index) else {
        return SourceSpan::new(input.len(), input.len());
    };
    let end = line_starts
        .get(index + 1)
        .map(|next| next.saturating_sub(1))
        .unwrap_or(input.len());
    SourceSpan::new(start, end)
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
    fn streaming_input_limit_is_enforced_before_retaining_chunk_bytes() {
        let mut input = StreamingInput::try_with_max_bytes(4).unwrap();
        input.feed("ab").unwrap();
        assert_eq!(
            input.feed("cde"),
            Err(InputError::LimitExceeded { bytes: 5, limit: 4 })
        );
        assert_eq!(input.len(), 2);
        assert_eq!(input.max_bytes(), 4);
    }

    #[test]
    fn zero_streaming_input_limit_is_rejected() {
        assert_eq!(
            StreamingInput::try_with_max_bytes(0),
            Err(InputError::InvalidLimit)
        );
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
    fn diagnostic_line_spans_preserve_utf8_and_crlf_boundaries() {
        let input = "α\r\nbeta\nγ";
        let starts = line_starts(input);

        assert_eq!(line_span(input, &starts, 0), SourceSpan::new(0, 0));
        assert_eq!(line_span(input, &starts, 1), SourceSpan::new(0, 3));
        assert_eq!(line_span(input, &starts, 2), SourceSpan::new(4, 8));
        assert_eq!(line_span(input, &starts, 3), SourceSpan::new(9, 11));
        assert_eq!(line_span(input, &starts, 4), SourceSpan::new(11, 11));
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
