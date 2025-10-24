//! RFC 5. Character Productions Compliance Tests
//!
//! Tests for YAML 1.2.2 specification Chapter 5 - Character Productions
//!
//! This module contains comprehensive test coverage for every MUST/MUST NOT
//! requirement in Chapter 5 of the YAML 1.2.2 RFC specification.
//!
//! ## Test Modules
//!
//! - [`test_5_1_character_set`] - Character Set (printable chars, input/output requirements)
//! - [`test_5_2_character_encodings`] - Character Encodings (UTF-8/16/32, BOM handling)
//! - [`test_5_3_indicator_characters`] - Indicator Characters (all YAML indicators)
//! - [`test_5_4_line_break_characters`] - Line Break Characters (normalization, compatibility)
//! - [`test_5_5_white_space_characters`] - White Space Characters (space/tab only)
//! - [`test_5_6_miscellaneous_characters`] - Miscellaneous Characters (utility productions)
//! - [`test_5_7_escaped_characters`] - Escaped Characters (escape sequences, validation)
//!
//! ## RFC Compliance Coverage
//!
//! Each test module covers all testable requirements from its corresponding RFC section:
//!
//! ### Character Set (5.1)
//! - ✅ MUST accept all printable characters on input  
//! - ✅ MUST only produce printable characters on output
//! - ✅ Non-printable characters MUST be escaped on output
//! - ✅ MUST allow all non-C0 characters in quoted scalars
//!
//! ### Character Encodings (5.2)
//! - ✅ MUST support UTF-8 and UTF-16 on input
//! - ✅ MUST support UTF-32 for JSON compatibility
//! - ✅ MUST support BOM detection using specified table
//! - ✅ All documents in stream MUST use same encoding
//! - ✅ BOMs MUST NOT appear inside document (except quoted scalars)
//!
//! ### Indicator Characters (5.3)
//! - ✅ MUST recognize all indicator characters
//! - ✅ Reserved indicators @ and ` MUST NOT start plain scalars
//! - ✅ Flow indicators forbidden in certain contexts
//!
//! ### Line Break Characters (5.4)
//! - ✅ MUST support LF (x0A) and CR (x0D) as line breaks
//! - ✅ MUST normalize all line breaks to single LF in scalar content  
//! - ✅ Non-ASCII line breaks MUST be treated as non-break chars
//!
//! ### White Space Characters (5.5)
//! - ✅ MUST recognize only space (x20) and tab (x09) as whitespace
//!
//! ### Escaped Characters (5.7)
//! - ✅ Escape sequences MUST only work in double-quoted scalars
//! - ✅ MUST support all listed escape sequences
//! - ✅ MUST support hex escapes: \xXX, \uXXXX, \UXXXXXXXX
//! - ✅ MUST reject invalid escape sequences
//!
//! ## Usage
//!
//! Run all character production tests:
//! ```bash
//! cargo test --test rfc_compliance::ch05_character_productions
//! ```
//!
//! Run specific section tests:
//! ```bash
//! cargo test test_5_1_character_set
//! cargo test test_5_7_escaped_characters
//! ```

pub mod test_5_1_character_set;
pub mod test_5_2_character_encodings;
pub mod test_5_3_indicator_characters;
pub mod test_5_4_line_break_characters;
pub mod test_5_5_white_space_characters;
pub mod test_5_6_miscellaneous_characters;
pub mod test_5_7_escaped_characters;
