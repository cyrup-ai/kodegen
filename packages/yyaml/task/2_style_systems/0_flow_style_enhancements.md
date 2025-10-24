# Task: Enhance Flow Style Parsing with Parametric Productions

## Description
Extend existing `src/parser/flow.rs` with missing YAML 1.2 flow productions [105]-[150], adding complete flow scalar and collection support with parametric context handling.

## Target Files
- **Primary**: `src/parser/flow.rs` (extend existing flow parsing logic)
- **Secondary**: `src/parser/flow.rs` (add parametric flow methods)

## Success Criteria
- [ ] Flow productions [105]-[150] fully implemented within existing flow parsing
- [ ] e-scalar, e-node empty node productions added
- [ ] Complete double-quoted scalar productions [107]-[116] with context parameters
- [ ] Single-quoted scalar productions [117]-[125] with parametric context
- [ ] Plain scalar productions [126]-[135] with context-dependent safety rules
- [ ] Complete flow sequence productions [137]-[139] integrated with existing logic
- [ ] Complete flow mapping productions [140]-[150] integrated with existing logic
- [ ] Zero breaking changes to current flow parsing functionality
- [ ] All existing flow parsing tests continue to pass

## Implementation Notes
- **Architecture**: Extend existing flow.rs, NOT replacement
- **Parametric Context**: Use context parameters for different flow parsing behaviors  
- **Safety Rules**: Implement plain scalar context-dependent safety rules
- **Integration**: Work with existing flow parsing while adding missing productions
- **Backward Compatibility**: Preserve all current flow parsing functionality

## Dependencies
- **Requires**: Milestone 1 completion (Core Productions)
- **Specifically**: structural_productions.rs for separation and line handling
- **Specifically**: character_productions.rs for character validation

## Complexity Estimate
**Medium-High** - Complex parametric context handling in existing flow parsing logic

## Constraints
- DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA
- Make ONLY MINIMAL, SURGICAL CHANGES required
- Never use unwrap() or expect() in src/*
- Preserve zero-allocation optimizations using Cow<str>