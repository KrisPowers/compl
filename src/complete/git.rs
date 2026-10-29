//! Branch and remote names, read straight from the repository's files.

use std::fs;
use std::path::{Path, PathBuf};

/// Local branches, then remote-tracking ones as `remote/branch`.
pub fn branches(cwd: &Path) -> Vec<String> {
    let Some(dir) = git_dir(cwd) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut remotes = Vec::new();
    walk(&dir.join("refs").join("heads"), "", &mut names);
    walk(&dir.join("refs").join("remotes"), "", &mut remotes);
    if let Ok(packed) = fs::read_to_string(dir.join("packed-refs")) {
        for line in packed.lines() {
            let Some((_, name)) = line.split_once(' ') else {
                continue;
            };
            if let Some(branch) = name.strip_prefix("refs/heads/") {
                names.push(branch.to_string());
            } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
                remotes.push(remote.to_string());
            }
        }
    }
    names.extend(remotes.into_iter().filter(|name| !name.ends_with("/HEAD")));
    names.sort();
    names.dedup();
    names
}

pub fn remotes(cwd: &Path) -> Vec<String> {
    let Some(dir) = git_dir(cwd) else {
        return Vec::new();
    };
    let config = fs::read_to_string(dir.join("config")).unwrap_or_default();
    config
        .lines()
        .filter_map(|line| line.trim().strip_prefix("[remote \"")?.strip_suffix("\"]"))
        .map(String::from)
        .collect()
}

/// The repository's shared git directory, following the `.git` file that
/// worktrees and submodules use.
fn git_dir(cwd: &Path) -> Option<PathBuf> {
    for dir in cwd.ancestors() {
        let dot = dir.join(".git");
        if dot.is_dir() {
            return Some(common(dot));
        }
        if let Ok(text) = fs::read_to_string(&dot) {
            let link = text.strip_prefix("gitdir:")?.trim();
            return Some(common(dir.join(link)));
        }
    }
    None
}

fn common(dir: PathBuf) -> PathBuf {
    match fs::read_to_string(dir.join("commondir")) {
        Ok(relative) => dir.join(relative.trim()),
        Err(_) => dir,
    }
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        let path = entry.path();
        if path.is_dir() {
            walk(&path, &format!("{name}/"), out);
        } else {
            out.push(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_branches_and_remotes() {
        let repo = std::env::temp_dir().join(format!("git-{}", std::process::id()));
        let git = repo.join(".git");
        fs::create_dir_all(git.join("refs/heads/feature")).unwrap();
        fs::create_dir_all(git.join("refs/remotes/origin")).unwrap();
        fs::write(git.join("refs/heads/main"), "").unwrap();
        fs::write(git.join("refs/heads/feature/login"), "").unwrap();
        fs::write(git.join("refs/remotes/origin/HEAD"), "").unwrap();
        let packed = "abc refs/heads/old\nabc refs/remotes/origin/main\n";
        fs::write(git.join("packed-refs"), packed).unwrap();
        fs::write(git.join("config"), "[core]\n[remote \"origin\"]\n\turl = x\n").unwrap();
        let nested = repo.join("src");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(branches(&nested), ["feature/login", "main", "old", "origin/main"]);
        assert_eq!(remotes(&nested), ["origin"]);
        fs::remove_dir_all(repo).unwrap();
    }
}
