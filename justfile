# kodegen justfile - example runners

# Setup the project - build and install daemon
setup:
    @echo "Building workspace (excluding daemon due to signing requirements)..."
    cargo build --workspace --exclude kodegen_daemon
    @echo "Building daemon binary..."
    cargo build --package kodegen_daemon --bin kodegend
    @echo "Installing daemon (uses GUI authorization)..."
    cargo run --package kodegen_install --bin kodegen_install -- --binary ./target/debug/kodegend
    @echo "✓ Setup complete! Daemon installed and ready."
    @kodegend status

# Install the daemon (uses GUI authorization, idempotent)
install-daemon:
    cargo build --package kodegen_daemon --bin kodegend
    cargo run --package kodegen_install --bin kodegen_install -- --binary ./target/debug/kodegend

# Run sequential_thinking example
run-sequential-thinking:
    cargo run --package kodegen_mcp_client --example sequential_thinking

# Run filesystem example
run-filesystem:
    cargo run --package kodegen_mcp_client --example filesystem

# Run terminal example
run-terminal:
    cargo run --package kodegen_mcp_client --example terminal

# Run process example
run-process:
    cargo run --package kodegen_mcp_client --example process

# Run introspection example
run-introspection:
    cargo run --package kodegen_mcp_client --example introspection

# Run prompt example
run-prompt:
    cargo run --package kodegen_mcp_client --example prompt

# Run claude_agent example
run-claude-agent:
    cargo run --package kodegen_mcp_client --example claude_agent

# Run citescrape example
run-citescrape:
    cargo run --package kodegen_mcp_client --example citescrape

# Run git example
run-git:
    cargo run --package kodegen_mcp_client --example git

# Run github example
run-github:
    cargo run --package kodegen_mcp_client --example github

# Run config example
run-config:
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

# Run full release (patch version bump)
release:
    #!/usr/bin/env zsh
    cd ./packages/bundler-release && cargo run --package kodegen_bundler_release -- release patch

# Regenerate workspace-hack dependencies
hakari-regenerate:
    cargo run --package cargo-hakari-regenerate --bin cargo-hakari-regenerate -- regenerate --progress

# Run cargo check and clippy with formatted output
check:
    #!/usr/bin/env zsh
    # Create tmp directory if it doesn't exist
    mkdir -p ./tmp

    # Get absolute path to tmp directory
    TMP_DIR=$(cd ./tmp && pwd)

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Running cargo check..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run cargo check and save to log
    cargo check --workspace 2>&1 | tee ${TMP_DIR}/cargo-check.log
    CHECK_EXIT=${pipestatus[1]}

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Running cargo clippy..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run cargo clippy and save to log
    cargo clippy --workspace --all-targets 2>&1 | tee ${TMP_DIR}/cargo-clippy.log
    CLIPPY_EXIT=${pipestatus[1]}

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Summary"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "📋 cargo check output: ${TMP_DIR}/cargo-check.log"
    echo "Last 50 lines:"
    echo "────────────────────────────────────────────────────────────────────────────"
    tail -50 ${TMP_DIR}/cargo-check.log
    echo ""
    echo "📋 cargo clippy output: ${TMP_DIR}/cargo-clippy.log"
    echo "Last 50 lines:"
    echo "────────────────────────────────────────────────────────────────────────────"
    tail -50 ${TMP_DIR}/cargo-clippy.log
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Exit with error if either command failed
    if [[ ${CHECK_EXIT} -ne 0 ]] || [[ ${CLIPPY_EXIT} -ne 0 ]]; then
        echo "❌ Checks failed (check: ${CHECK_EXIT}, clippy: ${CLIPPY_EXIT})"
        exit 1
    else
        echo "✅ All checks passed"
    fi
