mod cache;
mod candidate;
mod cli;
mod config;
mod debug;
mod fzf;
mod history;
mod init;
mod install_man;
mod paths;
mod providers;
mod safety;

use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::candidate::Candidate;
use crate::cli::{Cli, HistoryAction, Subcmd};
use crate::config::Config;
use crate::providers::Settings;

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# zxcv configuration
# Precedence: CLI args > environment variables > this file > built-in defaults

# provider = "anthropic"   # anthropic | openai | ollama | gemini

# [providers.anthropic]
# model = "claude-sonnet-4-6"
# api_key = "sk-ant-..."   # or set ANTHROPIC_API_KEY env

# [providers.openai]
# model = "gpt-5"
# api_key = "sk-..."       # or set OPENAI_API_KEY env

# [providers.ollama]
# endpoint = "http://localhost:11434"
# model = "llama3"

# [providers.gemini]
# model = "gemini-2.5-flash"
# api_key = "..."          # or set GEMINI_API_KEY env

# Additional destructive-command regex patterns.
# [safety]
# extra_patterns = ["^my-dangerous-cmd"]
"#;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        return run_subcommand(cmd);
    }
    let config = config::load()?;
    let settings = config::resolve(&cli, &config)?;
    debug::log(format!(
        "settings: provider={} model={} endpoint={:?} api_key={}",
        settings.provider.id(),
        settings.model,
        settings.endpoint,
        if settings.api_key.is_some() {
            "<set>"
        } else {
            "<unset>"
        }
    ));

    if cli.internal {
        run_internal(cli, settings).await
    } else {
        run_interactive(cli, &config).await
    }
}

fn run_subcommand(cmd: Subcmd) -> Result<()> {
    match cmd {
        Subcmd::Init { shell } => {
            print!("{}", init::script(shell));
            Ok(())
        }
        Subcmd::Config => run_config_edit(),
        Subcmd::History { action } => run_history(action),
        Subcmd::InstallMan { prefix } => install_man::install(prefix),
    }
}

fn run_config_edit() -> Result<()> {
    let path = paths::config_file()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, DEFAULT_CONFIG_TEMPLATE)
            .with_context(|| format!("failed to write default config to {}", path.display()))?;
        eprintln!("zxcv: created default config at {}", path.display());
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("failed to launch editor `{editor}`"))?;
    if !status.success() {
        bail!("editor `{editor}` exited with status {status}");
    }
    Ok(())
}

fn run_history(action: Option<HistoryAction>) -> Result<()> {
    match action {
        None => {
            let h = history::load()?;
            let entries = history::sorted_by_frecency(&h);
            if entries.is_empty() {
                eprintln!("zxcv: history is empty");
                return Ok(());
            }
            for e in entries {
                println!(
                    "{}\t{}\tcount={}\tquery={}",
                    e.command, e.description, e.count, e.query
                );
            }
            Ok(())
        }
        Some(HistoryAction::Clear) => {
            let path = paths::history_file()?;
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
            eprintln!("zxcv: history cleared");
            Ok(())
        }
    }
}

async fn run_interactive(cli: Cli, config: &Config) -> Result<()> {
    fzf::ensure_available()?;
    show_setup_hint_if_needed();
    let detector = safety::Detector::from_config(&config.safety)?;

    let mut history = history::load()?;
    let initial_query = cli.query.as_deref().unwrap_or("");
    let history_candidates: Vec<Candidate> = history::sorted_by_frecency(&history)
        .iter()
        .map(|e| e.to_candidate())
        .collect();

    let Some(selected) = fzf::pick(initial_query, &history_candidates)? else {
        // user cancelled
        return Ok(());
    };

    if detector.is_dangerous(&selected.command) {
        let matched = detector.matched(&selected.command);
        if !confirm_dangerous(&selected.command, &matched)? {
            debug::log("run_interactive: user declined dangerous command");
            return Ok(());
        }
    }

    history::record(&mut history, initial_query, &selected);
    history::save(&history)?;

    println!("{}", selected.command);
    Ok(())
}

fn confirm_dangerous(command: &str, matched: &[String]) -> Result<bool> {
    eprintln!("[zxcv] Warning: potentially destructive command detected:");
    eprintln!("       {command}");
    for p in matched {
        eprintln!("       matched pattern: {p}");
    }
    eprint!("       Use anyway? [y/N] ");
    io::stderr().flush().ok();
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().eq_ignore_ascii_case("y"))
}

async fn run_internal(cli: Cli, settings: Settings) -> Result<()> {
    let query = cli.query.unwrap_or_default();
    debug::log(format!("run_internal: query={query:?}"));

    let history = history::load()?;
    let history_candidates: Vec<Candidate> = history::sorted_by_frecency(&history)
        .iter()
        .map(|e| e.to_candidate())
        .collect();
    debug::log(format!(
        "run_internal: history_candidates={}",
        history_candidates.len()
    ));

    let llm_candidates = if query.trim().is_empty() {
        debug::log("run_internal: query is empty, skipping LLM call");
        let _ = writeln!(
            io::stdout(),
            "[zxcv] Type a description first, then press Ctrl-G to generate candidates.\t\
             Type a description first, then press Ctrl-G to generate candidates."
        );
        Vec::new()
    } else {
        let provider_id = settings.provider.id();
        match cache::load(provider_id, &settings.model, &query)? {
            Some(c) => {
                debug::log(format!("run_internal: cache hit, {} candidates", c.len()));
                c
            }
            None => {
                debug::log(format!(
                    "run_internal: calling {} (model={})",
                    provider_id, settings.model
                ));
                match providers::generate(&settings, &query).await {
                    Ok(c) => {
                        debug::log(format!(
                            "run_internal: LLM returned {} candidates",
                            c.len()
                        ));
                        cache::save(provider_id, &settings.model, &query, &c)?;
                        c
                    }
                    Err(e) => {
                        debug::log(format!("run_internal: LLM call failed: {e:#}"));
                        let full = e.to_string().replace('\t', " ");
                        let first_line = full.lines().next().unwrap_or("unknown error");
                        let preview = full.replace('\n', " | ");
                        let _ = writeln!(io::stdout(), "[zxcv error] {first_line}\t{preview}");
                        Vec::new()
                    }
                }
            }
        }
    };

    let merged = merge(history_candidates, llm_candidates);
    debug::log(format!(
        "run_internal: writing {} merged candidates to stdout",
        merged.len()
    ));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    fzf::write_candidates(&mut out, &merged)?;
    out.flush().ok();
    Ok(())
}

/// Print a one-time setup hint when zxcv is invoked outside the shell widget and the
/// sentinel flag file does not yet exist. Blocks on Enter so the user sees it before fzf
/// repaints the terminal.
fn show_setup_hint_if_needed() {
    if std::env::var("ZXCV_FROM_WIDGET").is_ok() {
        return;
    }
    let Ok(path) = paths::hint_flag_file() else {
        return;
    };
    if path.exists() {
        return;
    }
    eprintln!(
        "\
[zxcv] Tip: for inline command insertion in your shell, add to ~/.zshrc:

       eval \"$(zxcv init zsh)\"
       bindkey '^[z' zxcv-widget   # Alt+Z

       To enable `man zxcv`, run once:

       zxcv install-man

       This one-time hint is being shown because this is your first run.
       To see it again, delete: {}
",
        path.display()
    );
    eprint!("       Press Enter to continue...");
    io::stderr().flush().ok();
    let mut buf = String::new();
    let _ = io::stdin().lock().read_line(&mut buf);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "shown\n");
}

fn merge(history: Vec<Candidate>, llm: Vec<Candidate>) -> Vec<Candidate> {
    let mut seen: HashSet<String> = history.iter().map(|c| c.command.clone()).collect();
    let mut result = history;
    for c in llm {
        if seen.insert(c.command.clone()) {
            result.push(c);
        }
    }
    result
}
