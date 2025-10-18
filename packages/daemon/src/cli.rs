use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "kodegen service manager")]
pub struct Args {
    /// Sub‑commands (run, install, etc.)
    #[command(subcommand)]
    pub sub: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Normal daemon operation (default if no sub‑command)
    Run {
        /// Stay in foreground even on plain Unix
        #[arg(long)]
        foreground: bool,

        /// Path to configuration file
        #[arg(long, short = 'c')]
        config: Option<String>,

        /// Use system-wide config (/etc/kodegend/kodegend.toml)
        #[arg(long, conflicts_with = "config")]
        system: bool,
    },
    /// Sign the daemon binary
    Sign {
        /// Path to binary to sign (defaults to current executable)
        #[arg(long)]
        binary: Option<String>,

        /// Signing identity (macOS) or certificate (Windows)
        #[arg(long)]
        identity: Option<String>,

        /// Verify signature only, don't sign
        #[arg(long)]
        verify: bool,

        /// Show sample signing configuration
        #[arg(long)]
        show_config: bool,

        /// Sign the currently running binary (self-sign)
        #[arg(long)]
        self_sign: bool,
    },
    /// Check daemon status (Exit 0 = running, 1 = stopped)
    Status,
    /// Start the daemon service (Exit 0 = success, 1 = failed)
    Start,
    /// Stop the daemon service (Exit 0 = success, 1 = failed)
    Stop,
    /// Restart the daemon service (Exit 0 = success, 1 = failed)
    Restart,
}
