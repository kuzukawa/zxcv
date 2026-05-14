# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo build                              # debug
cargo build --release                    # optimized
cargo test                               # unit tests (currently safety::tests only)
cargo test safety::tests::detects_rm_rf_root   # run a single test
cargo clippy --all-targets -- -D warnings      # CI gate
cargo run -- "your query"                # iterate without reinstalling
ZXCV_DEBUG=1 cargo run -- "query"        # write debug trace to /tmp/zxcv-debug.log
cargo run --example gen-man              # render man pages to target/man/
```

Rust 1.86+, edition 2024. `fzf` must be on PATH to actually exercise the picker.

## Architecture

`zxcv` is a single Rust binary that self-re-execs through `fzf`. There is no daemon, no IPC — three runtime modes share one process image, dispatched in `main.rs`:

| Mode               | How entered                              | Output                                               |
|--------------------|------------------------------------------|------------------------------------------------------|
| Interactive (root) | `zxcv [QUERY]`                            | Spawns `fzf`, prints selected command to stdout      |
| `--internal`       | Invoked by fzf's `ctrl-g:reload(...)` bind | Emits TSV candidates (cache hit or LLM call)         |
| `--history-only`   | Invoked by fzf's `ctrl-c:reload(...)` bind | Emits TSV history candidates, no LLM call            |
| Subcommands        | `init`, `config`, `history`, `install-man` | Side-effects only                                    |

The `fzf::pick` function constructs the reload bindings by quoting `std::env::current_exe()` — the running binary literally calls itself. Tab-delimited `command<TAB>description` is the wire format between modes; `fzf::sanitize` strips embedded tab/newline/CR before writing.

### Provider abstraction

`src/providers/mod.rs` is the seam. Each provider module (`anthropic`, `openai`, `ollama`, `gemini`) exposes one `generate(&Settings, &str) -> Result<Vec<Candidate>>` async fn. The shared `SYSTEM_PROMPT` and `parse_candidates_json` keep response shape uniform — all providers must coerce their reply into `{ "candidates": [{"command": "...", "description": "..."}] }`. `MAX_CANDIDATES = 5` is enforced at parse time.

When adding a new provider:
1. Add variant to `Provider` enum + match arms in `parse`/`id`/`default_model`/`api_key_env`.
2. Add `pub mod foo;` and a match arm in `generate(...)`.
3. Wire its key/model/endpoint through `config::ProviderEntry` (no schema change needed — keyed by string).

### Settings resolution

`config::resolve` is the single source of truth for precedence: **CLI flag > env var > config file > built-in default**. Every new tunable should flow through this function and end up on `Settings`. `provider_explicit` is tracked separately so error messages can tell users to set a provider vs. set a key.

### Caching

`src/cache.rs` hashes `(provider, model, query)` with `DefaultHasher` into a 16-hex filename under `$XDG_CACHE_HOME/zxcv/llm_cache/`. Cache hits return the same `Vec<Candidate>` as a fresh call — no schema migration, no eviction. If the cache schema changes, bump the hash inputs or delete the directory.

### History & frecency

`history::frecency` is zoxide-style with hard-coded age buckets (1h / 1d / 1w / older → 4× / 2× / 0.5× / 0.25×). Entries are keyed by `(query, command)` so the same command picked for different prompts stays distinct. Stored as TOML at `$XDG_STATE_HOME/zxcv/history.toml`.

### Safety detector

`src/safety.rs` compiles `BUILTIN_PATTERNS + extra_patterns` into a single `RegexSet`. Built-ins are anchored to start-of-line or shell separator (`; & |`) so `echo rm -rf /` does NOT trigger — keep new patterns conservative for the same reason. The detector is consulted only on the final selected command in `run_interactive`, not on every LLM candidate.

### Shell integration

`src/init.rs` emits the zsh/bash widget as a string. The widget sets `ZXCV_FROM_WIDGET=1` before invoking `zxcv`; `main.rs::show_setup_hint_if_needed` keys off that env var to suppress the first-run hint when called via widget. Don't break this — users running through the widget should never see the hint.

### Paths

All filesystem locations go through `src/paths.rs`, which respects `XDG_CONFIG_HOME` / `XDG_STATE_HOME` / `XDG_CACHE_HOME` / `XDG_DATA_HOME`. Do not hard-code `~/.config/zxcv/...` elsewhere.

### Man pages

`install_man.rs` and `examples/gen-man.rs` both render man pages from `Cli::command()` via `clap_mangen`. Adding a subcommand automatically gets a man page; descriptions in `cli.rs` doc-comments become the man body.

## Release

Releases go through `cargo-dist` (config in `dist-workspace.toml`). Targets: `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`. Windows is deliberately excluded — the codebase scopes itself to macOS + Linux. Homebrew formula publishes to `kuzukawa/homebrew-tap` and declares `fzf` as a runtime dependency.

## Debugging

`ZXCV_DEBUG=1` enables the file logger (`debug::log`). When tracing fzf misbehavior, the log includes the exact `--bind=` strings and reload command — those are the most common breakage points. Override the path with `ZXCV_DEBUG_LOG`.
