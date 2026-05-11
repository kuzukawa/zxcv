use anyhow::{Context, Result};
use regex::RegexSet;

use crate::config::SafetyConfig;

/// Built-in destructive command patterns. Conservative — they should rarely false-positive on
/// benign commands (e.g., `rm -rf /tmp/foo` does NOT trigger; only root, $HOME, ~, *).
///
/// Each pattern is anchored to a command boundary: start-of-line or a shell separator
/// (`;`, `|`, `&`), but NOT a plain space, so `echo rm -rf /` does not trigger.
const BUILTIN_PATTERNS: &[&str] = &[
    r"(?:^|[;&|])\s*rm\s+(?:-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*)\s+/(?:\s|$)",
    r"(?:^|[;&|])\s*rm\s+(?:-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*)\s+~(?:\s|/|$)",
    r"(?:^|[;&|])\s*rm\s+(?:-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*)\s+\$HOME",
    r"(?:^|[;&|])\s*rm\s+(?:-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*)\s+\*(?:\s|$)",
    r"(?:^|[;&|])\s*dd\s+.*of=/dev/(?:sd|nvme|hd|disk)",
    r"(?:^|[;&|])\s*mkfs(?:\.|\s)",
    r"(?:^|[;&|])\s*shutdown(?:\s|$)",
    r"(?:^|[;&|])\s*reboot(?:\s|$)",
    r"(?:^|[;&|])\s*halt(?:\s|$)",
    r"(?:^|[;&|])\s*init\s+0(?:\s|$)",
    r"(?:^|[;&|])\s*poweroff(?:\s|$)",
    r":\(\)\s*\{\s*:\|:&\s*\};:", // fork bomb
    r">\s*/dev/(?:sd|nvme|hd|disk)",
    r"(?:^|[;&|])\s*chmod\s+-R\s+0?777",
    r"(?:^|[;&|])\s*chown\s+-R\s+\S+\s+/",
    r">\s*/etc/(?:passwd|shadow|sudoers)",
];

#[derive(Debug)]
pub struct Detector {
    set: RegexSet,
    sources: Vec<String>,
}

impl Detector {
    pub fn new(extra: &[String]) -> Result<Self> {
        let mut patterns: Vec<String> = BUILTIN_PATTERNS.iter().map(|s| (*s).to_string()).collect();
        patterns.extend_from_slice(extra);
        let set = RegexSet::new(&patterns)
            .context("failed to compile safety patterns (check extra_patterns in config)")?;
        Ok(Self {
            set,
            sources: patterns,
        })
    }

    pub fn from_config(cfg: &SafetyConfig) -> Result<Self> {
        Self::new(&cfg.extra_patterns)
    }

    pub fn is_dangerous(&self, command: &str) -> bool {
        self.set.is_match(command)
    }

    /// Return the matching pattern strings for diagnostics.
    pub fn matched(&self, command: &str) -> Vec<String> {
        self.set
            .matches(command)
            .iter()
            .filter_map(|i| self.sources.get(i).cloned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rm_rf_root() {
        let d = Detector::new(&[]).unwrap();
        assert!(d.is_dangerous("rm -rf /"));
        assert!(d.is_dangerous("rm -rf ~"));
        assert!(d.is_dangerous("rm -rf $HOME"));
        assert!(d.is_dangerous("rm -rf *"));
    }

    #[test]
    fn does_not_false_positive() {
        let d = Detector::new(&[]).unwrap();
        assert!(!d.is_dangerous("ls -la"));
        assert!(!d.is_dangerous("find . -type f"));
        assert!(!d.is_dangerous("rm -rf /tmp/foo"));
        assert!(!d.is_dangerous("rm -rf ./build"));
        assert!(!d.is_dangerous("echo rm -rf /"));
    }

    #[test]
    fn detects_dd_to_disk() {
        let d = Detector::new(&[]).unwrap();
        assert!(d.is_dangerous("dd if=/dev/zero of=/dev/sda bs=1M"));
        assert!(!d.is_dangerous("dd if=/dev/zero of=/tmp/x bs=1M"));
    }

    #[test]
    fn detects_fork_bomb() {
        let d = Detector::new(&[]).unwrap();
        assert!(d.is_dangerous(":(){ :|:& };:"));
    }

    #[test]
    fn detects_mkfs() {
        let d = Detector::new(&[]).unwrap();
        assert!(d.is_dangerous("mkfs.ext4 /dev/sdb1"));
    }

    #[test]
    fn extra_pattern_works() {
        let d = Detector::new(&["^my-cmd".to_string()]).unwrap();
        assert!(d.is_dangerous("my-cmd"));
        assert!(!d.is_dangerous("not-my-cmd"));
    }

    #[test]
    fn invalid_extra_pattern_is_reported() {
        let err = Detector::new(&["(".to_string()]).unwrap_err();
        assert!(err.to_string().contains("safety patterns"));
    }
}
