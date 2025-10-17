use std::io;
use bytes::Bytes;

use super::types::{KeyCode, Terminal};

impl Terminal {
    /// Feed PTY output bytes into the terminal parser
    pub fn feed_output(&mut self, data: &[u8]) {
        if let Ok(mut parser) = self.parser.write() {
            parser.process(data);
        } else {
            log::error!("Failed to acquire write lock on parser in feed_output");
        }
    }

    /// Send input bytes to the terminal
    pub async fn send_input(&self, bytes: Bytes) -> io::Result<()> {
        self.sender
            .send(bytes)
            .await
            .map_err(|e| io::Error::other(format!("Failed to send input: {}", e)))
    }

    /// Send a character to the terminal
    pub async fn send_char(&self, c: char) -> io::Result<()> {
        self.send_input(Bytes::from(c.to_string().into_bytes()))
            .await
    }

    /// Send a key code to the terminal (special keys like arrows, backspace, etc.)
    pub async fn send_keycode(&self, code: KeyCode) -> io::Result<()> {
        let bytes = match code {
            KeyCode::Backspace => Bytes::from(vec![8]),
            KeyCode::Enter => Bytes::from(vec![b'\n']),
            KeyCode::Left => Bytes::from(vec![27, 91, 68]),
            KeyCode::Right => Bytes::from(vec![27, 91, 67]),
            KeyCode::Up => Bytes::from(vec![27, 91, 65]),
            KeyCode::Down => Bytes::from(vec![27, 91, 66]),
            KeyCode::Tab => Bytes::from(vec![9]),
            KeyCode::Delete => Bytes::from(vec![27, 91, 51, 126]),
            KeyCode::Home => Bytes::from(vec![27, 79, 72]),
            KeyCode::End => Bytes::from(vec![27, 79, 70]),
            KeyCode::PageUp => Bytes::from(vec![27, 91, 53, 126]),
            KeyCode::PageDown => Bytes::from(vec![27, 91, 54, 126]),
            KeyCode::Esc => Bytes::from(vec![27]),
        };

        self.send_input(bytes).await
    }
}
