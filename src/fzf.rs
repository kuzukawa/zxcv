use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};

use crate::candidate::Candidate;
use crate::debug;

const HEADER_HISTORY: &str =
    "MODE: HISTORY (frecency)  |  Enter: pick  |  Ctrl-G: generate with LLM  |  Esc: quit";
const HEADER_LLM: &str =
    "MODE: LLM CANDIDATES  |  Enter: pick  |  Ctrl-C: back to history  |  Esc: quit";

pub fn ensure_available() -> Result<()> {
    let status = Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => bail!("`fzf` exists on PATH but `fzf --version` did not succeed"),
        Err(_) => bail!("`fzf` is required but was not found on PATH. Please install fzf."),
    }
}

/// Run fzf with the given pre-loaded candidates. Returns the selected candidate, or `None` if the
/// user cancelled.
pub fn pick(initial_query: &str, candidates: &[Candidate]) -> Result<Option<Candidate>> {
    let exe = std::env::current_exe().context("failed to determine current executable path")?;
    let exe_quoted = shell_single_quote(&exe.to_string_lossy());

    // fzf auto-shell-quotes {q}, so do NOT wrap it in additional quotes here.
    let reload_cmd = format!("{exe_quoted} --internal -- {{q}}");
    let history_cmd = format!("{exe_quoted} --history-only");
    // clear-query removes the current input so fzf's filter doesn't hide the reloaded rows
    // (whose English columns may not fuzzy-match a Japanese query).
    // ctrl-c cancels any ongoing LLM reload and restores history without exiting fzf.
    let bind_llm = format!(
        "ctrl-g:reload({reload_cmd})+clear-query+change-header({HEADER_LLM})"
    );
    let bind_cancel = format!(
        "ctrl-c:reload({history_cmd})+change-header({HEADER_HISTORY})"
    );

    let mut cmd = Command::new("fzf");
    cmd.arg("--delimiter=\t")
        .arg("--with-nth=1")
        .arg("--preview=printf '%s' {2}")
        .arg("--preview-window=down:3:wrap")
        .arg(format!("--bind={bind_llm}"))
        .arg(format!("--bind={bind_cancel}"))
        .arg(format!("--query={initial_query}"))
        .arg(format!("--header={HEADER_HISTORY}"))
        .arg("--no-multi")
        .arg("--ansi")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    debug::log(format!("fzf: bind_llm={bind_llm} bind_cancel={bind_cancel}"));
    debug::log(format!(
        "fzf: args={:?}",
        cmd.get_args().collect::<Vec<_>>()
    ));

    let mut child = cmd.spawn().context("failed to spawn fzf")?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("failed to open fzf stdin"))?;
        write_candidates(stdin, candidates)?;
    }
    let output = child
        .wait_with_output()
        .context("failed to wait for fzf to exit")?;

    if !output.status.success() {
        // fzf exits 130 when the user cancels. Treat any non-success as cancellation.
        return Ok(None);
    }

    let selected = String::from_utf8(output.stdout)
        .context("fzf returned non-UTF-8 output")?
        .trim_end_matches('\n')
        .to_string();
    if selected.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_candidate_line(&selected)))
}

pub fn write_candidates(w: &mut impl Write, candidates: &[Candidate]) -> Result<()> {
    for c in candidates {
        let command = sanitize(&c.command);
        let description = sanitize(&c.description);
        let display = sanitize(&display_command(c));
        // Wire format:
        //   col1: display command (can include install marker)
        //   col2: description
        //   col3: raw command to execute when selected
        writeln!(w, "{display}\t{description}\t{command}")
            .context("failed to write candidate to fzf")?;
    }
    Ok(())
}

fn sanitize(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn parse_candidate_line(line: &str) -> Candidate {
    let mut parts = line.splitn(3, '\t');
    let first = parts.next().unwrap_or("").to_string();
    let description = parts.next().unwrap_or("").to_string();
    let raw_command = parts.next().unwrap_or("");
    let command = if raw_command.is_empty() {
        first
    } else {
        raw_command.to_string()
    };
    Candidate {
        command,
        description,
    }
}

fn display_command(candidate: &Candidate) -> String {
    if requires_install(&candidate.description) {
        format!("[NEEDS INSTALL] {}", candidate.command)
    } else {
        candidate.command.clone()
    }
}

fn requires_install(description: &str) -> bool {
    description.contains("Requires install:")
}

fn shell_single_quote(s: &str) -> String {
    // POSIX-safe single-quote escape: ' -> '\''
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}
