use std::{
    collections::HashMap,
    sync::{Arc, RwLock, atomic::AtomicBool},
};
use tokio::sync::mpsc::channel;
use vt100::Parser;

use super::types::{BellStyle, ColorMode, TermSize, Terminal, TerminalConfig};

/// Builder for creating Terminal instances with a fluent API
#[derive(Default)]
pub struct TerminalBuilder {
    rows: Option<u16>,
    cols: Option<u16>,
    command: Option<String>,
    cwd: Option<String>,
    env_vars: HashMap<String, String>,
    shell: bool,
    shell_path: Option<String>,
    colors: ColorMode,
    scrollback: usize,
    cursor_blink: bool,
    application_cursor_keys: bool,
    bracketed_paste: bool,
    mouse_reporting: bool,
    alternate_screen: bool,
    exit_on_close: bool,
    bell_style: BellStyle,
}

impl TerminalBuilder {
    /// Create a new terminal builder with optimized defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Default to a comfortable terminal size
            rows: Some(30),
            cols: Some(100),
            command: None,
            cwd: None,
            env_vars: HashMap::from([
                // Enable truecolor support by default
                ("COLORTERM".to_string(), "truecolor".to_string()),
                // Ensure UTF-8 support
                ("LANG".to_string(), "en_US.UTF-8".to_string()),
                // Prefer more modern terminal features
                ("TERM".to_string(), "xterm-256color".to_string()),
            ]),
            shell: false,
            shell_path: None,
            colors: ColorMode::TrueColor,
            scrollback: 10000, // Generous scrollback by default
            cursor_blink: true,
            application_cursor_keys: true,
            bracketed_paste: true,
            mouse_reporting: true,
            alternate_screen: false, // Start in normal screen mode
            exit_on_close: true,     // Clean exit by default
            bell_style: BellStyle::Visual,
        }
    }

    /// Set terminal dimensions
    #[must_use]
    pub fn size(mut self, rows: u16, cols: u16) -> Self {
        self.rows = Some(rows);
        self.cols = Some(cols);
        self
    }

    /// Set command to execute
    pub fn command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }

    /// Set working directory
    pub fn cwd(mut self, dir: impl Into<String>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables
    pub fn envs<K, V, I>(mut self, vars: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in vars {
            self.env_vars.insert(k.into(), v.into());
        }
        self
    }

    /// Use interactive shell mode
    #[must_use]
    pub fn shell(mut self, enable: bool) -> Self {
        self.shell = enable;
        self
    }

    /// Specify which shell executable to use (overrides default detection)
    pub fn shell_path(mut self, path: impl Into<String>) -> Self {
        self.shell_path = Some(path.into());
        self
    }

    /// Set color mode
    #[must_use]
    pub fn colors(mut self, mode: ColorMode) -> Self {
        self.colors = mode;
        self
    }

    /// Set scrollback buffer size
    #[must_use]
    pub fn scrollback(mut self, lines: usize) -> Self {
        self.scrollback = lines;
        self
    }

    /// Force color output for child processes
    #[must_use]
    pub fn force_color(mut self, enable: bool) -> Self {
        if enable {
            self.env_vars
                .insert("FORCE_COLOR".to_string(), "1".to_string());
            self.env_vars
                .insert("CLICOLOR_FORCE".to_string(), "1".to_string());
            self.env_vars
                .insert("RUST_LOG_STYLE".to_string(), "always".to_string());
        }
        self
    }

    /// Configure cursor blinking
    #[must_use]
    pub fn cursor_blink(mut self, enable: bool) -> Self {
        self.cursor_blink = enable;
        self
    }

    /// Enable/disable application cursor keys mode
    #[must_use]
    pub fn application_cursor_keys(mut self, enable: bool) -> Self {
        self.application_cursor_keys = enable;
        self
    }

    /// Enable/disable bracketed paste mode
    #[must_use]
    pub fn bracketed_paste(mut self, enable: bool) -> Self {
        self.bracketed_paste = enable;
        self
    }

    /// Enable/disable mouse reporting
    #[must_use]
    pub fn mouse_reporting(mut self, enable: bool) -> Self {
        self.mouse_reporting = enable;
        self
    }

    /// Use alternate screen buffer
    #[must_use]
    pub fn alternate_screen(mut self, enable: bool) -> Self {
        self.alternate_screen = enable;
        self
    }

    /// Control whether to kill process on terminal close
    #[must_use]
    pub fn exit_on_close(mut self, enable: bool) -> Self {
        self.exit_on_close = enable;
        self
    }

    /// Set bell style
    #[must_use]
    pub fn bell_style(mut self, style: BellStyle) -> Self {
        self.bell_style = style;
        self
    }

    /// Preset for minimal terminal (good for running simple commands)
    #[must_use]
    pub fn minimal(mut self) -> Self {
        self.scrollback = 100;
        self.cursor_blink = false;
        self.application_cursor_keys = false;
        self.bracketed_paste = false;
        self.mouse_reporting = false;
        self.alternate_screen = false;
        self.exit_on_close = true;
        self
    }

    /// Preset for full-featured interactive terminal
    #[must_use]
    pub fn interactive(mut self) -> Self {
        self.scrollback = 10000;
        self.cursor_blink = true;
        self.application_cursor_keys = true;
        self.bracketed_paste = true;
        self.mouse_reporting = true;
        self.alternate_screen = true;
        self.exit_on_close = false;
        self.colors = ColorMode::TrueColor;
        self
    }

    /// Build the terminal with all the configured options
    #[must_use]
    pub fn build(self) -> Terminal {
        // Use sensible defaults for anything not specified
        let rows = self.rows.unwrap_or(30);
        let cols = self.cols.unwrap_or(100);

        // Create parser and channels
        let parser = Arc::new(RwLock::new(Parser::new(rows, cols, self.scrollback)));

        let (sender, receiver) = channel(100);

        let term_size = TermSize { cols, rows };

        // Create configuration from builder settings
        let config = TerminalConfig {
            command: self.command,
            cwd: self.cwd,
            env_vars: self.env_vars,
            shell: self.shell,
            shell_path: self.shell_path,
            colors: self.colors,
            scrollback: self.scrollback,
            cursor_blink: self.cursor_blink,
            application_cursor_keys: self.application_cursor_keys,
            bracketed_paste: self.bracketed_paste,
            mouse_reporting: self.mouse_reporting,
            alternate_screen: self.alternate_screen,
            exit_on_close: self.exit_on_close,
            bell_style: self.bell_style,
        };

        Terminal {
            parser,
            sender: Some(sender),
            receiver: Some(receiver),
            size: term_size,
            pty_closed: Arc::new(AtomicBool::new(false)),
            config,
            child_process: None,
            reader_task: None,
            writer_task: None,
        }
    }
}
