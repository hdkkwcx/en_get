# assets/

Everything in these folders is embedded into `en_get.exe` and copied, keeping
sub-folders, to the student's machine:

| Folder    | Copied to (Windows)                        | Typical contents                         |
|-----------|--------------------------------------------|------------------------------------------|
| `pi/`     | `%USERPROFILE%\.pi\agent\`                 | `settings.json`, `AGENTS.md`, `extensions/`, `skills/` |
| `r/`      | `%USERPROFILE%\Documents\`                 | `.Rprofile` (CRAN mirror), `.Renviron`   |
| `typst/`  | `%APPDATA%\typst\packages\local\`          | `<name>/<version>/typst.toml` + files    |

`.gitkeep` files are ignored. This README is not copied (it is outside the tool
folders). **Never put API keys or passwords here.** Rebuild the exe after any change.
