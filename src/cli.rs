use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "proof", version, about = "Branded delivery proof generator")]
pub struct Cli {
    /// Directory containing delivery assets
    pub input: PathBuf,

    /// Client name (appears on cover page)
    #[arg(short, long)]
    pub client: Option<String>,

    /// Document title
    #[arg(short, long)]
    pub title: Option<String>,

    /// Delivery date (defaults to today)
    #[arg(short, long)]
    pub date: Option<String>,

    /// Grid columns for contact sheet (3-8)
    #[arg(long, default_value = "4", value_parser = clap::value_parser!(u8).range(3..=8))]
    pub columns: u8,

    /// Output PDF file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Only output asset manifest to stdout (skip PDF)
    #[arg(long)]
    pub manifest_only: bool,

    /// Show what would be processed without generating output
    #[arg(long)]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Disable TUI dashboard (use plain text output)
    #[arg(long)]
    pub no_tui: bool,
}
