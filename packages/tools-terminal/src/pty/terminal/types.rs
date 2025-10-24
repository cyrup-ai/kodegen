use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, RwLock},
};

use bytes::Bytes;
use tokio::{
    sync::{
        Mutex,
        mpsc::{Receiver, Sender},
    },
    task,
};
use vt100::{Parser, Screen};

/// Represents a virtual terminal component
pub struct Terminal {
    pub(super) parser: Arc<RwLock<Parser>>,
    pub(super) sender: Option<Sender<Bytes>>,
    pub(super) receiver: Option<Receiver<Bytes>>,
    pub(super) size: TermSize,
    pub(super) pty_closed: Arc<AtomicBool>,
    pub(super) config: TerminalConfig,
    pub(super) child_process: Option<Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>>,
    pub(super) reader_task: Option<task::JoinHandle<()>>,
    pub(super) writer_task: Option<task::JoinHandle<()>>,
}

impl Clone for Terminal {
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            sender: self.sender.clone(),
            receiver: None,
            size: self.size.clone(),
            pty_closed: self.pty_closed.clone(),
            config: self.config.clone(),
            child_process: self.child_process.clone(),
            reader_task: None,
            writer_task: None,
        }
    }
}

impl Terminal {
    /// Check if the PTY has been detected as closed by the output reader task
    #[must_use]
    pub fn is_pty_closed(&self) -> bool {
        self.pty_closed.load(Ordering::SeqCst)
    }

    /// Get the terminal screen
    /// Returns None if the parser lock is poisoned.
    #[must_use]
    pub fn screen(&self) -> Option<Screen> {
        match self.parser.read() {
            Ok(p) => Some(p.screen().clone()),
            Err(e) => {
                log::error!("Parser lock poisoned when trying to read screen: {e}");
                None
            }
        }
    }
}

/// Size information for the terminal
#[derive(Debug, Clone)]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}

/// Terminal color mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    Basic,    // No colors
    Color,    // Basic 8 colors
    Color256, // 256 colors
    #[default]
    TrueColor, // 24-bit true color
}

/// Terminal bell style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BellStyle {
    None, // No bell
    #[default]
    Visual, // Visual bell
    Audible, // Audible bell
}

/// Configuration for terminal behavior and appearance
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub shell: bool,
    pub shell_path: Option<String>,
    pub colors: ColorMode,
    pub scrollback: usize,
    pub cursor_blink: bool,
    pub application_cursor_keys: bool,
    pub bracketed_paste: bool,
    pub mouse_reporting: bool,
    pub alternate_screen: bool,
    pub exit_on_close: bool,
    pub bell_style: BellStyle,
}

/// Keyboard key codes for terminal input
#[derive(Debug, Clone, Copy)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Tab,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Esc,
    // Add other key codes as needed
}
