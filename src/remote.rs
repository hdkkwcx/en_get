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
u="${USER:-$(id -un)}"

# list_folders [N]: print "i) name" for each folder this login can write to
# (kong01 and kong02 students both see theirs), or with N set $pick to the N-th.
list_folders() {
  i=0
  for d in "$users_dir"/*/; do
    d="${d%/}"
    [ -d "$d" ] && [ -w "$d" ] || continue
    i=$((i + 1))
    if [ -z "$1" ]; then echo "    $i) ${d##*/}"; elif [ "$i" = "$1" ]; then pick="$d"; fi
  done
}

# Logins are shared, so never guess: always ask unless --dir-root was given.
if [ -z "$dir_root" ]; then
  if ! (exec < "$tty") 2>/dev/null; then  # subshell: a failed redirect must not exit the script
    echo "en_get: cannot ask which folder in $users_dir is yours (no terminal)." >&2
    echo "Rerun with: en_get --only remote --dir-root $users_dir/<your folder>" >&2
    exit 1
  fi
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
            if [ ! -e "$dir_root" ]; then mkdir "$dir_root" && echo "  created $dir_root"; fi ;;
        esac ;;
      *)
        pick=""
        list_folders "$ans"
        if [ -n "$pick" ]; then dir_root="$pick"; else echo "  There is no number $ans."; fi ;;
    esac
    if [ -n "$dir_root" ] && { [ ! -d "$dir_root" ] || [ ! -w "$dir_root" ]; }; then
      echo "  $dir_root is not a folder that $u can write to; choose another."
      dir_root=""
    fi
  done
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

# Hook direnv into the shared login's shell startup file (once per account),
# so terminals opened inside a student's folder load that folder's .envrc.
case "${SHELL##*/}" in
  bash) rc="$HOME/.bashrc"; sh_name=bash ;;
  zsh)  rc="$HOME/.zshrc";  sh_name=zsh ;;
  *)    rc="" ;;
esac
if [ -z "$rc" ]; then
  echo "  note: login shell '${SHELL:-unknown}' is not bash or zsh; add the direnv hook by hand (https://direnv.net/docs/hook.html)"
elif [ -f "$rc" ] && grep -q 'direnv hook' "$rc"; then
  echo "  [same]    $rc (direnv hook already set up)"
else
  tmp="$(mktemp)"
  {
    if [ -f "$rc" ]; then cat "$rc"; fi
    printf '\n# >>> en_get: load per-folder .envrc (direnv) >>>\n'
    printf 'if command -v direnv >/dev/null 2>&1; then eval "$(direnv hook %s)"; fi\n' "$sh_name"
    printf '# <<< en_get <<<\n'
  } > "$tmp"
  put_file "$tmp" "$rc"
fi

# direnv may live outside the PATH of a non-interactive ssh shell.
direnv_bin="$(command -v direnv 2>/dev/null || true)"
for c in "$HOME/.local/bin/direnv" "$HOME/bin/direnv" /usr/local/bin/direnv; do
  if [ -z "$direnv_bin" ] && [ -x "$c" ]; then direnv_bin="$c"; fi
done
if [ -n "$direnv_bin" ]; then
  (cd "$dir_root" && "$direnv_bin" allow .) && echo "  direnv: allowed $dir_root/.envrc"
  echo "  In VS Code, open $dir_root on this server; its terminals load your settings automatically."
else
  echo "  WARNING: direnv is not installed on this server, so $dir_root/.envrc will NOT load by itself." >&2
  echo "  Ask the tutor to install it (e.g. sudo apt install direnv, see https://direnv.net)." >&2
  echo "  Until then, in each terminal run: source $dir_root/.envrc" >&2
  exit 10
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
    println!(
        "  ssh {host}: pi-vscode-context.ts -> $dir_root/.pi/extensions/, write $dir_root/.envrc"
    );
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
        // 10 = our script: files written, but nothing will load .envrc.
        Some(10) => Err(format!(
            "files installed, but direnv is not installed on '{host}', so .envrc won't load \
             automatically. Ask the tutor to install direnv (see WARNING above)."
        )),
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

    /// The server-side environment the script sees.
    struct Env<'a> {
        home: &'a Path,
        login_shell: &'a str,
        /// Put a fake `direnv` (logs its args to $HOME/direnv.log) on PATH.
        direnv: bool,
        /// File playing the role of the terminal; None = no terminal.
        answers: Option<&'a Path>,
    }

    fn sh(shell: &str, code: &str, env: &Env) -> std::process::Output {
        let mut path = "/usr/bin:/bin".to_string();
        if env.direnv {
            let bin = env.home.join("fakebin");
            fs::create_dir_all(&bin).unwrap();
            let fake = bin.join("direnv");
            fs::write(&fake, "#!/bin/sh\necho \"$*\" >> \"$HOME/direnv.log\"\n").unwrap();
            fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755))
                .unwrap();
            path = format!("{}:{path}", bin.display());
        }
        let tty = env.answers.map_or("/nonexistent/tty".into(), |p| p.to_str().unwrap().to_owned());
        Command::new(shell)
            .arg("-c")
            .arg(code)
            .env("EN_GET_TTY", tty)
            .env("HOME", env.home)
            .env("SHELL", env.login_shell)
            .env("PATH", path)
            .output()
            .unwrap()
    }

    fn ok(shell: &str, out: &std::process::Output) -> String {
        assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn read(path: impl AsRef<Path>) -> String {
        fs::read_to_string(path.as_ref())
            .unwrap_or_else(|e| panic!("{}: {e}", path.as_ref().display()))
    }

    fn dirs(base: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let (home, student) = (base.join("home"), base.join("student"));
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&student).unwrap();
        (home, student)
    }

    fn assert_installed(dir: &Path) -> String {
        assert_eq!(read(dir.join(".pi/extensions/pi-vscode-context.ts")), EXTENSION_TS);
        let envrc = read(dir.join(".envrc"));
        assert!(envrc.starts_with(&format!("export dir_root=\"{}\"\n", dir.display())));
        assert!(envrc.contains("export PI_CODING_AGENT_DIR=\"$dir_root/.pi\"\n"));
        envrc
    }

    /// The exact command sent over ssh, with --dir-root: installs files, adds the
    /// direnv hook to ~/.bashrc once, runs `direnv allow`, backs up changed files.
    #[test]
    fn explicit_dir_root_installs_files_and_hooks_direnv_once() {
        for shell in shells() {
            let (home, dir) = dirs(&temp_dir(&format!("remote-{shell}")));
            fs::write(home.join(".bashrc"), "alias ll='ls -l'\n").unwrap();
            let env = Env { home: &home, login_shell: "/bin/bash", direnv: true, answers: None };
            let cmd = remote_command(Some(dir.to_str().unwrap()));

            let stdout = ok(shell, &sh(shell, &cmd, &env));
            assert!(stdout.contains("direnv: allowed"), "{stdout}");
            let envrc = assert_installed(&dir);
            let bashrc = read(home.join(".bashrc"));
            assert!(bashrc.starts_with("alias ll='ls -l'\n"));
            assert!(bashrc.contains("then eval \"$(direnv hook bash)\"; fi\n"));
            assert_eq!(read(home.join(".bashrc.bak")), "alias ll='ls -l'\n");
            assert_eq!(read(home.join("direnv.log")), "allow .\n");

            fs::write(dir.join(".envrc"), "old").unwrap();
            let stdout = ok(shell, &sh(shell, &cmd, &env));
            assert!(stdout.contains("direnv hook already set up"), "{stdout}");
            assert_eq!(read(home.join(".bashrc")), bashrc, "hook must be added only once");
            assert_eq!(read(dir.join(".envrc.bak")), "old");
            assert_eq!(read(dir.join(".envrc")), envrc);
        }
    }

    /// Shared kong01/kong02 logins: list folders this login can write to, let the
    /// student pick by number or type a new name; re-ask on bad input.
    #[test]
    fn asks_which_writable_folder_is_yours() {
        for shell in shells() {
            let base = temp_dir(&format!("pick-{shell}"));
            let (home, _) = dirs(&base);
            let users = base.join("users");
            for name in ["046242_ken_swift", "046243_amy", "046244_other"] {
                fs::create_dir_all(users.join(name)).unwrap();
            }
            let readonly = users.join("046244_other");
            fs::set_permissions(&readonly, std::os::unix::fs::PermissionsExt::from_mode(0o555))
                .unwrap();
            let script = script_with(None, users.to_str().unwrap());
            let answers = base.join("answers");
            let env = Env {
                home: &home,
                login_shell: "/bin/bash",
                direnv: true,
                answers: Some(&answers),
            };

            fs::write(&answers, "\n9\r\n2\r\n").unwrap();
            let stdout = ok(shell, &sh(shell, &script, &env));
            assert!(stdout.contains("1) 046242_ken_swift") && stdout.contains("2) 046243_amy"));
            assert!(!stdout.contains("046244_other"), "read-only folder must not be listed");
            assert!(stdout.contains("There is no number 9"));
            assert_installed(&users.join("046243_amy"));
            assert!(!users.join("046242_ken_swift/.envrc").exists());

            fs::write(&answers, "bad/name\n046244_other\nnew_student\n").unwrap();
            let stdout = ok(shell, &sh(shell, &script, &env));
            assert!(stdout.contains("Use only letters"));
            assert!(stdout.contains("046244_other is not a folder that"));
            assert_installed(&users.join("new_student"));

            // No terminal and no --dir-root: fail with a hint, change nothing.
            let out = sh(shell, &script, &Env { answers: None, ..env });
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("--dir-root"));
        }
    }

    /// zsh gets the hook in ~/.zshrc; without direnv the files are still
    /// installed but the script exits 10 with a warning; other shells get a note.
    #[test]
    fn zsh_hook_missing_direnv_and_other_shells() {
        for shell in shells() {
            let base = temp_dir(&format!("nodirenv-{shell}"));
            let (home, dir) = dirs(&base);
            let cmd = remote_command(Some(dir.to_str().unwrap()));

            let env = Env { home: &home, login_shell: "/bin/zsh", direnv: false, answers: None };
            let out = sh(shell, &cmd, &env);
            assert_eq!(out.status.code(), Some(10), "{shell}");
            assert!(
                String::from_utf8_lossy(&out.stderr).contains("WARNING: direnv is not installed")
            );
            assert_installed(&dir);
            assert!(read(home.join(".zshrc")).contains("eval \"$(direnv hook zsh)\""));
            assert!(!home.join(".bashrc").exists());

            let home2 = base.join("home2");
            fs::create_dir_all(&home2).unwrap();
            let env =
                Env { home: &home2, login_shell: "/usr/bin/fish", direnv: true, answers: None };
            let stdout = ok(shell, &sh(shell, &cmd, &env));
            assert!(stdout.contains("add the direnv hook by hand"));
            assert!(!home2.join(".bashrc").exists() && !home2.join(".zshrc").exists());
        }
    }
}
