# Task: Implement Core RFC Compliance Tests

## Description
Implement comprehensive test coverage for character productions, structural productions, flow style, and block style in the RFC compliance test suite.

## Target Files
- **Primary**: `tests/rfc_compliance/ch05_character_productions/test_*.rs`
- **Primary**: `tests/rfc_compliance/ch06_structural_productions/test_*.rs`  
- **Primary**: `tests/rfc_compliance/ch07_flow_style/test_*.rs`
- **Primary**: `tests/rfc_compliance/ch08_block_style/test_*.rs`

## Success Criteria
- [ ] Character production tests: test_5_1_character_set.rs, test_5_2_character_encodings.rs, test_5_4_line_break_characters.rs, test_5_5_white_space_characters.rs, test_5_6_miscellaneous_characters.rs, test_5_7_escaped_characters.rs
- [ ] Structural production tests: test_6_1_indentation_spaces.rs, test_6_2_separation_spaces.rs, test_6_5_line_folding.rs, test_6_6_comments.rs, test_6_8_directives.rs
- [ ] Flow style tests: test_7_3_flow_scalar_styles.rs, test_7_4_flow_collection_styles.rs with parametric context testing
- [ ] Block style tests: test_8_1_block_scalar_styles.rs, test_8_2_block_collection_styles.rs with chomping and indentation testing
- [ ] All YAML 1.2 spec examples from these sections included as test cases
- [ ] Parametric production testing with different contexts (BlockIn/Out/Key, FlowIn/Out/Key)
- [ ] Edge cases and error conditions covered

## Implementation Notes
- **Test Coverage**: Every spec example must have corresponding test case
- **Parametric Testing**: Test same production with different context parameters
- **Error Testing**: Include invalid input testing for comprehensive coverage
- **Integration**: Tests work with existing parser implementation
- **Documentation**: Test names and comments reference specific spec sections

## Dependencies
- **Can Run Parallel**: With implementation tracks (flow/block enhancements)
- **Requires**: Test infrastructure setup completed
- **Benefits From**: Implementation progress for integration testing

## Complexity Estimate
**High** - Comprehensive test coverage requires deep spec understanding and extensive test case creation

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required
- Use expect() in tests/* (allowed in test code)
- DO NOT use unwrap() in tests/* (still not allowed)