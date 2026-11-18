//! Install the "VSCode Pi Agent Bridge" extension through the VS Code CLI.

use std::env;
use std::path::PathBuf;
use std::process::Command;

pub const EXTENSION_ID: &str = "zenyui.vscode-pi-bridge";
const MARKETPLACE: &str =
    "https://marketplace.visualstudio.com/items?itemName=zenyui.vscode-pi-bridge";

#[cfg(windows)]
const CLI_NAME: &str = "code.cmd";
#[cfg(not(windows))]
const CLI_NAME: &str = "code";

/// Default install locations first (users often lack `code` on PATH), then PATH.
fn find_code() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(windows) {
        for (var, sub) in [("LOCALAPPDATA", "Programs"), ("ProgramFiles", "")] {
            if let Some(base) = env::var_os(var) {
                let dir = PathBuf::from(base).join(sub).join("Microsoft VS Code");
                candidates.push(dir.join("bin").join(CLI_NAME));
            }
        }
    } else {
        candidates.push(PathBuf::from(
            "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
        ));
    }
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|dir| dir.join(CLI_NAME)));
    }
    candidates.into_iter().find(|p| p.is_file())
}

pub fn install(dry_run: bool) -> Result<(), String> {
    let Some(code) = find_code() else {
        return Err(format!(
            "VS Code not found. Install VS Code, then install the extension from {MARKETPLACE}"
        ));
    };
    let args = ["--install-extension", EXTENSION_ID, "--force"];
    println!("  {} {}", code.display(), args.join(" "));
    if dry_run {
        return Ok(());
    }
    let status = Command::new(&code)
        .args(args)
        .status()
        .map_err(|e| format!("cannot start the VS Code CLI: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("VS Code CLI failed ({status}). Install manually: {MARKETPLACE}"))
    }
}
