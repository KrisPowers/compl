//! The shells autocomplete runs, how each one is started with the prompt
//! markers installed, and how each one's startup file is set up.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context as _, Result};
use clap::ValueEnum;
use portable_pty::CommandBuilder;

const BASH: &str = include_str!("../shell/bash.sh");
const ZSHENV: &str = include_str!("../shell/zsh/zshenv.zsh");
const ZPROFILE: &str = include_str!("../shell/zsh/zprofile.zsh");
const ZSHRC: &str = include_str!("../shell/zsh/zshrc.zsh");
const FISH: &str = include_str!("../shell/fish.fish");
const PWSH: &str = include_str!("../shell/pwsh.ps1");

const INIT_POSIX: &str = include_str!("../shell/init/posix.sh");
const INIT_FISH: &str = include_str!("../shell/init/fish.fish");
const INIT_PWSH: &str = include_str!("../shell/init/pwsh.ps1");

const SETUP_START: &str = "# autocomplete start";
const SETUP_END: &str = "# autocomplete end";
const CMD_HAS_NO_STARTUP_FILE: &str = "cmd has no startup file to add autocomplete to; \
     point your terminal profile at `autocomplete run --shell cmd` instead";

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    Powershell,
    Cmd,
}

/// How a shell quotes and escapes words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quoting {
    Posix,
    Pwsh,
    Cmd,
}

impl Shell {
    /// The shell you are most likely using: `$SHELL` on Unix, PowerShell on
    /// Windows.
    pub fn detect() -> Shell {
        if cfg!(windows) {
            return if on_path("pwsh") {
                Shell::Pwsh
            } else {
                Shell::Powershell
            };
        }
        let shell = env::var("SHELL").unwrap_or_default();
        match Path::new(&shell).file_name().and_then(|name| name.to_str()) {
            Some("zsh") => Shell::Zsh,
            Some("fish") => Shell::Fish,
            Some("pwsh") => Shell::Pwsh,
            _ => Shell::Bash,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Pwsh => "pwsh",
            Shell::Powershell => "powershell",
            Shell::Cmd => "cmd",
        }
    }

    pub fn quoting(self) -> Quoting {
        match self {
            Shell::Bash | Shell::Zsh | Shell::Fish => Quoting::Posix,
            Shell::Pwsh | Shell::Powershell => Quoting::Pwsh,
            Shell::Cmd => Quoting::Cmd,
        }
    }

    /// The directory separator to add after a completed directory.
    pub fn separator(self) -> char {
        if cfg!(windows) && self.quoting() != Quoting::Posix {
            '\\'
        } else {
            '/'
        }
    }
}

pub fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn on_path(program: &str) -> bool {
    let file = if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_string()
    };
    env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(&file).is_file()))
}

/// The command that starts `shell` with autocomplete's prompt markers loaded
/// after the user's own startup files.
pub fn command(shell: Shell, exe: Option<PathBuf>, login: bool) -> Result<CommandBuilder> {
    let program = exe.map_or_else(|| OsString::from(shell.name()), PathBuf::into_os_string);
    let mut cmd = CommandBuilder::new(program);
    cmd.cwd(env::current_dir()?);
    cmd.env("AUTOCOMPLETE_SESSION", "1");
    match shell {
        Shell::Bash => {
            let dir = integration(shell, &[("bashrc", BASH)])?;
            cmd.arg("--rcfile");
            cmd.arg(dir.join("bashrc"));
            cmd.arg("-i");
            if login {
                cmd.env("AUTOCOMPLETE_LOGIN", "1");
            }
        }
        Shell::Zsh => {
            let files = [(".zshenv", ZSHENV), (".zprofile", ZPROFILE), (".zshrc", ZSHRC)];
            let dir = integration(shell, &files)?;
            if let Some(user) = env::var_os("ZDOTDIR") {
                cmd.env("AUTOCOMPLETE_ZDOTDIR", user);
            }
            cmd.env("AUTOCOMPLETE_INTEGRATION", &dir);
            cmd.env("ZDOTDIR", &dir);
            cmd.arg("-i");
            if login {
                cmd.arg("-l");
            }
        }
        Shell::Fish => {
            let dir = integration(shell, &[("integration.fish", FISH)])?;
            let script = dir.join("integration.fish").display().to_string();
            cmd.arg("-i");
            if login {
                cmd.arg("-l");
            }
            cmd.arg("-C");
            cmd.arg(format!("source {}", fish_quote(&script)));
        }
        Shell::Pwsh | Shell::Powershell => {
            if login && shell == Shell::Pwsh && !cfg!(windows) {
                cmd.arg("-Login");
            }
            cmd.args(["-NoLogo", "-NoExit", "-EncodedCommand"]);
            cmd.arg(encode_command(PWSH));
        }
        Shell::Cmd => {
            let prompt = env::var("PROMPT").unwrap_or_else(|_| "$P$G".to_string());
            cmd.env("PROMPT", format!("$E]6973;A;$P$E\\{prompt}$E]6973;B$E\\"));
        }
    }
    Ok(cmd)
}

/// Writes a shell's integration files into autocomplete's cache directory and
/// returns that directory. Files are replaced atomically so shells starting at
/// the same moment never read half a file.
fn integration(shell: Shell, files: &[(&str, &str)]) -> Result<PathBuf> {
    let dir = cache_dir().join(shell.name());
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    for (name, content) in files {
        let path = dir.join(name);
        if fs::read_to_string(&path).ok().as_deref() == Some(*content) {
            continue;
        }
        let tmp = dir.join(format!("{name}.{}", std::process::id()));
        fs::write(&tmp, content).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(dir)
}

pub fn cache_dir() -> PathBuf {
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|home| home.join(".cache")))
    };
    base.unwrap_or_else(env::temp_dir).join("autocomplete")
}

/// PowerShell's `-EncodedCommand` takes base64 of UTF-16LE. Passing the
/// script this way works under any execution policy.
fn encode_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &byte)| n | (u32::from(byte) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn posix_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn fish_quote(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn pwsh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The snippet a startup file evaluates: it starts `autocomplete run` for
/// interactive terminals, and returns to the outer shell if that fails.
pub fn init(shell: Shell) -> Result<String> {
    let exe = env::current_exe()?.display().to_string();
    let script = match shell {
        Shell::Bash => INIT_POSIX
            .replace("__LOGIN__", "shopt -q login_shell")
            .replace("__EXE__", &posix_quote(&exe)),
        Shell::Zsh => INIT_POSIX
            .replace("__LOGIN__", "[[ -o login ]]")
            .replace("__EXE__", &posix_quote(&exe)),
        Shell::Fish => INIT_FISH.replace("__EXE__", &fish_quote(&exe)),
        Shell::Pwsh | Shell::Powershell => INIT_PWSH.replace("__EXE__", &pwsh_quote(&exe)),
        Shell::Cmd => bail!(CMD_HAS_NO_STARTUP_FILE),
    };
    Ok(script.replace("__SHELL__", shell.name()))
}

/// Adds the init line to the shell's startup file, or removes it.
pub fn setup(shell: Shell, remove: bool) -> Result<()> {
    let file = startup_file(shell)?;
    let text = fs::read_to_string(&file).unwrap_or_default();
    let block = block_range(&text);
    if remove {
        let Some(range) = block else {
            println!("autocomplete is not set up in {}", file.display());
            return Ok(());
        };
        let mut kept = text;
        kept.replace_range(range, "");
        if shell == Shell::Fish && kept.trim().is_empty() {
            fs::remove_file(&file)?;
        } else {
            fs::write(&file, kept)?;
        }
        println!("Removed autocomplete from {}", file.display());
        return Ok(());
    }
    if block.is_some() {
        println!("autocomplete is already set up in {}", file.display());
        return Ok(());
    }
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = text;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&format!("{SETUP_START}\n{}\n{SETUP_END}\n", init_line(shell)?));
    fs::write(&file, out).with_context(|| format!("writing {}", file.display()))?;
    println!("Added autocomplete to {}. Open a new terminal to start using it.", file.display());
    Ok(())
}

/// The byte range of the block `setup` added, including its trailing newline.
fn block_range(text: &str) -> Option<std::ops::Range<usize>> {
    let start = text.find(SETUP_START)?;
    let end = start + text[start..].find(SETUP_END)? + SETUP_END.len();
    let end = end + usize::from(text[end..].starts_with('\n'));
    Some(start..end)
}

/// The startup-file line. It calls `autocomplete` by name when it is on PATH
/// (so upgrades keep working) and by full path otherwise.
fn init_line(shell: Shell) -> Result<String> {
    let exe = if on_path("autocomplete") {
        None
    } else {
        Some(env::current_exe()?.display().to_string())
    };
    let name = shell.name();
    Ok(match (shell, exe) {
        (Shell::Bash | Shell::Zsh, None) => format!(
            "command -v autocomplete >/dev/null 2>&1 && eval \"$(autocomplete init {name})\""
        ),
        (Shell::Bash | Shell::Zsh, Some(exe)) => {
            let exe = posix_quote(&exe);
            format!("[ -x {exe} ] && eval \"$({exe} init {name})\"")
        }
        (Shell::Fish, None) => {
            "if type -q autocomplete\n    autocomplete init fish | source\nend".to_string()
        }
        (Shell::Fish, Some(exe)) => {
            let exe = fish_quote(&exe);
            format!("if test -x {exe}\n    {exe} init fish | source\nend")
        }
        (Shell::Pwsh | Shell::Powershell, None) => format!(
            "if (Get-Command autocomplete -ErrorAction SilentlyContinue) \
             {{ Invoke-Expression (& autocomplete init {name} | Out-String) }}"
        ),
        (Shell::Pwsh | Shell::Powershell, Some(exe)) => {
            let exe = pwsh_quote(&exe);
            format!(
                "if (Test-Path {exe}) {{ Invoke-Expression (& {exe} init {name} | Out-String) }}"
            )
        }
        (Shell::Cmd, _) => bail!(CMD_HAS_NO_STARTUP_FILE),
    })
}

fn startup_file(shell: Shell) -> Result<PathBuf> {
    let home_dir = || home().context("cannot find your home directory");
    Ok(match shell {
        Shell::Bash if cfg!(target_os = "macos") => home_dir()?.join(".bash_profile"),
        Shell::Bash => home_dir()?.join(".bashrc"),
        Shell::Zsh => match env::var_os("ZDOTDIR") {
            Some(dir) => PathBuf::from(dir).join(".zshrc"),
            None => home_dir()?.join(".zshrc"),
        },
        Shell::Fish => {
            let config = match env::var_os("XDG_CONFIG_HOME") {
                Some(dir) => PathBuf::from(dir),
                None => home_dir()?.join(".config"),
            };
            config.join("fish").join("conf.d").join("autocomplete.fish")
        }
        Shell::Pwsh | Shell::Powershell => profile(shell)?,
        Shell::Cmd => bail!(CMD_HAS_NO_STARTUP_FILE),
    })
}

/// Asks PowerShell for `$PROFILE`, which moves with redirected Documents
/// folders and differs between PowerShell 7 and Windows PowerShell.
fn profile(shell: Shell) -> Result<PathBuf> {
    let output = Command::new(shell.name())
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [Text.Encoding]::UTF8; $PROFILE",
        ])
        .output()
        .with_context(|| format!("running {}", shell.name()))?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || path.is_empty() {
        bail!("could not read $PROFILE from {}", shell.name());
    }
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_powershell_commands() {
        assert_eq!(encode_command("a"), "YQA=");
        assert_eq!(encode_command("ab"), "YQBiAA==");
        assert_eq!(encode_command("abc"), "YQBiAGMA");
    }

    #[test]
    fn finds_and_removes_the_setup_block() {
        let text = format!("before\n\n{SETUP_START}\nline\n{SETUP_END}\nafter\n");
        let range = block_range(&text).unwrap();
        let mut kept = text.clone();
        kept.replace_range(range, "");
        assert_eq!(kept, "before\n\nafter\n");
        assert_eq!(block_range("nothing here"), None);
    }

    #[test]
    fn init_scripts_name_the_shell() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh] {
            let script = init(shell).unwrap();
            assert!(script.contains(&format!("--shell {}", shell.name())));
            assert!(!script.contains("__EXE__") && !script.contains("__LOGIN__"));
        }
        assert!(init(Shell::Cmd).is_err());
    }
}
