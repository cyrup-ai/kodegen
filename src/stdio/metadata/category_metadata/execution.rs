//! Execution tools: terminal

use kodegen_mcp_schema::terminal::{TERMINAL, TerminalInput};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn execution_tools() -> Vec<ToolMetadata> {
    vec![
        // TERMINAL (unified tool)
        ToolMetadata {
            name: TERMINAL,
            category: "terminal",
            description: "Execute shell commands in persistent, stateful terminal sessions. \
                         Terminals maintain environment variables, working directory, and shell \
                         state across commands. Use different terminal numbers (1, 2, 3...) for \
                         parallel work. Streams output in real-time as the command executes. \
                         Automatically reuses existing terminals or creates new sessions as needed.",
            schema: build_schema::<TerminalInput>(),
        },
    ]
}
