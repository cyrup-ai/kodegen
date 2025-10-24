# Task: Create Document Stream and Schema Support Module

## Description
Create new `src/parser/document_stream.rs` module implementing YAML 1.2 document stream productions [202]-[211] and schema support (Failsafe, JSON, Core) for multi-document streams.

## Target Files
- **Primary**: `src/parser/document_stream.rs` (new file)
- **Secondary**: `src/parser/mod.rs` (add module import)
- **Integration**: `src/parser/document.rs` (integrate multi-document support)

## Success Criteria
- [ ] Document stream productions [202]-[211] fully implemented
- [ ] l-document-prefix with BOM and comment support
- [ ] Document marker productions (---, ..., forbidden patterns)
- [ ] l-any-document and l-yaml-stream productions
- [ ] Directive document support with YAML version and TAG directives
- [ ] Schema support implementation (Failsafe, JSON, Core schemas)
- [ ] Multi-document stream parsing capability
- [ ] Integration with existing document parsing in parser/document.rs
- [ ] BOM placement validation and encoding detection

## Implementation Notes
- **Architecture**: New module for document-level stream handling
- **Multi-Document**: Support for parsing multiple documents in single stream
- **Schema Support**: Implement the three standard YAML schemas
- **Directive Handling**: YAML version and TAG directive processing
- **Integration**: Work with existing document.rs without breaking functionality

## Dependencies
- **Requires**: Milestone 2 completion (Style Systems)
- **Specifically**: Flow and block parsing fully functional
- **Specifically**: All core productions working for document parsing

## Complexity Estimate
**Medium-High** - Multi-document stream handling and schema implementation

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required
- Never use unwrap() or expect() in src/*
- Preserve zero-allocation optimizations using Cow<str>