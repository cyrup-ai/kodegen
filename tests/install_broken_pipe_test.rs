// Integration test for broken pipe handling in install command
// Tests that the install command handles stdout write failures gracefully

use std::io::Read;
use std::process::{Command, Stdio};

#[test]
fn test_install_handles_broken_pipe() {
    // Spawn kodegen install with piped stdout and stderr
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "kodegen", "--", "install"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn kodegen install command");

    // Read only 1 byte from stdout then drop it to break the pipe
    if let Some(mut stdout) = child.stdout.take() {
        let mut buf = [0u8; 1];
        let _ = stdout.read(&mut buf);
        // Dropping stdout here breaks the pipe
        drop(stdout);
    }

    // Wait for the process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for process");

    // Verify exit code is 0 despite broken pipe
    assert!(
        output.status.success(),
        "Install command should exit successfully (code 0) even with broken pipe, got: {:?}",
        output.status
    );

    // Verify stderr contains the warning message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: Could not write formatted output"),
        "stderr should contain warning about write failure. Got: {stderr}"
    );

    // Verify stderr contains the fallback install results header
    assert!(
        stderr.contains("Install results:"),
        "stderr should contain fallback install results. Got: {stderr}"
    );

    // The fallback should show install results with checkmarks or X marks
    // At minimum, one of these patterns should appear in the fallback output
    let has_result_markers = stderr.contains("✓") || stderr.contains("✗");
    assert!(
        has_result_markers,
        "stderr should contain result markers (✓ or ✗) in fallback output. Got: {stderr}"
    );
}

#[test]
#[ignore] // This test spawns cargo which is slow - run with --ignored flag
fn test_install_with_immediate_stdout_close() {
    // Alternative test: spawn and immediately close stdout without reading
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "kodegen", "--", "install"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn kodegen install command");

    // Immediately drop stdout to simulate immediate pipe closure
    drop(child.stdout.take());

    // Wait for the process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for process");

    // Should still exit successfully
    assert!(
        output.status.success(),
        "Install command should exit successfully even with immediately closed stdout, got: {:?}",
        output.status
    );

    // Should have fallback output in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Install results:") || stderr.contains("Warning"),
        "stderr should contain fallback output or warning. Got: {stderr}"
    );
}
