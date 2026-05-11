//! Generates man pages for `zxcv` and its subcommands into `target/man/`.
//!
//! Run with: `cargo run --example gen-man`

use std::fs;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap_mangen::Man;

#[path = "../src/cli.rs"]
mod cli;

fn main() {
    let out_dir = PathBuf::from("target").join("man");
    fs::create_dir_all(&out_dir).expect("failed to create target/man");

    let cmd = cli::Cli::command();
    render(&cmd, &out_dir, None);

    println!("man pages written to {}", out_dir.display());
}

fn render(cmd: &clap::Command, out_dir: &Path, parent: Option<&str>) {
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
        .expect("failed to render man page");
    fs::write(&path, buf).expect("failed to write man page");
    println!("  {}", path.display());

    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        render(sub, out_dir, Some(&name_owned));
    }
}
