One-click setup for students (Windows x64). Download `en_get.exe` and double-click it.

What it does:
- Installs the **modern-cug-report** Typst template, so `#import "@local/modern-cug-report:0.1.3": *` works anywhere
- Copies VS Code `settings.json`, `keybindings.json` and the Typst snippets into `%APPDATA%\Code\User` (old files are kept as `*.bak`)
- Installs the VS Code extension **VSCode Pi Agent Bridge** (`zenyui.vscode-pi-bridge`)
- On `remote-p520`: **asks which `/data/users/<folder>` is yours** (or type a new name to create it), puts `pi-vscode-context.ts` in `<folder>/.pi/extensions/`, writes `<folder>/.envrc`, and sets up direnv so it loads

Before running:
- VS Code is installed
- `ssh remote-p520` works from a Windows terminal (login `kong01` or `kong02`)
- **direnv** is installed on the server. The exe warns if it's missing.

If Windows SmartScreen appears, click **More info → Run anyway**.
