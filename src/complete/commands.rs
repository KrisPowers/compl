//! Command names: programs on PATH and each shell's built-in commands.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;

use crate::shell::Shell;

#[rustfmt::skip]
const POSIX: &[&str] = &[
    "alias", "bg", "bind", "break", "builtin", "cd", "command", "complete", "continue", "declare",
    "dirs", "disown", "echo", "eval", "exec", "exit", "export", "false", "fg", "hash", "help",
    "history", "jobs", "kill", "let", "local", "logout", "popd", "printf", "pushd", "pwd", "read",
    "readonly", "return", "set", "shift", "shopt", "source", "test", "times", "trap", "true",
    "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait",
];

#[rustfmt::skip]
const FISH: &[&str] = &[
    "abbr", "alias", "and", "begin", "bg", "bind", "block", "break", "builtin", "case", "cd",
    "cdh", "command", "commandline", "complete", "contains", "continue", "count", "dirh", "dirs",
    "disown", "echo", "else", "emit", "end", "eval", "exec", "exit", "false", "fg", "fish_config",
    "for", "funced", "funcsave", "function", "functions", "history", "if", "jobs", "math",
    "nextd", "not", "or", "popd", "prevd", "printf", "pushd", "pwd", "random", "read",
    "realpath", "return", "set", "set_color", "source", "status", "string", "switch", "test",
    "time", "true", "type", "ulimit", "umask", "wait", "while",
];

#[rustfmt::skip]
const POWERSHELL: &[&str] = &[
    "Add-Content", "Clear-Host", "Compress-Archive", "ConvertFrom-Json", "ConvertTo-Json",
    "Copy-Item", "Expand-Archive", "Export-Csv", "ForEach-Object", "Format-List", "Format-Table",
    "Get-Alias", "Get-ChildItem", "Get-Command", "Get-Content", "Get-Date", "Get-FileHash",
    "Get-Help", "Get-History", "Get-Item", "Get-Location", "Get-Member", "Get-Module",
    "Get-Process", "Get-Service", "Get-Variable", "Group-Object", "Import-Csv", "Import-Module",
    "Invoke-Expression", "Invoke-Item", "Invoke-RestMethod", "Invoke-WebRequest",
    "Measure-Object", "Move-Item", "New-Item", "Out-File", "Pop-Location", "Push-Location",
    "Read-Host", "Remove-Item", "Rename-Item", "Resolve-Path", "Select-Object", "Select-String",
    "Set-Content", "Set-Location", "Set-Variable", "Sort-Object", "Split-Path", "Start-Process",
    "Start-Sleep", "Stop-Process", "Test-Path", "Where-Object", "Write-Error", "Write-Host",
    "Write-Output", "cat", "cd", "clear", "cls", "copy", "cp", "del", "dir", "echo", "exit",
    "gci", "kill", "ls", "md", "mkdir", "move", "mv", "ni", "popd", "ps", "pushd", "pwd", "rd",
    "ren", "rm", "rmdir", "sl", "type",
];

#[rustfmt::skip]
const CMD: &[&str] = &[
    "assoc", "call", "cd", "chdir", "cls", "color", "copy", "date", "del", "dir", "echo",
    "endlocal", "erase", "exit", "for", "ftype", "goto", "if", "md", "mkdir", "mklink", "move",
    "path", "pause", "popd", "prompt", "pushd", "rd", "ren", "rename", "rmdir", "set", "setlocal",
    "shift", "start", "time", "title", "type", "ver", "verify", "vol",
];

pub fn builtins(shell: Shell) -> &'static [&'static str] {
    match shell {
        Shell::Bash | Shell::Zsh => POSIX,
        Shell::Fish => FISH,
        Shell::Pwsh | Shell::Powershell => POWERSHELL,
        Shell::Cmd => CMD,
    }
}

/// Every runnable program in the directories of `path`.
pub fn scan(path: &OsStr) -> Vec<String> {
    let mut names = BTreeSet::new();
    for dir in std::env::split_paths(path) {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        names.extend(entries.flatten().filter_map(|entry| program(&entry)));
    }
    names.into_iter().collect()
}

#[cfg(unix)]
fn program(entry: &fs::DirEntry) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(entry.path()).ok()?;
    let runnable = meta.is_file() && meta.permissions().mode() & 0o111 != 0;
    runnable.then(|| entry.file_name().to_string_lossy().into_owned())
}

/// On Windows a program is a file with a PATHEXT extension, named without it,
/// the way PowerShell and cmd let you type it.
#[cfg(windows)]
fn program(entry: &fs::DirEntry) -> Option<String> {
    use std::sync::OnceLock;
    static EXTENSIONS: OnceLock<Vec<String>> = OnceLock::new();
    let extensions = EXTENSIONS.get_or_init(|| {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| ext.to_ascii_lowercase())
            .collect()
    });
    let name = entry.file_name().to_string_lossy().into_owned();
    let dot = name.rfind('.')?;
    let known = extensions.contains(&name[dot..].to_ascii_lowercase());
    (known && dot > 0 && entry.path().is_file()).then(|| name[..dot].to_string())
}
