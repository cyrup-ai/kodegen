//! User interaction prompts and colored output macros
//!
//! ERROR HANDLING STRATEGY FOR DECORATIVE I/O:
//! All termcolor operations use `let _ =` to deliberately ignore errors.
//! Colored output is decorative and non-essential. If stderr/stdout is unavailable
//! (broken pipe, no TTY, etc.), the program continues gracefully without colors.

use crate::error::Result;
use std::io::{self, Write};
use termcolor::{BufferWriter, ColorChoice, WriteColor};

/// Macro for printing warnings with yellow color
///
/// Note: All termcolor operations use `let _ =` to deliberately ignore errors.
/// Colored output is decorative and non-essential. If stderr is unavailable
/// (broken pipe, no TTY, etc.), the program continues gracefully.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        let bufwtr = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Yellow)));
        let _ = write!(&mut buffer, "⚠️  ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, $($arg)*);
        let _ = bufwtr.print(&buffer);
    }};
}

/// Macro for printing errors with red color
///
/// Note: All termcolor operations use `let _ =` to deliberately ignore errors.
/// Colored output is decorative and non-essential. If stderr is unavailable
/// (broken pipe, no TTY, etc.), the program continues gracefully.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        let bufwtr = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Red)));
        let _ = write!(&mut buffer, "❌ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, $($arg)*);
        let _ = bufwtr.print(&buffer);
    }};
}

/// Macro for printing success messages with green color
///
/// Note: All termcolor operations use `let _ =` to deliberately ignore errors.
/// Colored output is decorative and non-essential. If stdout is unavailable
/// (broken pipe, no TTY, etc.), the program continues gracefully.
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        let bufwtr = termcolor::BufferWriter::stdout(termcolor::ColorChoice::Auto);
        let mut buffer = bufwtr.buffer();
        let _ = buffer.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Green)));
        let _ = write!(&mut buffer, "✓ ");
        let _ = buffer.reset();
        let _ = writeln!(&mut buffer, $($arg)*);
        let _ = bufwtr.print(&buffer);
    }};
}

/// Prompt user for .p8 file path with validation loop
///
/// Features:
/// - Allows graceful cancellation via "q", "quit", or Ctrl+D (EOF)
/// - Re-prompts on empty input
/// - Expands ~ to home directory
/// - Validates tilde expansion succeeded
/// - Shows expanded path to user
/// - Validates file with `validate_p8_file()`
/// - Re-prompts on validation errors
/// - Returns Some(path) on success, None if user cancels
pub fn prompt_for_p8_path() -> Result<Option<String>> {
    use crate::macos::validation::{expand_tilde_path, validate_p8_file};
    
    loop {
        // Prompt for input
        print!("Path to .p8 file (or 'q' to cancel): ");
        io::stdout().flush()?;  // IO errors auto-convert via From trait
        
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;
        
        // Handle EOF (Ctrl+D on Unix, Ctrl+Z on Windows)
        if bytes_read == 0 {
            println!("\nSetup cancelled.");
            return Ok(None);
        }
        
        let input = input.trim();
        
        // Handle quit command
        if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("quit") {
            println!("Setup cancelled by user.");
            return Ok(None);
        }
        
        // Empty input check
        if input.is_empty() {
            error!("Path cannot be empty");
            println!("   Enter 'q' to cancel setup");
            continue;
        }
        
        // Expand tilde using helper function with error checking
        let expanded = match expand_tilde_path(input) {
            Ok(path) => path,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };
        
        // Show expanded path if it changed
        if input != expanded {
            let bufwtr = BufferWriter::stdout(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            // Informational output - errors ignored (see module-level docs)
            let _ = writeln!(&mut buffer, "   → {expanded}");
            let _ = bufwtr.print(&buffer);
        }
        
        // Validate the expanded path
        let path = std::path::Path::new(&expanded);
        match validate_p8_file(path) {
            Ok(()) => {
                success!("File validated");
                return Ok(Some(expanded));
            },
            Err(e) => {
                // Display error and re-prompt
                error!("{}", e);
                println!("   Please try again or enter 'q' to cancel\n");
                continue;
            }
        }
    }
}

/// Prompt user for yes/no answer, looping until valid input
///
/// Features:
/// - Accepts "y", "yes", "n", "no" (case insensitive)
/// - Re-prompts on invalid input with clear error message
/// - Re-prompts on empty input
/// - Handles EOF (Ctrl+D) gracefully, treating as "no"
///
/// # Arguments
/// * `question` - The question to ask the user (without "(y/n):" suffix)
///
/// # Returns
/// * `Ok(true)` - User answered yes
/// * `Ok(false)` - User answered no or EOF detected
/// * `Err(SetupError)` - IO error occurred
pub fn prompt_yes_no(question: &str) -> Result<bool> {
    loop {
        print!("{question} (y/n): ");
        io::stdout().flush()?;
        
        let mut response = String::new();
        let bytes_read = io::stdin().read_line(&mut response)?;
        
        // Handle EOF (Ctrl+D)
        if bytes_read == 0 {
            println!("\nEOF detected, treating as 'no'");
            return Ok(false);
        }
        
        let response = response.trim().to_lowercase();
        
        match response.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            "" => {
                eprintln!("⚠️  Empty input. Please enter 'y' for yes or 'n' for no.");
                continue;
            },
            _ => {
                eprintln!("⚠️  Invalid input: '{response}'. Please enter 'y' or 'n'.");
                continue;
            }
        }
    }
}
