use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn copy(text: &str) -> Result<&'static str> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "linux") {
        &[
            ("wl-copy", &["--trim-newline"]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    } else {
        &[]
    };

    let mut last_err = None;
    for (bin, args) in candidates {
        match run_copy_command(bin, args, text) {
            Ok(()) => return Ok(bin),
            Err(e) => last_err = Some(e),
        }
    }

    if let Some(e) = last_err {
        Err(e)
    } else {
        bail!("clipboard copy is not supported on this OS")
    }
}

fn run_copy_command(bin: &str, args: &[&str], text: &str) -> Result<()> {
    let mut child = match Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return Err(
                anyhow::Error::new(e).context(format!("failed to spawn clipboard command `{bin}`"))
            );
        }
    };

    let Some(stdin) = child.stdin.as_mut() else {
        bail!("clipboard command `{bin}` did not expose stdin");
    };
    stdin
        .write_all(text.as_bytes())
        .with_context(|| format!("failed to write to clipboard command `{bin}`"))?;

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for clipboard command `{bin}`"))?;
    if !status.success() {
        bail!("clipboard command `{bin}` exited with status {status}");
    }
    Ok(())
}
