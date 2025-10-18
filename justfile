# kodegen justfile - example runners

# Setup the project - build and install daemon
setup:
    @echo "Building workspace (excluding daemon due to signing requirements)..."
    cargo build --workspace --exclude kodegen_daemon
    @echo "Building daemon binary..."
    cargo build --package kodegen_daemon --bin kodegend
    @echo "Installing daemon (uses GUI authorization)..."
    cargo run --package kodegen_install --bin kodegen-install -- --binary ./target/debug/kodegend
    @echo "✓ Setup complete! Daemon installed and ready."
    @kodegend status

# Install the daemon (uses GUI authorization, idempotent)
install-daemon:
    cargo build --package kodegen_daemon --bin kodegend
    cargo run --package kodegen_install --bin kodegen-install -- --binary ./target/debug/kodegend

# Run sequential_thinking example
run-sequential-thinking: install-daemon
    cargo run --package kodegen_mcp_client --example sequential_thinking

# Run filesystem example
run-filesystem: install-daemon
    cargo run --package kodegen_mcp_client --example filesystem

# Run terminal example
run-terminal: install-daemon
    cargo run --package kodegen_mcp_client --example terminal

# Run process example
run-process: install-daemon
    cargo run --package kodegen_mcp_client --example process

# Run introspection example
run-introspection: install-daemon
    cargo run --package kodegen_mcp_client --example introspection

# Run prompt example
run-prompt: install-daemon
    cargo run --package kodegen_mcp_client --example prompt

# Run claude_agent example
run-claude-agent: install-daemon
    cargo run --package kodegen_mcp_client --example claude_agent

# Run citescrape example
run-citescrape: install-daemon
    cargo run --package kodegen_mcp_client --example citescrape

# Run git example
run-git: install-daemon
    cargo run --package kodegen_mcp_client --example git

# Run github example
run-github: install-daemon
    cargo run --package kodegen_mcp_client --example github

# Run config example
run-config: install-daemon
    cargo run --package kodegen_mcp_client --example config

# Check daemon status
daemon-status:
    kodegend status

# Start daemon
daemon-start:
    kodegend start

# Stop daemon
daemon-stop:
    kodegend stop

# Restart daemon
daemon-restart:
    kodegend restart

# View daemon logs
daemon-logs:
    kodegend logs
