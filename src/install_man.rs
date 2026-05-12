use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_mangen::Man;

use crate::cli::Cli;
use crate::paths;

/// Render and install man pages under `<prefix>/man/man1/`. If `prefix` is `None`, use
/// `$XDG_DATA_HOME` (or `~/.local/share`).
pub fn install(prefix: Option<PathBuf>) -> Result<()> {
    let prefix = match prefix {
        Some(p) => p,
        None => paths::default_data_prefix()?,
    };
    let man_dir = prefix.join("man").join("man1");
    fs::create_dir_all(&man_dir)
        .with_context(|| format!("failed to create {}", man_dir.display()))?;

    let cmd = Cli::command();
    let count = render(&cmd, &man_dir, None)?;

    println!("Installed {count} man page(s) to {}", man_dir.display());
    let man_root = prefix.join("man");
    if !is_in_manpath(&man_root) {
        eprintln!(
            "\nNote: {} may not be in your MANPATH. To enable `man zxcv`, add this to your shell rc:\n  export MANPATH=\"{}:${{MANPATH}}\"",
            man_root.display(),
            man_root.display()
        );
    }
    Ok(())
}

fn render(cmd: &clap::Command, out_dir: &Path, parent: Option<&str>) -> Result<usize> {
    let name_owned = match parent {
        Some(p) => format!("{p}-{}", cmd.get_name()),
        None => cmd.get_name().to_string(),
    };
    let path = out_dir.join(format!("{name_owned}.1"));

    // clap::Command::name/bin_name require &'static str; leak the small allocation.
    let name_static: &'static str = Box::leak(name_owned.clone().into_boxed_str());
    let renamed = if parent.is_some() {
        cmd.clone().name(name_static).bin_name(name_static)
    } else {
        cmd.clone().name(name_static)
    };

    let mut buf: Vec<u8> = Vec::new();
    Man::new(renamed)
        .render(&mut buf)
        .context("failed to render man page")?;
    fs::write(&path, buf).with_context(|| format!("failed to write {}", path.display()))?;

    let mut count = 1;
    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        count += render(sub, out_dir, Some(&name_owned))?;
    }
    Ok(count)
}

/// Return true if `man_root` is included in the system manpath (via `manpath` command
/// or the `MANPATH` env var).
fn is_in_manpath(man_root: &Path) -> bool {
    let target = man_root.to_string_lossy();
    if let Ok(out) = std::process::Command::new("manpath").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            if s.split(':').any(|p| p.trim() == target) {
                return true;
            }
        }
    }
    if let Ok(env) = std::env::var("MANPATH") {
        return env.split(':').any(|p| p.trim() == target);
    }
    false
}
