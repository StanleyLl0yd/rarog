use std::fmt::{self, Write};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(value: impl Into<String>) -> Result<Self, WebIdlError> {
        let value = value.into();
        if value.is_empty() {
            return Err(WebIdlError::new(
                WebIdlErrorKind::InvalidIdentifier,
                "WebIDL identifiers must not be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebIdlDiagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebIdlErrorKind {
    InvalidIdentifier,
    Frontend,
    UnsupportedDefinition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebIdlError {
    pub kind: WebIdlErrorKind,
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl WebIdlError {
    pub fn new(kind: WebIdlErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for WebIdlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebIdlError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveType {
    Boolean,
    Byte,
    Octet,
    Short,
    UnsignedShort,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
    Float,
    UnrestrictedFloat,
    Double,
    UnrestrictedDouble,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringType {
    DomString,
    ByteString,
    UsvString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebIdlType {
    Any,
    Primitive(PrimitiveType),
    String(StringType),
    Object,
    Symbol,
    Undefined,
    Named(Identifier),
    Sequence(Box<WebIdlType>),
    FrozenArray(Box<WebIdlType>),
    Promise(Box<WebIdlType>),
    Record {
        key: StringType,
        value: Box<WebIdlType>,
    },
    Union(Vec<WebIdlType>),
    Nullable(Box<WebIdlType>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub name: Identifier,
    pub value_type: WebIdlType,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceMember {
    Attribute {
        name: Identifier,
        value_type: WebIdlType,
        readonly: bool,
        static_: bool,
    },
    Operation {
        name: Option<Identifier>,
        return_type: WebIdlType,
        arguments: Vec<Argument>,
        static_: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceDefinition {
    pub name: Identifier,
    pub inherits: Option<Identifier>,
    pub members: Vec<InterfaceMember>,
    pub partial: bool,
    pub mixin: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryMember {
    pub name: Identifier,
    pub value_type: WebIdlType,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryDefinition {
    pub name: Identifier,
    pub inherits: Option<Identifier>,
    pub members: Vec<DictionaryMember>,
    pub partial: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDefinition {
    pub name: Identifier,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedefDefinition {
    pub name: Identifier,
    pub value_type: WebIdlType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncludesDefinition {
    pub target: Identifier,
    pub mixin: Identifier,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Definition {
    Interface(InterfaceDefinition),
    Dictionary(DictionaryDefinition),
    Enum(EnumDefinition),
    Typedef(TypedefDefinition),
    Includes(IncludesDefinition),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebIdlModule {
    pub definitions: Vec<Definition>,
    pub diagnostics: Vec<WebIdlDiagnostic>,
}

impl WebIdlModule {
    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        for definition in &self.definitions {
            write_definition(&mut output, definition);
        }
        for diagnostic in &self.diagnostics {
            write_diagnostic(&mut output, diagnostic);
        }
        output
    }
}

pub trait WebIdlFrontend {
    fn parse(&self, source: &str) -> Result<WebIdlModule, WebIdlError>;
}

pub fn parse_with(
    frontend: &dyn WebIdlFrontend,
    source: &str,
) -> Result<WebIdlModule, WebIdlError> {
    frontend.parse(source)
}

fn write_definition(output: &mut String, definition: &Definition) {
    match definition {
        Definition::Interface(interface) => {
            output.push_str("interface|");
            write_identifier(output, &interface.name);
            write_optional_identifier(output, interface.inherits.as_ref());
            let _ = writeln!(output, "|{}|{}", interface.partial, interface.mixin);
            for member in &interface.members {
                write_interface_member(output, member);
            }
        }
        Definition::Dictionary(dictionary) => {
            output.push_str("dictionary|");
            write_identifier(output, &dictionary.name);
            write_optional_identifier(output, dictionary.inherits.as_ref());
            let _ = writeln!(output, "|{}", dictionary.partial);
            for member in &dictionary.members {
                output.push_str(" dictionary-member|");
                write_identifier(output, &member.name);
                output.push('|');
                write_type(output, &member.value_type);
                let _ = writeln!(output, "|{}", member.required);
            }
        }
        Definition::Enum(enum_definition) => {
            output.push_str("enum|");
            write_identifier(output, &enum_definition.name);
            output.push('|');
            for value in &enum_definition.values {
                write_atom(output, value);
            }
            output.push('\n');
        }
        Definition::Typedef(typedef) => {
            output.push_str("typedef|");
            write_identifier(output, &typedef.name);
            output.push('|');
            write_type(output, &typedef.value_type);
            output.push('\n');
        }
        Definition::Includes(includes) => {
            output.push_str("includes|");
            write_identifier(output, &includes.target);
            output.push('|');
            write_identifier(output, &includes.mixin);
            output.push('\n');
        }
    }
}

fn write_interface_member(output: &mut String, member: &InterfaceMember) {
    match member {
        InterfaceMember::Attribute {
            name,
            value_type,
            readonly,
            static_,
        } => {
            output.push_str(" attribute|");
            write_identifier(output, name);
            output.push('|');
            write_type(output, value_type);
            let _ = writeln!(output, "|{readonly}|{static_}");
        }
        InterfaceMember::Operation {
            name,
            return_type,
            arguments,
            static_,
        } => {
            output.push_str(" operation|");
            write_optional_identifier(output, name.as_ref());
            output.push('|');
            write_type(output, return_type);
            let _ = writeln!(output, "|{static_}");
            for argument in arguments {
                output.push_str("  argument|");
                write_identifier(output, &argument.name);
                output.push('|');
                write_type(output, &argument.value_type);
                let _ = writeln!(output, "|{}|{}", argument.optional, argument.variadic);
            }
        }
    }
}

fn write_type(output: &mut String, value_type: &WebIdlType) {
    match value_type {
        WebIdlType::Any => output.push_str("any"),
        WebIdlType::Primitive(primitive) => {
            let _ = write!(output, "primitive:{primitive:?}");
        }
        WebIdlType::String(string_type) => {
            let _ = write!(output, "string:{string_type:?}");
        }
        WebIdlType::Object => output.push_str("object"),
        WebIdlType::Symbol => output.push_str("symbol"),
        WebIdlType::Undefined => output.push_str("undefined"),
        WebIdlType::Named(identifier) => {
            output.push_str("named:");
            write_identifier(output, identifier);
        }
        WebIdlType::Sequence(inner) => write_wrapped_type(output, "sequence", inner),
        WebIdlType::FrozenArray(inner) => write_wrapped_type(output, "frozen-array", inner),
        WebIdlType::Promise(inner) => write_wrapped_type(output, "promise", inner),
        WebIdlType::Record { key, value } => {
            let _ = write!(output, "record:{key:?}<");
            write_type(output, value);
            output.push('>');
        }
        WebIdlType::Union(members) => {
            output.push_str("union<");
            for member in members {
                write_type(output, member);
                output.push(';');
            }
            output.push('>');
        }
        WebIdlType::Nullable(inner) => write_wrapped_type(output, "nullable", inner),
    }
}

fn write_wrapped_type(output: &mut String, label: &str, inner: &WebIdlType) {
    output.push_str(label);
    output.push('<');
    write_type(output, inner);
    output.push('>');
}

fn write_identifier(output: &mut String, identifier: &Identifier) {
    write_atom(output, identifier.as_str());
}

fn write_optional_identifier(output: &mut String, identifier: Option<&Identifier>) {
    output.push('|');
    if let Some(identifier) = identifier {
        write_identifier(output, identifier);
    } else {
        output.push('-');
    }
}

fn write_atom(output: &mut String, value: &str) {
    let _ = write!(output, "{}:{value}", value.len());
}

fn write_diagnostic(output: &mut String, diagnostic: &WebIdlDiagnostic) {
    let _ = write!(output, "diagnostic|{:?}|", diagnostic.level);
    match diagnostic.span {
        Some(span) => {
            let _ = write!(output, "{}:{}|", span.start, span.end);
        }
        None => output.push_str("-|"),
    }
    write_atom(output, &diagnostic.message);
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(value: &str) -> Identifier {
        Identifier::new(value).unwrap()
    }

    #[test]
    fn identifier_rejects_empty_values() {
        let error = Identifier::new("").unwrap_err();
        assert_eq!(error.kind, WebIdlErrorKind::InvalidIdentifier);
    }

    #[test]
    fn normalized_module_snapshot_is_deterministic() {
        let module = WebIdlModule {
            definitions: vec![Definition::Interface(InterfaceDefinition {
                name: identifier("Window"),
                inherits: Some(identifier("EventTarget")),
                members: vec![
                    InterfaceMember::Attribute {
                        name: identifier("name"),
                        value_type: WebIdlType::String(StringType::DomString),
                        readonly: false,
                        static_: false,
                    },
                    InterfaceMember::Operation {
                        name: Some(identifier("close")),
                        return_type: WebIdlType::Undefined,
                        arguments: Vec::new(),
                        static_: false,
                    },
                ],
                partial: false,
                mixin: false,
            })],
            diagnostics: Vec::new(),
        };

        let first = module.snapshot();
        let second = module.clone().snapshot();
        assert_eq!(first, second);
        assert!(first.contains("6:Window"));
        assert!(first.contains("11:EventTarget"));
        assert!(first.contains("string:DomString"));
    }

    struct FixtureFrontend;

    impl WebIdlFrontend for FixtureFrontend {
        fn parse(&self, source: &str) -> Result<WebIdlModule, WebIdlError> {
            if source.trim().is_empty() {
                return Err(WebIdlError::new(
                    WebIdlErrorKind::Frontend,
                    "fixture source must not be empty",
                ));
            }
            Ok(WebIdlModule {
                definitions: vec![Definition::Typedef(TypedefDefinition {
                    name: identifier("Count"),
                    value_type: WebIdlType::Primitive(PrimitiveType::UnsignedLong),
                })],
                diagnostics: Vec::new(),
            })
        }
    }

    #[test]
    fn frontend_contract_returns_owned_normalized_ir() {
        let module = {
            let source = String::from("typedef unsigned long Count;");
            parse_with(&FixtureFrontend, &source).unwrap()
        };

        assert_eq!(module.definitions.len(), 1);
        assert!(module.snapshot().contains("5:Count"));
    }

    #[test]
    fn nested_type_snapshot_preserves_shape() {
        let value_type = WebIdlType::Nullable(Box::new(WebIdlType::Promise(Box::new(
            WebIdlType::Sequence(Box::new(WebIdlType::Named(identifier("Node")))),
        ))));
        let module = WebIdlModule {
            definitions: vec![Definition::Typedef(TypedefDefinition {
                name: identifier("MaybeNodes"),
                value_type,
            })],
            diagnostics: Vec::new(),
        };

        assert!(
            module
                .snapshot()
                .contains("nullable<promise<sequence<named:4:Node>>>")
        );
    }
}
