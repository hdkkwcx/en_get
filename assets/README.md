# assets/

Everything in these folders is embedded into `en_get.exe` and copied, keeping
sub-folders, to the student's machine:

| Folder    | Copied to (Windows)                        | Contents                                 |
|-----------|--------------------------------------------|------------------------------------------|
| `pi/`     | `%USERPROFILE%\.pi\agent\`                 | (empty) Pi settings, AGENTS.md, skills   |
| `r/`      | `%USERPROFILE%\Documents\`                 | (empty) `.Rprofile`, `.Renviron`         |
| `typst/`  | `%APPDATA%\typst\packages\local\`          | `modern-cug-report/<version>/` from github.com/CUG-hydro/modern-cug-report.typ |
| `vscode/` | `%APPDATA%\Code\User\`                     | `settings.json`, `keybindings.json`, `snippets/typst.json` |

`.gitkeep` files are ignored, and this README is not copied (it is outside the
tool folders). Changed files are backed up as `*.bak`. **Never put API keys or
passwords here.** Rebuild the exe after any change.

To update the Typst template, re-clone the repo and copy its version folders
(`0.1.x/`), `thumbnail.png` and `LICENSE` into `typst/modern-cug-report/`.

The Pi bridge companion for the server lives in `../remote/` (not here); it is
sent over SSH, not copied locally.
