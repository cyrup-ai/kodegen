//! Abstract Syntax Tree definitions for YAML parsing
//!
//! This module defines the complete AST structure for representing parsed YAML documents
//! with full source location tracking and type safety.

use crate::lexer::{Position, ScalarStyle};
use std::borrow::Cow;

/// A complete YAML stream containing multiple documents
#[derive(Debug, Clone, PartialEq)]
pub struct Stream<'input> {
    pub documents: Vec<Document<'input>>,
}

impl<'input> Stream<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(documents: Vec<Document<'input>>) -> Self {
        Self { documents }
    }

    #[inline]
    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    #[inline]
    #[must_use] 
    pub const fn len(&self) -> usize {
        self.documents.len()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Document<'input>> {
        self.documents.iter()
    }
}

/// A single YAML document
#[derive(Debug, Clone, PartialEq)]
pub struct Document<'input> {
    pub content: Option<Node<'input>>,
    pub has_explicit_start: bool,
    pub has_explicit_end: bool,
    pub position: Position,
}

impl<'input> Document<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(
        content: Option<Node<'input>>,
        has_explicit_start: bool,
        has_explicit_end: bool,
        position: Position,
    ) -> Self {
        Self {
            content,
            has_explicit_start,
            has_explicit_end,
            position,
        }
    }

    #[inline]
    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.content.is_none()
    }
}

/// A YAML node representing any value in the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Node<'input> {
    /// Scalar value (string, number, boolean, null)
    Scalar(ScalarNode<'input>),
    /// Sequence/array [item1, item2, ...]
    Sequence(SequenceNode<'input>),
    /// Mapping/object {key1: value1, key2: value2, ...}
    Mapping(MappingNode<'input>),
    /// Anchor definition &anchor
    Anchor(AnchorNode<'input>),
    /// Alias reference *alias
    Alias(AliasNode<'input>),
    /// Tagged value !tag value
    Tagged(TaggedNode<'input>),
    /// Explicit null value
    Null(NullNode),
}

impl<'input> Node<'input> {
    /// Get the position of this node
    #[must_use] 
    pub const fn position(&self) -> Position {
        match self {
            Node::Scalar(n) => n.position,
            Node::Sequence(n) => n.position,
            Node::Mapping(n) => n.position,
            Node::Anchor(n) => n.position,
            Node::Alias(n) => n.position,
            Node::Tagged(n) => n.position,
            Node::Null(n) => n.position,
        }
    }

    /// Check if this node is a scalar
    #[inline]
    #[must_use] 
    pub const fn is_scalar(&self) -> bool {
        matches!(self, Node::Scalar(_))
    }

    /// Check if this node is a sequence
    #[inline]
    #[must_use] 
    pub const fn is_sequence(&self) -> bool {
        matches!(self, Node::Sequence(_))
    }

    /// Check if this node is a mapping
    #[inline]
    #[must_use] 
    pub const fn is_mapping(&self) -> bool {
        matches!(self, Node::Mapping(_))
    }

    /// Check if this node is null
    #[inline]
    #[must_use] 
    pub const fn is_null(&self) -> bool {
        matches!(self, Node::Null(_))
    }

    /// Get scalar value if this is a scalar node
    #[must_use] 
    pub const fn as_scalar(&self) -> Option<&ScalarNode<'input>> {
        match self {
            Node::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    /// Get sequence if this is a sequence node
    #[must_use] 
    pub const fn as_sequence(&self) -> Option<&SequenceNode<'input>> {
        match self {
            Node::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// Get mapping if this is a mapping node
    #[must_use] 
    pub const fn as_mapping(&self) -> Option<&MappingNode<'input>> {
        match self {
            Node::Mapping(map) => Some(map),
            _ => None,
        }
    }
}

/// A scalar value node
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarNode<'input> {
    pub value: Cow<'input, str>,
    pub style: ScalarStyle,
    pub tag: Option<Cow<'input, str>>,
    pub position: Position,
}

impl<'input> ScalarNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(
        value: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<Cow<'input, str>>,
        position: Position,
    ) -> Self {
        Self {
            value,
            style,
            tag,
            position,
        }
    }

    /// Get the string value
    #[inline]
    #[must_use] 
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Parse as integer
    #[must_use] 
    pub fn as_int(&self) -> Option<i64> {
        self.value.parse().ok()
    }

    /// Parse as float
    #[must_use] 
    pub fn as_float(&self) -> Option<f64> {
        self.value.parse().ok()
    }

    /// Parse as boolean
    #[must_use] 
    pub fn as_bool(&self) -> Option<bool> {
        match self.value.as_ref() {
            "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" => Some(true),
            "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" => Some(false),
            _ => None,
        }
    }

    /// Check if this represents a null value
    #[must_use] 
    pub fn is_null(&self) -> bool {
        matches!(self.value.as_ref(), "null" | "Null" | "NULL" | "~" | "")
    }
}

/// A sequence/array node
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceNode<'input> {
    pub items: Vec<Node<'input>>,
    pub style: SequenceStyle,
    pub position: Position,
}

impl<'input> SequenceNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(items: Vec<Node<'input>>, style: SequenceStyle, position: Position) -> Self {
        Self {
            items,
            style,
            position,
        }
    }

    #[inline]
    #[must_use] 
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Node<'input>> {
        self.items.iter()
    }

    #[inline]
    #[must_use] 
    pub fn get(&self, index: usize) -> Option<&Node<'input>> {
        self.items.get(index)
    }
}

/// Sequence representation style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceStyle {
    /// Flow style: [item1, item2, item3]
    Flow,
    /// Block style:
    /// - item1
    /// - item2
    /// - item3
    Block,
}

/// A mapping/object node
#[derive(Debug, Clone, PartialEq)]
pub struct MappingNode<'input> {
    pub pairs: Vec<MappingPair<'input>>,
    pub style: MappingStyle,
    pub position: Position,
}

impl<'input> MappingNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(pairs: Vec<MappingPair<'input>>, style: MappingStyle, position: Position) -> Self {
        Self {
            pairs,
            style,
            position,
        }
    }

    #[inline]
    #[must_use] 
    pub const fn len(&self) -> usize {
        self.pairs.len()
    }

    #[inline]
    #[must_use] 
    pub const fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, MappingPair<'input>> {
        self.pairs.iter()
    }

    /// Find value by key (string comparison)
    #[must_use] 
    pub fn get(&self, key: &str) -> Option<&Node<'input>> {
        for pair in &self.pairs {
            if let Some(scalar) = pair.key.as_scalar()
                && scalar.as_str() == key
            {
                return Some(&pair.value);
            }
        }
        None
    }

    /// Get all keys as strings (if they are scalars)
    #[must_use] 
    pub fn keys(&self) -> Vec<&str> {
        self.pairs
            .iter()
            .filter_map(|pair| pair.key.as_scalar().map(|s| s.as_str()))
            .collect()
    }
}

/// Mapping representation style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStyle {
    /// Flow style: {key1: value1, key2: value2}
    Flow,
    /// Block style:
    /// key1: value1
    /// key2: value2
    Block,
}

/// A key-value pair in a mapping
#[derive(Debug, Clone, PartialEq)]
pub struct MappingPair<'input> {
    pub key: Node<'input>,
    pub value: Node<'input>,
}

impl<'input> MappingPair<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(key: Node<'input>, value: Node<'input>) -> Self {
        Self { key, value }
    }
}

/// An anchor definition node
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorNode<'input> {
    pub name: Cow<'input, str>,
    pub node: Box<Node<'input>>,
    pub position: Position,
}

impl<'input> AnchorNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(name: Cow<'input, str>, node: Box<Node<'input>>, position: Position) -> Self {
        Self {
            name,
            node,
            position,
        }
    }
}

/// An alias reference node
#[derive(Debug, Clone, PartialEq)]
pub struct AliasNode<'input> {
    pub name: Cow<'input, str>,
    pub position: Position,
}

impl<'input> AliasNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(name: Cow<'input, str>, position: Position) -> Self {
        Self { name, position }
    }
}

/// A tagged value node
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedNode<'input> {
    pub handle: Option<Cow<'input, str>>,
    pub suffix: Cow<'input, str>,
    pub node: Box<Node<'input>>,
    pub position: Position,
}

impl<'input> TaggedNode<'input> {
    #[inline]
    #[must_use] 
    pub const fn new(
        handle: Option<Cow<'input, str>>,
        suffix: Cow<'input, str>,
        node: Box<Node<'input>>,
        position: Position,
    ) -> Self {
        Self {
            handle,
            suffix,
            node,
            position,
        }
    }

    /// Get the full tag name
    #[must_use] 
    pub fn tag_name(&self) -> String {
        match &self.handle {
            Some(handle) => format!("{}{}", handle, self.suffix),
            None => self.suffix.to_string(),
        }
    }
}

/// An explicit null node
#[derive(Debug, Clone, PartialEq)]
pub struct NullNode {
    pub position: Position,
}

impl NullNode {
    #[inline]
    #[must_use] 
    pub const fn new(position: Position) -> Self {
        Self { position }
    }
}

/// Visitor pattern for AST traversal
pub trait NodeVisitor<'input> {
    type Output;
    type Error;

    fn visit_scalar(&mut self, node: &ScalarNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_sequence(&mut self, node: &SequenceNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_mapping(&mut self, node: &MappingNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_anchor(&mut self, node: &AnchorNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_alias(&mut self, node: &AliasNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_tagged(&mut self, node: &TaggedNode<'input>) -> Result<Self::Output, Self::Error>;
    fn visit_null(&mut self, node: &NullNode) -> Result<Self::Output, Self::Error>;

    fn visit_node(&mut self, node: &Node<'input>) -> Result<Self::Output, Self::Error> {
        match node {
            Node::Scalar(n) => self.visit_scalar(n),
            Node::Sequence(n) => self.visit_sequence(n),
            Node::Mapping(n) => self.visit_mapping(n),
            Node::Anchor(n) => self.visit_anchor(n),
            Node::Alias(n) => self.visit_alias(n),
            Node::Tagged(n) => self.visit_tagged(n),
            Node::Null(n) => self.visit_null(n),
        }
    }
}

/// Mutable visitor pattern for AST transformation
pub trait NodeVisitorMut<'input> {
    type Output;
    type Error;

    fn visit_scalar_mut(
        &mut self,
        node: &mut ScalarNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_sequence_mut(
        &mut self,
        node: &mut SequenceNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_mapping_mut(
        &mut self,
        node: &mut MappingNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_anchor_mut(
        &mut self,
        node: &mut AnchorNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_alias_mut(
        &mut self,
        node: &mut AliasNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_tagged_mut(
        &mut self,
        node: &mut TaggedNode<'input>,
    ) -> Result<Self::Output, Self::Error>;
    fn visit_null_mut(&mut self, node: &mut NullNode) -> Result<Self::Output, Self::Error>;

    fn visit_node_mut(&mut self, node: &mut Node<'input>) -> Result<Self::Output, Self::Error> {
        match node {
            Node::Scalar(n) => self.visit_scalar_mut(n),
            Node::Sequence(n) => self.visit_sequence_mut(n),
            Node::Mapping(n) => self.visit_mapping_mut(n),
            Node::Anchor(n) => self.visit_anchor_mut(n),
            Node::Alias(n) => self.visit_alias_mut(n),
            Node::Tagged(n) => self.visit_tagged_mut(n),
            Node::Null(n) => self.visit_null_mut(n),
        }
    }
}

/// Utilities for working with AST nodes
pub mod utils {
    use super::*;

    /// Collect all anchor names in a document
    #[must_use] 
    pub fn collect_anchors<'input>(node: &'input Node<'input>) -> Vec<&'input str> {
        let mut anchors = Vec::new();
        collect_anchors_recursive(node, &mut anchors);
        anchors
    }

    fn collect_anchors_recursive<'input>(
        node: &'input Node<'input>,
        anchors: &mut Vec<&'input str>,
    ) {
        match node {
            Node::Anchor(anchor) => {
                anchors.push(&anchor.name);
                collect_anchors_recursive(&anchor.node, anchors);
            }
            Node::Sequence(seq) => {
                for item in &seq.items {
                    collect_anchors_recursive(item, anchors);
                }
            }
            Node::Mapping(map) => {
                for pair in &map.pairs {
                    collect_anchors_recursive(&pair.key, anchors);
                    collect_anchors_recursive(&pair.value, anchors);
                }
            }
            Node::Tagged(tagged) => {
                collect_anchors_recursive(&tagged.node, anchors);
            }
            _ => {}
        }
    }

    /// Collect all alias names in a document
    #[must_use] 
    pub fn collect_aliases<'input>(node: &'input Node<'input>) -> Vec<&'input str> {
        let mut aliases = Vec::new();
        collect_aliases_recursive(node, &mut aliases);
        aliases
    }

    fn collect_aliases_recursive<'input>(
        node: &'input Node<'input>,
        aliases: &mut Vec<&'input str>,
    ) {
        match node {
            Node::Alias(alias) => {
                aliases.push(&alias.name);
            }
            Node::Anchor(anchor) => {
                collect_aliases_recursive(&anchor.node, aliases);
            }
            Node::Sequence(seq) => {
                for item in &seq.items {
                    collect_aliases_recursive(item, aliases);
                }
            }
            Node::Mapping(map) => {
                for pair in &map.pairs {
                    collect_aliases_recursive(&pair.key, aliases);
                    collect_aliases_recursive(&pair.value, aliases);
                }
            }
            Node::Tagged(tagged) => {
                collect_aliases_recursive(&tagged.node, aliases);
            }
            _ => {}
        }
    }

    /// Calculate the depth of nesting in an AST
    pub fn calculate_depth<'input>(node: &Node<'input>) -> usize {
        match node {
            Node::Anchor(anchor) => calculate_depth(&anchor.node),
            Node::Tagged(tagged) => calculate_depth(&tagged.node),
            Node::Sequence(seq) => 1 + seq.items.iter().map(calculate_depth).max().unwrap_or(0),
            Node::Mapping(map) => {
                1 + map
                    .pairs
                    .iter()
                    .map(|pair| {
                        std::cmp::max(calculate_depth(&pair.key), calculate_depth(&pair.value))
                    })
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Count total number of nodes in an AST
    pub fn count_nodes<'input>(node: &Node<'input>) -> usize {
        match node {
            Node::Anchor(anchor) => 1 + count_nodes(&anchor.node),
            Node::Tagged(tagged) => 1 + count_nodes(&tagged.node),
            Node::Sequence(seq) => 1 + seq.items.iter().map(count_nodes).sum::<usize>(),
            Node::Mapping(map) => {
                1 + map
                    .pairs
                    .iter()
                    .map(|pair| count_nodes(&pair.key) + count_nodes(&pair.value))
                    .sum::<usize>()
            }
            _ => 1,
        }
    }
}
