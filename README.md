# KODEGEN.ᴀɪ

**Ultimate MCP Auto-Coding Toolset**

KODEGEN.ᴀɪ delivers a blazing-fast Rust-native MCP Server (Model Context Protocol) with 75 elite auto-coding tools designed for professional, autonomous code generation and predictable high-quality results. Every tool has been thoughtfully hyper-optimized for speed (code it faster) and context efficiency (code it cheaper).

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE.md)

## Overview

KODEGEN.ᴀɪ is built for context efficient LLM code generation that's fast, reliable and memory-efficient.

### Key Features

#### 🗂️ Warp Speed Mods
14 filesystem tools optimized for coding workflows with atomic operations and concurrent traversal. Read massive files with offsets, batch-process multiple files, search codebases with streaming results, and make surgical edits with diff precision.

#### 🔧 Git Superpowers
20 comprehensive git tools powered by Gitoxide for blazing-fast repository operations. Init, clone, branch, commit, checkout, fetch, merge, and manage worktrees—all with async-first design and production-grade error handling.

#### 💻 Terminal as a Tool
Full VT100 pseudoterminal sessions with smart state detection and real-time output streaming. Perfect when AI agents need full system access for running builds, executing tests, or managing deployments.

#### 🧠 Reasoning Chains
Stateful thinking sessions with branching, revision, and unlimited context across extended problem-solving. Break down complex problems with actor-model concurrency for lock-free performance.

#### 🔮 Agents with Agents
N-depth agent delegation with full prompt control for hierarchical, coordinated agent pyramids. Spawn specialized Claude sub-agents for deep research, complex code generation, or parallel analysis.

#### 🌐 Web Crawling & Search
4 tools for autonomous web documentation crawling with full-text search. Background crawling with Tantivy indexing, rate limiting, and multi-format output (Markdown/HTML/JSON). Perfect for building searchable knowledge bases from documentation sites.

#### 🐙 GitHub Integration
16 tools for comprehensive GitHub API integration. Create and manage issues, pull requests, reviews, and comments. Search code across repositories, manage PRs, request Copilot reviews, and automate entire GitHub workflows from your AI agents.

#### 📊 LLM Observability
Track tool usage, analyze patterns, and optimize workflows with built-in introspection. Every invocation is tracked for AI self-improvement.

#### 📝 Agents Manage Prompts
Create and manage reusable prompt templates with Jinja2 rendering and schema validation. Build prompt libraries and standardize agent instructions programmatically.

## Installation

### Quick Install

Install KODEGEN.ᴀɪ with a single command:

```bash
curl -fsSL https://kodegen.ai/install | sh
```

This will:
- ✅ Install Rust nightly
- ✅ Build and install `kodegen` binary
- ✅ Auto-configure all detected MCP clients
- ✅ Get you ready to code!

### Automatic Editor Configuration

The installer automatically runs `kodegen install` which configures:

- ✅ **Claude Desktop** - Auto-configures `claude_desktop_config.json`
- ✅ **Windsurf** - Auto-configures Windsurf MCP settings
- ✅ **Cursor** - Auto-configures Cursor AI settings
- ✅ **Zed** - Auto-configures Zed editor settings
- ✅ **Roo Code** - Auto-configures Roo Code settings

### Manual Installation

For manual installation or to build from source:

```bash
# Clone the repository
git clone https://github.com/cyrup-ai/kodegen
cd kodegen

# Run installation script (handles all binaries with verification)
./install.sh
```

The script will:
- ✅ Compile and install kodegen MCP server
- ✅ Compile and install kodegend daemon  
- ✅ Verify both binaries are ready before proceeding
- ✅ Auto-configure detected MCP clients
- ✅ Install and start the daemon service

### Manual MCP Client Configuration

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "kodegen": {
      "command": "kodegen"
    }
  }
}
```

## Configuration

### Runtime Tool Selection

Control which tools are active at runtime by passing arguments to the kodegen binary:

**Method 1: Comma-Separated List**
```json
{
  "mcpServers": {
    "kodegen": {
      "command": "kodegen",
      "args": ["--tools", "filesystem,terminal,citescrape"]
    }
  }
}
```

**Method 2: Multiple Flags**
```json
{
  "mcpServers": {
    "kodegen": {
      "command": "kodegen",
      "args": [
        "--tool", "filesystem",
        "--tool", "terminal",
        "--tool", "citescrape"
      ]
    }
  }
}
```

**Available Tool Categories:**
- `filesystem` - 14 file operation tools
- `terminal` - 5 terminal/process management tools
- `process` - 2 process management tools
- `sequential_thinking` - 1 reasoning chain tool
- `claude_agent` - 5 sub-agent orchestration tools
- `citescrape` - 4 web scraping and search tools
- `prompt` - 4 prompt template management tools
- `introspection` - 2 observability tools
- `git` - 20 git repository tools (init, clone, branch, commit, checkout, fetch, merge, worktree operations)
- `github` - 16 GitHub API tools (issues, PRs, reviews, comments, code search, Copilot integration)
- `config` - 2 configuration management tools (always enabled)

If no arguments are provided, all compiled tool categories are enabled by default.

### Runtime Configuration

AI agents can modify configuration at runtime:

```javascript
set_config_value({
  "key": "file_read_line_limit",
  "value": 5000
})
```

**Configuration Options:**
- `file_read_line_limit` - Maximum lines to read per file (default: 2000)
- `file_write_line_limit` - Maximum lines to write per operation (default: 1000)
- `fuzzy_search_threshold` - Similarity threshold for fuzzy matching (default: 0.8)
- `blocked_commands` - List of commands to block in terminal sessions
- `allowed_paths` - Whitelist of paths for file operations
- `max_search_results` - Maximum search results to return (default: 100)
- `terminal_timeout` - Terminal command timeout in seconds (default: 300)

## Advanced Users

### Custom Builds with Feature Gates

Create hyper-optimized binaries by compiling only the tools you need:

```bash
# Build with only filesystem and terminal tools
cargo build --release \
  --no-default-features \
  --features "filesystem,terminal"

# Install custom build
cargo install --path . \
  --no-default-features \
  --features "filesystem,terminal,sequential_thinking"
```

**Available Feature Flags:**
- `filesystem` - 14 tools (~800KB)
- `terminal` - 5 tools (~300KB)
- `sequential_thinking` - 1 tool (~150KB)
- `claude_agent` - 5 tools (~400KB)
- `citescrape` - 3 tools (~600KB)
- `prompt` - 4 tools (~250KB)
- `introspection` - 2 tools (~100KB)
- `process` - 2 tools (~150KB)

### Common Build Profiles

**Minimal Coding Assistant (Filesystem + Terminal)**
```bash
cargo install --path . \
  --no-default-features \
  --features "filesystem,terminal"

# Binary: ~1.2MB (vs 3.5MB full build)
# Perfect for: Basic file operations and command execution
```

**Thinking Agent (Filesystem + Sequential Thinking + Agents)**
```bash
cargo install --path . \
  --no-default-features \
  --features "filesystem,sequential_thinking,claude_agent"

# Binary: ~1.8MB
# Perfect for: Research, analysis, and multi-step reasoning
```

**Documentation Crawler (Filesystem + Citescrape + Sequential Thinking)**
```bash
cargo install --path . \
  --no-default-features \
  --features "filesystem,citescrape,sequential_thinking"

# Binary: ~2.5MB
# Perfect for: Building searchable docs from websites, knowledge base creation
```

**Full-Featured Build (Default)**
```bash
cargo install --path .

# Binary: ~3.5MB
# Includes: All 33 tools across 7 categories
```

### Combining Compile-Time and Runtime Filtering

For maximum optimization:

```bash
# 1. Build with only filesystem and terminal features
cargo install --path . \
  --no-default-features \
  --features "filesystem,terminal"

# 2. Configure MCP client to use only filesystem tools
{
  "mcpServers": {
    "kodegen": {
      "command": "kodegen",
      "args": ["--tool", "filesystem"]
    }
  }
}

# Result: Smallest binary + fastest startup + minimal memory footprint
```

### Performance Comparison

| Build Configuration | Binary Size | Startup Time | Memory Usage |
|---------------------|-------------|--------------|--------------|
| Full Build (All Features) | ~3.5MB | ~25ms | ~8MB |
| Minimal (filesystem + terminal) | ~1.2MB | ~12ms | ~4MB |
| Filesystem Only | ~900KB | ~8ms | ~3MB |

*Note: Measurements are approximate and may vary by platform.*

## Tool Reference

### Filesystem Tools (14 tools)

#### `read_file`
Read file contents with offset/length support for massive files.

```javascript
read_file({
  "file_path": "src/main.rs",
  "offset": 0,
  "limit": 100
})
```

#### `write_file`
Write or append content to files with atomic operations.

```javascript
write_file({
  "file_path": "output.txt",
  "content": "Hello, world!",
  "append": false
})
```

#### `edit_block`
Surgical text replacement with automatic fuzzy matching.

```javascript
edit_block({
  "file_path": "src/main.rs",
  "old_string": "fn process_data",
  "new_string": "async fn process_data"
})
```

#### `start_search`
Start streaming search across codebase with regex support.

```javascript
start_search({
  "pattern": "TODO:",
  "path": ".",
  "regex": false
})
```

**Other filesystem tools:** `read_multiple_files`, `move_file`, `delete_file`, `delete_directory`, `create_directory`, `get_file_info`, `list_directory`, `get_more_search_results`, `stop_search`, `list_searches`

### Terminal Tools (5 tools)

#### `start_terminal_command`
Spawn full VT100 pseudoterminal session.

```javascript
start_terminal_command({
  "command": "cargo build --release",
  "working_directory": "."
})
```

**Other terminal tools:** `read_terminal_output`, `send_terminal_input`, `stop_terminal_command`, `list_terminal_commands`

### Sequential Thinking (1 tool)

#### `sequential_thinking`
Break down complex problems with stateful reasoning sessions.

```javascript
sequential_thinking({
  "thought": "Analyzing the architecture patterns...",
  "thought_number": 1,
  "session_id": "planning-session-1"
})
```

### Agent Orchestration (5 tools)

#### `spawn_claude_agent`
Spawn specialized Claude sub-agents for delegation.

```javascript
spawn_claude_agent({
  "task": "Research API design patterns for Rust",
  "prompt_template": "research_agent"
})
```

**Other agent tools:** `read_claude_agent_output`, `send_claude_agent_prompt`, `terminate_claude_agent_session`, `list_claude_agents`

### Prompt Management (4 tools)

#### `add_prompt`
Create reusable prompt templates with Jinja2.

```javascript
add_prompt({
  "name": "code_review",
  "template": "Review this code: {{ code }}",
  "description": "Code review prompt"
})
```

**Other prompt tools:** `edit_prompt`, `delete_prompt`, `get_prompt`

### Introspection (2 tools)

- `get_usage_stats` - Track tool usage and performance metrics
- `get_recent_tool_calls` - Inspect recent tool invocations

### Process Management (2 tools)

- `list_processes` - List system processes with filtering
- `kill_process` - Terminate processes by PID

### Configuration (2 tools)

- `get_config` - Retrieve current configuration values
- `set_config_value` - Modify configuration at runtime

## Examples

### Complete Workflow: Refactoring a Rust Project

```javascript
// 1. Search for function to refactor
start_search({
  "pattern": "fn process_data",
  "path": "src/"
})

// 2. Read the file
read_file({
  "file_path": "src/processor.rs"
})

// 3. Make the function async
edit_block({
  "file_path": "src/processor.rs",
  "old_string": "fn process_data(input: &str) -> Result<Data>",
  "new_string": "async fn process_data(input: &str) -> Result<Data>"
})

// 4. Run tests
start_terminal_command({
  "command": "cargo test"
})

// 5. Check output
read_terminal_output({
  "session_id": "terminal-123"
})
```

### Multi-Agent Research

```javascript
// Spawn a research agent
spawn_claude_agent({
  "task": "Research best practices for error handling in Rust async code"
})

// Spawn another for code generation
spawn_claude_agent({
  "task": "Generate example error handling code based on research findings"
})

// Monitor agents
list_claude_agents({})

// Read results
read_claude_agent_output({
  "agent_id": "agent-001"
})
```

### Sequential Thinking for Architecture Planning

```javascript
// Start thinking session
sequential_thinking({
  "session_id": "architecture-planning",
  "thought": "Need to design a scalable API layer",
  "thought_number": 1
})

// Branch to explore alternatives
sequential_thinking({
  "session_id": "architecture-planning",
  "thought": "Option A: REST API with versioning",
  "thought_number": 2,
  "branch_from": 1
})

// Revise earlier thinking
sequential_thinking({
  "session_id": "architecture-planning",
  "thought": "Actually, REST is better for our use case due to caching",
  "thought_number": 4,
  "revises": 2
})
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start for Contributors

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/kodegen.git`
3. Create a feature branch: `git checkout -b feature/amazing-tool`
4. Make your changes
5. Run tests: `cargo test`
6. Commit and push: `git push origin feature/amazing-tool`
7. Open a Pull Request

### Development Guidelines

- Follow the tool pattern in `packages/filesystem/src/read_file.rs`
- All tools implement the `Tool` trait
- Write comprehensive `prompt()` methods for LLM learning
- Add JsonSchema to all Args types
- Register tools in both routers (tool + prompt)
- Update documentation

## Community

- **GitHub Repository:** [kodegen/kodegen](https://github.com/kodegen/kodegen)
- **Website:** [kodegen.ai](https://kodegen.ai)
- **Documentation:** [kodegen.ai/docs](https://kodegen.ai/docs)

## License

KODEGEN.ᴀɪ is dual-licensed under Apache-2.0 and MIT. See [LICENSE.md](LICENSE.md) for details.

## Credits

Made with (love) by [David Maple](https://www.linkedin.com/in/davemaple/)

---

**Welcome to KODEGEN.ᴀɪ!** 🚀
