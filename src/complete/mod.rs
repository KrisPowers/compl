//! Works out the suggestions for a command line: command names in command
//! position, then subcommands, options and arguments from the built-in specs.
//! Paths are only suggested where a spec says a command takes them.

mod commands;
mod git;
mod parse;
mod paths;
mod project;
mod specs;

use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::shell::{Quoting, Shell};
use parse::{parse, quote, Token};
use specs::Args;

/// The most suggestions kept for one command line.
const MAX_ITEMS: usize = 200;

/// Words that run the command after them, so completion skips past them.
#[rustfmt::skip]
const PREFIXES: &[&str] = &[
    "sudo", "doas", "time", "nohup", "command", "builtin", "exec", "env", "nice", "then", "do",
    "else", "!",
];

/// One suggestion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// What the list shows.
    pub label: String,
    /// The token as it will be typed, already quoted for the shell.
    pub text: String,
    pub description: Option<String>,
}

impl Item {
    /// `open` leaves a quote unclosed, so more can be typed after a directory.
    fn new(
        label: String,
        value: &str,
        description: Option<String>,
        open: bool,
        token: &Token,
        quoting: Quoting,
    ) -> Item {
        Item {
            label,
            text: quote(value, token.quote, quoting, open),
            description,
        }
    }

    /// A suggestion that is typed as it is shown.
    fn word(value: &str, description: Option<String>, token: &Token, quoting: Quoting) -> Item {
        Item::new(value.to_string(), value, description, false, token, quoting)
    }
}

/// The suggestions for a command line, in order, and the token they replace.
#[derive(Debug, Default)]
pub struct Completion {
    /// The token under the cursor, exactly as typed.
    pub raw: String,
    pub items: Vec<Item>,
}

/// What the completer knows about the shell it serves.
pub struct Context {
    pub shell: Shell,
    pub cwd: PathBuf,
    path: Option<OsString>,
    commands: Option<Vec<String>>,
}

impl Context {
    pub fn new(shell: Shell, cwd: PathBuf) -> Context {
        Context {
            shell,
            cwd,
            path: std::env::var_os("PATH"),
            commands: None,
        }
    }

    /// Takes the PATH the shell reported, rescanning it only when it changed.
    pub fn set_path(&mut self, path: &str) {
        if self.path.as_deref() != Some(OsStr::new(path)) {
            self.path = Some(path.into());
            self.commands = None;
        }
    }

    fn commands(&mut self) -> &[String] {
        let path = self.path.clone().unwrap_or_default();
        self.commands.get_or_insert_with(|| commands::scan(&path))
    }
}

pub fn complete(line: &str, ctx: &mut Context) -> Completion {
    let parsed = parse(line, ctx.shell.quoting());
    let token = parsed.current;
    let skip = parsed
        .words
        .iter()
        .take_while(|word| PREFIXES.contains(&word.as_str()) || is_assignment(word))
        .count();
    let mut items = match parsed.words[skip..].split_first() {
        // Nothing is suggested on an empty line.
        None if token.raw.is_empty() => Vec::new(),
        // A command with a spec, typed in full, already lists what can
        // follow it, as if the space were typed.
        None if specs::find(&token.value).is_some() => {
            arguments(&token.value, &[], &Token::default(), ctx)
                .into_iter()
                .map(|item| Item {
                    text: format!("{} {}", token.raw, item.text),
                    ..item
                })
                .collect()
        }
        None => command_names(&token, ctx),
        Some((name, rest)) => arguments(name, rest, &token, ctx),
    };
    // What is already typed is not a suggestion.
    items.retain(|item| !item.text.eq_ignore_ascii_case(&token.raw));
    items.sort_by(|a, b| compare(&a.label, &b.label));
    items.dedup_by(|a, b| a.text == b.text);
    items.truncate(MAX_ITEMS);
    Completion {
        raw: token.raw,
        items,
    }
}

fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn command_names(token: &Token, ctx: &mut Context) -> Vec<Item> {
    if token.value.contains(['/', '\\']) || token.value.starts_with(['.', '~']) {
        return paths::list(token, ctx, false);
    }
    let quoting = ctx.shell.quoting();
    let prefix = token.value.as_str();
    let builtins = commands::builtins(ctx.shell);
    let mut names: Vec<String> = ctx
        .commands()
        .iter()
        .filter(|name| starts_with(name, prefix))
        .cloned()
        .collect();
    names.extend(
        builtins
            .iter()
            .filter(|name| starts_with(name, prefix))
            .map(|name| name.to_string()),
    );
    names
        .iter()
        .map(|name| Item::word(name, None, token, quoting))
        .collect()
}

fn arguments(name: &str, rest: &[String], token: &Token, ctx: &mut Context) -> Vec<Item> {
    let quoting = ctx.shell.quoting();
    let Some(spec) = specs::find(name) else {
        return Vec::new();
    };
    let mut node = spec;
    let mut positional = 0;
    for word in rest.iter().filter(|word| !word.starts_with('-')) {
        match node.subcommand(word) {
            Some(sub) if positional == 0 => node = sub,
            _ => positional += 1,
        }
    }
    if token.value.starts_with('-') {
        return node
            .options
            .iter()
            // Options are case-sensitive: -d and -D differ.
            .filter(|option| option.name.starts_with(token.value.as_str()))
            .map(|option| Item::word(&option.name, option.description.clone(), token, quoting))
            .collect();
    }
    let mut items = Vec::new();
    if positional == 0 {
        for sub in &node.subcommands {
            if starts_with(&sub.name, &token.value) {
                items.push(Item::word(&sub.name, sub.description.clone(), token, quoting));
            }
        }
    }
    let values = match node.args {
        Some(Args::Path) => return [items, paths::list(token, ctx, false)].concat(),
        Some(Args::Directory) => return [items, paths::list(token, ctx, true)].concat(),
        Some(Args::Branch) => git::branches(&ctx.cwd),
        Some(Args::Remote) => git::remotes(&ctx.cwd),
        Some(Args::Script) => project::scripts(&ctx.cwd),
        Some(Args::Target) => project::targets(&ctx.cwd),
        None => Vec::new(),
    };
    for value in values {
        if starts_with(&value, &token.value) {
            items.push(Item::word(&value, None, token, quoting));
        }
    }
    items
}

/// Case-insensitive prefix match.
fn starts_with(candidate: &str, prefix: &str) -> bool {
    let mut chars = candidate.chars();
    prefix.chars().all(|p| {
        chars
            .next()
            .is_some_and(|c| c == p || c.to_lowercase().eq(p.to_lowercase()))
    })
}

/// Alphabetical order, ignoring case, with digits and symbols ahead of
/// letters. A word sorts before longer words it starts.
pub fn compare(a: &str, b: &str) -> Ordering {
    let key = |c: char| (c.is_alphabetic(), c.to_lowercase().next().unwrap_or(c));
    let mut left = a.chars().map(key);
    let mut right = b.chars().map(key);
    loop {
        match (left.next(), right.next()) {
            (Some(x), Some(y)) if x != y => return x.cmp(&y),
            (Some(_), Some(_)) => {}
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return a.cmp(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn texts(line: &str, ctx: &mut Context) -> Vec<String> {
        complete(line, ctx).items.into_iter().map(|item| item.text).collect()
    }

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("complete-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in ["Documents", "Downloads", "My Music", ".config", "src/bin"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        for file in ["notes.txt", "src/main.rs"] {
            fs::write(dir.join(file), "").unwrap();
        }
        fs::write(dir.join("Makefile"), "build:\n\tcc\ntest lint: build\nVAR := 1\n").unwrap();
        fs::write(dir.join("package.json"), r#"{"scripts":{"dev":"x","deploy":"y"}}"#).unwrap();
        dir
    }

    #[test]
    fn orders_symbols_and_digits_first() {
        let mut words = vec!["b", "a", "_x", "1", "B", "ab", "a1"];
        words.sort_by(|a, b| compare(a, b));
        assert_eq!(words, ["1", "_x", "a", "a1", "ab", "B", "b"]);
    }

    #[test]
    fn matches_prefixes_ignoring_case() {
        assert!(starts_with("Documents", "doc"));
        assert!(starts_with("git", ""));
        assert!(!starts_with("go", "git"));
    }

    #[test]
    fn cd_lists_only_directories() {
        let dir = sandbox("cd");
        let mut ctx = Context::new(Shell::Bash, dir.clone());
        assert_eq!(
            texts("cd ", &mut ctx),
            ["Documents/", "Downloads/", "My\\ Music/", "src/"]
        );
        assert_eq!(texts("cd do", &mut ctx), ["Documents/", "Downloads/"]);
        assert_eq!(texts("cd .c", &mut ctx), [".config/"]);
        assert_eq!(texts("cd src/", &mut ctx), ["src/bin/"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quotes_paths_for_each_shell() {
        let dir = sandbox("quote");
        let mut ctx = Context::new(Shell::Pwsh, dir.clone());
        let sep = Shell::Pwsh.separator();
        assert_eq!(texts("cd my", &mut ctx), [format!("'My Music{sep}")]);
        let mut ctx = Context::new(Shell::Bash, dir.clone());
        assert_eq!(texts("cd 'my", &mut ctx), ["'My Music/"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn paths_only_where_a_spec_takes_them() {
        let dir = sandbox("files");
        let mut ctx = Context::new(Shell::Zsh, dir.clone());
        assert!(texts("frobnicate ", &mut ctx).is_empty());
        assert!(texts("frobnicate no", &mut ctx).is_empty());
        assert_eq!(texts("cat src/m", &mut ctx), ["src/main.rs"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completes_subcommands_and_options() {
        let mut ctx = Context::new(Shell::Bash, std::env::temp_dir());
        assert_eq!(texts("git chec", &mut ctx), ["checkout"]);
        assert!(texts("git ", &mut ctx).contains(&"commit".to_string()));
        assert!(texts("git commit --am", &mut ctx).contains(&"--amend".to_string()));
        assert_eq!(texts("sudo git sta", &mut ctx), ["stash", "status"]);
        assert_eq!(texts("cargo b", &mut ctx), ["bench", "build"]);
    }

    #[test]
    fn completes_scripts_and_targets() {
        let dir = sandbox("project");
        let mut ctx = Context::new(Shell::Bash, dir.clone());
        assert_eq!(texts("npm run d", &mut ctx), ["deploy", "dev"]);
        assert_eq!(texts("make ", &mut ctx), ["build", "lint", "test"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn nothing_on_an_empty_line() {
        let mut ctx = Context::new(Shell::Bash, std::env::temp_dir());
        assert!(complete("", &mut ctx).items.is_empty());
        assert!(complete("ls | ", &mut ctx).items.is_empty());
    }

    #[test]
    fn leaves_out_what_is_already_typed() {
        let mut ctx = Context::new(Shell::Bash, std::env::temp_dir());
        assert!(texts("echo", &mut ctx).is_empty());
        assert!(!texts("git status", &mut ctx).contains(&"status".to_string()));
    }

    #[test]
    fn a_full_command_lists_what_follows_it() {
        let dir = sandbox("full");
        let mut ctx = Context::new(Shell::Bash, dir.clone());
        let expected = ["cd Documents/", "cd Downloads/", "cd My\\ Music/", "cd src/"];
        assert_eq!(texts("cd", &mut ctx), expected);
        assert!(texts("git", &mut ctx).contains(&"git commit".to_string()));
        fs::remove_dir_all(dir).unwrap();
    }
}
