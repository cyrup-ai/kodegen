use regex::Regex;
use std::collections::HashSet;
use log::warn;
use once_cell::sync::Lazy;

// Compile regex once at startup (not on every command validation)
static ENV_VAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\w+=\S+\s*")
        .expect("Hardcoded regex pattern r\"\\w+=\\S+\\s*\" is invalid - this is a compile-time bug")
});

/// Command manager for validating and parsing commands
#[derive(Clone)]
pub struct CommandManager {
    blocked_commands: Vec<String>,
}

impl CommandManager {
    /// Create a new command manager instance with a list of blocked commands
    /// Commands are automatically converted to lowercase for case-insensitive blocking
    #[must_use]
    pub fn new(blocked_commands: Vec<String>) -> Self {
        // Normalize all blocked commands to lowercase
        let normalized = blocked_commands
            .into_iter()
            .map(|cmd| cmd.to_lowercase())
            .collect();
        
        Self {
            blocked_commands: normalized,
        }
    }

    /// Create with default blocked commands
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(vec![
            // File operations (destructive)
            "rm".to_string(),
            "rmdir".to_string(),
            "del".to_string(),
            "deltree".to_string(),
            "mv".to_string(),          // NEW: Can rename/move critical files
            "dd".to_string(),
            "shred".to_string(),
            "wipe".to_string(),
            "truncate".to_string(),    // NEW: Can destroy file contents
            
            // Filesystem operations
            "format".to_string(),
            "fdisk".to_string(),
            "mkfs".to_string(),
            "mount".to_string(),       // NEW: Can mount malicious filesystems
            "umount".to_string(),      // NEW: Can unmount critical filesystems
            
            // System control
            "reboot".to_string(),      // NEW
            "shutdown".to_string(),    // NEW
            "halt".to_string(),        // NEW
            "poweroff".to_string(),    // NEW
            "init".to_string(),        // NEW: Can change runlevels
            
            // Process control
            "kill".to_string(),        // NEW
            "killall".to_string(),     // NEW
            "pkill".to_string(),       // NEW
            "killall5".to_string(),    // NEW
            
            // Privilege escalation
            "sudo".to_string(),
            "su".to_string(),
            "doas".to_string(),        // NEW: OpenBSD sudo alternative
            
            // Permission/ownership changes
            "chmod".to_string(),
            "chown".to_string(),
            "chgrp".to_string(),       // NEW: Change group ownership
            "chattr".to_string(),      // NEW: Change file attributes (immutable, etc.)
            
            // User management
            "passwd".to_string(),
            "useradd".to_string(),
            "userdel".to_string(),
            "usermod".to_string(),     // NEW
            "groupadd".to_string(),
            "groupdel".to_string(),
            "groupmod".to_string(),    // NEW
            "visudo".to_string(),      // NEW: Edit sudoers file
            
            // Network operations (exfiltration risk)
            "nc".to_string(),          // NEW: netcat
            "netcat".to_string(),      // NEW
            "wget".to_string(),        // NEW
            "curl".to_string(),        // NEW
            "ftp".to_string(),         // NEW
            "sftp".to_string(),        // NEW
            "scp".to_string(),         // NEW
            "rsync".to_string(),       // NEW
            "ssh".to_string(),         // NEW: Can tunnel/forward
            "telnet".to_string(),      // NEW
            
            // Code execution
            "eval".to_string(),        // NEW: Execute arbitrary code
            "exec".to_string(),        // NEW
            "source".to_string(),      // NEW: Execute script in current shell
            ".".to_string(),           // NEW: Dot command (source alias)
            
            // Command injection vectors
            "find".to_string(),        // NEW: -exec flag allows command injection
            "xargs".to_string(),       // NEW: Executes commands from input
            
            // System modification
            "sysctl".to_string(),      // NEW: Modify kernel parameters
            "modprobe".to_string(),    // NEW: Load kernel modules
            "insmod".to_string(),      // NEW: Insert kernel module
            "rmmod".to_string(),       // NEW: Remove kernel module
            
            // Symlink creation (can bypass restrictions)
            "ln".to_string(),          // NEW: Create hard/soft links
            "link".to_string(),        // NEW
            "unlink".to_string(),      // NEW
        ])
    }

    /// Extract the base command (first word, lowercase, trimmed) from a command string
    /// Handles full paths by extracting just the executable name
    #[must_use]
    pub fn get_base_command(&self, command: &str) -> String {
        let first_word = command
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim();
        
        // Extract basename from path (handles /bin/rm, /usr/bin/sudo, ../../bin/chmod, etc.)
        let basename = if first_word.contains('/') || first_word.contains('\\') {
            // Use std::path::Path for cross-platform path handling
            std::path::Path::new(first_word)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(first_word)
        } else {
            first_word
        };
        
        basename.to_lowercase()
    }

    /// Extract all commands from a command string, handling quotes, escaping, and separators
    /// Returns empty Vec on parsing error (safer than permissive fallback)
    #[must_use]
    pub fn extract_commands(&self, command_string: &str) -> Vec<String> {
        match self.extract_commands_internal(command_string) {
            Ok(commands) => commands,
            Err(e) => {
                // Log the error with full command for debugging
                log::error!(
                    "Error extracting commands from '{}': {}. Treating as potentially malicious.",
                    command_string, e
                );
                
                // SAFER: Return empty Vec to trigger validation failure
                // validate_command() will check if empty and use get_base_command() as fallback
                // This prevents bypasses via deliberately broken parsing
                Vec::new()
            }
        }
    }

    /// Internal implementation for extracting commands
    /// Handles quotes, escape sequences, command separators, and nested structures
    fn extract_commands_internal(&self, command_string: &str) -> Result<Vec<String>, String> {
        let command_string = command_string.trim();
        if command_string.is_empty() {
            return Ok(Vec::new());
        }

        // Define command separators
        let separators = [";", "&&", "||", "|", "&"];
        let mut commands: Vec<String> = Vec::new();

        // State for parsing
        let mut in_quote = false;
        let mut quote_char = '\0';
        let mut current_cmd = String::new();
        let mut escaped = false;

        let chars: Vec<char> = command_string.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            // Handle escape characters
            if ch == '\\' && !escaped {
                escaped = true;
                current_cmd.push(ch);
                i += 1;
                continue;
            }

            // If this character is escaped, just add it
            if escaped {
                escaped = false;
                current_cmd.push(ch);
                i += 1;
                continue;
            }

            // Handle quotes (both single and double)
            if (ch == '"' || ch == '\'') && !in_quote {
                in_quote = true;
                quote_char = ch;
                current_cmd.push(ch);
                i += 1;
                continue;
            } else if ch == quote_char && in_quote {
                in_quote = false;
                quote_char = '\0';
                current_cmd.push(ch);
                i += 1;
                continue;
            }

            // If we're inside quotes, just add the character
            if in_quote {
                current_cmd.push(ch);
                i += 1;
                continue;
            }

            // Handle subshells - if we see an opening parenthesis
            if ch == '(' {
                let subshell_end = Self::find_matching_paren(&chars, i)?;
                if subshell_end > i + 1 {
                    let subshell_content: String =
                        chars[(i + 1)..(subshell_end - 1)].iter().collect();
                    // Recursively extract commands from the subshell
                    let sub_commands = self.extract_commands_internal(&subshell_content)?;
                    commands.extend(sub_commands);
                    i = subshell_end;
                    continue;
                }
            }

            // Check for separators
            let mut is_separator = false;
            for separator in &separators {
                if Self::starts_with_at(&chars, i, separator) {
                    // We found a separator - extract the command before it
                    if !current_cmd.trim().is_empty()
                        && let Some(base_command) = self.extract_base_command(current_cmd.trim()) {
                            commands.push(base_command);
                        }

                    // Move past the separator
                    i += separator.len();
                    current_cmd.clear();
                    is_separator = true;
                    break;
                }
            }

            if !is_separator {
                current_cmd.push(ch);
                i += 1;
            }
        }

        // Don't forget to add the last command
        if !current_cmd.trim().is_empty()
            && let Some(base_command) = self.extract_base_command(current_cmd.trim()) {
                commands.push(base_command);
            }

        // Remove duplicates and return
        let unique_commands: Vec<String> = commands
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        Ok(unique_commands)
    }

    /// Find the matching closing parenthesis for an opening parenthesis at position start
    fn find_matching_paren(chars: &[char], start: usize) -> Result<usize, String> {
        if start >= chars.len() || chars[start] != '(' {
            return Err("Invalid starting position for parenthesis matching".to_string());
        }

        let mut open_parens = 1;
        let mut j = start + 1;

        while j < chars.len() && open_parens > 0 {
            if chars[j] == '(' {
                open_parens += 1;
            } else if chars[j] == ')' {
                open_parens -= 1;
            }
            j += 1;
        }

        if open_parens == 0 {
            Ok(j)
        } else {
            Err("Unmatched parentheses".to_string())
        }
    }

    /// Check if the chars slice starts with the given string at the given position
    fn starts_with_at(chars: &[char], pos: usize, s: &str) -> bool {
        let s_chars: Vec<char> = s.chars().collect();
        if pos + s_chars.len() > chars.len() {
            return false;
        }

        for (i, &expected_char) in s_chars.iter().enumerate() {
            if chars[pos + i] != expected_char {
                return false;
            }
        }
        true
    }

    /// Extract the actual command name from a command string
    /// Removes environment variables and returns the base command
    pub fn extract_base_command(&self, command_str: &str) -> Option<String> {
        if let Ok(cmd) = Self::extract_base_command_internal(command_str) {
            cmd
        } else {
            warn!("Error extracting base command from: {}", command_str);
            None
        }
    }

    /// Internal implementation for extracting base command
    fn extract_base_command_internal(command_str: &str) -> Result<Option<String>, String> {
        // Remove environment variables using pre-compiled regex
        let without_env_vars = ENV_VAR_REGEX.replace_all(command_str, "");
        let trimmed = without_env_vars.trim();

        // If nothing remains after removing env vars, return None
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Get the first token (the command)
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(None);
        }

        let first_token = tokens[0];

        // Check if it starts with special characters that might indicate it's not a regular command
        if first_token.starts_with('(') || first_token.starts_with('$') {
            return Ok(None);
        }

        Ok(Some(first_token.to_lowercase()))
    }

    /// Validate a command against blocked commands list
    /// Returns true if command is allowed, false if blocked
    #[must_use]
    pub fn validate_command(&self, command: &str) -> bool {
        let commands = self.extract_commands(command);
        let base_command = self.get_base_command(command);

        // If extract_commands() returned empty (parsing error), check base command only
        // This is safer than allowing the command through
        if commands.is_empty() {
            log::warn!(
                "Command parsing failed, checking base command only: '{}'",
                base_command
            );
            return !self.blocked_commands.contains(&base_command);
        }

        // Check if any of the extracted commands are in the blocked list
        for cmd in &commands {
            if self.blocked_commands.contains(cmd) {
                log::debug!("Blocked command detected: '{}'", cmd);
                return false; // Command is blocked
            }
        }

        // No commands were blocked
        true
    }

    /// Get the list of blocked commands
    #[must_use]
    pub fn get_blocked_commands(&self) -> &[String] {
        &self.blocked_commands
    }
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}
