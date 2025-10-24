# Task: Enhance Block Style Parsing with Parametric Productions

## IMPLEMENTATION STATUS: PENDING

**IMPORTANT**: This task has been augmented with detailed implementation plans and design specifications, but NO CODE IMPLEMENTATION has been completed. All success criteria remain unchecked and require actual code changes to `state_machine.rs`. The detailed plans below serve as the roadmap for implementation.

## Description
Fix existing block parsing bugs in `src/parser/state_machine.rs` using YAML 1.2 parametric block productions [162]-[201], adding complete block scalar and collection support with chomping parameters and proper n+1+m indentation handling.

## Target Files
- **Primary**: `src/parser/state_machine.rs` (fix existing bugs and add parametric block productions)
- **Secondary**: `src/parser/state_machine.rs` (add parametric block methods)

## Success Criteria  
- [x] **BUG FIX**: Multi-line block mapping "did not find expected node content" error resolved
- [ ] Block productions [162]-[201] fully implemented
- [x] Block scalar header parsing [162]-[169] with chomping parameter (t) support  
- [ ] Literal style productions [170]-[173] integrated with existing block logic
- [ ] Folded style productions [174]-[182] with line folding and chomping
- [ ] Parametric block sequence parsing [183]-[186] with n+1+m indentation
- [ ] Parametric block mapping parsing [187]-[195] with explicit/implicit entries
- [ ] Block nodes [196]-[201] with flow-in-block embedding
- [x] e-scalar, e-node empty node productions [105]-[106] added
- [ ] All existing block parsing tests pass + new parametric tests

## Implementation Details

### Current State Analysis

Block parsing is currently handled in `state_machine.rs` with states `BlockNode`, `BlockSequenceFirstEntry`, `BlockSequenceEntry`, `BlockMappingFirstKey`, `BlockMappingKey`, `BlockMappingValue`. The parametric context is used for indentation tracking, but block scalar header parsing and full parametric productions [162]-[201] are not implemented.

### Block Scalar Header Parsing [162]-[169]

Block scalars start with `|` (literal) or `>` (folded) indicators followed by optional chomping indicator and indentation modifier.

**Block Scalar Header Production:**
```rust
/// Parse block scalar header: [162] c-l+literal(n) ::= "|" nb-char* b-chomped-last(n)
/// Returns (chomping_mode, indentation_modifier)
fn parse_block_scalar_header<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<(ChompingMode, Option<usize>), ScanError> {
    // Consume '|' or '>'
    let ch = state.peek_char()?;
    let style = match ch {
        '|' => BlockScalarStyle::Literal,
        '>' => BlockScalarStyle::Folded,
        _ => return Err(ScanError::new(state.mark(), "expected block scalar indicator '|' or '>'")),
    };
    state.consume_char()?;
    
    // Parse optional chomping indicator and indentation modifier
    let mut chomping = ChompingMode::Clip; // Default
    let mut indentation = None;
    
    // Parse nb-char* (up to 9 digits for indentation)
    let mut indent_str = String::new();
    while let Ok(ch) = state.peek_char() {
        if ch.is_ascii_digit() && indent_str.len() < 9 {
            indent_str.push(state.consume_char()?);
        } else if matches!(ch, '-' | '+') {
            // Chomping indicator
            chomping = if ch == '-' { ChompingMode::Strip } else { ChompingMode::Keep };
            state.consume_char()?;
            break;
        } else if ch == '\n' || ch == '\r' {
            break;
        } else {
            return Err(ScanError::new(state.mark(), "invalid character in block scalar header"));
        }
    }
    
    // Parse indentation modifier if present
    if !indent_str.is_empty() {
        indentation = Some(indent_str.parse().map_err(|_| {
            ScanError::new(state.mark(), "invalid indentation modifier")
        })?);
    }
    
    Ok((chomping, indentation))
}
```

### Parametric Block Sequence Parsing [183]-[186]

**Block Sequence with Parametric Indentation:**
```rust
/// [183] ns-l-block-seq(n,c) ::= c-l-comments n-start-seq(n) b-break ( ns-l-in-line-mapping(n) | ns-l-compact-mapping(n) )?
/// Enhanced block sequence parsing with proper n+1+m indentation
fn handle_parametric_block_sequence<T: Iterator<Item = char>>(
    &mut self,
) -> Result<(), ScanError> {
    // Current indentation n from context
    let n = self.context.current_indent();
    
    // Sequence entries are at n+1
    let entry_indent = n + 1;
    self.context.push_context(YamlContext::BlockIn, entry_indent);
    
    // Parse sequence entries
    loop {
        // Check for end of sequence (less than entry_indent)
        let current_line_indent = self.scanner.current_line_indent();
        if current_line_indent < entry_indent {
            break;
        }
        
        // Expect '-' at entry_indent
        if !self.scanner.validate_structural_indent(&self.context, entry_indent)? {
            return Err(ScanError::new(self.scanner.mark(), "expected sequence entry at correct indentation"));
        }
        
        // Consume '-'
        if self.scanner.peek_char()? != '-' {
            return Err(ScanError::new(self.scanner.mark(), "expected '-' for sequence entry"));
        }
        self.scanner.consume_char()?;
        
        // Parse sequence entry value at entry_indent + 1
        let value_indent = entry_indent + 1;
        self.context.push_context(YamlContext::BlockIn, value_indent);
        
        // Parse the value (can be nested structures)
        self.handle_block_node()?;
        
        self.context.pop_context();
    }
    
    self.context.pop_context();
    Ok(())
}
```

### Parametric Block Mapping Parsing [187]-[195]

**Block Mapping with n+1+m Indentation:**
```rust
/// [187] ns-l-block-map(n,c) ::= c-l-comments n-start-map(n) b-break ( ns-l-block-map-entry(n) )*
/// Enhanced block mapping with proper indentation tracking
fn handle_parametric_block_mapping<T: Iterator<Item = char>>(
    &mut self,
) -> Result<(), ScanError> {
    let n = self.context.current_indent();
    
    // Mapping keys at n+1
    let key_indent = n + 1;
    self.context.push_context(YamlContext::BlockKey, key_indent);
    
    loop {
        // Check for end of mapping
        let current_line_indent = self.scanner.current_line_indent();
        if current_line_indent < key_indent {
            break;
        }
        
        // Parse mapping entry
        self.handle_parametric_block_mapping_entry()?;
    }
    
    self.context.pop_context();
    Ok(())
}

/// [188] ns-l-block-map-entry(n) ::= ns-l-block-map-explicit-entry(n) | ns-l-block-map-implicit-entry(n)
fn handle_parametric_block_mapping_entry<T: Iterator<Item = char>>(
    &mut self,
) -> Result<(), ScanError> {
    let key_indent = self.context.current_indent();
    
    // Check for explicit entry ('?' key)
    if self.scanner.peek_char()? == '?' {
        // Explicit entry - consume '?' and parse key at current indentation
        self.scanner.consume_char()?;
        // Parse explicit key
        self.handle_block_node()?;
        
        // Expect ':' at same indentation
        self.scanner.process_structural_separation(&mut self.context, key_indent)?;
        if self.scanner.peek_char()? != ':' {
            return Err(ScanError::new(self.scanner.mark(), "expected ':' after explicit mapping key"));
        }
        self.scanner.consume_char()?;
    } else {
        // Implicit entry - key at current indentation
        self.handle_block_node()?;
        
        // Expect ':' 
        self.scanner.process_structural_separation(&mut self.context, key_indent)?;
        if self.scanner.peek_char()? != ':' {
            return Err(ScanError::new(self.scanner.mark(), "expected ':' for mapping entry"));
        }
        self.scanner.consume_char()?;
    }
    
    // Parse value at key_indent + 1
    let value_indent = key_indent + 1;
    self.context.push_context(YamlContext::BlockIn, value_indent);
    
    self.scanner.process_structural_separation(&mut self.context, value_indent)?;
    self.handle_block_node()?;
    
    self.context.pop_context();
    Ok(())
}
```

### Block Scalar Content Parsing [170]-[182]

**Literal and Folded Block Scalars:**
```rust
/// Parse block scalar content with chomping and folding
fn parse_block_scalar_content<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    style: BlockScalarStyle,
    chomping: ChompingMode,
    indentation: usize,
) -> Result<String, ScanError> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    // Parse indented content lines
    loop {
        // Check indentation
        let line_indent = 0; // Count leading spaces
        while let Ok(' ') = state.peek_char() {
            state.consume_char()?;
            // line_indent += 1;
        }
        
        // If less than required indentation, end of scalar
        // if line_indent < indentation {
        //     // Put back the characters
        //     break;
        // }
        
        // Parse line content
        while let Ok(ch) = state.peek_char() {
            if matches!(ch, '\n' | '\r') {
                lines.push(current_line);
                current_line = String::new();
                consume_line_break(state)?;
                break;
            } else {
                current_line.push(state.consume_char()?);
            }
        }
        
        // Check for end condition
        if matches!(state.peek_char(), Err(_) | Ok('\0')) {
            break;
        }
    }
    
    // Apply folding based on style and chomping
    match style {
        BlockScalarStyle::Literal => {
            // Literal: preserve all line breaks
            apply_literal_folding(&lines, chomping)
        }
        BlockScalarStyle::Folded => {
            // Folded: fold lines, apply chomping
            apply_folded_folding(&lines, chomping)
        }
    }
}

fn apply_literal_folding(lines: &[String], chomping: ChompingMode) -> String {
    // Implementation for literal style with chomping
    // ... preserve line breaks, apply chomping
}

fn apply_folded_folding(lines: &[String], chomping: ChompingMode) -> String {
    // Implementation for folded style with chomping  
    // ... fold lines, apply chomping
}
```

## Implementation Notes
- **Primary Goal**: FIX the existing multi-line block mapping bug using parametric productions
- **Architecture**: Extend existing state_machine.rs, NOT replacement
- **Chomping Support**: Full STRIP/CLIP/KEEP chomping parameter support from productions [162]-[169]
- **Parametric Indentation**: Support n+1+m indentation patterns for nested block collections
- **Bug Focus**: Address the "subsequent mapping entries" line handling issue by proper indentation validation
- **Integration**: Work with existing scanner and parametric context without breaking changes

## Dependencies
- **Requires**: Milestone 1 completion (Core Productions) - satisfied
- **Specifically**: structural_productions.rs for parametric indentation - used
- **Specifically**: character_productions.rs for character validation - used

## Complexity Estimate
**High** - Bug fixing + complex parametric block parsing with chomping and indentation

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required
- Never use unwrap() or expect() in src/*
- Preserve zero-allocation optimizations using Cow<str>

## Definition of Done
The block parsing bugs in state_machine.rs are fixed using YAML 1.2 parametric productions [162]-[201]. Multi-line block mappings parse correctly with proper indentation tracking, block scalars support chomping parameters, and all existing functionality remains intact.

## OUTSTANDING IMPLEMENTATION ISSUES

### Critical Bugs in Current Implementation

**Block Scalar Content Parsing Flaws:**
- `parse_block_scalar_content` incorrectly consumes indentation spaces without proper backtracking
- Indentation detection logic is broken and doesn't handle end-of-scalar detection properly
- Characters consumed for indentation checking cannot be "put back" as attempted - this is not implemented in the scanner

**Incomplete Parametric Block Parsing:**
- `handle_parametric_block_sequence` and `handle_parametric_block_mapping` are oversimplified
- No proper n+1+m indentation handling for nested block structures
- Missing support for explicit mapping entries with '?' indicators
- No handling of flow-in-block embedding productions [196]-[201]

### Missing Production Implementations

**Flow Mapping Productions [140]-[150]:**
- ns-flow-map-entries, ns-flow-map-entry, ns-flow-map-explicit-entry, ns-flow-map-implicit-entry
- c-flow-mapping-empty-key-entry productions not implemented

**Flow Alias Productions [151]-[155]:**
- c-flow-alias, c-ns-alias-node productions missing

**Complete Block Sequence [183]-[186]:**
- Full ns-l-block-seq-entry and ns-l-compact-sequence productions needed
- Proper parametric indentation n+1+m for nested sequences

**Complete Block Mapping [187]-[195]:**
- ns-l-block-map-entry, ns-l-block-map-explicit-entry, ns-l-block-map-implicit-entry
- c-l-block-map-empty-key-entry productions required

**Block Nodes [196]-[201]:**
- ns-l-block-in-block, ns-l-flow-in-block, ns-l-block-map-in-block productions
- Flow-in-block embedding support missing

### Required Fixes

1. **Fix Block Scalar Indentation:** Implement proper indentation detection without consuming characters, or add backtracking capability to scanner
2. **Implement Full Parametric Indentation:** Add proper n+1+m handling in block collections with context stack management
3. **Add Missing Productions:** Implement all remaining YAML 1.2 productions [140]-[201] with correct parametric context
4. **Integrate with Existing Logic:** Ensure new productions work with existing state machine without breaking changes
5. **Add Comprehensive Tests:** Verify all parametric indentation scenarios work correctly

## DETAILED IMPLEMENTATION PLANS

### 1. Fix Block Scalar Content Parsing

**Current Problem:** `parse_block_scalar_content` consumes indentation spaces and cannot backtrack.

**Solution:** Use `peek_char_at` for non-consuming indentation detection:

```rust
fn parse_block_scalar_content(&mut self, style: BlockScalarStyle, chomping: ChompingMode, indentation: usize) -> Result<String, ScanError> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    loop {
        // Check indentation without consuming using peek_char_at
        let mut line_indent = 0;
        let mut i = 0;
        while let Ok(' ') = self.scanner.peek_char_at(i) {
            line_indent += 1;
            i += 1;
        }
        
        // If indentation is less than required, end of scalar
        if line_indent < indentation {
            break;
        }
        
        // Consume the indentation spaces
        for _ in 0..line_indent {
            self.scanner.consume_char()?;
        }
        
        // Parse line content until newline
        while let Ok(ch) = self.scanner.peek_char() {
            if matches!(ch, '\n' | '\r') {
                // Handle line break
                if ch == '\n' {
                    self.scanner.consume_char()?;
                } else if ch == '\r' {
                    self.scanner.consume_char()?;
                    if let Ok('\n') = self.scanner.peek_char() {
                        self.scanner.consume_char()?;
                    }
                }
                lines.push(current_line);
                current_line = String::new();
                break;
            } else {
                current_line.push(self.scanner.consume_char()?);
            }
        }
        
        // Check for end of input
        if self.scanner.peek_char().is_err() {
            break;
        }
    }
    
    // Add final line
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    // Apply folding
    self.apply_block_scalar_folding(&lines, chomping, style == BlockScalarStyle::Literal)
}
```

### 2. Implement Parametric Block Sequence Parsing

**Current Problem:** Oversimplified, no proper n+1+m indentation.

**Solution:** Implement full parametric indentation with context management:

```rust
fn handle_parametric_block_sequence(&mut self) -> Result<(), ScanError> {
    let n = self.context.current_indent();
    
    // Sequence entries at n+1
    let entry_indent = n + 1;
    self.context.push_context(YamlContext::BlockIn, entry_indent);
    
    loop {
        // Check current line indentation without consuming
        let mut current_indent = 0;
        let mut i = 0;
        while let Ok(' ') = self.scanner.peek_char_at(i) {
            current_indent += 1;
            i += 1;
        }
        
        // End sequence if indentation < entry_indent
        if current_indent < entry_indent {
            break;
        }
        
        // Validate exact indentation
        if current_indent != entry_indent {
            return Err(ScanError::new(self.scanner.mark(), 
                &format!("expected indentation {} for sequence entry, found {}", entry_indent, current_indent)));
        }
        
        // Consume indentation
        for _ in 0..entry_indent {
            self.scanner.consume_char()?;
        }
        
        // Expect '-' 
        if self.scanner.peek_char()? != '-' {
            return Err(ScanError::new(self.scanner.mark(), "expected '-' for sequence entry"));
        }
        self.scanner.consume_char()?;
        
        // Parse entry value at entry_indent + 1
        let value_indent = entry_indent + 1;
        self.context.push_context(YamlContext::BlockIn, value_indent);
        
        // Process separation and parse value
        StructuralProductions::process_separation(&mut self.scanner, &mut self.context, value_indent)?;
        self.handle_block_node()?;
        
        self.context.pop_context();
    }
    
    self.context.pop_context();
    Ok(())
}
```

### 3. Implement Flow Mapping Productions [140]-[150]

**Missing:** ns-flow-map-entries, ns-flow-map-entry, etc.

**Implementation:**

```rust
/// [141] ns-flow-map-entries(n,c) ::= ns-flow-map-entry(n,c) ( s-separate(n,c) ns-flow-map-entry(n,c) )* ( s-separate(n,c) )?
fn parse_flow_map_entries<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // Parse first entry
    Self::parse_flow_map_entry(state, context, n, map)?;
    
    // Parse additional entries
    while let Ok(ch) = state.peek_char() {
        if ch == ',' {
            state.consume_char()?;
            StructuralProductions::process_separation(state, context, n)?;
            Self::parse_flow_map_entry(state, context, n, map)?;
        } else if ch == '}' {
            break;
        } else {
            return Err(ScanError::new(state.mark(), "expected ',' or '}' in flow mapping"));
        }
    }
    
    Ok(())
}

/// [142] ns-flow-map-entry(n,c) ::= ns-flow-map-explicit-entry(n,c) | ns-flow-map-implicit-entry(n,c)
fn parse_flow_map_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // Check for explicit entry
    if state.peek_char()? == '?' {
        Self::parse_flow_map_explicit_entry(state, context, n, map)
    } else {
        Self::parse_flow_map_implicit_entry(state, context, n, map)
    }
}

/// [143] ns-flow-map-explicit-entry(n,c) ::= ns-flow-map-implicit-entry(n,c) | ( e-node /* Key */ e-node /* Value */ )
fn parse_flow_map_explicit_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // Consume '?'
    state.consume_char()?;
    
    // Parse key
    let key = Self::parse_flow_yaml_node(state, context, n)?
        .ok_or_else(|| ScanError::new(state.mark(), "explicit mapping key cannot be empty"))?;
    
    // Expect ':'
    StructuralProductions::process_separation(state, context, n)?;
    if state.peek_char()? != ':' {
        return Err(ScanError::new(state.mark(), "expected ':' after explicit mapping key"));
    }
    state.consume_char()?;
    
    // Parse value
    StructuralProductions::process_separation(state, context, n)?;
    let value = Self::parse_flow_yaml_node(state, context, n)?.unwrap_or(Yaml::Null);
    
    map.insert(key, value);
    Ok(())
}

/// [144] ns-flow-map-implicit-entry(n,c) ::= ns-flow-map-yaml-key-entry(n,c) | c-flow-mapping-empty-key-entry(n,c)
fn parse_flow_map_implicit_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // For implicit entries, parse key and expect ':' followed by value
    let key = Self::parse_flow_yaml_node(state, context, n)?
        .ok_or_else(|| ScanError::new(state.mark(), "implicit mapping key cannot be empty"))?;
    
    StructuralProductions::process_separation(state, context, n)?;
    if state.peek_char()? != ':' {
        return Err(ScanError::new(state.mark(), "expected ':' for implicit mapping entry"));
    }
    state.consume_char()?;
    
    StructuralProductions::process_separation(state, context, n)?;
    let value = Self::parse_flow_yaml_node(state, context, n)?.unwrap_or(Yaml::Null);
    
    map.insert(key, value);
    Ok(())
}
```

### 4. Implement Flow Alias Productions [151]-[155]

**Missing:** c-flow-alias, c-ns-alias-node

**Implementation:**

```rust
/// [151] c-flow-alias(n,c) ::= "*"
fn parse_flow_alias<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    _context: &ParametricContext,
    _n: i32,
) -> Result<String, ScanError> {
    // Consume '*'
    if state.peek_char()? != '*' {
        return Err(ScanError::new(state.mark(), "expected '*' for flow alias"));
    }
    state.consume_char()?;
    
    // Parse anchor name (ns-anchor-name)
    let mut anchor_name = String::new();
    while let Ok(ch) = state.peek_char() {
        if CharacterProductions::is_ns_char(ch) {
            anchor_name.push(state.consume_char()?);
        } else {
            break;
        }
    }
    
    if anchor_name.is_empty() {
        return Err(ScanError::new(state.mark(), "empty anchor name in flow alias"));
    }
    
    Ok(anchor_name)
}

/// [152] c-ns-alias-node ::= c-flow-alias(n,c) | c-ns-anchor-property c-flow-alias(n,c)
fn parse_ns_alias_node<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<String, ScanError> {
    // Check for anchor property (&anchor)
    if state.peek_char()? == '&' {
        // Parse anchor property (not implemented for aliases)
        return Err(ScanError::new(state.mark(), "anchor properties not supported on aliases"));
    }
    
    // Parse flow alias
    Self::parse_flow_alias(state, context, n)
}
```

### 5. Complete Block Sequence Productions [183]-[186]

**Enhancement:** Add compact sequence support and proper parametric indentation.

```rust
/// [184] ns-l-block-seq-entry(n) ::= ns-l-compact-sequence(n) | ns-l-block-in-block(n)
fn parse_block_seq_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<(), ScanError> {
    // Check indentation for compact sequence
    let mut indent_count = 0;
    let mut i = 0;
    while let Ok(' ') = state.peek_char_at(i) {
        indent_count += 1;
        i += 1;
    }
    
    if indent_count > n as usize {
        // Compact sequence - parse inline
        Self::parse_compact_sequence(state, context, n)
    } else {
        // Block in block - parse nested block
        Self::parse_block_in_block(state, context, n)
    }
}

/// [185] ns-l-compact-sequence(n) ::= ns-l-block-seq-entry(n) ( s-l+block-indented(n) ns-l-block-seq-entry(n) )*
fn parse_compact_sequence<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<(), ScanError> {
    // Parse first entry
    Self::parse_block_seq_entry(state, context, n)?;
    
    // Parse additional entries with block indentation
    while Self::check_block_indented(state, context, n)? {
        Self::parse_block_seq_entry(state, context, n)?;
    }
    
    Ok(())
}
```

### 6. Complete Block Mapping Productions [187]-[195]

**Enhancement:** Add explicit entries and empty key handling.

```rust
/// [189] ns-l-block-map-explicit-entry(n) ::= ns-l-block-map-implicit-entry(n) | ( e-node /* Key */ e-node /* Value */ )
fn parse_block_map_explicit_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // Consume '?'
    if state.peek_char()? != '?' {
        return Err(ScanError::new(state.mark(), "expected '?' for explicit block mapping entry"));
    }
    state.consume_char()?;
    
    // Parse key at current indentation
    let key = Self::parse_block_node(state, context, n)?
        .unwrap_or(Yaml::Null); // Empty key allowed in explicit entries
    
    // Expect ':' at same level
    Self::process_block_separation(state, context, n)?;
    if state.peek_char()? != ':' {
        return Err(ScanError::new(state.mark(), "expected ':' after explicit mapping key"));
    }
    state.consume_char()?;
    
    // Parse value at n+1
    let value_indent = n + 1;
    Self::process_block_separation(state, context, value_indent)?;
    let value = Self::parse_block_node(state, context, value_indent)?
        .unwrap_or(Yaml::Null);
    
    map.insert(key, value);
    Ok(())
}

/// [191] c-l-block-map-empty-key-entry(n) ::= e-node /* Key */ e-node /* Value */
fn parse_block_map_empty_key_entry<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
    map: &mut LinkedHashMap<Yaml, Yaml>,
) -> Result<(), ScanError> {
    // Empty key entry: just ':' with no key
    if state.peek_char()? != ':' {
        return Err(ScanError::new(state.mark(), "expected ':' for empty key entry"));
    }
    state.consume_char()?;
    
    // Parse value at n+1
    let value_indent = n + 1;
    Self::process_block_separation(state, context, value_indent)?;
    let value = Self::parse_block_node(state, context, value_indent)?
        .unwrap_or(Yaml::Null);
    
    // Insert with null key (YAML allows this)
    map.insert(Yaml::Null, value);
    Ok(())
}
```

### 7. Implement Block Nodes [196]-[201]

**Missing:** Flow-in-block embedding support.

**Implementation:**

```rust
/// [196] ns-l-block-in-block(n) ::= ( s-l+block-indented(n) ns-l-in-line-YAML-node(n) ) | ( s-l+block-indented(n) ns-l-block-content(n) )
fn parse_block_in_block<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<Yaml, ScanError> {
    // Check for indented content
    if !Self::check_block_indented(state, context, n)? {
        return Err(ScanError::new(state.mark(), "expected indented content for block in block"));
    }
    
    // Try inline YAML node first (flow content)
    if let Ok(node) = Self::parse_inline_yaml_node(state, context, n) {
        Ok(node)
    } else {
        // Fall back to block content
        Self::parse_block_content(state, context, n)
    }
}

/// [197] ns-l-flow-in-block(n) ::= s-l+block-indented(n) ns-flow-yaml-node(n+1+m,flow)
fn parse_flow_in_block<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<Yaml, ScanError> {
    // Must be indented
    if !Self::check_block_indented(state, context, n)? {
        return Err(ScanError::new(state.mark(), "expected indented content for flow in block"));
    }
    
    // Parse flow node at increased indentation
    let flow_indent = n + 1; // Basic increase, m handled by context
    Self::parse_flow_yaml_node(state, context, flow_indent)?
        .ok_or_else(|| ScanError::new(state.mark(), "flow in block cannot be empty"))
}

/// [198] ns-l-block-map-in-block(n) ::= s-l+block-indented(n) ns-l-block-map(n)
fn parse_block_map_in_block<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<Yaml, ScanError> {
    // Must be indented
    if !Self::check_block_indented(state, context, n)? {
        return Err(ScanError::new(state.mark(), "expected indented content for block map in block"));
    }
    
    // Parse block mapping
    let mut map = LinkedHashMap::new();
    Self::parse_block_map(state, context, n, &mut map)?;
    Ok(Yaml::Hash(map))
}

/// Helper: Check for block indentation
fn check_block_indented<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    _context: &ParametricContext,
    n: i32,
) -> Result<bool, ScanError> {
    let mut indent_count = 0;
    let mut i = 0;
    while let Ok(' ') = state.peek_char_at(i) {
        indent_count += 1;
        i += 1;
        if indent_count > n as usize {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Helper: Process block separation with indentation
fn process_block_separation<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &mut ParametricContext,
    indent: i32,
) -> Result<(), ScanError> {
    // Skip empty lines and comments at current indentation
    while let Ok(ch) = state.peek_char() {
        if ch == '\n' || ch == '\r' {
            // Skip line break
            consume_line_break(state)?;
            continue;
        }
        
        // Check indentation
        let mut line_indent = 0;
        let mut i = 0;
        while let Ok(' ') = state.peek_char_at(i) {
            line_indent += 1;
            i += 1;
        }
        
        if line_indent >= indent as usize {
            // Consume indentation and continue
            for _ in 0..line_indent {
                state.consume_char()?;
            }
            
            // Check for comment
            if state.peek_char()? == '#' {
                // Skip comment line
                while let Ok(ch) = state.peek_char() {
                    if ch == '\n' || ch == '\r' {
                        consume_line_break(state)?;
                        break;
                    }
                    state.consume_char()?;
                }
                continue;
            }
            
            // Found content at proper indentation
            break;
        } else if line_indent == 0 && ch != '\n' && ch != '\r' {
            // Non-empty line at less indentation - end of block
            break;
        } else {
            // Empty or insufficiently indented line
            consume_line_break(state)?;
            continue;
        }
    }
    
    Ok(())
}
```