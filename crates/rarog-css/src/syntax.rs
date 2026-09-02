use cssparser::{
    AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser,
    ParserInput, ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser,
    StyleSheetParser, Token, parse_important,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedDeclaration {
    pub name: String,
    pub value: String,
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedRule {
    pub selectors: Vec<String>,
    pub declarations: Vec<ParsedDeclaration>,
}

pub(crate) fn parse_stylesheet(input: &str) -> Vec<ParsedRule> {
    let mut input_state = ParserInput::new(input);
    let mut input = Parser::new(&mut input_state);
    let mut parser = StylesheetRuleParser;
    StyleSheetParser::new(&mut input, &mut parser)
        .filter_map(Result::ok)
        .collect()
}

pub(crate) fn parse_declarations(input: &str) -> Vec<ParsedDeclaration> {
    let mut input_state = ParserInput::new(input);
    let mut input = Parser::new(&mut input_state);
    parse_declaration_body(&mut input)
}

struct StylesheetRuleParser;

impl<'i> AtRuleParser<'i> for StylesheetRuleParser {
    type Prelude = ();
    type AtRule = ParsedRule;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for StylesheetRuleParser {
    type Prelude = Vec<String>;
    type QualifiedRule = ParsedRule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        input.parse_comma_separated(|input| {
            let start = input.position();
            while input.next().is_ok() {}
            let selector = input.slice_from(start).trim();
            if selector.is_empty() {
                Err(input.new_custom_error(()))
            } else {
                Ok(selector.to_owned())
            }
        })
    }

    fn parse_block<'t>(
        &mut self,
        selectors: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        Ok(ParsedRule {
            selectors,
            declarations: parse_declaration_body(input),
        })
    }
}

struct DeclarationListParser;

impl<'i> DeclarationParser<'i> for DeclarationListParser {
    type Declaration = ParsedDeclaration;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        let start = input.position();
        while input.next().is_ok() {}
        let raw_value = input.slice_from(start).trim();
        let (value, important) = split_important(raw_value);
        if value.is_empty() {
            return Err(input.new_custom_error(()));
        }
        Ok(ParsedDeclaration {
            name: name.to_ascii_lowercase(),
            value: value.to_owned(),
            important,
        })
    }
}

impl<'i> AtRuleParser<'i> for DeclarationListParser {
    type Prelude = ();
    type AtRule = ParsedDeclaration;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for DeclarationListParser {
    type Prelude = ();
    type QualifiedRule = ParsedDeclaration;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, ParsedDeclaration, ()> for DeclarationListParser {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false
    }
}

fn split_important(value: &str) -> (&str, bool) {
    let mut input_state = ParserInput::new(value);
    let mut input = Parser::new(&mut input_state);
    let start = input.position();
    let mut important_start = None;

    loop {
        let state = input.state();
        match input.next_including_whitespace_and_comments() {
            Ok(Token::Delim('!')) => important_start = Some(state),
            Ok(_) => {}
            Err(error) if matches!(error.kind, BasicParseErrorKind::EndOfInput) => break,
            Err(_) => break,
        }
    }

    let Some(important_start) = important_start else {
        return (value.trim(), false);
    };
    input.reset(&important_start);
    if input.try_parse(parse_important).is_ok() && input.expect_exhausted().is_ok() {
        return (input.slice(start..important_start.position()).trim(), true);
    }
    (value.trim(), false)
}

fn parse_declaration_body<'i>(input: &mut Parser<'i, '_>) -> Vec<ParsedDeclaration> {
    let mut parser = DeclarationListParser;
    RuleBodyParser::new(input, &mut parser)
        .filter_map(Result::ok)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_respects_nested_css_syntax_boundaries() {
        let rules = parse_stylesheet(
            ".a { unknown: \"};\"; width: 10px; } .b { height: 20px; display: block; }",
        );

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selectors, vec![".a"]);
        assert_eq!(
            rules[0]
                .declarations
                .iter()
                .map(|declaration| declaration.name.as_str())
                .collect::<Vec<_>>(),
            vec!["unknown", "width"]
        );
        assert_eq!(rules[1].selectors, vec![".b"]);
        assert_eq!(rules[1].declarations.len(), 2);
    }

    #[test]
    fn unsupported_at_rule_does_not_hide_following_rule() {
        let rules =
            parse_stylesheet("@media screen { .hidden { width: 1px; } } .shown { width: 2px; }");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selectors, vec![".shown"]);
        assert_eq!(rules[0].declarations[0].name, "width");
    }

    #[test]
    fn declaration_parser_does_not_split_inside_component_values() {
        let declarations = parse_declarations("unknown: url(\"a;b\"); width: 12px; height: 8px;");

        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].name, "unknown");
        assert_eq!(declarations[1].name, "width");
        assert_eq!(declarations[2].name, "height");
    }

    #[test]
    fn terminal_important_is_separated_from_property_value() {
        let declarations = parse_declarations(
            "width: 10px !important; unknown: func(!important); height: 8px ! important junk;",
        );

        assert_eq!(declarations[0].value, "10px");
        assert!(declarations[0].important);
        assert_eq!(declarations[1].value, "func(!important)");
        assert!(!declarations[1].important);
        assert_eq!(declarations[2].value, "8px ! important junk");
        assert!(!declarations[2].important);
    }

    #[test]
    fn deeply_nested_malformed_values_are_recoverable() {
        let nested = format!("{}x{}", "(".repeat(96), ")".repeat(96));
        let source = format!(".a {{ unknown: {nested}; }} .b {{ width: 2px; }}");
        let rules = parse_stylesheet(&source);

        assert!(rules.iter().any(|rule| rule.selectors == [".b"]));
    }
}
