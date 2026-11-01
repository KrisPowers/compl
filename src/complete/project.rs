//! Names a project defines for its tools: package.json scripts and Makefile
//! targets.

use std::fs;
use std::path::Path;

/// Scripts in the nearest package.json, for `npm run` and friends.
pub fn scripts(cwd: &Path) -> Vec<String> {
    for dir in cwd.ancestors() {
        let Ok(text) = fs::read_to_string(dir.join("package.json")) else {
            continue;
        };
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        return match json.get("scripts").and_then(|scripts| scripts.as_object()) {
            Some(scripts) => scripts.keys().cloned().collect(),
            None => Vec::new(),
        };
    }
    Vec::new()
}

/// Targets in the Makefile in `cwd`.
pub fn targets(cwd: &Path) -> Vec<String> {
    let Some(text) = ["GNUmakefile", "makefile", "Makefile"]
        .iter()
        .find_map(|name| fs::read_to_string(cwd.join(name)).ok())
    else {
        return Vec::new();
    };
    let mut names: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with([' ', '\t', '#', '.']))
        .filter_map(|line| {
            let (names, rest) = line.split_once(':')?;
            // `NAME := value` is an assignment, not a rule.
            (!rest.starts_with('=')).then_some(names)
        })
        .flat_map(str::split_whitespace)
        .filter(|name| {
            name.chars()
                .all(|c| c.is_alphanumeric() || "-_./".contains(c))
        })
        .map(String::from)
        .collect();
    names.sort();
    names.dedup();
    names
}
