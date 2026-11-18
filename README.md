# en_get

One-click environment setup for students. A single `.exe` with no dependencies
that does the following:

| Step        | What it does |
|-------------|--------------|
| `typst`     | Installs the **modern-cug-report** template globally: `#import "@local/modern-cug-report:0.1.3": *` works anywhere |
| `vscode`    | Puts `settings.json`, `keybindings.json` and the Typst snippets (`snippets\typst.json`) into `%APPDATA%\Code\User` |
| `extension` | Installs the VS Code extension **VSCode Pi Agent Bridge** (`zenyui.vscode-pi-bridge`) |
| `remote`    | Over SSH to `remote-p520`, puts `pi-vscode-context.ts` into `$dir_root/.pi/extensions/` and writes `$dir_root/.envrc`. `dir_root` is your `/data/users/<folder>`; on a shared login it **asks which folder is yours** (or type a new name to create one) |
| `pi`, `r`   | Copies Pi / R config files (none bundled yet) |

Any file that already exists and differs is kept as `*.bak`. If one step fails,
the others still run.

## For students
Before running it you need:
- **VS Code** installed.
- An SSH alias `remote-p520` in `%USERPROFILE%\.ssh\config`, so that
  `ssh remote-p520` logs you in.
- Internet access the first time you compile Typst (it downloads `@preview` packages).

Then double-click `en_get.exe`. If Windows SmartScreen appears, click **More info → Run anyway**.

From a terminal:
- `en_get --help`
- `--only typst` (repeatable; steps: pi, r, typst, vscode, extension, remote)
- `--dry-run`
- `--dir-root /data/users/<your folder>` if the server folder isn't detected
- `--ssh-host <alias>`

## For maintainers
Bundled files are in `assets/` (see `assets/README.md`) and `remote/pi-vscode-context.ts`.

```sh
cargo test
cargo run -- --root /tmp/fake-home --no-pause            # local copy steps into a fake home
cargo run -- --dry-run --no-pause                        # preview everything, change nothing
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/en_get.exe
```

Cross-compiling from macOS needs `rustup target add x86_64-pc-windows-gnu` and
`brew install mingw-w64`. The linker is set in `.cargo/config.toml`.
