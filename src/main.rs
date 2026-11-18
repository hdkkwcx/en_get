//! en_get: one-click setup for students. Copies the config files embedded from
//! `assets/` (Pi, R, Typst template, VS Code settings), installs the VS Code
//! Pi bridge extension, and sets up the student's folder on remote-p520.

mod copy;
mod extension;
mod remote;

use clap::{Parser, ValueEnum};
use include_dir::{include_dir, Dir};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

#[derive(Parser)]
#[command(
    about = "Set up Pi, R, Typst and VS Code for students (double-click to do everything)",
    after_help = "Examples:\n  en_get                                  do everything\n  en_get --only typst --only vscode       just the Typst template + VS Code files\n  en_get --only remote --dir-root /data/users/046242_ken_swift"
)]
struct Args {
    /// Only run these steps (repeatable). Default: all
    #[arg(long, value_enum)]
    only: Vec<Step>,
    /// Show what would happen without changing anything
    #[arg(long)]
    dry_run: bool,
    /// Treat this folder as the user's home directory (for testing; skips extension/remote)
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    /// SSH host alias of the server
    #[arg(long, default_value = remote::DEFAULT_HOST)]
    ssh_host: String,
    /// Your folder on the server (default: detected under /data/users)
    #[arg(long, value_name = "PATH")]
    dir_root: Option<String>,
    /// Don't wait for Enter before exiting
    #[arg(long)]
    no_pause: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum Step {
    Pi,
    R,
    Typst,
    Vscode,
    Extension,
    Remote,
}

impl Step {
    const ALL: [Step; 6] =
        [Step::Pi, Step::R, Step::Typst, Step::Vscode, Step::Extension, Step::Remote];

    fn name(self) -> &'static str {
        match self {
            Step::Pi => "pi",
            Step::R => "r",
            Step::Typst => "typst",
            Step::Vscode => "vscode",
            Step::Extension => "extension",
            Step::Remote => "remote",
        }
    }

    /// For copy steps: where `assets/<name>/` goes. None for action steps.
    fn target(self, base: &Base) -> Option<PathBuf> {
        Some(match self {
            // %USERPROFILE%\.pi\agent
            Step::Pi => base.home.join(".pi").join("agent"),
            // R on Windows reads .Rprofile / .Renviron from Documents (R_USER)
            Step::R => base.documents.clone(),
            // %APPDATA%\typst\packages\local  -> usable as @local/<name>:<version>
            Step::Typst => base.appdata.join("typst").join("packages").join("local"),
            // %APPDATA%\Code\User  (settings.json, keybindings.json, snippets\)
            Step::Vscode => base.appdata.join("Code").join("User"),
            Step::Extension | Step::Remote => return None,
        })
    }
}

/// The user folders that targets are built from.
struct Base {
    home: PathBuf,
    documents: PathBuf,
    appdata: PathBuf,
}

impl Base {
    fn detect() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("cannot find the user's home folder")?;
        let documents = dirs::document_dir().unwrap_or_else(|| home.join("Documents"));
        let appdata = dirs::data_dir().ok_or("cannot find the AppData folder")?;
        Ok(Base { home, documents, appdata })
    }

    /// Fake Windows-like layout under `root`, used by --root and tests.
    fn from_root(root: &Path) -> Self {
        Base {
            home: root.to_path_buf(),
            documents: root.join("Documents"),
            appdata: root.join("AppData").join("Roaming"),
        }
    }
}

fn copy_step(step: Step, target: &Path, dry_run: bool) -> Result<copy::Stats, String> {
    println!("  -> {}", target.display());
    let stats = match ASSETS.get_dir(step.name()) {
        Some(dir) => copy::install(dir, dir.path(), target, dry_run)?,
        None => copy::Stats::default(),
    };
    if stats.total() == 0 {
        println!("  (no files bundled)");
    }
    Ok(stats)
}

/// Runs every step; returns the names of the ones that failed.
fn run(args: &Args) -> Vec<&'static str> {
    let base = match &args.root {
        Some(root) => Ok(Base::from_root(root)),
        None => Base::detect(),
    };
    let steps: Vec<Step> = if !args.only.is_empty() {
        args.only.clone()
    } else if args.root.is_some() {
        Step::ALL.into_iter().filter(|s| s.target(&Base::from_root(Path::new(""))).is_some()).collect()
    } else {
        Step::ALL.to_vec()
    };
    if args.dry_run {
        println!("Dry run: nothing will be changed.");
    }

    let mut copied = copy::Stats::default();
    let mut failed = Vec::new();
    for step in steps {
        println!("\n== {}", step.name());
        let result = match step {
            Step::Extension => extension::install(args.dry_run),
            Step::Remote => remote::run(&args.ssh_host, args.dir_root.as_deref(), args.dry_run),
            _ => base.as_ref().map_err(Clone::clone).and_then(|base| {
                let target = step.target(base).expect("copy step has a target");
                copy_step(step, &target, args.dry_run).map(|s| copied.add(&s))
            }),
        };
        if let Err(error) = result {
            eprintln!("  ERROR: {error}");
            failed.push(step.name());
        }
    }

    println!(
        "\nFiles: {} new, {} updated (old versions saved as *.bak), {} unchanged.",
        copied.new, copied.updated, copied.same
    );
    if failed.is_empty() {
        println!("All steps finished.");
    } else {
        eprintln!("Failed steps: {} (see ERROR lines above).", failed.join(", "));
    }
    failed
}

fn pause() {
    println!("\nPress Enter to exit...");
    let _ = io::stdin().read_line(&mut String::new());
}

fn main() -> ExitCode {
    let args = Args::parse();
    let failed = run(&args);
    // Keep the console window open when a student double-clicks the exe.
    if !args.no_pause {
        pause();
    }
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_under_root() {
        let base = Base::from_root(Path::new("/home/stu"));
        let t = |s: Step| s.target(&base).unwrap();
        assert_eq!(t(Step::Pi), Path::new("/home/stu/.pi/agent"));
        assert_eq!(t(Step::R), Path::new("/home/stu/Documents"));
        assert_eq!(t(Step::Typst), Path::new("/home/stu/AppData/Roaming/typst/packages/local"));
        assert_eq!(t(Step::Vscode), Path::new("/home/stu/AppData/Roaming/Code/User"));
        assert!(Step::Extension.target(&base).is_none());
        assert!(Step::Remote.target(&base).is_none());
    }

    #[test]
    fn bundled_assets_land_in_the_right_places() {
        let root = copy::temp_dir("assets");
        let base = Base::from_root(&root);
        for step in Step::ALL {
            if let (Some(target), Some(dir)) = (step.target(&base), ASSETS.get_dir(step.name())) {
                copy::install(dir, dir.path(), &target, false).unwrap();
            }
        }
        let roaming = root.join("AppData/Roaming");
        for file in [
            "typst/packages/local/modern-cug-report/0.1.3/typst.toml",
            "typst/packages/local/modern-cug-report/0.1.3/lib.typ",
            "Code/User/settings.json",
            "Code/User/keybindings.json",
            "Code/User/snippets/typst.json",
        ] {
            assert!(roaming.join(file).is_file(), "missing {file}");
        }
        // Placeholders never leak into targets.
        assert!(!root.join(".pi/agent/.gitkeep").exists());
        assert!(!root.join("Documents/.gitkeep").exists());
    }
}
