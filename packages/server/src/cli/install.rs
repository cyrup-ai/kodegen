use anyhow::Result;
use kodegen_client_autoconfig::{install_all_clients, InstallResult};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::io::Write;

/// Run the install command to configure MCP-compatible editors
pub fn run_install() -> Result<()> {
    let results = install_all_clients()?;

    // Try to write formatted output, but don't fail if stdout is broken
    if let Err(e) = write_formatted_output(&results) {
        // Stdout is broken (pipe closed, redirected to full disk, etc.)
        // Fall back to simple output via stderr
        eprintln!("Warning: Could not write formatted output: {}", e);
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

/// Write formatted output to stdout
/// Returns error if any write operation fails (broken pipe, full disk, etc.)
fn write_formatted_output(results: &[InstallResult]) -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "\n┌─────────────────────────────────────────────┐")?;
    writeln!(&mut stdout, "│   🔍 MCP Editor Configuration Results       │")?;
    writeln!(&mut stdout, "└─────────────────────────────────────────────┘\n")?;
    stdout.reset()?;

    let mut configured = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for result in results {
        if result.success {
            // Success - green checkmark
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(&mut stdout, "  ✓ ")?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "{}", result.client_name)?;
        } else {
            // Failed - red X or dim skip
            if result.message == "Not installed" {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                write!(&mut stdout, "  ○ ")?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                write!(&mut stdout, "  ✗ ")?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stdout, "{}", result.client_name)?;
            }
        }

        // Config path
        if let Some(ref path) = result.config_path {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}", path.display())?;
        }

        // Status message
        if result.success {
            if result.message.contains("Already") {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                skipped += 1;
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "     {}\n", result.message)?;
                configured += 1;
            }
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
            writeln!(&mut stdout, "     {}\n", result.message)?;
            failed += 1;
        }
        stdout.reset()?;
    }

    // Summary
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(&mut stdout, "─────────────────────────────────────────────")?;
    stdout.reset()?;

    write!(&mut stdout, "  ")?;
    if configured > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(&mut stdout, "{} configured", configured)?;
        stdout.reset()?;
    }
    if skipped > 0 {
        if configured > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        write!(&mut stdout, "{} already configured", skipped)?;
        stdout.reset()?;
    }
    if failed > 0 {
        if configured > 0 || skipped > 0 { write!(&mut stdout, " • ")?; }
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
        write!(&mut stdout, "{} not installed", failed)?;
        stdout.reset()?;
    }
    writeln!(&mut stdout, "\n")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kodegen_client_autoconfig::InstallResult;
    use std::path::PathBuf;

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
    fn test_write_formatted_output_with_valid_results() {
        // Test that write_formatted_output handles valid results
        let results = create_mock_results();
        
        // Function should handle output gracefully
        // In a real terminal, this would succeed
        // When stdout is redirected in tests, it may fail, but that's expected
        let _result = write_formatted_output(&results);
        
        // Test passes if it doesn't panic
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
        let results: Vec<InstallResult> = vec![];
        let _result = write_formatted_output(&results);
        
        // Should not panic with empty results
    }

    #[test]
    fn test_mixed_results() {
        // Test with mix of success/failure results
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
        
        let _result = write_formatted_output(&results);
        
        // Should handle mixed results without panicking
    }
}
