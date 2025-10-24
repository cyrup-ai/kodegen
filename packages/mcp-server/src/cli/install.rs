use anyhow::Result;
use kodegen_bundler_autoconfig::{InstallResult, install_all_clients};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Run the install command to configure MCP-compatible editors
pub fn run_install() -> Result<()> {
    let results = install_all_clients()?;
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Try to write formatted output, but don't fail if stdout is broken
    if let Err(e) = write_formatted_output(&mut stdout, &results) {
        // Stdout is broken (pipe closed, redirected to full disk, etc.)
        // Fall back to simple output via stderr
        eprintln!("Warning: Could not write formatted output: {e}");
        eprintln!("\nInstall results:");
        for result in &results {
            if result.success {
                eprintln!("  ✓ {}: {}", result.client_name, result.message);
            } else {
                eprintln!("  ✗ {}: {}", result.client_name, result.message);
            }
        }
    }

    Ok(())
}

/// Write formatted output to the provided writer
/// Returns error if any write operation fails (broken pipe, full disk, etc.)
fn write_formatted_output<W: Write + WriteColor>(
    writer: &mut W,
    results: &[InstallResult],
) -> Result<()> {
    // Header
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(writer, "\n┌─────────────────────────────────────────────┐")?;
    writeln!(writer, "│   🔍 MCP Editor Configuration Results       │")?;
    writeln!(writer, "└─────────────────────────────────────────────┘\n")?;
    writer.reset()?;

    let mut configured = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for result in results {
        if result.success {
            // Success - green checkmark
            writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(writer, "  ✓ ")?;
            writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(writer, "{}", result.client_name)?;
        } else {
            // Failed - red X or dim skip
            if result.message == "Not installed" {
                writer.set_color(
                    ColorSpec::new()
                        .set_fg(Some(Color::Black))
                        .set_intense(true),
                )?;
                write!(writer, "  ○ ")?;
                writeln!(writer, "{}", result.client_name)?;
            } else {
                writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(writer, "  ✗ ")?;
                writer.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(writer, "{}", result.client_name)?;
            }
        }

        // Config path
        if let Some(ref path) = result.config_path {
            writer.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Black))
                    .set_intense(true),
            )?;
            writeln!(writer, "     {}", path.display())?;
        }

        // Status message
        if result.success {
            if result.message.contains("Already") {
                writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(writer, "     {}\n", result.message)?;
                skipped += 1;
            } else {
                writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(writer, "     {}\n", result.message)?;
                configured += 1;
            }
        } else {
            writer.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Black))
                    .set_intense(true),
            )?;
            writeln!(writer, "     {}\n", result.message)?;
            failed += 1;
        }
        writer.reset()?;
    }

    // Summary
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(writer, "─────────────────────────────────────────────")?;
    writer.reset()?;

    write!(writer, "  ")?;
    if configured > 0 {
        writer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(writer, "{configured} configured")?;
        writer.reset()?;
    }
    if skipped > 0 {
        if configured > 0 {
            write!(writer, " • ")?;
        }
        writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        write!(writer, "{skipped} already configured")?;
        writer.reset()?;
    }
    if failed > 0 {
        if configured > 0 || skipped > 0 {
            write!(writer, " • ")?;
        }
        writer.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Black))
                .set_intense(true),
        )?;
        write!(writer, "{failed} not installed")?;
        writer.reset()?;
    }
    writeln!(writer, "\n")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kodegen_bundler_autoconfig::InstallResult;
    use std::io;
    use std::path::PathBuf;
    use termcolor::{ColorSpec, WriteColor};

    /// Test writer that always fails with `BrokenPipe` error
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "test broken pipe",
            ))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "test broken pipe",
            ))
        }
    }

    impl WriteColor for FailingWriter {
        fn supports_color(&self) -> bool {
            false
        }

        fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "test broken pipe",
            ))
        }

        fn reset(&mut self) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "test broken pipe",
            ))
        }
    }

    // Helper to create mock install results
    fn create_mock_results() -> Vec<InstallResult> {
        vec![
            InstallResult {
                client_name: "VSCode".to_string(),
                client_id: "vscode".to_string(),
                success: true,
                message: "Successfully configured".to_string(),
                config_path: Some(PathBuf::from("/path/to/vscode/config")),
            },
            InstallResult {
                client_name: "Cursor".to_string(),
                client_id: "cursor".to_string(),
                success: true,
                message: "Already configured".to_string(),
                config_path: Some(PathBuf::from("/path/to/cursor/config")),
            },
            InstallResult {
                client_name: "Zed".to_string(),
                client_id: "zed".to_string(),
                success: false,
                message: "Not installed".to_string(),
                config_path: None,
            },
        ]
    }

    #[test]
    fn test_write_formatted_output_with_broken_pipe() {
        // Test that write_formatted_output returns error when writer fails
        let results = create_mock_results();
        let mut failing_writer = FailingWriter;

        // Should return error when writer fails
        let result = write_formatted_output(&mut failing_writer, &results);
        assert!(
            result.is_err(),
            "Expected error when writing to broken pipe"
        );

        // Verify it's the expected error type
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("broken pipe") || error_msg.contains("test broken pipe"),
                "Expected broken pipe error, got: {error_msg}"
            );
        }
    }

    #[test]
    fn test_write_formatted_output_with_valid_writer() {
        // Test with a buffer writer (won't fail)
        use termcolor::Buffer;

        let results = create_mock_results();
        let mut buffer = Buffer::ansi();

        // Should succeed with a working writer
        let result = write_formatted_output(&mut buffer, &results);
        assert!(result.is_ok(), "Expected success with working writer");

        // Verify some output was written
        let output = String::from_utf8_lossy(buffer.as_slice());
        assert!(output.contains("MCP Editor Configuration Results"));
        assert!(output.contains("VSCode"));
    }

    #[test]
    fn test_install_result_structure() {
        // Verify InstallResult structure works correctly
        let result = InstallResult {
            client_name: "TestEditor".to_string(),
            client_id: "test-editor".to_string(),
            success: true,
            message: "Test message".to_string(),
            config_path: Some(PathBuf::from("/test/path")),
        };

        assert_eq!(result.client_name, "TestEditor");
        assert_eq!(result.client_id, "test-editor");
        assert!(result.success);
        assert_eq!(result.message, "Test message");
        assert!(result.config_path.is_some());
    }

    #[test]
    fn test_empty_results() {
        // Test with empty results list
        use termcolor::Buffer;

        let results: Vec<InstallResult> = vec![];
        let mut buffer = Buffer::ansi();
        let result = write_formatted_output(&mut buffer, &results);

        // Should succeed with empty results
        assert!(result.is_ok(), "Expected success with empty results");
    }

    #[test]
    fn test_mixed_results() {
        // Test with mix of success/failure results
        use termcolor::Buffer;

        let results = vec![
            InstallResult {
                client_name: "Success1".to_string(),
                client_id: "success1".to_string(),
                success: true,
                message: "Configured".to_string(),
                config_path: Some(PathBuf::from("/path1")),
            },
            InstallResult {
                client_name: "Failure1".to_string(),
                client_id: "failure1".to_string(),
                success: false,
                message: "Failed to configure".to_string(),
                config_path: None,
            },
            InstallResult {
                client_name: "Success2".to_string(),
                client_id: "success2".to_string(),
                success: true,
                message: "Already configured".to_string(),
                config_path: Some(PathBuf::from("/path2")),
            },
        ];

        let mut buffer = Buffer::ansi();
        let result = write_formatted_output(&mut buffer, &results);

        // Should handle mixed results without errors
        assert!(result.is_ok(), "Expected success with mixed results");

        // Verify output contains expected data
        let output = String::from_utf8_lossy(buffer.as_slice());
        assert!(output.contains("Success1"));
        assert!(output.contains("Failure1"));
        assert!(output.contains("Success2"));
    }

    /// Integration test: Verify broken pipe handling with real binary
    /// This test spawns the actual kodegen binary and breaks the pipe
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_integration_broken_pipe_handling() {
        use std::io::Read;
        use std::process::{Command, Stdio};

        // Build the binary first
        let build_status = Command::new("cargo")
            .args(["build", "--bin", "kodegen"])
            .status()
            .expect("Failed to build kodegen");

        assert!(build_status.success(), "Failed to build kodegen binary");

        // Spawn kodegen install and immediately close stdout (breaks pipe)
        let mut child = Command::new("./target/debug/kodegen")
            .arg("install")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn kodegen");

        // Read only 1 byte from stdout then drop it (breaks the pipe)
        if let Some(mut stdout) = child.stdout.take() {
            let mut buf = [0u8; 1];
            let _ = stdout.read(&mut buf);
            drop(stdout); // This breaks the pipe
        }

        // Wait for process to complete
        let output = child.wait_with_output().expect("Failed to wait for child");

        // CRITICAL: Should exit successfully despite broken pipe
        assert!(
            output.status.success(),
            "kodegen install should succeed even with broken pipe, but got exit code: {:?}",
            output.status.code()
        );

        // Should have fallback output in stderr
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Verify fallback warning message appears
        assert!(
            stderr.contains("Warning: Could not write formatted output")
                || stderr.contains("Install results:"),
            "Expected fallback message in stderr when pipe breaks, got: {stderr}"
        );
    }
}
