//! Set up the student's folder on the remote server over SSH:
//! `$dir_root/.pi/extensions/pi-vscode-context.ts` and `$dir_root/.envrc`.
//!
//! The setup script is base64-encoded and run as `echo … | base64 -d | sh`,
//! so it works whatever the remote login shell is, and needs no quoting.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::process::Command;

pub const DEFAULT_HOST: &str = "remote-p520";
const EXTENSION_TS: &str = include_str!("../remote/pi-vscode-context.ts");

/// Same rules as the tutor's `remote-run`: a plain SSH config alias.
pub fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty()
        || host.starts_with('-')
        || !host.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(format!("'{host}' is not a valid SSH host alias"));
    }
    Ok(())
}

pub fn validate_dir_root(dir: &str) -> Result<(), String> {
    if !dir.starts_with('/') || dir.contains(['\'', '\n', '\r', '"', '$', '`', '\\']) {
        return Err(format!("--dir-root must be a plain absolute path, got '{dir}'"));
    }
    Ok(())
}

/// POSIX sh script run on the server. `dir_root` = None -> detect under /data/users.
pub fn script(dir_root: Option<&str>) -> String {
    const BODY: &str = r#"
if [ -z "$dir_root" ]; then
  u="${USER:-$(id -un)}"
  if [ -d "/data/users/$u" ]; then
    dir_root="/data/users/$u"
  else
    for d in /data/users/*/; do
      d="${d%/}"
      [ -O "$d" ] || continue
      if [ -n "$dir_root" ]; then
        echo "en_get: several folders in /data/users belong to $u; rerun with --dir-root" >&2
        exit 1
      fi
      dir_root="$d"
    done
  fi
fi
if [ -z "$dir_root" ] || [ ! -d "$dir_root" ]; then
  echo "en_get: cannot find your folder under /data/users; rerun with --dir-root /data/users/<your folder>" >&2
  exit 1
fi
echo "  your folder: $dir_root"

# put_file NEW DEST: install NEW at DEST, keeping a .bak of a different old file
put_file() {
  if [ -f "$2" ] && cmp -s "$1" "$2"; then
    echo "  [same]    $2"
  elif [ -f "$2" ]; then
    mv "$2" "$2.bak" && cp "$1" "$2"
    echo "  [updated] $2 (old version saved as $2.bak)"
  else
    cp "$1" "$2"
    echo "  [new]     $2"
  fi
  rm -f "$1"
}

mkdir -p "$dir_root/.pi/extensions"
tmp="$(mktemp)"
printf '%s' "$ext_b64" | base64 -d > "$tmp"
put_file "$tmp" "$dir_root/.pi/extensions/pi-vscode-context.ts"

tmp="$(mktemp)"
cat > "$tmp" <<EOF
export dir_root="$dir_root"
export GH_CONFIG_DIR="\$dir_root/.gh"
export PI_CODING_AGENT_DIR="\$dir_root/.pi"
export GIT_CONFIG_GLOBAL="\$dir_root/.gitconfig"
EOF
put_file "$tmp" "$dir_root/.envrc"

if command -v direnv >/dev/null 2>&1; then
  (cd "$dir_root" && direnv allow .) && echo "  direnv: allowed $dir_root/.envrc"
else
  echo "  note: direnv is not installed on the server; load it with: source $dir_root/.envrc"
fi
"#;
    format!(
        "set -e\ndir_root='{}'\next_b64='{}'\n{BODY}",
        dir_root.unwrap_or(""),
        STANDARD.encode(EXTENSION_TS)
    )
}

/// The single command string sent to ssh.
pub fn remote_command(dir_root: Option<&str>) -> String {
    format!("echo {} | base64 -d | sh", STANDARD.encode(script(dir_root)))
}

pub fn run(host: &str, dir_root: Option<&str>, dry_run: bool) -> Result<(), String> {
    validate_host(host)?;
    if let Some(dir) = dir_root {
        validate_dir_root(dir)?;
    }
    let command = remote_command(dir_root);
    println!("  ssh {host}: pi-vscode-context.ts -> $dir_root/.pi/extensions/, write $dir_root/.envrc");
    if dry_run {
        println!("  (dry run) would run: ssh -- {host} <{} byte setup command>", command.len());
        return Ok(());
    }
    let status = Command::new("ssh")
        .args(["--", host, &command])
        .status()
        .map_err(|e| format!("cannot start ssh: {e} (is OpenSSH installed?)"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "ssh to '{host}' failed ({status}). Check that '{host}' is in your .ssh\\config \
             and that `ssh {host}` logs in."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy::temp_dir;
    use std::fs;

    #[test]
    fn host_and_dir_validation() {
        assert!(validate_host("remote-p520").is_ok());
        for bad in ["", "-oProxy", "user@host", "a b", "h;id"] {
            assert!(validate_host(bad).is_err(), "{bad:?}");
        }
        assert!(validate_dir_root("/data/users/046242_ken_swift").is_ok());
        for bad in ["data/users/x", "/data/'x", "/data/$HOME", "/a\nb"] {
            assert!(validate_dir_root(bad).is_err(), "{bad:?}");
        }
    }

    /// Runs the exact command sent over ssh through local `sh`, and the script
    /// itself through `dash` when available (Debian/Ubuntu's `/bin/sh`).
    #[cfg(unix)]
    #[test]
    fn remote_command_installs_files_and_backs_up_envrc() {
        check_remote_setup("sh", &|dir| remote_command(Some(dir)));
        if Command::new("dash").arg("-c").arg("true").status().is_ok_and(|s| s.success()) {
            check_remote_setup("dash", &|dir| script(Some(dir)));
        }
    }

    #[cfg(unix)]
    fn check_remote_setup(shell: &str, make: &dyn Fn(&str) -> String) {
        let root = temp_dir(&format!("remote-{shell}"));
        let dir = root.to_str().unwrap();
        let run = || {
            let out = Command::new(shell).arg("-c").arg(make(dir)).output().unwrap();
            assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
        };
        run();
        let ts = fs::read_to_string(root.join(".pi/extensions/pi-vscode-context.ts")).unwrap();
        assert_eq!(ts, EXTENSION_TS);
        let envrc = fs::read_to_string(root.join(".envrc")).unwrap();
        assert!(envrc.starts_with(&format!("export dir_root=\"{dir}\"\n")));
        assert!(envrc.contains("export PI_CODING_AGENT_DIR=\"$dir_root/.pi\"\n"));

        fs::write(root.join(".envrc"), "old").unwrap();
        run();
        assert_eq!(fs::read_to_string(root.join(".envrc.bak")).unwrap(), "old");
        assert_eq!(fs::read_to_string(root.join(".envrc")).unwrap(), envrc);
    }
}
