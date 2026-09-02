use crate::{SelectorDependency, SelectorDependencyScope, SelectorInvalidationKey, Specificity};
use cssparser::{BasicParseErrorKind, ParseError, Parser, ParserInput, Token};
use rarog_dom::{Document, NodeId, NodeKind};
use std::collections::BTreeSet;

const MAX_SELECTOR_COMPOUNDS: usize = 64;
const MAX_SIMPLE_SELECTORS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeSelector {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoClass {
    Root,
    FirstChild,
    LastChild,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompoundSelector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: Vec<AttributeSelector>,
    pub pseudo_classes: Vec<PseudoClass>,
    universal: bool,
}

impl CompoundSelector {
    fn is_empty(&self) -> bool {
        !self.universal
            && self.tag.is_none()
            && self.id.is_none()
            && self.classes.is_empty()
            && self.attributes.is_empty()
            && self.pseudo_classes.is_empty()
    }

    fn simple_count(&self) -> usize {
        usize::from(self.universal || self.tag.is_some())
            + usize::from(self.id.is_some())
            + self.classes.len()
            + self.attributes.len()
            + self.pseudo_classes.len()
    }

    fn invalidation_key(&self) -> SelectorInvalidationKey {
        SelectorInvalidationKey {
            tag: self.tag.clone(),
            id: self.id.clone(),
            classes: self.classes.iter().cloned().collect(),
            attributes: self
                .attributes
                .iter()
                .map(|attribute| attribute.name.clone())
                .collect(),
        }
    }

    fn matches(&self, document: &Document, node: NodeId) -> bool {
        let Some(node_data) = document.node(node) else {
            return false;
        };
        let NodeKind::Element(element) = &node_data.kind else {
            return false;
        };

        if let Some(tag) = &self.tag {
            if element.tag_name.as_str() != tag {
                return false;
            }
        }
        if let Some(id) = &self.id {
            if element.attributes.get("id") != Some(id) {
                return false;
            }
        }
        if !self.classes.is_empty() {
            let element_classes = element
                .attributes
                .get("class")
                .map(|value| value.split_whitespace().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            if self
                .classes
                .iter()
                .any(|class| !element_classes.contains(class.as_str()))
            {
                return false;
            }
        }
        for attribute in &self.attributes {
            let Some(value) = element.attributes.get(&attribute.name) else {
                return false;
            };
            if attribute
                .value
                .as_ref()
                .is_some_and(|expected| value != expected)
            {
                return false;
            }
        }
        for pseudo_class in &self.pseudo_classes {
            if !matches_pseudo_class(document, node, *pseudo_class) {
                return false;
            }
        }
        true
    }

    fn snapshot(&self) -> String {
        let mut output = self.tag.clone().unwrap_or_else(|| "*".into());
        if let Some(id) = &self.id {
            output.push('#');
            output.push_str(id);
        }
        for class in &self.classes {
            output.push('.');
            output.push_str(class);
        }
        for attribute in &self.attributes {
            output.push('[');
            output.push_str(&attribute.name);
            if let Some(value) = &attribute.value {
                output.push_str("=\"");
                output.push_str(&value.replace('"', "\\\""));
                output.push('"');
            }
            output.push(']');
        }
        for pseudo_class in &self.pseudo_classes {
            output.push(':');
            output.push_str(match pseudo_class {
                PseudoClass::Root => "root",
                PseudoClass::FirstChild => "first-child",
                PseudoClass::LastChild => "last-child",
            });
        }
        output
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    pub compounds: Vec<CompoundSelector>,
    pub combinators: Vec<Combinator>,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let mut specificity = Specificity::default();
        for compound in &self.compounds {
            specificity.ids = specificity
                .ids
                .saturating_add(u16::from(compound.id.is_some()));
            let class_like = compound
                .classes
                .len()
                .saturating_add(compound.attributes.len())
                .saturating_add(compound.pseudo_classes.len())
                .min(u16::MAX as usize) as u16;
            specificity.classes = specificity.classes.saturating_add(class_like);
            specificity.types = specificity
                .types
                .saturating_add(u16::from(compound.tag.is_some()));
        }
        specificity
    }

    pub fn matches(&self, document: &Document, node: NodeId) -> bool {
        let Some(last) = self.compounds.len().checked_sub(1) else {
            return false;
        };
        self.matches_from(document, last, node)
    }

    fn matches_from(&self, document: &Document, part: usize, node: NodeId) -> bool {
        if !self.compounds[part].matches(document, node) {
            return false;
        }
        if part == 0 {
            return true;
        }

        match self.combinators[part - 1] {
            Combinator::Child => document
                .node(node)
                .and_then(|node| node.parent)
                .is_some_and(|parent| self.matches_from(document, part - 1, parent)),
            Combinator::Descendant => {
                let mut ancestor = document.node(node).and_then(|node| node.parent);
                while let Some(current) = ancestor {
                    if self.matches_from(document, part - 1, current) {
                        return true;
                    }
                    ancestor = document.node(current).and_then(|node| node.parent);
                }
                false
            }
            Combinator::NextSibling => previous_element_sibling(document, node)
                .is_some_and(|sibling| self.matches_from(document, part - 1, sibling)),
            Combinator::SubsequentSibling => preceding_element_siblings(document, node)
                .any(|sibling| self.matches_from(document, part - 1, sibling)),
        }
    }

    pub fn invalidation_key(&self) -> SelectorInvalidationKey {
        self.compounds
            .last()
            .map(CompoundSelector::invalidation_key)
            .unwrap_or_default()
    }

    pub(crate) fn dependencies(&self) -> Vec<SelectorDependency> {
        let mut dependencies = Vec::new();
        let last = self.compounds.len().saturating_sub(1);

        for (index, combinator) in self.combinators.iter().enumerate() {
            let scope = match combinator {
                Combinator::Descendant | Combinator::Child => SelectorDependencyScope::Descendants,
                Combinator::NextSibling | Combinator::SubsequentSibling => {
                    SelectorDependencyScope::FollowingSiblings
                }
            };
            dependencies.push(SelectorDependency {
                trigger: self.compounds[index].invalidation_key(),
                scope,
            });
        }

        if let Some(target) = self.compounds.get(last) {
            if !target.attributes.is_empty() {
                dependencies.push(SelectorDependency {
                    trigger: target.invalidation_key(),
                    scope: SelectorDependencyScope::SelfNode,
                });
            }
        }

        let has_contextual_pseudo = self
            .compounds
            .iter()
            .any(|compound| !compound.pseudo_classes.is_empty());
        if has_contextual_pseudo {
            dependencies.push(SelectorDependency {
                trigger: SelectorInvalidationKey::default(),
                scope: SelectorDependencyScope::SelfNode,
            });
        }
        let has_sibling_position_pseudo = self.compounds.iter().any(|compound| {
            compound
                .pseudo_classes
                .iter()
                .any(|pseudo| matches!(pseudo, PseudoClass::FirstChild | PseudoClass::LastChild))
        });
        if has_sibling_position_pseudo {
            dependencies.push(SelectorDependency {
                trigger: SelectorInvalidationKey::default(),
                scope: SelectorDependencyScope::SiblingSet,
            });
        }

        dependencies.sort_by(|left, right| {
            left.scope
                .cmp(&right.scope)
                .then_with(|| left.trigger.tag.cmp(&right.trigger.tag))
                .then_with(|| left.trigger.id.cmp(&right.trigger.id))
                .then_with(|| left.trigger.classes.cmp(&right.trigger.classes))
                .then_with(|| left.trigger.attributes.cmp(&right.trigger.attributes))
        });
        dependencies.dedup();
        dependencies
    }

    pub(crate) fn snapshot(&self) -> String {
        let mut output = String::new();
        for (index, compound) in self.compounds.iter().enumerate() {
            if index > 0 {
                output.push_str(match self.combinators[index - 1] {
                    Combinator::Descendant => " ",
                    Combinator::Child => " > ",
                    Combinator::NextSibling => " + ",
                    Combinator::SubsequentSibling => " ~ ",
                });
            }
            output.push_str(&compound.snapshot());
        }
        output
    }
}

pub fn parse_selector(input: &str) -> Option<Selector> {
    let mut input_state = ParserInput::new(input);
    let mut input = Parser::new(&mut input_state);
    parse_selector_input(&mut input).ok()
}

fn parse_selector_input<'i>(input: &mut Parser<'i, '_>) -> Result<Selector, ParseError<'i, ()>> {
    let mut compounds = Vec::new();
    let mut combinators = Vec::new();
    let mut current = CompoundSelector::default();
    let mut pending_whitespace = false;

    loop {
        let token = match input.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(error) if matches!(error.kind, BasicParseErrorKind::EndOfInput) => break,
            Err(error) => return Err(error.into()),
        };

        match token {
            Token::Comment(_) => continue,
            Token::WhiteSpace(_) => {
                if !current.is_empty() {
                    pending_whitespace = true;
                }
                continue;
            }
            Token::Delim('>') | Token::Delim('+') | Token::Delim('~') => {
                if current.is_empty() {
                    return Err(input.new_custom_error(()));
                }
                push_compound(input, &mut compounds, std::mem::take(&mut current))?;
                combinators.push(match token {
                    Token::Delim('>') => Combinator::Child,
                    Token::Delim('+') => Combinator::NextSibling,
                    Token::Delim('~') => Combinator::SubsequentSibling,
                    _ => unreachable!(),
                });
                pending_whitespace = false;
            }
            token => {
                if pending_whitespace && !current.is_empty() {
                    push_compound(input, &mut compounds, std::mem::take(&mut current))?;
                    combinators.push(Combinator::Descendant);
                }
                pending_whitespace = false;
                parse_simple_selector(input, token, &mut current)?;
                if current.simple_count() > MAX_SIMPLE_SELECTORS {
                    return Err(input.new_custom_error(()));
                }
            }
        }
    }

    if current.is_empty() {
        if compounds.is_empty() || combinators.len() >= compounds.len() {
            return Err(input.new_custom_error(()));
        }
    } else {
        push_compound(input, &mut compounds, current)?;
    }

    if compounds.len() != combinators.len().saturating_add(1) {
        return Err(input.new_custom_error(()));
    }

    Ok(Selector {
        compounds,
        combinators,
    })
}

fn push_compound<'i>(
    input: &Parser<'i, '_>,
    compounds: &mut Vec<CompoundSelector>,
    compound: CompoundSelector,
) -> Result<(), ParseError<'i, ()>> {
    if compound.is_empty() || compounds.len() >= MAX_SELECTOR_COMPOUNDS {
        return Err(input.new_custom_error(()));
    }
    compounds.push(compound);
    Ok(())
}

fn parse_simple_selector<'i>(
    input: &mut Parser<'i, '_>,
    token: Token<'i>,
    compound: &mut CompoundSelector,
) -> Result<(), ParseError<'i, ()>> {
    match token {
        Token::Ident(tag) if compound.is_empty() => {
            compound.tag = Some(tag.to_ascii_lowercase());
        }
        Token::Delim('*') if compound.is_empty() => {
            compound.universal = true;
        }
        Token::IDHash(id) if compound.id.is_none() => {
            compound.id = Some(id.to_string());
        }
        Token::Delim('.') => match next_compact_token(input)? {
            Token::Ident(class) => compound.classes.push(class.to_string()),
            _ => return Err(input.new_custom_error(())),
        },
        Token::SquareBracketBlock => {
            let attribute = input.parse_nested_block(parse_attribute_selector)?;
            compound.attributes.push(attribute);
        }
        Token::Colon => match next_compact_token(input)? {
            Token::Ident(name) => {
                let pseudo_class = match name.to_ascii_lowercase().as_str() {
                    "root" => PseudoClass::Root,
                    "first-child" => PseudoClass::FirstChild,
                    "last-child" => PseudoClass::LastChild,
                    _ => return Err(input.new_custom_error(())),
                };
                compound.pseudo_classes.push(pseudo_class);
            }
            _ => return Err(input.new_custom_error(())),
        },
        _ => return Err(input.new_custom_error(())),
    }
    Ok(())
}

fn next_compact_token<'i>(input: &mut Parser<'i, '_>) -> Result<Token<'i>, ParseError<'i, ()>> {
    loop {
        let token = input.next_including_whitespace_and_comments()?.clone();
        match token {
            Token::Comment(_) => continue,
            Token::WhiteSpace(_) => return Err(input.new_custom_error(())),
            token => return Ok(token),
        }
    }
}

fn parse_attribute_selector<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<AttributeSelector, ParseError<'i, ()>> {
    let name = match input.next() {
        Ok(Token::Ident(name)) => name.to_ascii_lowercase(),
        Ok(_) => return Err(input.new_custom_error(())),
        Err(error) => return Err(error.into()),
    };

    if input.is_exhausted() {
        return Ok(AttributeSelector { name, value: None });
    }

    match input.next() {
        Ok(Token::Delim('=')) => {}
        Ok(_) => return Err(input.new_custom_error(())),
        Err(error) => return Err(error.into()),
    }
    let value = match input.next() {
        Ok(Token::Ident(value) | Token::QuotedString(value)) => value.to_string(),
        Ok(_) => return Err(input.new_custom_error(())),
        Err(error) => return Err(error.into()),
    };
    input.expect_exhausted().map_err(ParseError::from)?;
    Ok(AttributeSelector {
        name,
        value: Some(value),
    })
}

fn matches_pseudo_class(document: &Document, node: NodeId, pseudo_class: PseudoClass) -> bool {
    match pseudo_class {
        PseudoClass::Root => {
            document.node(node).and_then(|node| node.parent) == Some(document.root())
        }
        PseudoClass::FirstChild => previous_element_sibling(document, node).is_none(),
        PseudoClass::LastChild => next_element_sibling(document, node).is_none(),
    }
}

fn previous_element_sibling(document: &Document, node: NodeId) -> Option<NodeId> {
    let parent = document.node(node)?.parent?;
    let children = document.children(parent)?;
    let position = children.iter().position(|child| *child == node)?;
    children[..position]
        .iter()
        .rev()
        .copied()
        .find(|candidate| is_element(document, *candidate))
}

fn next_element_sibling(document: &Document, node: NodeId) -> Option<NodeId> {
    let parent = document.node(node)?.parent?;
    let children = document.children(parent)?;
    let position = children.iter().position(|child| *child == node)?;
    children[position + 1..]
        .iter()
        .copied()
        .find(|candidate| is_element(document, *candidate))
}

fn preceding_element_siblings<'a>(
    document: &'a Document,
    node: NodeId,
) -> impl Iterator<Item = NodeId> + 'a {
    let siblings = document
        .node(node)
        .and_then(|node| node.parent)
        .and_then(|parent| document.children(parent))
        .and_then(|children| {
            children
                .iter()
                .position(|child| *child == node)
                .map(|position| &children[..position])
        })
        .unwrap_or(&[]);
    siblings
        .iter()
        .rev()
        .copied()
        .filter(|candidate| is_element(document, *candidate))
}

fn is_element(document: &Document, node: NodeId) -> bool {
    document
        .node(node)
        .is_some_and(|node| matches!(node.kind, NodeKind::Element(_)))
}
