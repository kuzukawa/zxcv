use std::collections::HashSet;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Result;

const PROMPT_COMMAND_LIMIT: usize = 320;

#[derive(Debug, Clone)]
pub struct Inventory {
    installed: HashSet<String>,
    prompt_commands: Vec<String>,
    omitted_prompt_count: usize,
    fingerprint: String,
}

impl Inventory {
    pub fn detect() -> Result<Self> {
        let mut installed = HashSet::new();
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if !is_executable_file(&p) {
                            continue;
                        }
                        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                            continue;
                        };
                        if !name.is_empty() {
                            installed.insert(name.to_string());
                        }
                    }
                }
            }
        }

        let mut all: Vec<String> = installed.iter().cloned().collect();
        all.sort_unstable();

        let omitted_prompt_count = all.len().saturating_sub(PROMPT_COMMAND_LIMIT);
        let prompt_commands = all
            .into_iter()
            .take(PROMPT_COMMAND_LIMIT)
            .collect::<Vec<_>>();
        let fingerprint = hash_names(installed.iter().map(String::as_str));

        Ok(Self {
            installed,
            prompt_commands,
            omitted_prompt_count,
            fingerprint,
        })
    }

    pub fn prompt_commands(&self) -> &[String] {
        &self.prompt_commands
    }

    pub fn omitted_prompt_count(&self) -> usize {
        self.omitted_prompt_count
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn installed_count(&self) -> usize {
        self.installed.len()
    }

    pub fn missing_commands(&self, command_line: &str) -> Vec<String> {
        let mut missing = Vec::new();
        for cmd in command_heads(command_line) {
            if is_shell_builtin(&cmd) {
                continue;
            }
            if !self.installed.contains(&cmd) {
                missing.push(cmd);
            }
        }
        missing.sort_unstable();
        missing.dedup();
        missing
    }
}

fn hash_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut values: Vec<&str> = names.collect();
    values.sort_unstable();
    let mut hasher = DefaultHasher::new();
    for name in values {
        name.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    meta.permissions().mode() & 0o111 != 0
}

fn command_heads(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for segment in split_shell_segments(line) {
        if let Some(cmd) = segment_head_command(&segment) {
            out.push(cmd.to_string());
        }
    }
    out
}

fn split_shell_segments(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single {
            buf.push(ch);
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            buf.push(ch);
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            buf.push(ch);
            continue;
        }

        if !in_single && !in_double {
            let is_sep = match ch {
                ';' => true,
                '|' => {
                    if chars.peek() == Some(&'|') {
                        let _ = chars.next();
                    }
                    true
                }
                '&' => {
                    if chars.peek() == Some(&'&') {
                        let _ = chars.next();
                    }
                    true
                }
                _ => false,
            };

            if is_sep {
                let seg = buf.trim();
                if !seg.is_empty() {
                    out.push(seg.to_string());
                }
                buf.clear();
                continue;
            }
        }

        buf.push(ch);
    }

    let seg = buf.trim();
    if !seg.is_empty() {
        out.push(seg.to_string());
    }
    out
}

fn segment_head_command(segment: &str) -> Option<&str> {
    let mut parts = segment.split_whitespace().peekable();

    while let Some(tok) = parts.peek().copied() {
        if is_assignment(tok) {
            let _ = parts.next();
            continue;
        }
        break;
    }

    let mut token = parts.next()?;

    while token == "sudo" || token == "env" || token == "command" || token == "nohup" {
        token = next_wrapped_command(token, &mut parts)?;
    }

    token = token.trim_start_matches('(');
    token = token.trim_start_matches('{');
    token = token.trim_start_matches('[');

    if token.is_empty() {
        return None;
    }

    Some(strip_quotes(token))
}

fn next_wrapped_command<'a>(
    wrapper: &str,
    parts: &mut std::iter::Peekable<std::str::SplitWhitespace<'a>>,
) -> Option<&'a str> {
    if wrapper == "sudo" {
        while let Some(tok) = parts.peek().copied() {
            if tok.starts_with('-') {
                let _ = parts.next();
                continue;
            }
            break;
        }
    } else if wrapper == "env" {
        while let Some(tok) = parts.peek().copied() {
            if tok.starts_with('-') || is_assignment(tok) {
                let _ = parts.next();
                continue;
            }
            break;
        }
    }
    parts.next()
}

fn strip_quotes(s: &str) -> &str {
    s.trim_matches('\'').trim_matches('"')
}

fn is_assignment(token: &str) -> bool {
    let Some((name, _value)) = token.split_once('=') else {
        return false;
    };
    is_valid_name(name)
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn is_shell_builtin(name: &str) -> bool {
    matches!(
        name,
        ":" | "."
            | "alias"
            | "bg"
            | "bind"
            | "break"
            | "builtin"
            | "cd"
            | "command"
            | "compgen"
            | "complete"
            | "continue"
            | "declare"
            | "dirs"
            | "disown"
            | "echo"
            | "enable"
            | "eval"
            | "exec"
            | "exit"
            | "export"
            | "false"
            | "fc"
            | "fg"
            | "getopts"
            | "hash"
            | "help"
            | "history"
            | "jobs"
            | "kill"
            | "let"
            | "local"
            | "logout"
            | "popd"
            | "printf"
            | "pushd"
            | "pwd"
            | "read"
            | "readonly"
            | "return"
            | "set"
            | "shift"
            | "source"
            | "suspend"
            | "test"
            | "times"
            | "trap"
            | "true"
            | "type"
            | "typeset"
            | "ulimit"
            | "umask"
            | "unalias"
            | "unset"
            | "wait"
    )
}

#[cfg(test)]
mod tests {
    use super::command_heads;

    #[test]
    fn detects_simple_pipeline_commands() {
        let cmds = command_heads("cat file.txt | rg foo | wc -l");
        assert_eq!(cmds, vec!["cat", "rg", "wc"]);
    }

    #[test]
    fn skips_assignments_and_handles_sudo() {
        let cmds = command_heads("FOO=1 BAR=2 sudo -n ls -la");
        assert_eq!(cmds, vec!["ls"]);
    }

    #[test]
    fn handles_env_wrapper() {
        let cmds = command_heads("env -i PATH=/usr/bin jq '.a' file.json");
        assert_eq!(cmds, vec!["jq"]);
    }

    #[test]
    fn ignores_operators_inside_quotes() {
        let cmds = command_heads("echo 'a | b' && printf \"%s\" hi");
        assert_eq!(cmds, vec!["echo", "printf"]);
    }
}
