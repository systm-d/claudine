# Claudine — project guide for AI agents & contributors

TUI/CLI to browse and manage Claude Code's local data in `~/.claude` (sessions,
memory/`CLAUDE.md`, `settings.json`, extensions: hooks/MCP/plugins,
marketplaces). Rust workspace: `claudine-core` (library: pure logic plus
`cli.rs`, `commands/*` and `tui/*`) + `claudine` (binary, thin shim).

## Read first

1. [CONVENTIONS.md](CONVENTIONS.md) — shared standards (edition, fmt, lints,
   commits, license, language policy).
2. [docs/superpowers/specs/](docs/superpowers/specs/) — feature specs and
   design docs for each phase.
3. [README.md](README.md) — features, installation, usage.

## Product rules

- **Multi-platform**: Linux, Windows, macOS. claudine manages `~/.claude`,
  which exists on every platform Claude Code runs on — nothing may assume a
  Linux-only environment.
- User-facing strings (CLI/TUI output) may be in **French** (e.g.
  `Erreur : …`); code identifiers and documentation stay in English.

## Where to change what

| Need | File |
|------|------|
| Home discovery / registration | `crates/claudine-core/src/home.rs`, `config.rs` |
| Settings (`settings.json`) read/write | `crates/claudine-core/src/settings.rs` |
| Hooks / MCP / plugins (extensions) | `crates/claudine-core/src/extensions.rs` |
| Marketplaces & plugin catalogue | `crates/claudine-core/src/marketplaces.rs` |
| Export bundle | `crates/claudine-core/src/export.rs` |
| Import bundle / path remap | `crates/claudine-core/src/import.rs`, `remap.rs` |
| Trash / housekeeping | `crates/claudine-core/src/housekeeping.rs` |
| Search across sessions | `crates/claudine-core/src/search.rs` |
| Session/project scan & model | `crates/claudine-core/src/scan.rs`, `model.rs` |
| Export manifest format | `crates/claudine-core/src/manifest.rs` |
| Encoded-path codec | `crates/claudine-core/src/pathcodec.rs` |
| CLI command / argument | `crates/claudine-core/src/cli.rs`, `commands/*` |
| TUI screen / widget | `crates/claudine-core/src/tui/*` |

## Quality gate

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
