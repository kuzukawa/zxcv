use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "zxcv",
    version,
    about = "Generate shell one-liner commands from natural language using an LLM."
)]
pub struct Cli {
    /// Natural-language description of the command you want.
    pub query: Option<String>,

    /// Override the provider (anthropic | openai | ollama | gemini).
    #[arg(long, global = true)]
    pub provider: Option<String>,

    /// Override the model name (provider-dependent default).
    #[arg(long, global = true)]
    pub model: Option<String>,

    /// Internal: emit history + LLM candidates as TSV to stdout (used by fzf reload).
    #[arg(long, hide = true)]
    pub internal: bool,

    #[command(subcommand)]
    pub command: Option<Subcmd>,
}

#[derive(Subcommand, Debug)]
pub enum Subcmd {
    /// Print shell integration script for the given shell.
    ///
    /// Add `eval "$(zxcv init zsh)"` to your ~/.zshrc, then bind a key to `zxcv-widget`.
    Init {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Open the config file in $EDITOR (creates it from a template if missing).
    Config,
    /// Manage selection history.
    History {
        #[command(subcommand)]
        action: Option<HistoryAction>,
    },
    /// Install man pages under <prefix>/man/man1/.
    ///
    /// Default prefix: $XDG_DATA_HOME (or ~/.local/share). Use --prefix /usr/local
    /// (with sudo) for a system-wide install at /usr/local/man/man1/.
    InstallMan {
        /// Install prefix; pages go under <prefix>/man/man1/.
        #[arg(long)]
        prefix: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum HistoryAction {
    /// Delete all history entries.
    Clear,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
pub enum Shell {
    Zsh,
    Bash,
}
