//! Set up the student's folder on the remote server over SSH:
//! `$dir_root/.pi/extensions/pi-vscode-context.ts` and `$dir_root/.envrc`.
//!
//! The setup script is base64-encoded and run as `echo … | base64 -d | sh`,
//! so it works whatever the remote login shell is, and needs no quoting.
//! Students often share one login (e.g. `kong02`) with one folder each under
//! /data/users, so unless `--dir-root` is given the script asks which folder
//! is theirs, reading the answer from the terminal (`ssh -t`).

use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::IsTerminal;
use std::process::Command;

pub const DEFAULT_HOST: &str = "remote-p520";
const USERS_DIR: &str = "/data/users";
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

/// POSIX sh script run on the server. `dir_root` = None -> ask under `users_dir`.
fn script_with(dir_root: Option<&str>, users_dir: &str) -> String {
    const BODY: &str = r#"
tty="${EN_GET_TTY:-/dev/tty}"

# list_folders [N]: print "i) name" for each folder we own, or with N set
# $pick to the N-th one.
list_folders() {
  i=0
  for d in "$users_dir"/*/; do
    d="${d%/}"
    [ -O "$d" ] || continue
    i=$((i + 1))
    if [ -z "$1" ]; then echo "    $i) ${d##*/}"; elif [ "$i" = "$1" ]; then pick="$d"; fi
  done
}

if [ -z "$dir_root" ]; then
  u="${USER:-$(id -un)}"
  if [ -d "$users_dir/$u" ]; then
    dir_root="$users_dir/$u"
  elif ! (exec < "$tty") 2>/dev/null; then  # subshell: a failed redirect must not exit the script
    echo "en_get: cannot ask which folder in $users_dir is yours (no terminal)." >&2
    echo "Rerun with: en_get --only remote --dir-root $users_dir/<your folder>" >&2
    exit 1
  else
    echo "  Which folder in $users_dir is yours? (login: $u)"
    list_folders
    exec 3< "$tty"  # open once, so each read takes the next answer
    while [ -z "$dir_root" ]; do
      printf '  Enter a number, or a new folder name to create (e.g. 046242_ken_swift): '
      IFS= read -r ans <&3 || { echo; echo "en_get: no answer given" >&2; exit 1; }
      ans=$(printf '%s' "$ans" | tr -d '\r')
      case "$ans" in
        '') ;;
        *[!0-9]*)
          case "$ans" in
            .*|*[!A-Za-z0-9._-]*) echo "  Use only letters, digits, '.', '_' and '-'." ;;
            *)
              dir_root="$users_dir/$ans"
              if [ ! -d "$dir_root" ]; then mkdir "$dir_root" && echo "  created $dir_root"; fi ;;
          esac ;;
        *)
          pick=""
          list_folders "$ans"
          if [ -n "$pick" ]; then dir_root="$pick"; else echo "  There is no number $ans."; fi ;;
      esac
    done
  fi
fi
if [ ! -d "$dir_root" ]; then
  echo "en_get: folder $dir_root does not exist" >&2
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
        "set -e\ndir_root='{}'\nusers_dir='{users_dir}'\next_b64='{}'\n{BODY}",
        dir_root.unwrap_or(""),
        STANDARD.encode(EXTENSION_TS)
    )
}

/// The single command string sent to ssh.
pub fn remote_command(dir_root: Option<&str>) -> String {
    format!("echo {} | base64 -d | sh", STANDARD.encode(script_with(dir_root, USERS_DIR)))
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
    let mut ssh = Command::new("ssh");
    // A terminal on the server lets the script ask which folder is yours.
    if dir_root.is_none() && std::io::stdin().is_terminal() {
        ssh.arg("-t");
    }
    let status = ssh
        .args(["--", host, &command])
        .status()
        .map_err(|e| format!("cannot start ssh: {e} (is OpenSSH installed?)"))?;
    match status.code() {
        Some(0) => Ok(()),
        // 255 = ssh itself failed (unknown host, auth, network).
        Some(255) => Err(format!(
            "could not connect to '{host}'. Check that '{host}' is in your .ssh\\config \
             and that `ssh {host}` logs in."
        )),
        _ => Err(format!("setup on '{host}' failed ({status}); see the message above.")),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::copy::temp_dir;
    use std::fs;
    use std::path::Path;

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

    /// `sh`, plus `dash` (Debian/Ubuntu's /bin/sh) when installed.
    fn shells() -> Vec<&'static str> {
        let mut shells = vec!["sh"];
        if Command::new("dash").arg("-c").arg("true").status().is_ok_and(|s| s.success()) {
            shells.push("dash");
        }
        shells
    }

    /// Run `code` with `shell`; `answers` (if any) play the role of the terminal.
    fn sh(shell: &str, code: &str, answers: Option<&Path>) -> std::process::Output {
        let tty = answers.map_or("/nonexistent/tty".into(), |p| p.to_str().unwrap().to_owned());
        Command::new(shell).arg("-c").arg(code).env("EN_GET_TTY", tty).output().unwrap()
    }

    fn assert_installed(dir: &Path) -> String {
        let ts = fs::read_to_string(dir.join(".pi/extensions/pi-vscode-context.ts")).unwrap();
        assert_eq!(ts, EXTENSION_TS);
        let envrc = fs::read_to_string(dir.join(".envrc")).unwrap();
        assert!(envrc.starts_with(&format!("export dir_root=\"{}\"\n", dir.display())));
        assert!(envrc.contains("export PI_CODING_AGENT_DIR=\"$dir_root/.pi\"\n"));
        envrc
    }

    /// The exact command sent over ssh, with --dir-root; second run backs up .envrc.
    #[test]
    fn explicit_dir_root_installs_files_and_backs_up_envrc() {
        for shell in shells() {
            let root = temp_dir(&format!("remote-{shell}"));
            let dir = root.to_str().unwrap();
            let run = || {
                let out = sh(shell, &remote_command(Some(dir)), None);
                assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
            };
            run();
            let envrc = assert_installed(&root);
            fs::write(root.join(".envrc"), "old").unwrap();
            run();
            assert_eq!(fs::read_to_string(root.join(".envrc.bak")).unwrap(), "old");
            assert_eq!(fs::read_to_string(root.join(".envrc")).unwrap(), envrc);
        }
    }

    /// Shared login owning several folders: the student picks one by number,
    /// or types a new name (invalid input and bad numbers are re-asked).
    #[test]
    fn asks_which_folder_when_login_is_shared() {
        for shell in shells() {
            let base = temp_dir(&format!("pick-{shell}"));
            let users = base.join("users");
            for name in ["046242_ken_swift", "046243_amy"] {
                fs::create_dir_all(users.join(name)).unwrap();
            }
            let script = script_with(None, users.to_str().unwrap());
            let answers = base.join("answers");

            fs::write(&answers, "\n9\r\n2\r\n").unwrap();
            let out = sh(shell, &script, Some(&answers));
            assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(stdout.contains("1) 046242_ken_swift") && stdout.contains("2) 046243_amy"));
            assert!(stdout.contains("There is no number 9"));
            assert_installed(&users.join("046243_amy"));
            assert!(!users.join("046242_ken_swift/.envrc").exists());

            fs::write(&answers, "bad/name\nnew_student\n").unwrap();
            let out = sh(shell, &script, Some(&answers));
            assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
            assert!(String::from_utf8_lossy(&out.stdout).contains("Use only letters"));
            assert_installed(&users.join("new_student"));

            // No terminal and no --dir-root: fail with a hint, change nothing.
            let out = sh(shell, &script, None);
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("--dir-root"));
        }
    }
}
