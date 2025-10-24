//! Concurrency primitives and utilities

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use crate::async_stream;
use crate::domain::concurrency::{ChannelResult, OneshotResult};
use cyrup_sugars::prelude::MessageChunk;

/// A multi-producer, single-consumer channel for sending values between tasks
pub struct Channel<T> {
    sender: mpsc::UnboundedSender<T>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<T>>>}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone()}
    }
}

impl<T: Send + 'static + MessageChunk + Default> Channel<T> {
    /// Create a new channel with the given buffer size
    pub fn new(_buffer: usize) -> Self {
        // Note: tokio mpsc unbounded_channel is used regardless of buffer size
        // for consistency with the async runtime
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver))}
    }

    /// Send a value into the channel
    pub fn send(&self, value: T) -> impl Stream<Item = ChannelResult> {
        let sender = self.sender.clone();
        async_stream::spawn_stream(move |tx| async move {
            let result = match sender.send(value) {
                Ok(()) => ChannelResult {
                    success: true,
                    error_message: None,
                },
                Err(_) => ChannelResult::bad_chunk("Send error".to_string()),
            };
            let _ = tx.send(result);
        })
    }

    /// Receive the next value from the channel
    pub fn recv(&self) -> impl Stream<Item = T> {
        let receiver = self.receiver.clone();
        async_stream::spawn_stream(move |tx| async move {
            let mut guard = receiver.lock().await;
            if let Some(value) = guard.recv().await {
                let _ = tx.send(value);
            }
        })
    }

    /// Create a new receiver that can be used to receive values from this channel
    pub fn subscribe(&self) -> impl Stream<Item = T> {
        let receiver = self.receiver.clone();
        async_stream::spawn_stream(move |tx| async move {
            let mut guard = receiver.lock().await;
            while let Some(value) = guard.recv().await {
                if tx.send(value).is_err() {
                    break;
                }
            }
        })
    }
}

/// A oneshot channel for sending a single value between tasks
pub struct OneshotChannel<T> {
    sender: Option<tokio::sync::oneshot::Sender<T>>,
    receiver: tokio::sync::oneshot::Receiver<T>}

impl<T> OneshotChannel<T> {
    /// Create a new oneshot channel
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        Self { 
            sender: Some(sender), 
            receiver 
        }
    }

    /// Send a value through the channel
    pub fn send(mut self, value: T) -> Result<(), T> {
        if let Some(sender) = self.sender.take() {
            sender.send(value).map_err(|value| value)
        } else {
            Err(value)
        }
    }

    /// Receive the value from the channel
    pub fn recv(self) -> impl Stream<Item = OneshotResult<T>> {
        async_stream::spawn_stream(move |tx| async move {
            let result = match self.receiver.await {
                Ok(value) => OneshotResult::Ok(value),
                Err(_) => OneshotResult::Err("Channel closed".to_string()),
            };
            let _ = tx.send(result);
        })
    }
}

impl<T> Default for OneshotChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Candle-prefixed type aliases for domain compatibility
pub type CandleChannel<T> = Channel<T>;
pub type CandleOneshotChannel<T> = OneshotChannel<T>;