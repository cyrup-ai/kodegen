# Task: Complete Structural Productions Module

## Description
Complete the existing `src/parser/structural_productions.rs` module implementing YAML 1.2 structural productions [63]-[81] with parametric indentation, line folding, comments, and separation handling.

The module has been implemented using existing infrastructure including ParametricContext for indentation tracking, delegation to character productions and scanner utilities, and integration with the state machine.

## Target Files
- **Primary**: `src/parser/structural_productions.rs` (already implemented - [link](../src/parser/structural_productions.rs))
- **Secondary**: `src/parser/mod.rs` (module import already present - [link](../src/parser/mod.rs))
- **Integration**: State machine integration in `src/parser/state_machine.rs` (structural operations called during block parsing - [link](../src/parser/state_machine.rs))

## Success Criteria
- [x] Structural productions [63]-[81] fully implemented
- [x] s-line-prefix, s-block-line-prefix, s-flow-line-prefix productions
- [x] s-indent(n), s-indent-less-than(n), s-indent-less-or-equal(n) parametric productions
- [x] l-empty(n,c) empty line handling with context parameters
- [x] Line folding productions (b-l-trimmed, b-as-space, b-l-folded, s-flow-folded)
- [x] Comment productions [75]-[79] (c-nb-comment-text, b-comment, etc.)
- [x] Separation productions s-separate(n,c), s-separate-lines(n)
- [x] Integration with existing indentation tracking via ParametricContext

## Implementation Details

### Core Patterns Demonstrated

**Parametric Indentation using Existing Context:**
```rust
pub fn validate_exact_indent<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &ParametricContext,
    n: i32,
) -> Result<bool, ScanError> {
    // First check context for cached indentation
    let current_indent = context.current_indent();
    if current_indent != n {
        return Ok(false);
    }
    // Validate that the next n characters are spaces
    // ... validation logic
}
```

**Context-Aware Line Prefix Processing:**
```rust
pub fn process_line_prefix<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
    context: &mut ParametricContext,
    n: i32,
) -> Result<(), ScanError> {
    match context.current_context {
        YamlContext::BlockOut | YamlContext::BlockIn => {
            Self::process_block_line_prefix(state, context, n)
        }
        YamlContext::FlowOut | YamlContext::FlowIn => Self::process_flow_line_prefix(state, context, n),
        _ => Ok(()),
    }
}
```

**Delegation to Existing Line Folding:**
```rust
#[must_use] 
pub fn apply_line_folding(
    lines: &[String],
    chomping: ChompingMode,
    literal_style: bool,
) -> String {
    // DELEGATE to existing line folding in scalars.rs
    crate::scanner::scalars::apply_block_scalar_folding(lines, chomping, literal_style)
}
```

**Reusing Comment Parsing Infrastructure:**
```rust
pub fn parse_comment_text<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<String, ScanError> {
    if state.peek_char()? != '#' {
        return Err(ScanError::new(state.mark(), "expected comment marker '#'"));
    }
    state.consume_char()?; // Consume '#'
    let mut comment = String::new();
    // ... parse comment using existing character validation
}
```

### State Machine Integration

The structural productions are integrated into the parsing pipeline through the state machine's block content handling:

**Block Content Processing with Structural Operations:**
```rust
fn handle_block_content_with_structure(&mut self) -> Result<(), ScanError> {
    // Process structural separation using parametric context
    let current_indent = self.context.current_indent();
    self.scanner
        .process_structural_separation(&mut self.context, current_indent)?;

    // Skip any comments using structural productions
    let _comments = self.scanner.skip_structural_comments()?;

    // Continue with existing block node parsing
    self.handle_block_node()
}
```

**State Machine Dispatch:**
```rust
match self.state {
    // ... other states ...
    State::BlockNode => self.handle_block_content_with_structure(),
    // ... other states ...
}
```

This integration ensures that:
- Structural separation is validated before parsing block content
- Comments are properly skipped using structural productions
- Indentation tracking remains consistent with parametric context
- Existing block parsing logic continues to work without modification

## Implementation Notes
- **Architecture**: Module integrates with existing grammar and state machine without duplication
- **Parametric Indentation**: Full support for s-indent(n) using cached context values
- **Line Folding**: Complete delegation to existing scalar folding behavior
- **Comments**: Comprehensive reuse of existing comment parsing utilities
- **Integration**: Works with existing block and flow parsing without breaking functionality

## Dependencies
- **Requires**: Milestone 0 completion (Foundation Systems) - satisfied
- **Specifically**: character_productions.rs for character validation - used
- **Specifically**: parametric grammar and state machine support - integrated

## Complexity Estimate
**High** - Complex parametric indentation handling and line folding behavior - completed

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required - no changes needed
- Never use unwrap() or expect() in src/* - complied
- Preserve zero-allocation optimizations using Cow<str> - maintained

## Definition of Done
The structural productions module is fully implemented and integrated with the existing YAML parser infrastructure. All productions [63]-[81] are available through the StructuralProductions struct and ScannerState extension methods, using existing parametric context and character validation systems without code duplication. The module is actively used in the state machine's block content processing for proper structural validation and comment handling.