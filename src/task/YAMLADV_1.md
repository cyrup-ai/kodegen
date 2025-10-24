# Implementing Document Markers and Directives in YAML Parser (YAML 1.2 Compliance)

## Overview

This document outlines modifications to the YAML parser source code to support document markers (--- and ...) and directives (%YAML and %TAG) as per the YAML 1.2 specification. These features enable proper handling of multi-document streams and tag resolution. The focus is solely on source code changes in the parser (lexer and parser modules), without including tests or benchmarks.

Key concepts from the YAML 1.2 spec:
- **Document Markers**: --- indicates the start of an explicit document; ... signals the end of a document.
- **Directives**: %YAML specifies the YAML version (e.g., 1.2); %TAG associates handles with prefixes for shorthand tags.
- The parser must detect these to manage document boundaries, reset state, and handle tag shorthands.

Assumptions:
- The parser is implemented in Rust using a lexer (e.g., logos or nom) and a recursive descent parser.
- Existing structure: `lexer.rs` for tokenization, `parser.rs` for parsing AST.
- No changes to CLI or other non-core modules.

## Lexer Modifications (src/lexer.rs)

The lexer needs to recognize special tokens for markers and directives. Add rules for:
- Lines starting with '---' (DocumentStart).
- Lines starting with '...' (DocumentEnd).
- Lines starting with '%' (Directive), and parse them as YAML or TAG directives.

### Token Enum Updates

Extend the token enum to include new variants:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Existing tokens...
    DocumentStart,  // ---
    DocumentEnd,    // ...
    DirectiveStart, // %
    // Sub-tokens for directives if needed (e.g., YamlVersion, TagHandle, TagPrefix)
}
```

### Lexer Rules

In the lexer's pattern matching (assuming logos or similar):

- Pattern for DocumentStart: `^---\s*$` -> emit Token::DocumentStart
- Pattern for DocumentEnd: `^\.\.\.\s*$` -> emit Token::DocumentEnd
- Pattern for Directive: `^%\s*(YAML|TAG)\s+` -> emit Token::DirectiveStart, followed by parsing the rest (e.g., version or handle/prefix).

For directives, parse inline:
- For %YAML: `^%YAML\s+1\.2\s*$` -> Token::YamlDirective(Version(1,2))
- For %TAG: `^%TAG\s+(![!]?)(\S+)\s+(\S+)\s*$` -> Token::TagDirective(Handle(String), Prefix(String))

Update the lexer's main loop to handle these tokens and skip whitespace/comments after them.

Example snippet (using nom for parsing):

```rust
fn parse_directive(input: &str) -> IResult<&str, Directive> {
    alt((
        map(tag("%YAML "), parse_yaml_version),
        map(tag("%TAG "), parse_tag_directive),
    ))(input)
}

fn parse_yaml_version(input: &str) -> IResult<&str, Directive> {
    let (input, major) = digit1(input)?;
    let (input, _) = tag(".")(input)?;
    let (input, minor) = digit1(input)?;
    let version = (major.parse::<u32>().unwrap(), minor.parse::<u32>().unwrap());
    Ok((input, Directive::Yaml(version)))
}

fn parse_tag_directive(input: &str) -> IResult<&str, Directive> {
    let (input, handle) = take_while_matched(|c: char| c.is_alphanumeric() || c == '!' )(input)?;
    let (input, _) = multispace1(input)?;
    let (input, prefix) = take_until(|c: char| c.is_whitespace())(input)?;
    Ok((input, Directive::Tag(handle.to_string(), prefix.to_string())))
}
```

Integrate this into the lexer's scan_directive method.

## Parser Modifications (src/parser.rs)

### State Management

Add state to track:
- Current document context.
- Parsed directives (version and tag map).
- Whether a document is active.

Struct updates:

```rust
pub struct ParserState {
    pub directives: Vec<Directive>,  // Store parsed directives
    pub tag_prefixes: HashMap<String, String>,  // Handle -> Prefix map
    pub current_version: (u32, u32),  // Default to (1,1) or handle errors
    pub in_document: bool,
    // Other existing state...
}

impl ParserState {
    fn reset_document(&mut self) {
        self.directives.clear();
        self.tag_prefixes.clear();
        self.current_version = (1, 2);  // Assume 1.2 by default
        self.in_document = false;
    }
}
```

### Parsing Flow

In the parse_stream method:

1. **Handle Directives and Markers**:
   - When encountering Token::DirectiveStart, parse the directive.
     - For %YAML: Validate version (must be 1.2 or 1.1 with warnings). Store in state.
     - For %TAG: Parse handle and prefix, add to tag_prefixes map. Warn on duplicates.
   - When encountering Token::DocumentStart (---):
     - If not in a document, start new document: reset state, set in_document=true.
     - Parse the document content.
   - When encountering Token::DocumentEnd (...):
     - If in a document, end it: process any pending nodes, set in_document=false.

2. **Document Parsing**:
   - After ---, expect node properties and content until ... or EOF.
   - Use the tag_prefixes map for resolving shorthands in tag properties.
   - On document end, emit the parsed AST node for the document.

Example parser logic:

```rust
pub fn parse_stream(input: Span) -> Result<Vec<Document>, ParseError> {
    let mut state = ParserState::default();
    let mut documents = Vec::new();
    let mut iter = lexer::tokenize(input);

    while let Some(token) = iter.next().transpose()? {
        match token {
            Token::DocumentStart => {
                if state.in_document {
                    return Err(ParseError::UnexpectedDocumentStart);
                }
                state.reset_document();
                state.in_document = true;
                // Parse document nodes
                let doc = parse_document(&mut iter, &mut state)?;
                documents.push(doc);
            }
            Token::DocumentEnd => {
                if !state.in_document {
                    return Err(ParseError::UnexpectedDocumentEnd);
                }
                // Finalize current document if needed
                state.in_document = false;
            }
            Token::DirectiveStart => {
                // Parse directive only if before first document or after ...
                if state.in_document {
                    return Err(ParseError::DirectiveInDocument);
                }
                let directive = parse_directive(&mut iter)?;
                match directive {
                    Directive::Yaml(version) => {
                        if version != (1,2) && version != (1,1) {
                            warn!("Unsupported YAML version: {:?}", version);
                        }
                        state.current_version = version;
                    }
                    Directive::Tag(handle, prefix) => {
                        if state.tag_prefixes.insert(handle.clone(), prefix).is_some() {
                            warn!("Duplicate TAG directive for handle: {}", handle);
                        }
                    }
                }
            }
            _ => {
                // Handle other tokens or error if unexpected
            }
        }
    }

    if state.in_document {
        // Implicit end of last document
        state.in_document = false;
    }

    Ok(documents)
}

fn parse_directive(iter: &mut TokenIter) -> Result<Directive, ParseError> {
    // Consume tokens for the directive
    // Implementation based on lexer/parser logic
    // Return Directive enum
}
```

### Tag Resolution Integration

- In node parsing (parse_node), when encountering a tag property:
  - If it's a shorthand (e.g., !handle/suffix), lookup handle in tag_prefixes.
  - Append prefix + suffix for full tag URI.
  - If handle not found, error or use default resolution.

Update parse_tag method:

```rust
fn parse_tag(&mut self, input: &mut TokenIter) -> Result<Tag, ParseError> {
    // If shorthand: check self.state.tag_prefixes
    // Full URI: return as-is
    // Non-specific (! or ?): defer resolution
    Ok(Tag::Resolved(full_uri))
}
```

### Error Handling

- Emit warnings for version mismatches (e.g., log via tracing or log crate).
- Errors for invalid directives (e.g., duplicate YAML, invalid syntax).
- Graceful handling: Skip unknown directives as per spec.

## Integration with Existing Parser

- Wrap existing parse_node in a document parser triggered by ---.
- After parsing a document, process directives from the state.
- Ensure stream parsing handles multiple documents by collecting in a Vec<Document>.

This implementation ensures compliance with YAML 1.2 for document streams, allowing multi-document parsing and tag shorthand resolution.

## References

- YAML 1.2 Specification: Chapter 9 - YAML Character Stream
- Section 6.8: Directives (YAML and TAG)
