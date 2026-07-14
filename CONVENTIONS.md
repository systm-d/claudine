# Conventions

The written source of truth for the standards shared across this project (and the
other CLIs built from the same `rust-cli-template` mood). Governance files link here
instead of restating these rules.

## Language & Edition

- Rust **edition 2024**, MSRV **1.85** (pinned via `Cargo.toml`'s
  `[workspace.package] rust-version = "1.85"`; `rust-toolchain.toml` only pins
  the toolchain channel (stable) and components).
- Formatting: `rustfmt` with `max_width = 100`, edition 2024 (`rustfmt.toml`).
  `cargo fmt --check` must pass.
- Lints: `unsafe_code = "forbid"`; clippy `all = { level = "warn", priority = -1 }`,
  inherited per crate via `[lints] workspace = true`.
  `cargo clippy --workspace --all-targets -- -D warnings` must pass.

## Project shape

- Workspace: `claudine-core` (library: pure logic **plus** `cli.rs`, `commands/*`
  and `tui/*`) + `claudine` (binary, thin shim — `fn main() -> ExitCode {
  claudine_core::run() }`).
- Module discipline: business logic, CLI parsing/dispatch, and the TUI all live in
  `claudine-core`; the `claudine` binary carries no logic of its own.
- **Multi-platform** by design (Linux, Windows, macOS): claudine reads and writes
  `~/.claude`, which exists on every platform Claude Code runs on. Nothing in the
  codebase may assume a Linux-only environment (no systemd, no libnotify, no
  `/sys` paths).

## Language of text

- Documentation (README, this file, governance) is in **English**.
- User-facing strings — CLI and TUI output — may be in **French** (e.g. error
  messages such as `Erreur : …`). Never `ERROR`/`FATAL`/`PANIC` in user-facing
  text.
- Code identifiers are in English.

## Git & releases

- **Conventional Commits** (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`,
  `test:`, `build:`, `style:`).
- **Keep a Changelog** format in `CHANGELOG.md`; **Semantic Versioning**.
- Dual license: **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`).

## Quality gate (run before every PR)

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
