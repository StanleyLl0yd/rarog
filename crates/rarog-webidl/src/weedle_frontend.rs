use weedle::Parse;

use crate::{
    Argument, Definition, DictionaryDefinition, DictionaryMember, EnumDefinition, Identifier,
    IncludesDefinition, InterfaceDefinition, InterfaceMember, PrimitiveType, SourceSpan,
    StringType, TypedefDefinition, WebIdlError, WebIdlErrorKind, WebIdlFrontend, WebIdlModule,
    WebIdlType,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct StandardsWebIdlFrontend;

impl WebIdlFrontend for StandardsWebIdlFrontend {
    fn parse(&self, source: &str) -> Result<WebIdlModule, WebIdlError> {
        let (remaining, definitions) =
            weedle::Definitions::parse(source).map_err(|error| parser_error(source, error))?;
        if !remaining.trim().is_empty() {
            let offset = source.len().saturating_sub(remaining.len());
            return Err(WebIdlError::new(
                WebIdlErrorKind::Frontend,
                "WebIDL parser left unconsumed input",
            )
            .with_span(point_span(source, offset)));
        }

        let mut normalized = Vec::with_capacity(definitions.len());
        for definition in definitions {
            normalized.push(normalize_definition(definition)?);
        }

        Ok(WebIdlModule {
            definitions: normalized,
            diagnostics: Vec::new(),
        })
    }
}

fn parser_error(source: &str, error: weedle::Err<weedle::Error<&str>>) -> WebIdlError {
    let offset = match error {
        weedle::Err::Error(error) | weedle::Err::Failure(error) => {
            source.len().saturating_sub(error.input.len())
        }
        weedle::Err::Incomplete(_) => source.len(),
    };
    WebIdlError::new(WebIdlErrorKind::Frontend, "failed to parse WebIDL")
        .with_span(point_span(source, offset))
}

fn point_span(source: &str, offset: usize) -> SourceSpan {
    SourceSpan::new(offset, offset.saturating_add(1).min(source.len()))
}

fn normalize_definition(definition: weedle::Definition<'_>) -> Result<Definition, WebIdlError> {
    match definition {
        weedle::Definition::Interface(definition) => {
            reject_attributes(definition.attributes.is_some(), "interface")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: definition
                    .inheritance
                    .map(|inheritance| identifier(inheritance.identifier.0))
                    .transpose()?,
                members: normalize_interface_members(definition.members.body)?,
                partial: false,
                mixin: false,
            }))
        }
        weedle::Definition::PartialInterface(definition) => {
            reject_attributes(definition.attributes.is_some(), "partial interface")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: None,
                members: normalize_interface_members(definition.members.body)?,
                partial: true,
                mixin: false,
            }))
        }
        weedle::Definition::InterfaceMixin(definition) => {
            reject_attributes(definition.attributes.is_some(), "interface mixin")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: None,
                members: normalize_mixin_members(definition.members.body)?,
                partial: false,
                mixin: true,
            }))
        }
        weedle::Definition::PartialInterfaceMixin(definition) => {
            reject_attributes(definition.attributes.is_some(), "partial interface mixin")?;
            Ok(Definition::Interface(InterfaceDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: None,
                members: normalize_mixin_members(definition.members.body)?,
                partial: true,
                mixin: true,
            }))
        }
        weedle::Definition::Dictionary(definition) => {
            reject_attributes(definition.attributes.is_some(), "dictionary")?;
            Ok(Definition::Dictionary(DictionaryDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: definition
                    .inheritance
                    .map(|inheritance| identifier(inheritance.identifier.0))
                    .transpose()?,
                members: normalize_dictionary_members(definition.members.body)?,
                partial: false,
            }))
        }
        weedle::Definition::PartialDictionary(definition) => {
            reject_attributes(definition.attributes.is_some(), "partial dictionary")?;
            Ok(Definition::Dictionary(DictionaryDefinition {
                name: identifier(definition.identifier.0)?,
                inherits: None,
                members: normalize_dictionary_members(definition.members.body)?,
                partial: true,
            }))
        }
        weedle::Definition::Enum(definition) => {
            reject_attributes(definition.attributes.is_some(), "enum")?;
            Ok(Definition::Enum(EnumDefinition {
                name: identifier(definition.identifier.0)?,
                values: definition
                    .values
                    .body
                    .list
                    .into_iter()
                    .map(|variant| variant.value.0.to_owned())
                    .collect(),
            }))
        }
        weedle::Definition::Typedef(definition) => {
            reject_attributes(definition.attributes.is_some(), "typedef")?;
            Ok(Definition::Typedef(TypedefDefinition {
                name: identifier(definition.identifier.0)?,
                value_type: normalize_attributed_type(definition.type_)?,
            }))
        }
        weedle::Definition::IncludesStatement(definition) => {
            reject_attributes(definition.attributes.is_some(), "includes statement")?;
            Ok(Definition::Includes(IncludesDefinition {
                target: identifier(definition.lhs_identifier.0)?,
                mixin: identifier(definition.rhs_identifier.0)?,
            }))
        }
        weedle::Definition::Callback(_) => unsupported("callback definitions"),
        weedle::Definition::CallbackInterface(_) => unsupported("callback interfaces"),
        weedle::Definition::Namespace(_) | weedle::Definition::PartialNamespace(_) => {
            unsupported("namespace definitions")
        }
        weedle::Definition::Implements(_) => unsupported("legacy implements statements"),
    }
}

fn normalize_interface_members(
    members: weedle::interface::InterfaceMembers<'_>,
) -> Result<Vec<InterfaceMember>, WebIdlError> {
    members
        .into_iter()
        .map(|member| match member {
            weedle::interface::InterfaceMember::Attribute(member) => {
                reject_attributes(member.attributes.is_some(), "interface attribute")?;
                let static_ = match member.modifier {
                    None => false,
                    Some(weedle::interface::StringifierOrInheritOrStatic::Static(_)) => true,
                    Some(_) => return unsupported("stringifier/inherit attributes"),
                };
                Ok(InterfaceMember::Attribute {
                    name: identifier(member.identifier.0)?,
                    value_type: normalize_attributed_type(member.type_)?,
                    readonly: member.readonly.is_some(),
                    static_,
                })
            }
            weedle::interface::InterfaceMember::Operation(member) => {
                reject_attributes(member.attributes.is_some(), "interface operation")?;
                if member.special.is_some() {
                    return unsupported("special getter/setter/deleter operations");
                }
                let static_ = match member.modifier {
                    None => false,
                    Some(weedle::interface::StringifierOrStatic::Static(_)) => true,
                    Some(_) => return unsupported("stringifier operations"),
                };
                Ok(InterfaceMember::Operation {
                    name: member
                        .identifier
                        .map(|identifier_value| identifier(identifier_value.0))
                        .transpose()?,
                    return_type: normalize_return_type(member.return_type)?,
                    arguments: normalize_arguments(member.args.body.list)?,
                    static_,
                })
            }
            weedle::interface::InterfaceMember::Const(_) => unsupported("interface constants"),
            weedle::interface::InterfaceMember::Constructor(_) => {
                unsupported("interface constructors")
            }
            weedle::interface::InterfaceMember::Iterable(_)
            | weedle::interface::InterfaceMember::AsyncIterable(_)
            | weedle::interface::InterfaceMember::Maplike(_)
            | weedle::interface::InterfaceMember::Setlike(_) => {
                unsupported("collection interface members")
            }
            weedle::interface::InterfaceMember::Stringifier(_) => {
                unsupported("standalone stringifiers")
            }
        })
        .collect()
}

fn normalize_mixin_members(
    members: weedle::mixin::MixinMembers<'_>,
) -> Result<Vec<InterfaceMember>, WebIdlError> {
    members
        .into_iter()
        .map(|member| match member {
            weedle::mixin::MixinMember::Attribute(member) => {
                reject_attributes(member.attributes.is_some(), "mixin attribute")?;
                if member.stringifier.is_some() {
                    return unsupported("stringifier mixin attributes");
                }
                Ok(InterfaceMember::Attribute {
                    name: identifier(member.identifier.0)?,
                    value_type: normalize_attributed_type(member.type_)?,
                    readonly: member.readonly.is_some(),
                    static_: false,
                })
            }
            weedle::mixin::MixinMember::Operation(member) => {
                reject_attributes(member.attributes.is_some(), "mixin operation")?;
                if member.stringifier.is_some() {
                    return unsupported("stringifier mixin operations");
                }
                Ok(InterfaceMember::Operation {
                    name: member
                        .identifier
                        .map(|identifier_value| identifier(identifier_value.0))
                        .transpose()?,
                    return_type: normalize_return_type(member.return_type)?,
                    arguments: normalize_arguments(member.args.body.list)?,
                    static_: false,
                })
            }
            weedle::mixin::MixinMember::Const(_) => unsupported("mixin constants"),
            weedle::mixin::MixinMember::Stringifier(_) => unsupported("mixin stringifiers"),
        })
        .collect()
}

fn normalize_dictionary_members(
    members: weedle::dictionary::DictionaryMembers<'_>,
) -> Result<Vec<DictionaryMember>, WebIdlError> {
    members
        .into_iter()
        .map(|member| {
            reject_attributes(member.attributes.is_some(), "dictionary member")?;
            if member.default.is_some() {
                return unsupported("dictionary member default values");
            }
            Ok(DictionaryMember {
                name: identifier(member.identifier.0)?,
                value_type: normalize_type(member.type_)?,
                required: member.required.is_some(),
            })
        })
        .collect()
}

fn normalize_arguments(
    arguments: Vec<weedle::argument::Argument<'_>>,
) -> Result<Vec<Argument>, WebIdlError> {
    arguments
        .into_iter()
        .map(|argument| match argument {
            weedle::argument::Argument::Single(argument) => {
                reject_attributes(argument.attributes.is_some(), "operation argument")?;
                if argument.default.is_some() {
                    return unsupported("operation argument default values");
                }
                Ok(Argument {
                    name: identifier(argument.identifier.0)?,
                    value_type: normalize_attributed_type(argument.type_)?,
                    optional: argument.optional.is_some(),
                    variadic: false,
                })
            }
            weedle::argument::Argument::Variadic(argument) => {
                reject_attributes(argument.attributes.is_some(), "variadic operation argument")?;
                Ok(Argument {
                    name: identifier(argument.identifier.0)?,
                    value_type: normalize_type(argument.type_)?,
                    optional: false,
                    variadic: true,
                })
            }
        })
        .collect()
}

fn normalize_attributed_type(
    value_type: weedle::types::AttributedType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    reject_attributes(value_type.attributes.is_some(), "type extended attributes")?;
    normalize_type(value_type.type_)
}

fn normalize_return_type(
    return_type: weedle::types::ReturnType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    match return_type {
        weedle::types::ReturnType::Undefined(_) => Ok(WebIdlType::Undefined),
        weedle::types::ReturnType::Type(value_type) => normalize_type(value_type),
    }
}

fn normalize_type(value_type: weedle::types::Type<'_>) -> Result<WebIdlType, WebIdlError> {
    match value_type {
        weedle::types::Type::Single(weedle::types::SingleType::Any(_)) => Ok(WebIdlType::Any),
        weedle::types::Type::Single(weedle::types::SingleType::NonAny(value_type)) => {
            normalize_non_any_type(value_type)
        }
        weedle::types::Type::Union(union) => {
            let members = union
                .type_
                .body
                .list
                .into_iter()
                .map(normalize_union_member)
                .collect::<Result<Vec<_>, _>>()?;
            nullable(WebIdlType::Union(members), union.q_mark.is_some())
        }
    }
}

fn normalize_union_member(
    member: weedle::types::UnionMemberType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    match member {
        weedle::types::UnionMemberType::Single(member) => {
            reject_attributes(member.attributes.is_some(), "union member type attributes")?;
            normalize_non_any_type(member.type_)
        }
        weedle::types::UnionMemberType::Union(union) => {
            let members = union
                .type_
                .body
                .list
                .into_iter()
                .map(normalize_union_member)
                .collect::<Result<Vec<_>, _>>()?;
            nullable(WebIdlType::Union(members), union.q_mark.is_some())
        }
    }
}

fn normalize_non_any_type(
    value_type: weedle::types::NonAnyType<'_>,
) -> Result<WebIdlType, WebIdlError> {
    use weedle::types::NonAnyType;

    match value_type {
        NonAnyType::Promise(value) => Ok(WebIdlType::Promise(Box::new(normalize_return_type(
            *value.generics.body,
        )?))),
        NonAnyType::Integer(value) => {
            let primitive = match value.type_ {
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
            };
            nullable(WebIdlType::Primitive(primitive), value.q_mark.is_some())
        }
        NonAnyType::FloatingPoint(value) => {
            let primitive = match value.type_ {
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
            };
            nullable(WebIdlType::Primitive(primitive), value.q_mark.is_some())
        }
        NonAnyType::Boolean(value) => nullable(
            WebIdlType::Primitive(PrimitiveType::Boolean),
            value.q_mark.is_some(),
        ),
        NonAnyType::Byte(value) => nullable(
            WebIdlType::Primitive(PrimitiveType::Byte),
            value.q_mark.is_some(),
        ),
        NonAnyType::Octet(value) => nullable(
            WebIdlType::Primitive(PrimitiveType::Octet),
            value.q_mark.is_some(),
        ),
        NonAnyType::ByteString(value) => nullable(
            WebIdlType::String(StringType::ByteString),
            value.q_mark.is_some(),
        ),
        NonAnyType::DOMString(value) => nullable(
            WebIdlType::String(StringType::DomString),
            value.q_mark.is_some(),
        ),
        NonAnyType::USVString(value) => nullable(
            WebIdlType::String(StringType::UsvString),
            value.q_mark.is_some(),
        ),
        NonAnyType::Sequence(value) => nullable(
            WebIdlType::Sequence(Box::new(normalize_type(*value.type_.generics.body)?)),
            value.q_mark.is_some(),
        ),
        NonAnyType::Object(value) => nullable(WebIdlType::Object, value.q_mark.is_some()),
        NonAnyType::Symbol(value) => nullable(WebIdlType::Symbol, value.q_mark.is_some()),
        NonAnyType::Error(value) => named_nullable("Error", value.q_mark.is_some()),
        NonAnyType::ArrayBuffer(value) => named_nullable("ArrayBuffer", value.q_mark.is_some()),
        NonAnyType::DataView(value) => named_nullable("DataView", value.q_mark.is_some()),
        NonAnyType::Int8Array(value) => named_nullable("Int8Array", value.q_mark.is_some()),
        NonAnyType::Int16Array(value) => named_nullable("Int16Array", value.q_mark.is_some()),
        NonAnyType::Int32Array(value) => named_nullable("Int32Array", value.q_mark.is_some()),
        NonAnyType::Uint8Array(value) => named_nullable("Uint8Array", value.q_mark.is_some()),
        NonAnyType::Uint16Array(value) => named_nullable("Uint16Array", value.q_mark.is_some()),
        NonAnyType::Uint32Array(value) => named_nullable("Uint32Array", value.q_mark.is_some()),
        NonAnyType::Uint8ClampedArray(value) => {
            named_nullable("Uint8ClampedArray", value.q_mark.is_some())
        }
        NonAnyType::Float32Array(value) => named_nullable("Float32Array", value.q_mark.is_some()),
        NonAnyType::Float64Array(value) => named_nullable("Float64Array", value.q_mark.is_some()),
        NonAnyType::ArrayBufferView(value) => {
            named_nullable("ArrayBufferView", value.q_mark.is_some())
        }
        NonAnyType::BufferSource(value) => named_nullable("BufferSource", value.q_mark.is_some()),
        NonAnyType::FrozenArrayType(value) => nullable(
            WebIdlType::FrozenArray(Box::new(normalize_type(*value.type_.generics.body)?)),
            value.q_mark.is_some(),
        ),
        NonAnyType::RecordType(value) => {
            let (key, _, value_type) = value.type_.generics.body;
            let key = match *key {
                weedle::types::RecordKeyType::Byte(_) => StringType::ByteString,
                weedle::types::RecordKeyType::DOM(_) => StringType::DomString,
                weedle::types::RecordKeyType::USV(_) => StringType::UsvString,
                weedle::types::RecordKeyType::NonAny(_) => {
                    return unsupported("non-string record key types");
                }
            };
            nullable(
                WebIdlType::Record {
                    key,
                    value: Box::new(normalize_type(*value_type)?),
                },
                value.q_mark.is_some(),
            )
        }
        NonAnyType::Identifier(value) => {
            let named = WebIdlType::Named(identifier(value.type_.0)?);
            nullable(named, value.q_mark.is_some())
        }
    }
}

fn named_nullable(name: &str, is_nullable: bool) -> Result<WebIdlType, WebIdlError> {
    nullable(WebIdlType::Named(identifier(name)?), is_nullable)
}

fn nullable(value_type: WebIdlType, is_nullable: bool) -> Result<WebIdlType, WebIdlError> {
    if is_nullable {
        Ok(WebIdlType::Nullable(Box::new(value_type)))
    } else {
        Ok(value_type)
    }
}

fn identifier(value: &str) -> Result<Identifier, WebIdlError> {
    Identifier::new(value)
}

fn reject_attributes(present: bool, context: &str) -> Result<(), WebIdlError> {
    if present {
        unsupported(&format!("extended attributes on {context}"))
    } else {
        Ok(())
    }
}

fn unsupported<T>(construct: &str) -> Result<T, WebIdlError> {
    Err(WebIdlError::new(
        WebIdlErrorKind::UnsupportedDefinition,
        format!("unsupported WebIDL construct: {construct}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_with;

    #[test]
    fn normalizes_supported_definitions_into_owned_ir() {
        let source = r#"
            interface mixin WindowExtras {
                readonly attribute DOMString title;
            };
            interface Window : EventTarget {
                attribute unsigned long count;
                static undefined reset(optional DOMString reason, long... values);
            };
            partial interface Window {
                readonly attribute boolean ready;
            };
            dictionary Options : BaseOptions {
                required boolean enabled;
                DOMString label;
            };
            partial dictionary Options {
                unsigned long retryCount;
            };
            enum Mode { "fast", "safe" };
            typedef (DOMString or long)? MaybeValue;
            Window includes WindowExtras;
        "#;

        let module = parse_with(&StandardsWebIdlFrontend, source).unwrap();
        let snapshot = module.snapshot();

        assert_eq!(module.definitions.len(), 8);
        assert!(snapshot.contains("interface|12:WindowExtras|-|false|true"));
        assert!(snapshot.contains("interface|6:Window|11:EventTarget|false|false"));
        assert!(snapshot.contains("operation||5:reset|undefined|true"));
        assert!(snapshot.contains("dictionary|7:Options|11:BaseOptions|false"));
        assert!(snapshot.contains("enum|4:Mode|4:fast4:safe"));
        assert!(snapshot.contains("nullable<union<string:DomString;primitive:Long;>>"));
        assert!(snapshot.contains("includes|6:Window|12:WindowExtras"));
    }

    #[test]
    fn rejects_constructs_missing_from_the_normalized_ir() {
        let error = parse_with(
            &StandardsWebIdlFrontend,
            "interface Example { const long value = 1; };",
        )
        .unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::UnsupportedDefinition);
        assert!(error.message.contains("interface constants"));
    }

    #[test]
    fn rejects_extended_attributes_instead_of_dropping_them() {
        let error = parse_with(
            &StandardsWebIdlFrontend,
            "[Exposed=Window] interface Example {};",
        )
        .unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::UnsupportedDefinition);
        assert!(error.message.contains("extended attributes"));
    }

    #[test]
    fn malformed_input_returns_a_frontend_error_with_a_span() {
        let error = parse_with(&StandardsWebIdlFrontend, "interface Example {").unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::Frontend);
        assert!(error.span.is_some());
    }

    #[test]
    fn parser_does_not_expose_borrowed_vendor_ast_data() {
        let module = {
            let source = String::from("typedef unsigned long Count;");
            parse_with(&StandardsWebIdlFrontend, &source).unwrap()
        };

        assert_eq!(module.definitions.len(), 1);
        assert!(module.snapshot().contains("5:Count"));
    }
}
