# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is
`en_get` is a Rust CLI that students on **Windows** double-click. In one run it:
- installs the `modern-cug-report` Typst template as a `@local` package;
- copies the VS Code settings, keybindings and Typst snippets;
- installs the `zenyui.vscode-pi-bridge` VS Code extension;
- sets up the student's folder on the server `remote-p520` over SSH.

The deliverable is a self-contained `en_get.exe`. It is developed on an Apple Silicon Mac and cross-compiled. The style follows the tutor's reference crate `kongdd/code-action/crates/cli-remote`: clap derive, size-optimised release profile, and tests next to the code. The SSH host validation is copied from its `remote-run.rs`.

## Commands
Rustup on this Mac is keg-only Homebrew. Non-interactive shells need `export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"`.

```sh
cargo test                                    # all tests
cargo test remote_command                     # single test (name filter)
cargo run -- --root /tmp/fake-home --no-pause # copy steps into a fake Windows-like home
cargo run -- --dry-run --no-pause             # preview all steps incl. extension + ssh
cargo build --release --target x86_64-pc-windows-gnu      # -> target/x86_64-pc-windows-gnu/release/en_get.exe
```
Cross-compiling needs the `x86_64-pc-windows-gnu` rustup target and `brew install mingw-w64`. The linker is configured in `.cargo/config.toml`.

## Architecture
- **Steps** (`Step` in `src/main.rs`) are of two kinds:
  - *Copy steps* (`pi`, `r`, `typst`, `vscode`): `Step::target()` returns `Some`, and `assets/<name>/` is copied there.
  - *Action steps* (`extension`, `remote`): `Step::target()` returns `None`.

  All steps run even if one fails; failures are collected and set the exit code. `--root` (test mode) defaults to copy steps only.
- **Assets are compile-time embedded** via `include_dir!` (and `include_str!` for `remote/pi-vscode-context.ts`), so any asset change needs a rebuild. `Base` holds the `home`, `documents` and `appdata` folders. They are detected via `dirs`, or faked under `--root` as `Documents/` and `AppData/Roaming/` to mimic Windows.
- **`src/copy.rs`**: identical file → skipped; a different existing file → renamed to `*.bak`, then overwritten; dry-run never writes. `.gitkeep` and `.DS_Store` are never copied.
- **`src/extension.rs`**: looks for `code.cmd` in the default VS Code install folders, then on PATH.
- **`src/remote.rs`**: builds a POSIX `sh` script. `dir_root` is `--dir-root`, else always asked interactively from the folders the login can write to. Students share the logins `kong01`/`kong02`, so it never guesses. The answer is read from `/dev/tty` via `ssh -t`, or from `$EN_GET_TTY` in tests. The script writes the `.ts` file and `.envrc` (with `.bak`). It appends a marked direnv-hook block to the login's `~/.bashrc`/`~/.zshrc` once, and runs `direnv allow`. If direnv is missing it exits **10**, which `run()` reports as a step error. `.envrc` only takes effect through direnv, and without it students share one Pi/gh/git identity. The script is sent as `ssh -- <host> "echo <base64> | base64 -d | sh"`, so it works with any remote login shell and has no quoting issues. Its test executes that exact command locally under `sh` and `dash`.
- `main` waits for Enter unless `--no-pause` is passed. Pass `--no-pause` in any scripted run.

## Constraints
- `assets/` ships to every student. Never put API keys or tokens there. `assets/vscode/settings.json` is the tutor's personal VS Code config and replaces students' files.
- `assets/typst/modern-cug-report/` is vendored from github.com/CUG-hydro/modern-cug-report.typ (version folders only). Keep `assets/vscode/snippets/typst.json`'s `@local/modern-cug-report:<version>` in sync with the newest vendored version.
