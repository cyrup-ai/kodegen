use crate::error::ScanError;
use crate::events::{TokenType, TScalarStyle};
use crate::linked_hash_map::LinkedHashMap;
use crate::parser::grammar::{YamlContext, ParametricContext};
use crate::scanner::Scanner;
use crate::yaml::Yaml;
use std::collections::HashMap;

/// YAML parsing state machine states
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum State {
    StreamStart,
    DirectiveHeader,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    NextDocument,
    BlockNode,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingValue,
    BlockSequenceEntry,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    End,
}

/// State machine parser that builds Yaml AST directly
pub struct StateMachine<T: Iterator<Item = char>> {
    pub scanner: Scanner<T>,
    pub states: Vec<State>,
    pub state: State,
    pub anchors: HashMap<String, usize>,
    pub anchor_id: usize,
    pub indents: Vec<usize>, // Keep for compatibility
    ast_stack: Vec<YamlBuilder>,
    pending_tag: Option<(String, String)>,
    tag_stack: Vec<Option<(String, String)>>, // Stack for nested tag scopes

    // ADD:
    pub context: ParametricContext,
    yaml_version: Option<(u32, u32)>,
    tag_handles: HashMap<String, String>,
}

/// Builder for constructing Yaml AST during parsing
#[derive(Debug)]
enum YamlBuilder {
    Sequence(Vec<Yaml>),
    Mapping(LinkedHashMap<Yaml, Yaml>, Option<Yaml>), // map, current_key
    Scalar(String),
}

impl<T: Iterator<Item = char>> StateMachine<T> {
    pub fn new(src: T) -> Self {
        Self {
            scanner: Scanner::new(src),
            states: Vec::new(),
            state: State::StreamStart,
            anchors: HashMap::new(),
            anchor_id: 1,
            indents: Vec::new(),
            ast_stack: Vec::new(),
            pending_tag: None,
            tag_stack: Vec::new(),

            // ADD:
            context: ParametricContext::new(),
            yaml_version: None,
            tag_handles: HashMap::new(),
        }
    }

    pub fn push_indent(&mut self, indent: usize) {
        self.indents.push(indent);
    }

    pub fn pop_indent(&mut self) {
        self.indents.pop();
    }

    pub fn pop_state(&mut self) {
        if let Some(state) = self.states.pop() {
            // Decrement depth when unwinding
            self.context.decrement_depth();

            // Handle special case: returning from BlockNode to BlockMappingKey means
            // we parsed nested content that should be added as a mapping value
            if self.state == State::BlockNode && state == State::BlockMappingKey {
                // Restore the saved tag for the outer mapping
                if let Some(saved_tag) = self.tag_stack.pop() {
                    self.pending_tag = saved_tag;
                }

                // Take the completed AST structure and add it as mapping value
                if let Some(builder) = self.ast_stack.pop() {
                    let yaml = self.finalize_builder(builder);
                    self.add_mapping_pair(yaml);
                }
            }
            
            // Check if we're leaving a context scope
            match (self.state, state) {
                (State::FlowSequenceEntry, State::BlockNode)
                | (State::FlowMappingValue, State::BlockNode)
                | (State::BlockMappingValue, State::BlockMappingKey)
                | (State::BlockSequenceEntry, State::BlockNode) => {
                    self.context.pop_context();
                }
                _ => {}
            }
            self.state = state;
        }
    }

    pub fn push_state(&mut self, st: State) {
        self.states.push(self.state);
        self.state = st;
    }

    pub fn register_anchor(&mut self, name: String) -> usize {
        let new_id = self.anchor_id;
        self.anchor_id += 1;
        self.anchors.insert(name, new_id);
        new_id
    }

    /// Execute the state machine and return the constructed Yaml AST
    pub fn parse(&mut self) -> Result<Yaml, ScanError> {
        while self.state != State::End {
            self.execute_state()?;
        }

        // Return the final constructed AST
        if let Some(builder) = self.ast_stack.pop() {
            Ok(self.finalize_builder(builder))
        } else {
            Ok(Yaml::Null)
        }
    }

    /// Execute a single state transition
    pub fn execute_state(&mut self) -> Result<(), ScanError> {
        match self.state {
            State::StreamStart => self.handle_stream_start(),
            State::DirectiveHeader => self.handle_directive_header(),
            State::DocumentStart => {
                self.handle_document_start();
                Ok(())
            },
            State::DocumentContent => { self.handle_document_content(); Ok(()) },
            State::DocumentEnd => self.handle_document_end(),
            State::NextDocument => self.handle_next_document(),
            State::BlockNode => self.handle_block_content_with_structure(),
            State::BlockMappingFirstKey => self.handle_block_mapping_first_key(),
            State::BlockMappingKey => self.handle_block_mapping_key(),
            State::BlockMappingValue => self.handle_block_mapping_value(),
            State::FlowSequenceFirstEntry => self.handle_flow_sequence_first_entry(),
            State::FlowSequenceEntry => self.handle_flow_sequence_entry(),
            State::FlowMappingFirstKey => self.handle_flow_mapping_first_key(),
            State::FlowMappingKey => self.handle_flow_mapping_key(),
            State::FlowMappingValue => self.handle_flow_mapping_value(),
            _ => {
                self.state = State::End;
                Ok(())
            }
        }
    }

    fn handle_stream_start(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::StreamStart(_) => {
                self.scanner.fetch_token();
                self.state = State::DirectiveHeader;
                Ok(())
            }
            _ => Err(ScanError::new(token.0, "expected stream start")),
        }
    }

    const fn handle_document_start(&mut self) {
        self.state = State::DocumentContent;
    }

    fn handle_document_content(&mut self) {
        // Document content starts at indent 0, BLOCK-OUT context
        self.context.push_context(YamlContext::BlockOut, 0);
        self.state = State::BlockNode;
    }

    fn handle_block_node(&mut self) -> Result<(), ScanError> {
        // Keep parsing until we handle a non-tag token
        loop {
            let token = self.scanner.peek_token()?;

            // ADD context validation:
            if !self.can_accept_token(&token.1) {
                return Err(ScanError::new(
                    token.0,
                    &format!(
                        "Token {:?} not allowed in {:?} context",
                        token.1, self.context.current_context
                    ),
                ));
            }

            match &token.1 {
                TokenType::Scalar(style, value) => {
                    self.scanner.fetch_token(); // Consume the scalar

                    let yaml = match style {
                        TScalarStyle::Literal | TScalarStyle::Folded => {
                            // Block scalars already processed by lexer - use directly
                            Yaml::String(value.clone())
                        }
                        _ => {
                            // Handle other scalar styles with existing logic
                            // Peek ahead to see if this is a mapping key
                            let next_token = self.scanner.peek_token()?;

                            if matches!(next_token.1, TokenType::Value) {
                                // This is a mapping key
                                let key = Yaml::parse_str(value);
                                
                                // Check if we already have a mapping in progress
                                if let Some(YamlBuilder::Mapping(_, current_key)) = self.ast_stack.last_mut()
                                    && current_key.is_none() {
                                        // We have a mapping waiting for a key
                                        *current_key = Some(key);
                                        self.state = State::BlockMappingValue;
                                        return Ok(());
                                    }
                                
                                // No mapping in progress, create a new one
                                self.ast_stack
                                    .push(YamlBuilder::Mapping(LinkedHashMap::new(), Some(key)));
                                self.state = State::BlockMappingValue;
                                return Ok(());
                            } else {
                                // Just a scalar value
                                Yaml::parse_str(value)
                            }
                        }
                    };

                    self.push_yaml(yaml);
                    self.pop_state();
                    return Ok(());
                }
                TokenType::BlockEntry => {
                    self.ast_stack.push(YamlBuilder::Sequence(Vec::new()));
                    self.handle_parametric_block_sequence()?;
                    if let Some(YamlBuilder::Sequence(items)) = self.ast_stack.pop() {
                        self.push_yaml(Yaml::Array(items));
                    }
                    return Ok(());
                }
                TokenType::Key => {
                    self.scanner.fetch_token();
                    self.ast_stack
                        .push(YamlBuilder::Mapping(LinkedHashMap::new(), None));
                    self.state = State::BlockMappingFirstKey;
                    return Ok(());
                }
                TokenType::FlowSequenceStart => {
                    self.scanner.fetch_token();
                    self.ast_stack.push(YamlBuilder::Sequence(Vec::new()));
                    self.state = State::FlowSequenceFirstEntry;
                    return Ok(());
                }
                TokenType::FlowMappingStart => {
                    self.scanner.fetch_token();
                    self.ast_stack
                        .push(YamlBuilder::Mapping(LinkedHashMap::new(), None));
                    self.state = State::FlowMappingFirstKey;
                    return Ok(());
                }
                TokenType::Tag(handle, suffix) => {
                    // Store the tag for the next value
                    self.pending_tag = Some((handle.clone(), suffix.clone()));
                    self.scanner.fetch_token();
                    // Continue looping to parse the value that follows the tag
                    continue;
                }
                TokenType::DocumentStart => {
                    // New document started, current document is finished
                    // Properly unwind state stack and finalize pending structures
                    while !self.states.is_empty() {
                        self.pop_state();
                    }

                    // Restore any saved tags from the tag stack before finalizing AST
                    if let Some(saved_tag) = self.tag_stack.pop() {
                        self.pending_tag = saved_tag;
                    }

                    // Finalize any pending AST structures
                    while let Some(builder) = self.ast_stack.pop() {
                        let yaml = self.finalize_builder(builder);
                        self.push_yaml(yaml);
                    }
                    self.state = State::DocumentEnd;
                    return Ok(());
                }
                TokenType::DocumentEnd => {
                    // Document end marker encountered
                    // Restore any saved tags before finishing document
                    if let Some(saved_tag) = self.tag_stack.pop() {
                        self.pending_tag = saved_tag;
                    }

                    self.scanner.fetch_token(); // consume
                    self.state = State::DocumentEnd;
                    return Ok(());
                }
                TokenType::StreamEnd => {
                    self.state = State::End;
                    return Ok(());
                }
                _ => {
                    self.push_yaml(Yaml::Null);
                    self.pop_state();
                    return Ok(());
                }
            }
        }
    }

    

    

    

    fn handle_block_mapping_first_key(&mut self) -> Result<(), ScanError> {
        // Block mapping key uses BLOCK-KEY context at n+1
        let n = self.context.current_indent();
        self.context.push_context(YamlContext::BlockKey, n + 1);

        self.state = State::BlockMappingKey;
        self.handle_mapping_key()
    }

    fn handle_block_mapping_key(&mut self) -> Result<(), ScanError> {
        self.handle_mapping_key()
    }

    fn handle_mapping_key(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::Scalar(_, value) => {
                self.scanner.fetch_token();
                let key = Yaml::parse_str(value);
                if let Some(YamlBuilder::Mapping(_, current_key)) = self.ast_stack.last_mut() {
                    *current_key = Some(key);
                }
                self.state = State::BlockMappingValue;
                Ok(())
            }
            TokenType::DocumentStart => {
                // New document started, current document is finished
                if let Some(YamlBuilder::Mapping(map, _)) = self.ast_stack.pop() {
                    // Restore saved tag before finalizing the mapping
                    if let Some(saved_tag) = self.tag_stack.pop() {
                        self.pending_tag = saved_tag;
                    }
                    self.push_yaml(Yaml::Hash(map));
                }
                self.state = State::DocumentEnd;
                Ok(())
            }
            TokenType::DocumentEnd => {
                // Document end marker, current document is finished
                if let Some(YamlBuilder::Mapping(map, _)) = self.ast_stack.pop() {
                    self.push_yaml(Yaml::Hash(map));
                }
                self.scanner.fetch_token(); // consume
                self.state = State::DocumentEnd;
                Ok(())
            }
            TokenType::StreamEnd => {
                // End of stream
                if let Some(YamlBuilder::Mapping(map, _)) = self.ast_stack.pop() {
                    self.push_yaml(Yaml::Hash(map));
                }
                self.state = State::End;
                Ok(())
            }
            _ => {
                // End of mapping
                if let Some(YamlBuilder::Mapping(map, _)) = self.ast_stack.pop() {
                    self.push_yaml(Yaml::Hash(map));
                }

                // Check if we're at the root level
                if self.states.is_empty() {
                    self.state = State::End;
                } else {
                    self.pop_state();
                }
                Ok(())
            }
        }
    }

    fn handle_block_mapping_value(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::Value => {
                self.scanner.fetch_token();

                // Fix 3: EXISTING context push logic but with proper n+m calculation
                let key_indent = self.context.current_indent();

                // Use EXISTING calculate_block_indent method from ParametricContext
                let value_indent = self.context.calculate_block_indent(key_indent, 1); // n+1 minimum
                self.context.push_context(YamlContext::BlockIn, value_indent);

                // Handle tags and other tokens after the colon
                loop {
                    let value_token = self.scanner.peek_token()?;
                    match &value_token.1 {
                        TokenType::Tag(handle, suffix) => {
                            // Store the tag for the value
                            self.pending_tag = Some((handle.clone(), suffix.clone()));
                            self.scanner.fetch_token();
                            // Continue to get the actual value
                            continue;
                        }
                        TokenType::Scalar(style, value) => {
                            // Consume the scalar first
                            self.scanner.fetch_token();
                            
                            // Check what token follows this scalar
                            let next_token = self.scanner.peek_token()?;

                            // If next token is Value (:), this is a mapping key in nested content
                            if matches!(next_token.1, TokenType::Value) {
                                // Save the current pending tag for the outer mapping
                                let saved_tag = self.pending_tag.take();
                                self.tag_stack.push(saved_tag);

                                // Create a new mapping and add this key to it
                                let key = Yaml::parse_str(value);
                                let nested_map = crate::linked_hash_map::LinkedHashMap::new();

                                self.ast_stack.push(crate::parser::state_machine::YamlBuilder::Mapping(nested_map, Some(key)));
                                self.context.increment_depth()?;
                                self.push_state(State::BlockMappingKey);
                                self.state = State::BlockMappingValue; // Parse the value for this key
                                return Ok(());
                            }

                            // Otherwise, treat as regular scalar value

                            let yaml_value = match style {
                                TScalarStyle::Literal | TScalarStyle::Folded => {
                                    // Block scalars already processed by scanner - use directly
                                    Yaml::String(value.clone())
                                }
                                _ => {
                                    // Handle other scalar styles with existing logic
                                    Yaml::parse_str(value)
                                }
                            };

                            self.add_mapping_pair(yaml_value);
                            self.state = State::BlockMappingKey;
                            return Ok(());
                        }
                        TokenType::DocumentStart => {
                            // NEW: Handle document boundaries in mapping values
                            self.add_mapping_pair(Yaml::Null);
                            self.state = State::DocumentEnd;
                            return Ok(());
                        }
                        TokenType::DocumentEnd => {
                            // NEW: Handle document end in mapping values
                            self.add_mapping_pair(Yaml::Null);
                            self.scanner.fetch_token(); // consume
                            self.state = State::DocumentEnd;
                            return Ok(());
                        }
                        _ => {
                            // Handle nested structures as the value per YAML 1.2 rule [194]
                            // Transition to BlockNode to parse nested content as mapping value
                            self.context.increment_depth()?;
                            self.push_state(State::BlockMappingKey); // Return here when nested content is parsed
                            self.state = State::BlockNode; // Parse nested structure as the mapping value
                            return Ok(());
                        }
                    }
                }
            }
            _ => {
                // EXISTING null value handling
                self.add_mapping_pair(Yaml::Null);
                self.state = State::BlockMappingKey;
                Ok(())
            }
        }
    }

    fn handle_block_content_with_structure(&mut self) -> Result<(), ScanError> {
        // USE new structural productions with existing context
        let current_indent = self.context.current_indent();
        self.scanner
            .process_structural_separation(&mut self.context, current_indent)?;

        // Skip any comments
        let _comments = self.scanner.skip_structural_comments()?;

        // Continue with existing block handling logic...
        self.handle_block_node()
    }

    fn handle_flow_sequence_first_entry(&mut self) -> Result<(), ScanError> {
        // Flow sequence switches to FLOW-IN context
        let current_indent = self.context.current_indent();
        self.context.push_context(YamlContext::FlowIn, current_indent);

        self.state = State::FlowSequenceEntry;
        Ok(())
    }

    fn handle_flow_sequence_entry(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::FlowSequenceEnd => {
                // Existing implementation - keep as-is
                self.scanner.fetch_token();
                if let Some(YamlBuilder::Sequence(items)) = self.ast_stack.pop() {
                    self.push_yaml(Yaml::Array(items));
                }
                self.pop_state();
                Ok(())
            }
            TokenType::FlowEntry => {
                // Existing implementation - keep as-is
                self.scanner.fetch_token();
                Ok(())
            }
            TokenType::Scalar(style, value) => {
                // ENHANCED: Use complete flow productions for scalar parsing
                self.scanner.fetch_token();

                let yaml = match style {
                    TScalarStyle::DoubleQuoted => {
                        // Re-parse with complete double-quoted productions
                        let mut temp_state =
                            crate::scanner::state::ScannerState::new(value.chars());
                        let parsed =
                            crate::parser::flow::FlowProductions::parse_double_quoted_scalar(
                                &mut temp_state,
                                &self.context,
                                self.context.current_indent(),
                            )?;
                        Yaml::String(parsed)
                    }
                    TScalarStyle::SingleQuoted => {
                        // Re-parse with complete single-quoted productions
                        let mut temp_state =
                            crate::scanner::state::ScannerState::new(value.chars());
                        let parsed =
                            crate::parser::flow::FlowProductions::parse_single_quoted_scalar(
                                &mut temp_state,
                                &self.context,
                                self.context.current_indent(),
                            )?;
                        Yaml::String(parsed)
                    }
                    TScalarStyle::Plain => {
                        // Re-parse with complete plain scalar productions
                        let mut temp_state =
                            crate::scanner::state::ScannerState::new(value.chars());
                        let parsed = crate::parser::flow::FlowProductions::parse_plain_scalar(
                            &mut temp_state,
                            &self.context,
                            self.context.current_indent(),
                        )?;
                        Yaml::String(parsed)
                    }
                    _ => Yaml::parse_str(value), // Fallback for other styles
                };

                if let Some(YamlBuilder::Sequence(items)) = self.ast_stack.last_mut() {
                    items.push(yaml);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    const fn handle_flow_mapping_first_key(&mut self) -> Result<(), ScanError> {
        self.state = State::FlowMappingKey;
        Ok(())
    }

    fn handle_flow_mapping_key(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        println!("StateMachine: handle_flow_mapping_key processing token {:?}", token.1);
        match &token.1 {
            TokenType::FlowMappingEnd => {
                self.scanner.fetch_token();
                if let Some(YamlBuilder::Mapping(map, _)) = self.ast_stack.pop() {
                    self.push_yaml(Yaml::Hash(map));
                }
                self.pop_state();
                Ok(())
            }
            TokenType::FlowEntry => {
                self.scanner.fetch_token();
                Ok(())
            }
            TokenType::Scalar(style, value) => {
                // ENHANCED: Use complete flow productions for scalar parsing
                self.scanner.fetch_token();

                let key = match style {
                    TScalarStyle::DoubleQuoted => {
                        // Re-parse with complete double-quoted productions
                        let mut temp_state =
                            crate::scanner::state::ScannerState::new(value.chars());
                        let parsed =
                            crate::parser::flow::FlowProductions::parse_double_quoted_scalar(
                                &mut temp_state,
                                &self.context,
                                self.context.current_indent(),
                            )?;
                        Yaml::String(parsed)
                    }
                    TScalarStyle::SingleQuoted => {
                        // Re-parse with complete single-quoted productions
                        let mut temp_state =
                            crate::scanner::state::ScannerState::new(value.chars());
                        let parsed =
                            crate::parser::flow::FlowProductions::parse_single_quoted_scalar(
                                &mut temp_state,
                                &self.context,
                                self.context.current_indent(),
                            )?;
                        Yaml::String(parsed)
                    }
                    TScalarStyle::Plain => {
                        // Use token value directly to avoid re-parsing
                        Yaml::String(value.clone())
                    }
                    _ => Yaml::parse_str(value), // Fallback for other styles
                };

                if let Some(YamlBuilder::Mapping(_, current_key)) = self.ast_stack.last_mut() {
                    *current_key = Some(key);
                }
                self.state = State::FlowMappingValue;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_flow_mapping_value(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        println!("StateMachine: handle_flow_mapping_value processing token {:?}", token.1);
        match &token.1 {
            TokenType::Value => {
                self.scanner.fetch_token();
                let value_token = self.scanner.peek_token()?;
                match &value_token.1 {
                    TokenType::Scalar(style, value) => {
                        // ENHANCED: Use complete flow productions for scalar parsing
                        self.scanner.fetch_token();

                        let yaml_value = match style {
                            TScalarStyle::DoubleQuoted => {
                                // Re-parse with complete double-quoted productions
                                let mut temp_state =
                                    crate::scanner::state::ScannerState::new(value.chars());
                                let parsed = crate::parser::flow::FlowProductions::parse_double_quoted_scalar(
                                    &mut temp_state,
                                    &self.context,
                                    self.context.current_indent()
                                )?;
                                Yaml::String(parsed)
                            }
                            TScalarStyle::SingleQuoted => {
                                // Re-parse with complete single-quoted productions
                                let mut temp_state =
                                    crate::scanner::state::ScannerState::new(value.chars());
                                let parsed = crate::parser::flow::FlowProductions::parse_single_quoted_scalar(
                                    &mut temp_state,
                                    &self.context,
                                    self.context.current_indent()
                                )?;
                                Yaml::String(parsed)
                            }
                            TScalarStyle::Plain => {
                                // Use token value directly - scanner already parsed correctly
                                Yaml::String(value.clone())
                            }
                            _ => Yaml::parse_str(value), // Fallback for other styles
                        };

                        self.add_mapping_pair(yaml_value);
                        self.state = State::FlowMappingKey;
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            TokenType::FlowEntry => {
                self.scanner.fetch_token();
                self.state = State::FlowMappingKey;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Add a key-value pair to the current mapping
    fn add_mapping_pair(&mut self, mut value: Yaml) {
        // Apply pending tag if present
        if let Some((handle, suffix)) = self.pending_tag.take() {
            let tag_uri = match handle.as_str() {
                "!!" => format!("tag:yaml.org,2002:{}", suffix),
                "!" => suffix,
                _ => format!("{}{}", handle, suffix),
            };
            value = Yaml::Tagged(tag_uri, Box::new(value));
        }

        if let Some(YamlBuilder::Mapping(map, current_key)) = self.ast_stack.last_mut()
            && let Some(key) = current_key.take()
        {
            map.insert(key, value);
        }
    }

    /// Push a constructed Yaml value onto the AST stack
    fn push_yaml(&mut self, mut yaml: Yaml) {
        // Apply pending tag if present
        if let Some((handle, suffix)) = self.pending_tag.take() {
            let tag_uri = match handle.as_str() {
                "!!" => format!("tag:yaml.org,2002:{}", suffix),
                "!" => suffix,
                _ => format!("{}{}", handle, suffix),
            };
            yaml = Yaml::Tagged(tag_uri, Box::new(yaml));
        }

        // If we have a container being built, add to it
        if let Some(builder) = self.ast_stack.last_mut() {
            match builder {
                YamlBuilder::Sequence(items) => {
                    items.push(yaml);
                }
                YamlBuilder::Mapping(map, current_key) => {
                    if let Some(key) = current_key.take() {
                        // We have a key waiting for a value
                        map.insert(key, yaml);
                    } else {
                        // No key yet, this must be a key
                        *current_key = Some(yaml);
                    }
                }
                _ => {}
            }
        } else {
            // This is the root document - push the yaml directly
            self.ast_stack.push(match yaml {
                Yaml::Array(items) => YamlBuilder::Sequence(items),
                Yaml::Hash(map) => YamlBuilder::Mapping(map, None),
                Yaml::String(s) => YamlBuilder::Scalar(s),
                other => YamlBuilder::Scalar(format!("{:?}", other)),
            });
        }
    }

    const fn can_accept_token(&self, token: &TokenType) -> bool {
        match self.context.current_context {
            YamlContext::FlowIn | YamlContext::FlowOut => {
                // Flow contexts cannot have block entries
                !matches!(token, TokenType::BlockEntry)
            }
            YamlContext::BlockKey => {
                // Block key context has restricted character set
                !matches!(
                    token,
                    TokenType::FlowSequenceStart | TokenType::FlowMappingStart
                )
            }
            _ => true,
        }
    }

    /// Finalize a YamlBuilder into a Yaml value
    fn finalize_builder(&self, builder: YamlBuilder) -> Yaml {
        match builder {
            YamlBuilder::Sequence(items) => Yaml::Array(items),
            YamlBuilder::Mapping(map, _) => Yaml::Hash(map),
            YamlBuilder::Scalar(s) => Yaml::String(s),
        }
    }

    /// Handle directive header processing
    fn handle_directive_header(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::VersionDirective(major, minor) => {
                self.process_yaml_directive(*major, *minor)?;
                self.scanner.fetch_token(); // consume
                Ok(())
            }
            TokenType::TagDirective(handle, prefix) => {
                self.process_tag_directive(handle.clone(), prefix.clone())?;
                self.scanner.fetch_token(); // consume
                Ok(())
            }
            TokenType::DocumentStart => {
                self.scanner.fetch_token(); // consume
                self.state = State::DocumentContent;
                Ok(())
            }
            TokenType::StreamEnd => {
                self.state = State::End;
                Ok(())
            }
            _ => {
                // Implicit document start
                self.state = State::DocumentContent;
                Ok(())
            }
        }
    }

    /// Handle document end processing
    const fn handle_document_end(&mut self) -> Result<(), ScanError> {
        self.state = State::NextDocument;
        Ok(())
    }

    /// Handle next document processing
    fn handle_next_document(&mut self) -> Result<(), ScanError> {
        let token = self.scanner.peek_token()?;
        match &token.1 {
            TokenType::StreamEnd => {
                self.state = State::End;
                Ok(())
            }
            _ => {
                // Start processing next document
                self.state = State::DirectiveHeader;
                Ok(())
            }
        }
    }

    /// Process YAML version directive
    fn process_yaml_directive(&mut self, major: u32, minor: u32) -> Result<(), ScanError> {
        // Validate YAML version
        if major != 1 || (minor != 1 && minor != 2) {
            return Err(ScanError::new(
                self.scanner.mark(),
                &format!("Unsupported YAML version: {}.{}", major, minor)
            ));
        }

        // Store for document processing
        self.yaml_version = Some((major, minor));
        Ok(())
    }

    /// Process TAG directive
    fn process_tag_directive(&mut self, handle: String, prefix: String) -> Result<(), ScanError> {
        // Register tag handle for document scope
        self.tag_handles.insert(handle, prefix);
        Ok(())
    }

    fn handle_parametric_block_sequence(&mut self) -> Result<(), ScanError> {
        let n = self.context.current_indent();
        let entry_indent = (n + 1) as usize;
        self.context.push_context(YamlContext::BlockIn, entry_indent as i32);
        loop {
            let peeked_indent = self.scanner.peek_line_indent()?;
            if peeked_indent < entry_indent {
                break;
            }
            if peeked_indent != entry_indent {
                return Err(ScanError::new(self.scanner.mark(), "Invalid indentation for sequence entry"));
            }
            for _ in 0..entry_indent {
                self.scanner.consume_char()?;
            }
            let token = self.scanner.peek_token()?;
            self.scanner.fetch_token(); // consume
            if !matches!(token.1, TokenType::BlockEntry) {
                return Err(ScanError::new(token.0, "Expected - for sequence entry"));
            }
            self.scanner.process_structural_separation(&mut self.context, entry_indent as i32)?;
            let next_token = self.scanner.peek_token()?;
            match next_token.1 {
                TokenType::Scalar(_, _) | TokenType::FlowSequenceStart | TokenType::FlowMappingStart | TokenType::Key => {
                    // Compact
                    self.handle_block_node()?;
                }
                _ => {
                    // Not compact
                    let value_indent = self.scanner.peek_line_indent()?;
                    if value_indent > entry_indent {
                        self.context.push_context(YamlContext::BlockIn, value_indent as i32);
                        self.handle_block_node()?;
                        self.context.pop_context();
                    } else {
                        if let Some(YamlBuilder::Sequence(items)) = self.ast_stack.last_mut() {
                            items.push(Yaml::Null);
                        }
                        continue;
                    }
                }
            }
            let value = if let Some(builder) = self.ast_stack.pop() {
                self.finalize_builder(builder)
            } else {
                Yaml::Null
            };
            if let Some(YamlBuilder::Sequence(items)) = self.ast_stack.last_mut() {
                items.push(value);
            }
        }
        self.context.pop_context();
        Ok(())
    }

    /// Check if at stream end
    pub fn at_stream_end(&self) -> bool {
        self.state == State::End
    }

    /// Parse next document from stream
    pub fn parse_next_document(&mut self) -> Result<Option<Yaml>, ScanError> {
        // If we're at NextDocument from a previous parse, transition to start next document
        if self.state == State::NextDocument {
            self.handle_next_document()?;
        }

        // Check if already at end (could have transitioned to End in handle_next_document)
        if self.at_stream_end() {
            return Ok(None);
        }

        // Reset document-level state
        self.yaml_version = None;
        self.tag_handles.clear();
        self.anchors.clear();
        self.anchor_id = 1;
        self.ast_stack.clear();

        // Parse until we reach DocumentEnd or stream end
        while self.state != State::End && self.state != State::NextDocument {
            self.execute_state()?;
        }

        // Return constructed document
        if let Some(builder) = self.ast_stack.pop() {
            Ok(Some(self.finalize_builder(builder)))
        } else if self.state == State::End {
            Ok(None) // End of stream
        } else {
            // Empty document
            Ok(Some(Yaml::Null))
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_scalar_parsing() {
        let yaml = r#"
outer:
  - item1  # BLOCK-IN at indent 2
  - key: value  # BLOCK-KEY at 4, BLOCK-IN at 7
    flow: [a, b]  # FLOW-IN inside []
"#;

        let mut sm = StateMachine::new(yaml.chars());

        // Parse and check context at various points
        // This would require adding debug hooks or inspection methods
        let result = sm.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_context_validation() {
        // Test context initialization
        let yaml = "key: value";
        let mut sm = StateMachine::new(yaml.chars());

        // Verify ParametricContext is initialized properly
        assert_eq!(sm.context.current_context, YamlContext::BlockOut);
        assert_eq!(sm.context.current_indent(), 0);

        let result = sm.parse();
        assert!(result.is_ok());
    }
}
