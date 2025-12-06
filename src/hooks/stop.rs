use anyhow::Result;

use super::{StopInput, StopResponse};

/// Run the stop hook for Stop events
///
/// This hook is called when Claude Code finishes responding.
pub async fn run() -> Result<()> {
    let input: StopInput = serde_json::from_reader(std::io::stdin())?;

    // Validate hook event name
    if input.hook_event_name != "Stop" {
        log::warn!(
            "Expected hook_event_name 'Stop', got '{}' in session {}",
            input.hook_event_name,
            input.session_id
        );
        let response = StopResponse {
            decision: None,
            reason: None,
        };
        println!("{}", serde_json::to_string(&response)?);
        return Ok(());
    }

    // Log stop event
    log::debug!(
        "Stop hook: session={}, transcript={}, permission_mode={}, stop_hook_active={}",
        input.session_id,
        input.transcript_path,
        input.permission_mode,
        input.stop_hook_active
    );

    // Stop hook handler - currently just logs
    // Future enhancements could include:
    // - Send notification about session completion
    // - Log session statistics
    // - Cleanup temporary resources

    // Output hook response to Claude Code
    let response = StopResponse {
        decision: None,
        reason: None,
    };
    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}
