use super::*;
use weedle::Parse;

#[derive(Clone, Copy, Debug, Default)]
pub struct StandardsWebIdlFrontend;

impl WebIdlFrontend for StandardsWebIdlFrontend {
    fn parse(&self, source: &str) -> Result<WebIdlModule, WebIdlError> {
        let definitions = parse_definitions(source)?;
        let mut normalized = Vec::with_capacity(definitions.len());
        for definition in &definitions {
            normalized.push(normalize_definition(definition)?);
        }
        Ok(WebIdlModule {
            definitions: normalized,
            diagnostics: Vec::new(),
        })
    }
}

fn parse_definitions(source: &str) -> Result<weedle::Definitions<'_>, WebIdlError> {
    match weedle::Definitions::parse(source) {
        Ok((remaining, definitions)) if remaining.trim().is_empty() => Ok(definitions),
        Ok((remaining, _)) => Err(frontend_error_at(
            source,
            remaining,
            "unparsed WebIDL input",
        )),
        Err(weedle::Err::Error(error) | weedle::Err::Failure(error)) => Err(frontend_error_at(
            source,
            error.input,
            "standards WebIDL parser rejected the input",
        )),
        Err(weedle::Err::Incomplete(_)) => Err(WebIdlError::new(
            WebIdlErrorKind::Frontend,
            "standards WebIDL parser requires more input",
        )),
    }
}

fn frontend_error_at(source: &str, remaining: &str, message: &str) -> WebIdlError {
    let start = source.len().saturating_sub(remaining.len());
    let end = start.saturating_add(1).min(source.len());
    WebIdlError::new(WebIdlErrorKind::Frontend, message).with_span(SourceSpan::new(start, end))
}

fn unsupported(message: impl Into<String>) -> WebIdlError {
    WebIdlError::new(WebIdlErrorKind::UnsupportedDefinition, message)
}

fn reject_extended_attributes<T>(attributes: &Option<T>, context: &str) -> Result<(), WebIdlError> {
    if attributes.is_some() {
        return Err(unsupported(format!(
            "extended attributes on {context} are not normalized yet"
        )));
    }
    Ok(())
}

fn identifier(value: &str) -> Result<Identifier, WebIdlError> {
    Identifier::new(value)
}

fn normalize_definition(definition: &weedle::Definition<'_>) -> Result<Definition, WebIdlError> {
    match definition {
        weedle::Definition::Interface(value) => {
            reject_extended_attributes(&value.attributes, "interface definitions")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(value.identifier.0)?,
                inherits: value
                    .inheritance
                    .as_ref()
                    .map(|inheritance| identifier(inheritance.identifier.0))
                    .transpose()?,
                members: normalize_interface_members(&value.members.body)?,
                partial: false,
                mixin: false,
            }))
        }
        weedle::Definition::PartialInterface(value) => {
            reject_extended_attributes(&value.attributes, "partial interface definitions")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(value.identifier.0)?,
                inherits: None,
                members: normalize_interface_members(&value.members.body)?,
                partial: true,
                mixin: false,
            }))
        }
        weedle::Definition::InterfaceMixin(value) => {
            reject_extended_attributes(&value.attributes, "interface mixin definitions")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(value.identifier.0)?,
                inherits: None,
                members: normalize_mixin_members(&value.members.body)?,
                partial: false,
                mixin: true,
            }))
        }
        weedle::Definition::PartialInterfaceMixin(value) => {
            reject_extended_attributes(&value.attributes, "partial interface mixin definitions")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(value.identifier.0)?,
                inherits: None,
                members: normalize_mixin_members(&value.members.body)?,
                partial: true,
                mixin: true,
            }))
        }
        weedle::Definition::Dictionary(value) => {
            reject_extended_attributes(&value.attributes, "dictionary definitions")?;
            Ok(Definition::Dictionary(DictionaryDefinition {
                name: identifier(value.identifier.0)?,
                inherits: value
                    .inheritance
                    .as_ref()
                    .map(|inheritance| identifier(inheritance.identifier.0))
                    .transpose()?,
                members: normalize_dictionary_members(&value.members.body)?,
                partial: false,
            }))
        }
        weedle::Definition::PartialDictionary(value) => {
            reject_extended_attributes(&value.attributes, "partial dictionary definitions")?;
            Ok(Definition::Dictionary(DictionaryDefinition {
                name: identifier(value.identifier.0)?,
                inherits: None,
                members: normalize_dictionary_members(&value.members.body)?,
                partial: true,
            }))
        }
        weedle::Definition::Enum(value) => {
            reject_extended_attributes(&value.attributes, "enum definitions")?;
            Ok(Definition::Enum(EnumDefinition {
                name: identifier(value.identifier.0)?,
                values: value
                    .values
                    .body
                    .list
                    .iter()
                    .map(|variant| variant.value.0.to_owned())
                    .collect(),
            }))
        }
        weedle::Definition::Typedef(value) => {
            reject_extended_attributes(&value.attributes, "typedef definitions")?;
            Ok(Definition::Typedef(TypedefDefinition {
                name: identifier(value.identifier.0)?,
                value_type: normalize_attributed_type(&value.type_)?,
            }))
        }
        weedle::Definition::IncludesStatement(value) => {
            reject_extended_attributes(&value.attributes, "includes statements")?;
            Ok(Definition::Includes(IncludesDefinition {
                target: identifier(value.lhs_identifier.0)?,
                mixin: identifier(value.rhs_identifier.0)?,
            }))
        }
        weedle::Definition::Callback(_)
        | weedle::Definition::CallbackInterface(_)
        | weedle::Definition::Namespace(_)
        | weedle::Definition::PartialNamespace(_)
        | weedle::Definition::Implements(_) => Err(unsupported(
            "this WebIDL definition kind is not normalized by the first standards adapter slice",
        )),
    }
}

fn normalize_interface_members(
    members: &[weedle::interface::InterfaceMember<'_>],
) -> Result<Vec<InterfaceMember>, WebIdlError> {
    members.iter().map(normalize_interface_member).collect()
}

fn normalize_interface_member(
    member: &weedle::interface::InterfaceMember<'_>,
) -> Result<InterfaceMember, WebIdlError> {
    match member {
        weedle::interface::InterfaceMember::Attribute(value) => {
            reject_extended_attributes(&value.attributes, "interface attributes")?;
            let static_ = match value.modifier {
                None => false,
                Some(weedle::interface::StringifierOrInheritOrStatic::Static(_)) => true,
                Some(_) => {
                    return Err(unsupported(
                        "stringifier/inherit interface attributes are not normalized yet",
                    ));
                }
            };
            Ok(InterfaceMember::Attribute {
                name: identifier(value.identifier.0)?,
                value_type: normalize_attributed_type(&value.type_)?,
                readonly: value.readonly.is_some(),
                static_,
            })
        }
        weedle::interface::InterfaceMember::Operation(value) => {
            reject_extended_attributes(&value.attributes, "interface operations")?;
            if value.special.is_some() {
                return Err(unsupported(
                    "getter/setter/deleter/legacycaller operations are not normalized yet",
                ));
            }
            let static_ = match value.modifier {
                None => false,
                Some(weedle::interface::StringifierOrStatic::Static(_)) => true,
                Some(weedle::interface::StringifierOrStatic::Stringifier(_)) => {
                    return Err(unsupported("stringifier operations are not normalized yet"));
                }
            };
            Ok(InterfaceMember::Operation {
                name: value
                    .identifier
                    .as_ref()
                    .map(|name| identifier(name.0))
                    .transpose()?,
                return_type: normalize_return_type(&value.return_type)?,
                arguments: normalize_arguments(&value.args.body.list)?,
                static_,
            })
        }
        weedle::interface::InterfaceMember::Const(_)
        | weedle::interface::InterfaceMember::Constructor(_)
        | weedle::interface::InterfaceMember::Iterable(_)
        | weedle::interface::InterfaceMember::AsyncIterable(_)
        | weedle::interface::InterfaceMember::Maplike(_)
        | weedle::interface::InterfaceMember::Setlike(_)
        | weedle::interface::InterfaceMember::Stringifier(_) => Err(unsupported(
            "this interface member kind is not normalized by the first standards adapter slice",
        )),
    }
}

fn normalize_mixin_members(
    members: &[weedle::mixin::MixinMember<'_>],
) -> Result<Vec<InterfaceMember>, WebIdlError> {
    members.iter().map(normalize_mixin_member).collect()
}

fn normalize_mixin_member(
    member: &weedle::mixin::MixinMember<'_>,
) -> Result<InterfaceMember, WebIdlError> {
    match member {
        weedle::mixin::MixinMember::Attribute(value) => {
            reject_extended_attributes(&value.attributes, "mixin attributes")?;
            if value.stringifier.is_some() {
                return Err(unsupported(
                    "stringifier mixin attributes are not normalized yet",
                ));
            }
            Ok(InterfaceMember::Attribute {
                name: identifier(value.identifier.0)?,
                value_type: normalize_attributed_type(&value.type_)?,
                readonly: value.readonly.is_some(),
                static_: false,
            })
        }
        weedle::mixin::MixinMember::Operation(value) => {
            reject_extended_attributes(&value.attributes, "mixin operations")?;
            if value.stringifier.is_some() {
                return Err(unsupported(
                    "stringifier mixin operations are not normalized yet",
                ));
            }
            Ok(InterfaceMember::Operation {
                name: value
                    .identifier
                    .as_ref()
                    .map(|name| identifier(name.0))
                    .transpose()?,
                return_type: normalize_return_type(&value.return_type)?,
                arguments: normalize_arguments(&value.args.body.list)?,
                static_: false,
            })
        }
        weedle::mixin::MixinMember::Const(_) | weedle::mixin::MixinMember::Stringifier(_) => {
            Err(unsupported("this mixin member kind is not normalized yet"))
        }
    }
}

fn normalize_dictionary_members(
    members: &[weedle::dictionary::DictionaryMember<'_>],
) -> Result<Vec<DictionaryMember>, WebIdlError> {
    members
        .iter()
        .map(|member| {
            reject_extended_attributes(&member.attributes, "dictionary members")?;
            if member.default.is_some() {
                return Err(unsupported(
                    "dictionary defaults are not represented in the current Rarog IR",
                ));
            }
            Ok(DictionaryMember {
                name: identifier(member.identifier.0)?,
                value_type: normalize_type(&member.type_)?,
                required: member.required.is_some(),
            })
        })
        .collect()
}

fn normalize_arguments(
    arguments: &[weedle::argument::Argument<'_>],
) -> Result<Vec<Argument>, WebIdlError> {
    arguments.iter().map(normalize_argument).collect()
}

fn normalize_argument(argument: &weedle::argument::Argument<'_>) -> Result<Argument, WebIdlError> {
    match argument {
        weedle::argument::Argument::Single(value) => {
            reject_extended_attributes(&value.attributes, "operation arguments")?;
            if value.default.is_some() {
                return Err(unsupported(
                    "argument default values are not represented in the current Rarog IR",
                ));
            }
            Ok(Argument {
                name: identifier(value.identifier.0)?,
                value_type: normalize_attributed_type(&value.type_)?,
                optional: value.optional.is_some(),
                variadic: false,
            })
        }
        weedle::argument::Argument::Variadic(value) => {
            reject_extended_attributes(&value.attributes, "variadic operation arguments")?;
            Ok(Argument {
                name: identifier(value.identifier.0)?,
                value_type: normalize_type(&value.type_)?,
                optional: false,
                variadic: true,
            })
        }
    }
}

fn normalize_attributed_type(
    value: &weedle::types::AttributedType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    reject_extended_attributes(&value.attributes, "type uses")?;
    normalize_type(&value.type_)
}

fn normalize_type(value: &weedle::types::Type<'_>) -> Result<WebIdlType, WebIdlError> {
    match value {
        weedle::types::Type::Single(weedle::types::SingleType::Any(_)) => Ok(WebIdlType::Any),
        weedle::types::Type::Single(weedle::types::SingleType::NonAny(value)) => {
            normalize_non_any_type(value)
        }
        weedle::types::Type::Union(value) => normalize_union(value),
    }
}

fn normalize_non_any_type(
    value: &weedle::types::NonAnyType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    use weedle::types::NonAnyType;

    match value {
        NonAnyType::Promise(value) => Ok(WebIdlType::Promise(Box::new(normalize_return_type(
            value.generics.body.as_ref(),
        )?))),
        NonAnyType::Integer(value) => with_nullable(
            WebIdlType::Primitive(normalize_integer_type(&value.type_)),
            value.q_mark.is_some(),
        ),
        NonAnyType::FloatingPoint(value) => with_nullable(
            WebIdlType::Primitive(normalize_floating_point_type(&value.type_)),
            value.q_mark.is_some(),
        ),
        NonAnyType::Boolean(value) => with_nullable(
            WebIdlType::Primitive(PrimitiveType::Boolean),
            value.q_mark.is_some(),
        ),
        NonAnyType::Byte(value) => with_nullable(
            WebIdlType::Primitive(PrimitiveType::Byte),
            value.q_mark.is_some(),
        ),
        NonAnyType::Octet(value) => with_nullable(
            WebIdlType::Primitive(PrimitiveType::Octet),
            value.q_mark.is_some(),
        ),
        NonAnyType::ByteString(value) => with_nullable(
            WebIdlType::String(StringType::ByteString),
            value.q_mark.is_some(),
        ),
        NonAnyType::DOMString(value) => with_nullable(
            WebIdlType::String(StringType::DomString),
            value.q_mark.is_some(),
        ),
        NonAnyType::USVString(value) => with_nullable(
            WebIdlType::String(StringType::UsvString),
            value.q_mark.is_some(),
        ),
        NonAnyType::Sequence(value) => with_nullable(
            WebIdlType::Sequence(Box::new(normalize_type(
                value.type_.generics.body.as_ref(),
            )?)),
            value.q_mark.is_some(),
        ),
        NonAnyType::Object(value) => with_nullable(WebIdlType::Object, value.q_mark.is_some()),
        NonAnyType::Symbol(value) => with_nullable(WebIdlType::Symbol, value.q_mark.is_some()),
        NonAnyType::FrozenArrayType(value) => with_nullable(
            WebIdlType::FrozenArray(Box::new(normalize_type(
                value.type_.generics.body.as_ref(),
            )?)),
            value.q_mark.is_some(),
        ),
        NonAnyType::RecordType(value) => normalize_record(value),
        NonAnyType::Identifier(value) => with_nullable(
            WebIdlType::Named(identifier(value.type_.0)?),
            value.q_mark.is_some(),
        ),
        NonAnyType::Error(value) => normalize_named_builtin("Error", value.q_mark.is_some()),
        NonAnyType::ArrayBuffer(value) => {
            normalize_named_builtin("ArrayBuffer", value.q_mark.is_some())
        }
        NonAnyType::DataView(value) => normalize_named_builtin("DataView", value.q_mark.is_some()),
        NonAnyType::Int8Array(value) => {
            normalize_named_builtin("Int8Array", value.q_mark.is_some())
        }
        NonAnyType::Int16Array(value) => {
            normalize_named_builtin("Int16Array", value.q_mark.is_some())
        }
        NonAnyType::Int32Array(value) => {
            normalize_named_builtin("Int32Array", value.q_mark.is_some())
        }
        NonAnyType::Uint8Array(value) => {
            normalize_named_builtin("Uint8Array", value.q_mark.is_some())
        }
        NonAnyType::Uint16Array(value) => {
            normalize_named_builtin("Uint16Array", value.q_mark.is_some())
        }
        NonAnyType::Uint32Array(value) => {
            normalize_named_builtin("Uint32Array", value.q_mark.is_some())
        }
        NonAnyType::Uint8ClampedArray(value) => {
            normalize_named_builtin("Uint8ClampedArray", value.q_mark.is_some())
        }
        NonAnyType::Float32Array(value) => {
            normalize_named_builtin("Float32Array", value.q_mark.is_some())
        }
        NonAnyType::Float64Array(value) => {
            normalize_named_builtin("Float64Array", value.q_mark.is_some())
        }
        NonAnyType::ArrayBufferView(value) => {
            normalize_named_builtin("ArrayBufferView", value.q_mark.is_some())
        }
        NonAnyType::BufferSource(value) => {
            normalize_named_builtin("BufferSource", value.q_mark.is_some())
        }
    }
}

fn normalize_named_builtin(name: &str, nullable: bool) -> Result<WebIdlType, WebIdlError> {
    with_nullable(WebIdlType::Named(identifier(name)?), nullable)
}

fn normalize_integer_type(value: &weedle::types::IntegerType) -> PrimitiveType {
    match value {
        weedle::types::IntegerType::Short(value) => {
            if value.unsigned.is_some() {
                PrimitiveType::UnsignedShort
            } else {
                PrimitiveType::Short
            }
        }
        weedle::types::IntegerType::Long(value) => {
            if value.unsigned.is_some() {
                PrimitiveType::UnsignedLong
            } else {
                PrimitiveType::Long
            }
        }
        weedle::types::IntegerType::LongLong(value) => {
            if value.unsigned.is_some() {
                PrimitiveType::UnsignedLongLong
            } else {
                PrimitiveType::LongLong
            }
        }
    }
}

fn normalize_floating_point_type(value: &weedle::types::FloatingPointType) -> PrimitiveType {
    match value {
        weedle::types::FloatingPointType::Float(value) => {
            if value.unrestricted.is_some() {
                PrimitiveType::UnrestrictedFloat
            } else {
                PrimitiveType::Float
            }
        }
        weedle::types::FloatingPointType::Double(value) => {
            if value.unrestricted.is_some() {
                PrimitiveType::UnrestrictedDouble
            } else {
                PrimitiveType::Double
            }
        }
    }
}

fn normalize_return_type(value: &weedle::types::ReturnType<'_>) -> Result<WebIdlType, WebIdlError> {
    match value {
        weedle::types::ReturnType::Undefined(_) => Ok(WebIdlType::Undefined),
        weedle::types::ReturnType::Type(value) => normalize_type(value),
    }
}

fn normalize_union(
    value: &weedle::types::MayBeNull<weedle::types::UnionType<'_>>,
) -> Result<WebIdlType, WebIdlError> {
    let members = value
        .type_
        .body
        .list
        .iter()
        .map(normalize_union_member)
        .collect::<Result<Vec<_>, _>>()?;
    with_nullable(WebIdlType::Union(members), value.q_mark.is_some())
}

fn normalize_union_member(
    value: &weedle::types::UnionMemberType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    match value {
        weedle::types::UnionMemberType::Single(value) => {
            reject_extended_attributes(&value.attributes, "union member types")?;
            normalize_non_any_type(&value.type_)
        }
        weedle::types::UnionMemberType::Union(value) => normalize_union(value),
    }
}

fn normalize_record(
    value: &weedle::types::MayBeNull<weedle::types::RecordType<'_>>,
) -> Result<WebIdlType, WebIdlError> {
    let (key, _, item) = &value.type_.generics.body;
    let key = match key.as_ref() {
        weedle::types::RecordKeyType::Byte(_) => StringType::ByteString,
        weedle::types::RecordKeyType::DOM(_) => StringType::DomString,
        weedle::types::RecordKeyType::USV(_) => StringType::UsvString,
        weedle::types::RecordKeyType::NonAny(_) => {
            return Err(unsupported(
                "non-string record keys are not normalized by Rarog",
            ));
        }
    };
    with_nullable(
        WebIdlType::Record {
            key,
            value: Box::new(normalize_type(item.as_ref())?),
        },
        value.q_mark.is_some(),
    )
}

fn with_nullable(value: WebIdlType, nullable: bool) -> Result<WebIdlType, WebIdlError> {
    if nullable {
        Ok(WebIdlType::Nullable(Box::new(value)))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standards_frontend_normalizes_interface_members_and_inheritance() {
        let module = StandardsWebIdlFrontend
            .parse(
                "interface Window : EventTarget { readonly attribute DOMString name; static attribute unsigned long count; undefined close(optional DOMString reason); };",
            )
            .unwrap();

        let Definition::Interface(interface) = &module.definitions[0] else {
            panic!("expected interface definition");
        };
        assert_eq!(interface.name.as_str(), "Window");
        assert_eq!(interface.inherits.as_ref().unwrap().as_str(), "EventTarget");
        assert_eq!(interface.members.len(), 3);
        assert!(module.snapshot().contains("string:DomString"));
        assert!(module.snapshot().contains("primitive:UnsignedLong"));
    }

    #[test]
    fn standards_frontend_normalizes_dictionary_enum_typedef_and_includes() {
        let source = r#"
            dictionary Parent {};
            dictionary Options : Parent { required DOMString label; long count; };
            enum Mode { "fast", "safe" };
            typedef sequence<DOMString> Names;
            interface Target {};
            interface mixin Extra { readonly attribute boolean enabled; };
            Target includes Extra;
        "#;
        let module = StandardsWebIdlFrontend.parse(source).unwrap();

        assert_eq!(module.definitions.len(), 7);
        let snapshot = module.snapshot();
        assert!(snapshot.contains("7:Options"));
        assert!(snapshot.contains("4:fast"));
        assert!(snapshot.contains("sequence<string:DomString>"));
        assert!(snapshot.contains("includes|6:Target|5:Extra"));
    }

    #[test]
    fn standards_frontend_owns_normalized_data_after_source_is_dropped() {
        let module = {
            let source = String::from("interface Node { attribute DOMString id; };");
            StandardsWebIdlFrontend.parse(&source).unwrap()
        };

        assert!(module.snapshot().contains("4:Node"));
        assert!(module.snapshot().contains("2:id"));
    }

    #[test]
    fn standards_frontend_reports_parse_position() {
        let error = StandardsWebIdlFrontend.parse("interface {").unwrap_err();
        assert_eq!(error.kind, WebIdlErrorKind::Frontend);
        assert!(error.span.is_some());
    }

    #[test]
    fn standards_frontend_fails_closed_on_unmodeled_extended_attributes() {
        let error = StandardsWebIdlFrontend
            .parse("[Exposed=Window] interface Window {};")
            .unwrap_err();
        assert_eq!(error.kind, WebIdlErrorKind::UnsupportedDefinition);
    }

    #[test]
    fn standards_frontend_normalizes_nested_type_shapes() {
        let module = StandardsWebIdlFrontend
            .parse(
                "typedef Promise<sequence<(DOMString or unsigned long)?>>? UnsupportedOuterNullable;",
            )
            .unwrap_err();
        assert_eq!(module.kind, WebIdlErrorKind::Frontend);

        let module = StandardsWebIdlFrontend
            .parse("typedef sequence<(DOMString or unsigned long)> Values;")
            .unwrap();
        let snapshot = module.snapshot();
        assert!(snapshot.contains("sequence<union<string:DomString;primitive:UnsignedLong;>>"));
    }
}
