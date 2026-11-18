//! Copy embedded files to disk, keeping a `.bak` of anything replaced.

use include_dir::{Dir, DirEntry};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// Placeholder files that keep empty folders in git; never copied.
pub const IGNORED: &[&str] = &[".gitkeep", ".DS_Store"];

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    New,
    Updated,
    Same,
}

#[derive(Default, Debug)]
pub struct Stats {
    pub new: usize,
    pub updated: usize,
    pub same: usize,
}

impl Stats {
    fn record(&mut self, action: &Action) {
        match action {
            Action::New => self.new += 1,
            Action::Updated => self.updated += 1,
            Action::Same => self.same += 1,
        }
    }
    pub fn add(&mut self, other: &Stats) {
        self.new += other.new;
        self.updated += other.updated;
        self.same += other.same;
    }
    pub fn total(&self) -> usize {
        self.new + self.updated + self.same
    }
}

/// `foo.txt` -> `foo.txt.bak`
pub fn backup_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

/// Write `contents` to `dest`. An existing, different file is renamed to `*.bak` first.
pub fn copy_file(contents: &[u8], dest: &Path, dry_run: bool) -> io::Result<Action> {
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

/// Copy every embedded file under `dir` into `target`, keeping paths relative to `prefix`.
pub fn install(dir: &Dir, prefix: &Path, target: &Path, dry_run: bool) -> Result<Stats, String> {
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

#[cfg(test)]
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("en_get-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
