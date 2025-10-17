# Contributing to KODEGEN.ᴀɪ

Thank you for your interest in contributing to KODEGEN.ᴀɪ! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust nightly toolchain
- Git
- Basic understanding of MCP (Model Context Protocol)
- Familiarity with async Rust (tokio)

### Setting Up Your Development Environment

1. **Fork the repository** on [GitHub](https://github.com/kodegen/kodegen)

2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR_USERNAME/kodegen.git
   cd kodegen
   ```

3. **Set up Rust nightly:**
   ```bash
   rustup default nightly
   ```

4. **Build the project:**
   ```bash
   cargo build
   ```

5. **Run tests:**
   ```bash
   cargo test
   ```

## Development Guidelines

### Code Style

- Use `cargo fmt` to format your code before committing
- Run `cargo clippy` and address any warnings
- Follow existing code patterns and conventions
- Write clear, descriptive commit messages

### Tool Development Pattern

All tools in KODEGEN follow a consistent pattern. Use `packages/filesystem/src/read_file.rs` as the canonical reference implementation.

#### Creating a New Tool

1. **Create tool struct with dependencies:**
   ```rust
   #[derive(Clone)]
   pub struct MyTool {
       config_manager: kodegen_config::ConfigManager,
   }
   ```

2. **Define Args struct with JsonSchema:**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
   pub struct MyToolArgs {
       /// Parameter description
       pub param: String,
   }
   ```

3. **Define PromptArgs struct:**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
   pub struct MyToolPromptArgs {}
   ```

4. **Implement the Tool trait:**
   ```rust
   impl Tool for MyTool {
       type Args = MyToolArgs;
       type PromptArgs = MyToolPromptArgs;

       fn name() -> &'static str {
           "my_tool"
       }

       fn description() -> &'static str {
           "Clear description of what this tool does"
       }

       async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
           // Implementation
       }

       async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
           // Provide teaching examples for LLMs
       }

       // Override behavior flags if needed
       fn read_only() -> bool { true }
       fn destructive() -> bool { false }
       fn idempotent() -> bool { true }
   }
   ```

5. **Register the tool** in `packages/server/src/main.rs`:
   - Instantiate in the `build_routers` function
   - Add to `tool_router`
   - Add to `prompt_router`

### Important Principles

#### DO:
- ✅ Tools own their dependencies (no global state)
- ✅ Use `JsonSchema` on all Args types
- ✅ Implement comprehensive `prompt()` methods
- ✅ Return `Result<Value, McpError>` from `execute()`
- ✅ Register tools in **both** routers (tool + prompt)
- ✅ Write clear error messages for LLM agents
- ✅ Add documentation and examples

#### DON'T:
- ❌ Use global state or config singletons
- ❌ Implement MCP integration manually (trait provides it)
- ❌ Skip the prompt implementation
- ❌ Forget to register in both routers
- ❌ Add backward compatibility (MCP delivers fresh schemas)
- ❌ Guess at behavior flags (be explicit)

### Prompt Implementation

The `prompt()` method is critical for LLM learning. It should:

- Provide realistic usage examples
- Show both simple and complex use cases
- Explain when to use the tool
- Demonstrate best practices
- Include edge cases and warnings

**Example:**
```rust
async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
    Ok(vec![
        PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text("How do I use this tool?"),
        },
        PromptMessage {
            role: PromptMessageRole::Assistant,
            content: PromptMessageContent::text(
                "Here's how to use my_tool:\n\n\
                 Basic usage:\n\
                 my_tool({ \"param\": \"value\" })\n\n\
                 Advanced usage:\n\
                 my_tool({ \"param\": \"complex_value\", \"option\": true })\n\n\
                 Best practices:\n\
                 - Always validate input\n\
                 - Handle errors gracefully"
            ),
        },
    ])
}
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p kodegen-filesystem

# Run with logging
RUST_LOG=debug cargo test
```

### Writing Tests

- Write unit tests for core logic
- Add integration tests for tool behavior
- Test error cases and edge conditions
- Verify JsonSchema generation

## Pull Request Process

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/amazing-tool
   ```

2. **Make your changes:**
   - Write code following guidelines
   - Add/update tests
   - Update documentation

3. **Commit your changes:**
   ```bash
   git add .
   git commit -m "Add amazing new tool for X"
   ```

4. **Push to your fork:**
   ```bash
   git push origin feature/amazing-tool
   ```

5. **Open a Pull Request:**
   - Provide a clear description
   - Reference any related issues
   - Include examples of the new functionality

6. **Code Review:**
   - Address feedback promptly
   - Keep commits clean and focused
   - Be open to suggestions

## Feature Flags

When adding new tool categories, use Cargo feature flags:

1. **Add feature to `Cargo.toml`:**
   ```toml
   [features]
   my_category = []
   ```

2. **Wrap code in feature gates:**
   ```rust
   #[cfg(feature = "my_category")]
   pub mod my_category;
   ```

3. **Update `cli.rs`** to include the new category

4. **Update documentation** to reflect the new feature

## Documentation

- Update `README.md` for major features
- Add inline documentation for public APIs
- Update the website docs at `kodegen.ai/docs/`
- Include usage examples

## Performance Considerations

- Profile code for hot paths
- Minimize allocations
- Use zero-copy where possible
- Benchmark significant changes

## Security

- Validate all file paths
- Respect `allowed_paths` configuration
- Block dangerous commands
- Never execute user input directly
- Report security issues privately

## Community

- **GitHub Issues:** Bug reports and feature requests
- **Discussions:** Questions and ideas
- **Pull Requests:** Code contributions

## License

By contributing to KODEGEN.ᴀɪ, you agree that your contributions will be licensed under the dual Apache-2.0/MIT license.

## Questions?

If you have questions about contributing, feel free to:
- Open a GitHub Discussion
- Comment on relevant issues
- Reach out to [David Maple](https://www.linkedin.com/in/davemaple/)

---

**Thank you for contributing to KODEGEN.ᴀɪ!** 🚀
