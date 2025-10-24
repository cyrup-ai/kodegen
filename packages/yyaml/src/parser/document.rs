use super::State;
// execute_state_machine removed - functionality moved to StateMachine::parse()
use crate::error::ScanError;
use crate::events::{Event, TScalarStyle, TokenType};

// REMOVED: All event-driven document parsing functions - replaced by state machine
// The old event-driven parser architecture has been completely eliminated
// Document parsing is now handled directly by the StateMachine in state_machine.rs

// These functions were part of the broken event-driven architecture:
// - document_start
// - explicit_document_start
// - process_directives
// - document_content
// - document_end

// Document parsing is now implemented in StateMachine::handle_document_* methods