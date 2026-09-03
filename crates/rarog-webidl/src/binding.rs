use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use crate::{
    Definition, DictionaryDefinition, EnumDefinition, Identifier, IncludesDefinition,
    InterfaceDefinition, TypedefDefinition, WebIdlError, WebIdlErrorKind, WebIdlModule,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingMetadata {
    pub definitions: Vec<Definition>,
}

impl BindingMetadata {
    pub fn snapshot(&self) -> String {
        WebIdlModule {
            definitions: self.definitions.clone(),
            diagnostics: Vec::new(),
        }
        .snapshot()
    }
}

pub fn build_binding_metadata(module: &WebIdlModule) -> Result<BindingMetadata, WebIdlError> {
    let mut named = BTreeMap::<Identifier, NamedEntry>::new();
    let mut includes = BTreeSet::<(Identifier, Identifier)>::new();

    for definition in &module.definitions {
        match definition {
            Definition::Interface(interface) => {
                insert_interface(&mut named, interface.clone())?;
            }
            Definition::Dictionary(dictionary) => {
                insert_dictionary(&mut named, dictionary.clone())?;
            }
            Definition::Enum(enum_definition) => {
                insert_unique(
                    &mut named,
                    enum_definition.name.clone(),
                    NamedEntry::Enum(enum_definition.clone()),
                )?;
            }
            Definition::Typedef(typedef) => {
                insert_unique(
                    &mut named,
                    typedef.name.clone(),
                    NamedEntry::Typedef(typedef.clone()),
                )?;
            }
            Definition::Includes(include) => {
                let relation = (include.target.clone(), include.mixin.clone());
                if !includes.insert(relation) {
                    return Err(validation(format!(
                        "duplicate WebIDL includes relation: {} includes {}",
                        include.target, include.mixin
                    )));
                }
            }
        }
    }

    validate_includes(&named, &includes)?;

    let mut definitions = Vec::with_capacity(named.len() + includes.len());
    for (name, entry) in named {
        definitions.push(finalize_named(name, entry)?);
    }
    definitions.extend(includes.into_iter().map(|(target, mixin)| {
        Definition::Includes(IncludesDefinition { target, mixin })
    }));

    Ok(BindingMetadata { definitions })
}

#[derive(Clone, Debug)]
enum NamedEntry {
    Interface(InterfaceParts),
    Dictionary(DictionaryParts),
    Enum(EnumDefinition),
    Typedef(TypedefDefinition),
}

impl NamedEntry {
    fn kind_label(&self) -> &'static str {
        match self {
            Self::Interface(parts) if parts.mixin => "interface mixin",
            Self::Interface(_) => "interface",
            Self::Dictionary(_) => "dictionary",
            Self::Enum(_) => "enum",
            Self::Typedef(_) => "typedef",
        }
    }
}

#[derive(Clone, Debug)]
struct InterfaceParts {
    mixin: bool,
    base: Option<InterfaceDefinition>,
    partials: Vec<InterfaceDefinition>,
}

#[derive(Clone, Debug, Default)]
struct DictionaryParts {
    base: Option<DictionaryDefinition>,
    partials: Vec<DictionaryDefinition>,
}

fn insert_interface(
    named: &mut BTreeMap<Identifier, NamedEntry>,
    definition: InterfaceDefinition,
) -> Result<(), WebIdlError> {
    let name = definition.name.clone();
    match named.entry(name.clone()) {
        Entry::Vacant(entry) => {
            let mut parts = InterfaceParts {
                mixin: definition.mixin,
                base: None,
                partials: Vec::new(),
            };
            add_interface_part(&name, &mut parts, definition)?;
            entry.insert(NamedEntry::Interface(parts));
            Ok(())
        }
        Entry::Occupied(mut entry) => match entry.get_mut() {
            NamedEntry::Interface(parts) if parts.mixin == definition.mixin => {
                add_interface_part(&name, parts, definition)
            }
            existing => Err(kind_conflict(&name, existing.kind_label(), interface_kind(definition.mixin))),
        },
    }
}

fn add_interface_part(
    name: &Identifier,
    parts: &mut InterfaceParts,
    definition: InterfaceDefinition,
) -> Result<(), WebIdlError> {
    if definition.partial {
        if definition.inherits.is_some() {
            return Err(validation(format!(
                "partial WebIDL interface {name} must not declare inheritance"
            )));
        }
        parts.partials.push(definition);
        return Ok(());
    }

    if definition.mixin && definition.inherits.is_some() {
        return Err(validation(format!(
            "WebIDL interface mixin {name} must not declare inheritance"
        )));
    }
    if parts.base.replace(definition).is_some() {
        return Err(validation(format!(
            "duplicate non-partial WebIDL {} definition: {name}",
            interface_kind(parts.mixin)
        )));
    }
    Ok(())
}

fn insert_dictionary(
    named: &mut BTreeMap<Identifier, NamedEntry>,
    definition: DictionaryDefinition,
) -> Result<(), WebIdlError> {
    let name = definition.name.clone();
    match named.entry(name.clone()) {
        Entry::Vacant(entry) => {
            let mut parts = DictionaryParts::default();
            add_dictionary_part(&name, &mut parts, definition)?;
            entry.insert(NamedEntry::Dictionary(parts));
            Ok(())
        }
        Entry::Occupied(mut entry) => match entry.get_mut() {
            NamedEntry::Dictionary(parts) => add_dictionary_part(&name, parts, definition),
            existing => Err(kind_conflict(&name, existing.kind_label(), "dictionary")),
        },
    }
}

fn add_dictionary_part(
    name: &Identifier,
    parts: &mut DictionaryParts,
    definition: DictionaryDefinition,
) -> Result<(), WebIdlError> {
    if definition.partial {
        if definition.inherits.is_some() {
            return Err(validation(format!(
                "partial WebIDL dictionary {name} must not declare inheritance"
            )));
        }
        parts.partials.push(definition);
        return Ok(());
    }

    if parts.base.replace(definition).is_some() {
        return Err(validation(format!(
            "duplicate non-partial WebIDL dictionary definition: {name}"
        )));
    }
    Ok(())
}

fn insert_unique(
    named: &mut BTreeMap<Identifier, NamedEntry>,
    name: Identifier,
    definition: NamedEntry,
) -> Result<(), WebIdlError> {
    match named.entry(name.clone()) {
        Entry::Vacant(entry) => {
            entry.insert(definition);
            Ok(())
        }
        Entry::Occupied(entry) => Err(kind_conflict(
            &name,
            entry.get().kind_label(),
            definition.kind_label(),
        )),
    }
}

fn validate_includes(
    named: &BTreeMap<Identifier, NamedEntry>,
    includes: &BTreeSet<(Identifier, Identifier)>,
) -> Result<(), WebIdlError> {
    for (target, mixin) in includes {
        match named.get(target) {
            Some(NamedEntry::Interface(parts)) if !parts.mixin => {}
            Some(entry) => {
                return Err(validation(format!(
                    "WebIDL includes target {target} is {}, not an interface",
                    entry.kind_label()
                )));
            }
            None => {
                return Err(validation(format!(
                    "WebIDL includes target {target} has no interface definition"
                )));
            }
        }

        match named.get(mixin) {
            Some(NamedEntry::Interface(parts)) if parts.mixin => {}
            Some(entry) => {
                return Err(validation(format!(
                    "WebIDL includes source {mixin} is {}, not an interface mixin",
                    entry.kind_label()
                )));
            }
            None => {
                return Err(validation(format!(
                    "WebIDL includes source {mixin} has no interface mixin definition"
                )));
            }
        }
    }
    Ok(())
}

fn finalize_named(name: Identifier, entry: NamedEntry) -> Result<Definition, WebIdlError> {
    match entry {
        NamedEntry::Interface(mut parts) => {
            let mut base = parts.base.ok_or_else(|| {
                validation(format!(
                    "partial WebIDL {} {name} has no non-partial definition",
                    interface_kind(parts.mixin)
                ))
            })?;
            for partial in parts.partials.drain(..) {
                base.members.extend(partial.members);
            }
            base.partial = false;
            Ok(Definition::Interface(base))
        }
        NamedEntry::Dictionary(mut parts) => {
            let mut base = parts.base.ok_or_else(|| {
                validation(format!(
                    "partial WebIDL dictionary {name} has no non-partial definition"
                ))
            })?;
            for partial in parts.partials.drain(..) {
                base.members.extend(partial.members);
            }
            base.partial = false;
            Ok(Definition::Dictionary(base))
        }
        NamedEntry::Enum(enum_definition) => Ok(Definition::Enum(enum_definition)),
        NamedEntry::Typedef(typedef) => Ok(Definition::Typedef(typedef)),
    }
}

fn interface_kind(mixin: bool) -> &'static str {
    if mixin { "interface mixin" } else { "interface" }
}

fn kind_conflict(name: &Identifier, existing: &str, incoming: &str) -> WebIdlError {
    validation(format!(
        "conflicting WebIDL definitions for {name}: {existing} and {incoming}"
    ))
}

fn validation(message: impl Into<String>) -> WebIdlError {
    WebIdlError::new(WebIdlErrorKind::Validation, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StandardsWebIdlFrontend, parse_with};

    fn parse(source: &str) -> WebIdlModule {
        parse_with(&StandardsWebIdlFrontend, source).unwrap()
    }

    #[test]
    fn metadata_order_is_independent_of_top_level_definition_order() {
        let first = build_binding_metadata(&parse(
            "typedef unsigned long Zebra; interface Beta {}; interface Alpha {};",
        ))
        .unwrap();
        let second = build_binding_metadata(&parse(
            "interface Alpha {}; typedef unsigned long Zebra; interface Beta {};",
        ))
        .unwrap();

        assert_eq!(first, second);
        let snapshot = first.snapshot();
        assert!(snapshot.find("5:Alpha").unwrap() < snapshot.find("4:Beta").unwrap());
        assert!(snapshot.find("4:Beta").unwrap() < snapshot.find("5:Zebra").unwrap());
    }

    #[test]
    fn metadata_merges_partial_definitions_and_orders_includes() {
        let module = parse(
            r#"
                partial interface Example { readonly attribute DOMString extra; };
                interface mixin Extra { attribute boolean enabled; };
                interface Example { attribute long base; };
                Example includes Extra;
                partial dictionary Options { DOMString label; };
                dictionary Options { required boolean enabled; };
            "#,
        );

        let metadata = build_binding_metadata(&module).unwrap();
        assert_eq!(metadata.definitions.len(), 4);
        let snapshot = metadata.snapshot();
        assert!(snapshot.contains("7:Example|-|false|false"));
        assert!(snapshot.contains("5:extra|string:DomString|true|false"));
        assert!(snapshot.contains("4:base|primitive:Long|false|false"));
        assert!(snapshot.contains("7:Options|-|false"));
        assert!(snapshot.contains("5:label|string:DomString|false"));
        assert!(snapshot.contains("includes|7:Example|5:Extra"));
    }

    #[test]
    fn metadata_rejects_duplicate_non_partial_definitions() {
        let error = build_binding_metadata(&parse(
            "interface Example {}; interface Example {};",
        ))
        .unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::Validation);
        assert!(error.message.contains("duplicate non-partial"));
    }

    #[test]
    fn metadata_rejects_orphan_partials() {
        let error = build_binding_metadata(&parse("partial interface Example {};"))
            .unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::Validation);
        assert!(error.message.contains("has no non-partial definition"));
    }

    #[test]
    fn metadata_rejects_conflicting_named_definition_kinds() {
        let error = build_binding_metadata(&parse(
            "interface Example {}; typedef unsigned long Example;",
        ))
        .unwrap_err();

        assert_eq!(error.kind, WebIdlErrorKind::Validation);
        assert!(error.message.contains("conflicting WebIDL definitions"));
    }

    #[test]
    fn metadata_rejects_invalid_includes_relations() {
        let missing_target = build_binding_metadata(&parse(
            "interface mixin Extra {}; Missing includes Extra;",
        ))
        .unwrap_err();
        assert_eq!(missing_target.kind, WebIdlErrorKind::Validation);
        assert!(missing_target.message.contains("no interface definition"));

        let non_mixin = build_binding_metadata(&parse(
            "interface Example {}; interface Extra {}; Example includes Extra;",
        ))
        .unwrap_err();
        assert_eq!(non_mixin.kind, WebIdlErrorKind::Validation);
        assert!(non_mixin.message.contains("not an interface mixin"));
    }
}
