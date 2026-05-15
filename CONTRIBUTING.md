# Contributing

Information for developper.

## Requirements

- **Rust toolchain 1.86+** — install via [rustup](https://rustup.rs/) (brings `cargo`, `clippy`, `rustfmt`)
- **[`fzf`](https://github.com/junegunn/fzf)** on your `PATH` — required to run and test the picker (recommended: `brew install fzf`)
- **API key** for at least one provider (Anthropic, OpenAI, or Gemini) — needed for end-to-end LLM testing; Ollama needs only a running local server

## Development

```sh
cargo build              # debug
cargo build --release    # optimized
cargo test               # unit tests (safety module)
cargo clippy --all-targets -- -D warnings
```

## Debugging

Set `ZXCV_DEBUG=1` to write a verbose log to `/tmp/zxcv-debug.log` (override
the path with `ZXCV_DEBUG_LOG`). Useful for inspecting LLM calls and fzf
plumbing.

```sh
ZXCV_DEBUG=1 zxcv "your query"
tail -f /tmp/zxcv-debug.log
```

Use `cargo run` to avoid reinstalling on every change:

```sh
ZXCV_DEBUG=1 cargo run -- "your query"
```