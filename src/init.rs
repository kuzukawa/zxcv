use crate::cli::Shell;

const ZSH_SCRIPT: &str = r#"# zxcv shell integration for zsh
# Usage:
#   eval "$(zxcv init zsh)"
#   bindkey '^[z' zxcv-widget   # Alt+Z — choose any key you like
zxcv-widget() {
    local result
    result=$(ZXCV_FROM_WIDGET=1 command zxcv "$LBUFFER" </dev/tty 2>/dev/tty) || return
    if [[ -n $result ]]; then
        LBUFFER=$result
    fi
    zle reset-prompt
}
zle -N zxcv-widget
"#;

const BASH_SCRIPT: &str = r#"# zxcv shell integration for bash
# Usage:
#   eval "$(zxcv init bash)"
#   bind -x '"\e": zxcv-widget'   # bind any key you like (Alt+Z is "\ez" in bash)
zxcv-widget() {
    local result
    result=$(ZXCV_FROM_WIDGET=1 command zxcv "$READLINE_LINE" </dev/tty 2>/dev/tty) || return
    if [[ -n $result ]]; then
        READLINE_LINE=$result
        READLINE_POINT=${#READLINE_LINE}
    fi
}
"#;

pub fn script(shell: Shell) -> &'static str {
    match shell {
        Shell::Zsh => ZSH_SCRIPT,
        Shell::Bash => BASH_SCRIPT,
    }
}
