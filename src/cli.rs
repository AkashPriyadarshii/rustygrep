use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rustygrep",
    about = "Fast grep with AI-native output",
    version,
    long_about = "rustygrep — a fast, parallel grep tool built in Rust.\n\n\
                   Features token-compressed output optimized for LLM coding agents.\n\
                   Respects .gitignore by default. Skips binary and hidden files."
)]
pub struct Cli {
    /// Pattern to search for (regex supported)
    pub pattern: Option<String>,

    /// Paths to search (files or directories)
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Pretty)]
    pub format: OutputFormat,

    /// Token-compressed output for LLM coding agents
    #[arg(long)]
    pub llm: bool,

    /// JSON Lines output (one object per match line)
    #[arg(long)]
    pub json: bool,

    /// JSON output, one object per file (legacy format)
    #[arg(long)]
    pub json_file: bool,

    /// Token budget: cap total output at N tokens (4 chars ≈ 1 token)
    #[arg(long)]
    pub llm_budget: Option<usize>,

    /// Disable line truncation in LLM output
    #[arg(long)]
    pub llm_no_truncate: bool,

    /// Show only top N files ranked by match count
    #[arg(long)]
    pub top: Option<usize>,

    /// Case insensitive search
    #[arg(short = 'i', long)]
    pub ignore_case: bool,

    /// Whole word match
    #[arg(short = 'w', long)]
    pub word_regexp: bool,

    /// Match count only
    #[arg(short = 'c', long)]
    pub count: bool,

    /// Files with matches only
    #[arg(short = 'l', long)]
    pub files_with_matches: bool,

    /// Show context lines after match
    #[arg(short = 'A', long = "after-context")]
    pub after_context: Option<usize>,

    /// Show context lines before match
    #[arg(short = 'B', long = "before-context")]
    pub before_context: Option<usize>,

    /// Show context lines around match
    #[arg(short = 'C', long = "context")]
    pub context: Option<usize>,

    /// Filter by file type (rs, py, js, ts, go, etc.)
    #[arg(short = 't', long = "type")]
    pub file_type: Option<String>,

    /// Exclude file type
    #[arg(short = 'T', long = "type-not")]
    pub file_type_not: Option<String>,

    /// Truncate long lines at N columns
    #[arg(short = 'M', long = "max-columns", default_value_t = 500)]
    pub max_columns: usize,

    /// Search hidden files and directories
    #[arg(long)]
    pub hidden: bool,

    /// Don't respect .gitignore files
    #[arg(long)]
    pub no_ignore: bool,

    /// Search binary files
    #[arg(long)]
    pub no_binary: bool,

    /// Invert match
    #[arg(short = 'v', long)]
    pub invert_match: bool,

    /// Number of parallel threads (0 = auto)
    #[arg(short = 'j', long, default_value_t = 0)]
    pub threads: usize,

    /// Show search stats (time, matches, files) to stderr
    #[arg(long)]
    pub stats: bool,

    /// Show only context lines, hide the match line itself
    #[arg(long)]
    pub context_only: bool,

    /// No color output
    #[arg(long)]
    pub no_color: bool,

    /// Maximum matches per file (0 = unlimited)
    #[arg(long, default_value_t = 0)]
    pub max_matches: usize,

    /// Subcommands
    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(Subcommand)]
pub enum SubCommand {
    /// Start MCP server for AI coding agents (stdio JSON-RPC)
    Mcp,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable with colors and line numbers
    Pretty,
    /// JSON Lines format (one object per match)
    Json,
    /// Token-compressed for LLM agents
    Llm,
}

impl Cli {
    pub fn context_lines(&self) -> (usize, usize) {
        let before = self.before_context.or(self.context).unwrap_or(0);
        let after = self.after_context.or(self.context).unwrap_or(0);
        (before, after)
    }
}
