from pathlib import Path

path = Path("crates/rarog-html/src/lib.rs")
s = path.read_text()

old = '''pub fn parse_with_diagnostics(input: &str) -> ParseOutput {
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
'''
new = '''pub fn parse_with_diagnostics(input: &str) -> ParseOutput {
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
'''
if s.count(old) != 1:
    raise SystemExit("parse_with_diagnostics anchor mismatch")
s = s.replace(old, new, 1)

start = s.index("fn line_span(input: &str, line: u64) -> SourceSpan {")
end = s.index("\n#[cfg(test)]", start)
old_helper = s[start:end]
new_helper = '''fn line_starts(input: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(input.match_indices('\\n').map(|(index, _)| index + 1));
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
'''
s = s[:start] + new_helper + s[end:]

needle = '''    #[test]
    fn malformed_input_uses_standards_diagnostics_without_breaking_dom() {
'''
test = '''    #[test]
    fn diagnostic_line_spans_preserve_utf8_and_crlf_boundaries() {
        let input = "α\\r\\nbeta\\nγ";
        let starts = line_starts(input);

        assert_eq!(line_span(input, &starts, 0), SourceSpan::new(0, 0));
        assert_eq!(line_span(input, &starts, 1), SourceSpan::new(0, 3));
        assert_eq!(line_span(input, &starts, 2), SourceSpan::new(4, 8));
        assert_eq!(line_span(input, &starts, 3), SourceSpan::new(9, 11));
        assert_eq!(line_span(input, &starts, 4), SourceSpan::new(11, 11));
    }

'''
if s.count(needle) != 1:
    raise SystemExit("test anchor mismatch")
s = s.replace(needle, test + needle, 1)

path.write_text(s)
