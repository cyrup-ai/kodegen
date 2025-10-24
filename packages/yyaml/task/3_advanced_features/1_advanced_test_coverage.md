# Task: Implement Advanced RFC Compliance Tests

## Description
Implement comprehensive test coverage for document stream productions and schema support, completing the RFC compliance test suite.

## Target Files
- **Primary**: `tests/rfc_compliance/ch09_document_stream/test_*.rs`
- **Primary**: `tests/rfc_compliance/ch10_schemas/test_*.rs`

## Success Criteria
- [ ] Document stream tests: test_9_1_documents.rs with document marker and directive tests
- [ ] Multi-document stream tests: test_9_2_streams.rs with multi-document stream parsing
- [ ] BOM placement tests and encoding detection tests
- [ ] Schema tests: test_10_1_failsafe_schema.rs with generic mapping/sequence/string tests  
- [ ] JSON schema tests: test_10_2_json_schema.rs with null/boolean/integer/float tests
- [ ] Core schema tests: test_10_3_core_schema.rs with core schema tag resolution tests
- [ ] Tag resolution and schema validation comprehensive testing
- [ ] All YAML 1.2 spec examples from document stream and schema sections
- [ ] Multi-document stream edge cases and error conditions
- [ ] Directive processing and validation testing

## Implementation Notes
- **Document Streams**: Comprehensive multi-document parsing validation
- **Schema Testing**: All three standard schemas (Failsafe, JSON, Core) fully tested
- **Tag Resolution**: Complete tag resolution testing across schemas
- **Error Cases**: Invalid document markers, malformed directives, schema violations
- **Integration**: Tests work with complete parser implementation

## Dependencies
- **Can Run Parallel**: With document stream module implementation
- **Requires**: Core test implementation progress for test utilities
- **Benefits From**: Document stream implementation for integration testing

## Complexity Estimate
**Medium** - Advanced testing requires understanding of multi-document streams and schemas

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required
- Use expect() in tests/* (allowed in test code)
- DO NOT use unwrap() in tests/* (still not allowed)