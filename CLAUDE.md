# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is
`en_get` is a single-file Rust CLI (`src/main.rs`) that students on **Windows** double-click to copy bundled Pi coding agent, R and Typst config files into the correct user folders. The deliverable is a self-contained `en_get.exe`. It is developed on an Apple Silicon Mac and cross-compiled. The style follows the tutor's reference crate `kongdd/code-action/crates/cli-remote`: clap derive, size-optimised release profile, and tests in the same file.

## Commands
Rustup on this Mac is keg-only Homebrew, so you may need `export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"`.

```sh
cargo test                                    # all tests
cargo test copy_new_same                      # single test (name filter)
cargo run -- --root /tmp/fake-home --dry-run --no-pause   # preview on the Mac against a fake home
cargo build --release --target x86_64-pc-windows-gnu      # -> target/x86_64-pc-windows-gnu/release/en_get.exe
```
Cross-compiling needs the `x86_64-pc-windows-gnu` rustup target and `brew install mingw-w64`. The linker is configured in `.cargo/config.toml`.

## Architecture
- **Assets are compile-time embedded.** `include_dir!("$CARGO_MANIFEST_DIR/assets")` bakes `assets/` into the binary. Changing any asset requires a rebuild. Each top-level subfolder (`pi`, `r`, `typst`) is one `Tool`, and its contents are copied, with relative paths preserved, to `Tool::target()`.
- **Target paths** live in `Tool::target(&Base)`. `Base` holds `home`, `documents` and `appdata`, either detected via the `dirs` crate or faked under `--root` (`Documents/`, `AppData/Roaming/`). This is how tests and Mac runs mimic the Windows layout. To add a new tool, add a `Tool` variant, its `name()` and `target()` arms, and an `assets/<name>/` folder.
- **Copy semantics** (`copy_file`): identical file → skip; a different existing file → renamed to `*.bak`, then overwritten; `--dry-run` never touches disk. The `IGNORED` names (`.gitkeep`, `.DS_Store`) are never copied, and `assets/README.md` sits outside the tool folders, so it isn't copied either.
- `main` waits for Enter unless `--no-pause` is passed, so the console stays open after a double-click. Pass `--no-pause` in any scripted or test run.

## Constraints
- `assets/` ships to every student. Never put API keys or personal tokens there, especially in Pi `settings.json`/auth files.
- Windows target paths (`%USERPROFILE%\.pi\agent`, `Documents` for R, `%APPDATA%\typst\packages\local`) are best guesses still to be confirmed with the tutor.
