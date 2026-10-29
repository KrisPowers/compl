//! File and directory suggestions.

use std::fs;
use std::path::{Path, PathBuf};

use super::{starts_with, Context, Item, Token};
use crate::shell::home;

/// Entries read from one directory at most, so huge directories stay fast.
const MAX_ENTRIES: usize = 5000;

const SEPARATORS: &[char] = if cfg!(windows) { &['/', '\\'] } else { &['/'] };

/// Entries of the directory the token points into whose names start with the
/// rest of the token. Hidden entries show once the name starts with a dot.
pub fn list(token: &Token, ctx: &Context, dirs_only: bool) -> Vec<Item> {
    let value = token.value.as_str();
    let split = value.rfind(SEPARATORS).map_or(0, |i| i + 1);
    let (dir, prefix) = value.split_at(split);
    let Ok(entries) = fs::read_dir(resolve(dir, &ctx.cwd)) else {
        return Vec::new();
    };
    // Keep whichever separator the user typed.
    let sep = dir.chars().next_back().unwrap_or(ctx.shell.separator());
    let quoting = ctx.shell.quoting();
    let mut items = Vec::new();
    for entry in entries.take(MAX_ENTRIES).flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.chars().any(char::is_control)
            || (name.starts_with('.') && !prefix.starts_with('.'))
            || !starts_with(&name, prefix)
        {
            continue;
        }
        let is_dir = entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()));
        if dirs_only && !is_dir {
            continue;
        }
        let label = if is_dir { format!("{name}{sep}") } else { name };
        let value = format!("{dir}{label}");
        items.push(Item::new(label, &value, None, is_dir, token, quoting));
    }
    items
}

/// The directory a typed directory part means, with `~` as the home folder.
fn resolve(dir: &str, cwd: &Path) -> PathBuf {
    if let Some(rest) = dir.strip_prefix('~') {
        if rest.is_empty() || rest.starts_with(SEPARATORS) {
            if let Some(home) = home() {
                return home.join(rest.trim_start_matches(SEPARATORS));
            }
        }
    }
    cwd.join(dir)
}
