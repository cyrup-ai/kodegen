/// Helper function to get default shell for the current platform
pub(super) fn get_default_shell() -> String {
    // 1. Try $SHELL environment variable first
    if let Ok(shell) = std::env::var("SHELL")
        && !shell.is_empty() {
            return shell;
        }

    // 2. On Unix, try to get user's login shell from passwd
    #[cfg(unix)]
    {
        // Try getent passwd first (works on Linux/macOS)
        if let Ok(user) = std::env::var("USER")
            && let Ok(output) = std::process::Command::new("getent")
                .args(["passwd", &user])
                .output()
            && let Ok(line) = String::from_utf8(output.stdout) {
                    // passwd format: name:password:uid:gid:gecos:home:shell
                    if let Some(shell) = line.trim().split(':').nth(6)
                        && !shell.is_empty() {
                            return shell.to_string();
                        }
                }

        // Fallback: POSIX-compliant /bin/sh (always exists)
        "/bin/sh".to_string()
    }

    // 3. Windows fallback
    #[cfg(windows)]
    {
        // Check for PowerShell first (more modern)
        if let Ok(output) = std::process::Command::new("where")
            .arg("pwsh.exe")
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return "pwsh.exe".to_string();
            }
        }

        // Fallback to cmd.exe
        "cmd.exe".to_string()
    }
}
