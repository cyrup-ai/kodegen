use anyhow::Result;
use kodegen_native_notify::{
    ImageData, ImagePlacement, MediaAttachment, NotificationBuilder,
    NotificationManager, Platform, RichText, Url,
};

use super::{read_hook_input, HookInput};

/// KODEGEN logo URL for branding in notifications
const LOGO_URL: &str = "https://kodegen.ai/assets/icon_128x128@2x.png";

/// Run the notify hook
pub async fn run() -> Result<()> {
    let input = read_hook_input()?;

    let notification_data = match input.hook_event_name.as_str() {
        "PostToolUse" => {
            // Check for tool errors FIRST - notify on ANY tool error
            if input.is_tool_error() {
                Some(build_error_notification(&input))
            } else {
                // Only notify for terminal tool on success
                match input.tool_name.as_deref() {
                    Some("mcp__plugin_kodegen_kodegen__terminal") => Some(build_terminal_notification(&input)),
                    _ => None, // Silent exit for non-terminal success
                }
            }
        }
        "SessionEnd" => Some(build_session_end_notification(&input)),
        _ => None,
    };

    let Some((title, body_html)) = notification_data else {
        return Ok(());
    };

    let mut builder = NotificationBuilder::new()
        .with_title(&title)
        .with_body(RichText::html(&body_html))
        .with_platforms(vec![Platform::MacOS, Platform::Windows, Platform::Linux]);

    // Add KODEGEN logo as app icon for Windows/macOS
    if let Ok(logo_url) = Url::parse(LOGO_URL) {
        builder = builder.with_media(MediaAttachment::Image {
            data: ImageData::Url(logo_url),
            placement: ImagePlacement::AppIcon,  // Circle icon on Windows, attachment on macOS
            alt_text: Some("KODEGEN".to_string()),
            dimensions: Some((128, 128)),
        });
    }

    let notification = builder.build()?;

    let manager = NotificationManager::new();
    let _ = manager.send(notification).await;
    manager.shutdown().await;
    Ok(())
}

/// Build HTML notification for tool errors (ANY tool)
fn build_error_notification(input: &HookInput) -> (String, String) {
    let tool_name = input.tool_name.as_deref().unwrap_or("unknown");
    let error_msg = input
        .error_message()
        .unwrap_or_else(|| "Unknown error".to_string());
    let cwd = &input.cwd;
    let transcript_link = format_transcript_link(&input.transcript_path);

    let title = format!("❌ {} failed", tool_name);
    let body = format!(
        r#"<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, sans-serif;">
            <img src="{}" width="24" height="24" alt="KODEGEN"/>
            <p><strong>Error:</strong> {}</p>
            <p><em>cwd:</em> {}</p>
            <p>{}</p>
        </div>"#,
        LOGO_URL,
        html_escape(&error_msg),
        html_escape(cwd),
        transcript_link
    );

    (title, body)
}

/// Build HTML notification for terminal command completion
fn build_terminal_notification(input: &HookInput) -> (String, String) {
    let command = input
        .terminal_input()
        .and_then(|ti| ti.command)
        .unwrap_or_else(|| "unknown".to_string());

    let output = input.terminal_output();
    let terminal_id = output.as_ref().and_then(|o| o.terminal).unwrap_or(0);
    let exit_code = output.as_ref().and_then(|o| o.exit_code);
    let duration_ms = output.as_ref().map(|o| o.duration_ms).unwrap_or(0);
    let completed = output.as_ref().map(|o| o.completed).unwrap_or(true);
    let terminal_output = output.as_ref().map(|o| o.output.as_str()).unwrap_or("");

    let cwd = &input.cwd;
    let transcript_link = format_transcript_link(&input.transcript_path);

    let cmd_short = truncate(&command, 40);
    let duration = format_duration(duration_ms);
    let output_preview = truncate_output(terminal_output, 20);

    let (icon, status) = match (exit_code, completed) {
        (Some(0), true) => ("✓", "success".to_string()),
        (Some(code), true) => ("✗", format!("exit {}", code)),
        (None, false) => ("⏳", "running".to_string()),
        _ => ("•", "unknown".to_string()),
    };

    let title = format!("{} terminal {}: {}", icon, terminal_id, cmd_short);
    let body = format!(
        r#"<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, sans-serif;">
            <img src="{}" width="24" height="24" alt="KODEGEN"/>
            <p><strong>{}</strong> in {}</p>
            <p><em>cwd:</em> {}</p>
            <pre style="background:#1e1e1e;color:#d4d4d4;padding:8px;border-radius:4px;font-size:12px;white-space:pre-wrap;word-wrap:break-word;">{}</pre>
            <p>{}</p>
        </div>"#,
        LOGO_URL,
        status,
        duration,
        html_escape(cwd),
        html_escape(&output_preview),
        transcript_link
    );

    (title, body)
}

/// Build HTML notification for session end
fn build_session_end_notification(input: &HookInput) -> (String, String) {
    let reason = input.reason.as_deref().unwrap_or("unknown");
    let session_short = truncate(&input.session_id, 8);
    let transcript_link = format_transcript_link(&input.transcript_path);

    let title = format!("Claude Session Ended ({})", session_short);
    let body = format!(
        r#"<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, sans-serif;">
            <img src="{}" width="24" height="24" alt="KODEGEN"/>
            <p>{}</p>
            <p>{}</p>
        </div>"#,
        LOGO_URL, reason, transcript_link
    );

    (title, body)
}

/// Format transcript path as clickable file:// hyperlink
fn format_transcript_link(path: &str) -> String {
    format!(r#"<a href="file://{}">View Transcript</a>"#, path)
}

/// HTML escape special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// Format duration in human-readable form
fn format_duration(ms: u64) -> String {
    if ms >= 60000 {
        format!("{}m {}s", ms / 60000, (ms % 60000) / 1000)
    } else if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

/// Truncate output to last N lines if more than max_lines
fn truncate_output(output: &str, max_lines: usize) -> String {
    let trimmed = output.trim();
    let lines: Vec<&str> = trimmed.lines().collect();

    if lines.len() <= max_lines {
        return trimmed.to_string();
    }

    // Take last max_lines
    let start = lines.len() - max_lines;
    let last_lines = &lines[start..];
    format!("...({} lines hidden)\n{}", start, last_lines.join("\n"))
}
