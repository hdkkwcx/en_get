//! en_get: copies bundled Pi / R / Typst config files into the right folders
//! on a student's computer. All files under `assets/` are embedded in the exe.

use clap::{Parser, ValueEnum};
use include_dir::{include_dir, Dir, DirEntry};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

/// Placeholder files that keep empty folders in git; never copied.
const IGNORED: &[&str] = &[".gitkeep", ".DS_Store"];

#[derive(Parser)]
#[command(
    about = "Set up Pi, R and Typst config files (double-click to install everything)",
    after_help = "Examples:\n  en_get                      install everything\n  en_get --only pi --dry-run  preview what Pi would get"
)]
struct Args {
    /// Only install these tools (repeatable). Default: all
    #[arg(long, value_enum)]
    only: Vec<Tool>,
    /// Show what would be copied without writing anything
    #[arg(long)]
    dry_run: bool,
    /// Treat this folder as the user's home directory (for testing)
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    /// Don't wait for Enter before exiting
    #[arg(long)]
    no_pause: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum Tool {
    Pi,
    R,
    Typst,
}

impl Tool {
    const ALL: [Tool; 3] = [Tool::Pi, Tool::R, Tool::Typst];

    /// Folder name under `assets/`.
    fn name(self) -> &'static str {
        match self {
            Tool::Pi => "pi",
            Tool::R => "r",
            Tool::Typst => "typst",
        }
    }

    /// Where this tool's files go on the student's machine.
    fn target(self, base: &Base) -> PathBuf {
        match self {
            // %USERPROFILE%\.pi\agent
            Tool::Pi => base.home.join(".pi").join("agent"),
            // R on Windows reads .Rprofile / .Renviron from Documents (R_USER)
            Tool::R => base.documents.clone(),
            // %APPDATA%\typst\packages\local  -> usable as @local/<name>:<version>
            Tool::Typst => base.appdata.join("typst").join("packages").join("local"),
        }
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

#[derive(Debug, PartialEq, Eq)]
enum Action {
    New,
    Updated,
    Same,
}

#[derive(Default, Debug)]
struct Stats {
    new: usize,
    updated: usize,
    same: usize,
}

impl Stats {
    fn record(&mut self, action: &Action) {
        match action {
            Action::New => self.new += 1,
            Action::Updated => self.updated += 1,
            Action::Same => self.same += 1,
        }
    }
    fn add(&mut self, other: &Stats) {
        self.new += other.new;
        self.updated += other.updated;
        self.same += other.same;
    }
    fn total(&self) -> usize {
        self.new + self.updated + self.same
    }
}

/// `foo.txt` -> `foo.txt.bak`
fn backup_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

/// Write `contents` to `dest`. An existing, different file is renamed to `*.bak` first.
fn copy_file(contents: &[u8], dest: &Path, dry_run: bool) -> io::Result<Action> {
    let action = match fs::read(dest) {
        Ok(old) if old == contents => return Ok(Action::Same),
        Ok(_) => Action::Updated,
        Err(e) if e.kind() == ErrorKind::NotFound => Action::New,
        Err(e) => return Err(e),
    };
    if dry_run {
        return Ok(action);
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if action == Action::Updated {
        fs::rename(dest, backup_path(dest))?;
    }
    fs::write(dest, contents)?;
    Ok(action)
}

/// Copy every embedded file under `dir` into `target`, keeping relative paths.
fn install(dir: &Dir, prefix: &Path, target: &Path, dry_run: bool) -> Result<Stats, String> {
    let mut stats = Stats::default();
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => stats.add(&install(sub, prefix, target, dry_run)?),
            DirEntry::File(file) => {
                let name = file.path().file_name().and_then(|n| n.to_str()).unwrap_or("");
                if IGNORED.contains(&name) {
                    continue;
                }
                let rel = file.path().strip_prefix(prefix).unwrap_or(file.path());
                let dest = target.join(rel);
                let action = copy_file(file.contents(), &dest, dry_run)
                    .map_err(|e| format!("{}: {e}", dest.display()))?;
                let tag = match action {
                    Action::New => "[new]    ",
                    Action::Updated => "[updated]",
                    Action::Same => "[same]   ",
                };
                println!("  {tag} {}", dest.display());
                stats.record(&action);
            }
        }
    }
    Ok(stats)
}

fn run(args: &Args) -> Result<(), String> {
    let base = match &args.root {
        Some(root) => Base::from_root(root),
        None => Base::detect()?,
    };
    let tools = if args.only.is_empty() { Tool::ALL.to_vec() } else { args.only.clone() };
    if args.dry_run {
        println!("Dry run: nothing will be written.\n");
    }
    let mut total = Stats::default();
    for tool in tools {
        let target = tool.target(&base);
        println!("== {} -> {}", tool.name(), target.display());
        let stats = match ASSETS.get_dir(tool.name()) {
            Some(dir) => install(dir, dir.path(), &target, args.dry_run)?,
            None => Stats::default(),
        };
        if stats.total() == 0 {
            println!("  (no files bundled for {})", tool.name());
        }
        total.add(&stats);
    }
    println!(
        "\nDone: {} new, {} updated (old versions saved as *.bak), {} unchanged.",
        total.new, total.updated, total.same
    );
    Ok(())
}

fn pause() {
    println!("\nPress Enter to exit...");
    let _ = io::stdin().read_line(&mut String::new());
}

fn main() -> ExitCode {
    let args = Args::parse();
    let code = match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nError: {error}");
            ExitCode::FAILURE
        }
    };
    // Keep the console window open when a student double-clicks the exe.
    if !args.no_pause {
        pause();
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("en_get-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn targets_under_root() {
        let base = Base::from_root(Path::new("/home/stu"));
        assert_eq!(Tool::Pi.target(&base), Path::new("/home/stu/.pi/agent"));
        assert_eq!(Tool::R.target(&base), Path::new("/home/stu/Documents"));
        assert_eq!(
            Tool::Typst.target(&base),
            Path::new("/home/stu/AppData/Roaming/typst/packages/local")
        );
    }

    #[test]
    fn copy_new_same_and_updated_with_backup() {
        let dest = temp_dir("copy").join("a/b/settings.json");
        assert_eq!(copy_file(b"v1", &dest, false).unwrap(), Action::New);
        assert_eq!(copy_file(b"v1", &dest, false).unwrap(), Action::Same);
        assert_eq!(copy_file(b"v2", &dest, false).unwrap(), Action::Updated);
        assert_eq!(fs::read(&dest).unwrap(), b"v2");
        assert_eq!(fs::read(backup_path(&dest)).unwrap(), b"v1");
    }

    #[test]
    fn dry_run_writes_nothing() {
        let dir = temp_dir("dry");
        let dest = dir.join("x/.Rprofile");
        assert_eq!(copy_file(b"hi", &dest, true).unwrap(), Action::New);
        assert!(!dir.join("x").exists());
    }

    #[test]
    fn placeholders_are_never_copied() {
        let dir = temp_dir("install");
        for tool in Tool::ALL {
            let sub = ASSETS.get_dir(tool.name()).expect("asset folder exists");
            install(sub, sub.path(), &dir, false).unwrap();
        }
        let leaked = fs::read_dir(&dir).unwrap().filter_map(Result::ok).any(|e| {
            IGNORED.contains(&e.file_name().to_str().unwrap_or(""))
        });
        assert!(!leaked);
    }
}
