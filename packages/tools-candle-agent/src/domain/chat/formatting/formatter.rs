//! Message formatter implementation with streaming support

use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use tokio_stream::Stream;

use pulldown_cmark::{Options as MdOptions, Parser as MdParser, html as md_html};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::domain::chat::formatting::{
    content::ImmutableMessageContent,
    error::{FormatError, FormatResult},
    events::FormattingEvent,
    options::{ImmutableFormatOptions, SyntaxTheme},
};
use crate::domain::util::unix_timestamp_nanos;

/// Global syntax set for code highlighting (loaded once)
static SYNTAX_SET: std::sync::LazyLock<SyntaxSet> =
    std::sync::LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Global theme set for code highlighting (loaded once)  
static THEME_SET: std::sync::LazyLock<ThemeSet> = std::sync::LazyLock::new(ThemeSet::load_defaults);

/// Streaming message formatter with atomic state tracking
pub struct StreamingMessageFormatter {
    /// Content counter (atomic)
    content_counter: AtomicU64,
    /// Active formatting operations (atomic)
    active_operations: AtomicUsize,
    /// Total operations (atomic)
    total_operations: AtomicU64,
    /// Successful operations (atomic)
    successful_operations: AtomicU64,
    /// Failed operations (atomic)
    failed_operations: AtomicU64,
    /// Event stream sender
    event_sender: Option<tokio::sync::mpsc::UnboundedSender<FormattingEvent>>,
    /// Formatter configuration
    options: ImmutableFormatOptions,
}

impl std::fmt::Debug for StreamingMessageFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingMessageFormatter")
            .field(
                "content_counter",
                &self
                    .content_counter
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "active_operations",
                &self
                    .active_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "total_operations",
                &self
                    .total_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "successful_operations",
                &self
                    .successful_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "failed_operations",
                &self
                    .failed_operations
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("event_sender", &self.event_sender.is_some())
            .field("options", &self.options)
            .finish()
    }
}

/// Type alias for formatter with streaming result
pub type FormatterWithStream = (
    StreamingMessageFormatter,
    Pin<Box<dyn Stream<Item = FormattingEvent> + Send>>,
);

impl StreamingMessageFormatter {
    /// Create new streaming message formatter
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if options validation fails
    #[inline]
    pub fn new(options: ImmutableFormatOptions) -> FormatResult<Self> {
        options.validate()?;
        Ok(Self {
            content_counter: AtomicU64::new(0),
            active_operations: AtomicUsize::new(0),
            total_operations: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            event_sender: None,
            options,
        })
    }

    /// Create formatter with event streaming
    ///
    /// # Errors
    ///
    /// Returns `FormatError::ConfigurationError` if options validation fails
    #[inline]
    pub fn with_streaming(options: ImmutableFormatOptions) -> FormatResult<FormatterWithStream> {
        options.validate()?;
        let stream = Box::pin(crate::async_stream::spawn_stream(|_sender| async move {
            // Stream is created but not used directly
        }));
        let formatter = Self {
            content_counter: AtomicU64::new(0),
            active_operations: AtomicUsize::new(0),
            total_operations: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            event_sender: None, // Will be set up separately if needed
            options,
        };
        Ok((formatter, stream))
    }

    /// Format markdown content to HTML
    ///
    /// Uses pulldown-cmark with all extensions enabled.
    fn format_markdown_to_html(markdown: &str) -> String {
        // Enable all markdown extensions for rich formatting
        let mut options = MdOptions::empty();
        options.insert(MdOptions::ENABLE_STRIKETHROUGH);
        options.insert(MdOptions::ENABLE_TABLES);
        options.insert(MdOptions::ENABLE_FOOTNOTES);
        options.insert(MdOptions::ENABLE_TASKLISTS);
        options.insert(MdOptions::ENABLE_SMART_PUNCTUATION);
        options.insert(MdOptions::ENABLE_HEADING_ATTRIBUTES);

        // Parse markdown
        let parser = MdParser::new_ext(markdown, options);

        // Convert to HTML
        let mut html_output = String::with_capacity(markdown.len() * 3 / 2);
        md_html::push_html(&mut html_output, parser);

        html_output
    }

    /// Format code with syntax highlighting to HTML
    ///
    /// Uses syntect with language-specific syntax definitions.
    ///
    /// # Errors
    ///
    /// Returns `FormatError::UnsupportedLanguage` if language not recognized
    /// Returns `FormatError::RenderError` if highlighting fails
    fn format_code_to_html(&self, code: &str, language: &str) -> FormatResult<String> {
        // Find syntax definition for language
        let syntax = SYNTAX_SET.find_syntax_by_token(language).ok_or_else(|| {
            FormatError::UnsupportedLanguage {
                language: language.to_string(),
            }
        })?;

        // Get theme based on formatter options
        let theme_name = match self.options.syntax_theme {
            SyntaxTheme::GitHub => "base16-ocean.light",
            SyntaxTheme::SolarizedLight => "Solarized (light)",
            SyntaxTheme::SolarizedDark => "Solarized (dark)",
            SyntaxTheme::Light => "InspiredGitHub",
            SyntaxTheme::Dark
            | SyntaxTheme::VSCode
            | SyntaxTheme::HighContrast
            | SyntaxTheme::Custom => "base16-ocean.dark", // fallback
        };

        let theme =
            THEME_SET
                .themes
                .get(theme_name)
                .ok_or_else(|| FormatError::ConfigurationError {
                    detail: format!("Theme '{theme_name}' not found in theme set"),
                })?;

        // Generate syntax-highlighted HTML
        highlighted_html_for_string(code, &SYNTAX_SET, syntax, theme).map_err(|e| {
            FormatError::RenderError {
                detail: format!("Syntax highlighting failed: {e}"),
            }
        })
    }

    /// Format content with streaming events
    ///
    /// # Errors
    ///
    /// Returns `FormatError::InvalidContent` if content validation fails
    #[inline]
    pub fn format_content(&self, content: &ImmutableMessageContent) -> FormatResult<u64> {
        // Validate content first
        content.validate()?;

        // Generate content ID
        let content_id = self.content_counter.fetch_add(1, Ordering::Relaxed);

        // Update counters
        self.active_operations.fetch_add(1, Ordering::Relaxed);
        self.total_operations.fetch_add(1, Ordering::Relaxed);

        // Send started event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(FormattingEvent::Started {
                content_id,
                content_type: content.content_type().to_string(),
                timestamp_nanos: Self::current_timestamp_nanos(),
            });
        }

        // Record start time for duration tracking
        let start_time = std::time::Instant::now();

        // Process content based on type and create formatted version
        let formatted_result = match content {
            ImmutableMessageContent::Markdown {
                content: md_content,
                ..
            } => {
                // Format markdown to HTML
                let html = Self::format_markdown_to_html(md_content);
                // Create new content with rendered HTML
                Ok(ImmutableMessageContent::Markdown {
                    content: md_content.clone(),
                    rendered_html: Some(html),
                })
            }
            ImmutableMessageContent::Code {
                content: code_content,
                language,
                ..
            } => {
                // Format code with syntax highlighting
                match self.format_code_to_html(code_content, language) {
                    Ok(html) => {
                        // Create new content with highlighted HTML
                        Ok(ImmutableMessageContent::Code {
                            content: code_content.clone(),
                            language: language.clone(),
                            highlighted: Some(html),
                        })
                    }
                    Err(e) => Err(e),
                }
            }
            // Pass through other content types unchanged
            ImmutableMessageContent::Plain { .. }
            | ImmutableMessageContent::Formatted { .. }
            | ImmutableMessageContent::Composite { .. } => Ok(content.clone()),
        };

        // Calculate duration
        let duration = start_time.elapsed();

        // Emit event based on result
        if let Some(ref sender) = self.event_sender {
            match formatted_result {
                Ok(ref result) => {
                    // Success - emit Completed event
                    let _ = sender.send(FormattingEvent::Completed {
                        content_id,
                        result: result.clone(),
                        duration,
                    });
                    // Update success counter
                    self.successful_operations.fetch_add(1, Ordering::Relaxed);
                }
                Err(ref error) => {
                    // Failure - emit Failed event
                    let _ = sender.send(FormattingEvent::Failed {
                        content_id,
                        error: error.clone(),
                        duration,
                    });
                    // Update failure counter
                    self.failed_operations.fetch_add(1, Ordering::Relaxed);
                }
            }
        } else {
            // No event sender - just update counters
            match formatted_result {
                Ok(_) => {
                    self.successful_operations.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    self.failed_operations.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Decrement active operations counter
        self.active_operations.fetch_sub(1, Ordering::Relaxed);

        // Return result (propagate error or return content_id)
        formatted_result.map(|_| content_id)
    }

    /// Get current timestamp in nanoseconds
    #[inline]
    fn current_timestamp_nanos() -> u64 {
        unix_timestamp_nanos()
    }

    /// Get formatting statistics (atomic reads)
    #[inline]
    pub fn stats(&self) -> FormatterStats {
        FormatterStats {
            active_operations: self.active_operations.load(Ordering::Relaxed) as u64,
            total_operations: self.total_operations.load(Ordering::Relaxed),
            successful_operations: self.successful_operations.load(Ordering::Relaxed),
            failed_operations: self.failed_operations.load(Ordering::Relaxed),
        }
    }

    /// Get formatter options (borrowed reference)
    #[inline]
    pub fn options(&self) -> &ImmutableFormatOptions {
        &self.options
    }

    /// Update formatter options
    ///
    /// # Errors
    ///
    /// Returns `FormatError` if options validation fails
    #[inline]
    pub fn update_options(&mut self, options: ImmutableFormatOptions) -> FormatResult<()> {
        options.validate()?;
        self.options = options;
        Ok(())
    }
}

/// Formatter statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatterStats {
    /// Number of currently active formatting operations
    pub active_operations: u64,
    /// Total number of formatting operations attempted
    pub total_operations: u64,
    /// Number of operations that completed successfully
    pub successful_operations: u64,
    /// Number of operations that failed
    pub failed_operations: u64,
}

impl FormatterStats {
    /// Calculate success rate as percentage
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        let completed = self.successful_operations + self.failed_operations;
        if completed == 0 {
            0.0
        } else {
            (self.successful_operations as f64 / completed as f64) * 100.0
        }
    }

    /// Calculate failure rate as percentage
    #[inline]
    #[must_use]
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }
}
