//! Built-in completion specs: each command's subcommands, options and the
//! kind of argument it takes. The specs live as JSON in `specs/`.

use std::sync::OnceLock;

use serde::Deserialize;

const SOURCES: &[&str] = &[
    include_str!("../../specs/cargo.json"),
    include_str!("../../specs/docker.json"),
    include_str!("../../specs/files.json"),
    include_str!("../../specs/gh.json"),
    include_str!("../../specs/git.json"),
    include_str!("../../specs/go.json"),
    include_str!("../../specs/kubectl.json"),
    include_str!("../../specs/make.json"),
    include_str!("../../specs/node.json"),
    include_str!("../../specs/packages.json"),
    include_str!("../../specs/python.json"),
    include_str!("../../specs/systemctl.json"),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub args: Option<Args>,
    #[serde(default)]
    pub subcommands: Vec<Spec>,
    #[serde(default)]
    pub options: Vec<Opt>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opt {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// What a command's arguments are.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Args {
    Path,
    Directory,
    Branch,
    Remote,
    Script,
    Target,
}

impl Spec {
    fn is(&self, name: &str) -> bool {
        std::iter::once(&self.name)
            .chain(&self.aliases)
            .any(|known| known.eq_ignore_ascii_case(name))
    }

    pub fn subcommand(&self, word: &str) -> Option<&Spec> {
        self.subcommands.iter().find(|sub| sub.is(word))
    }
}

fn all() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        SOURCES
            .iter()
            .flat_map(|source| {
                serde_json::from_str::<Vec<Spec>>(source).expect("built-in specs are valid")
            })
            .collect()
    })
}

/// The spec for a command, found by name whether it was typed as `git`,
/// `/usr/bin/git` or `git.exe`.
pub fn find(command: &str) -> Option<&'static Spec> {
    let base = command.rsplit(['/', '\\']).next().unwrap_or(command);
    let lower = base.to_ascii_lowercase();
    let name = [".exe", ".cmd", ".bat", ".com"]
        .iter()
        .find_map(|ext| lower.strip_suffix(ext))
        .unwrap_or(lower.as_str());
    all().iter().find(|spec| spec.is(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_parse() {
        assert!(all().len() > 20);
    }

    #[test]
    fn finds_specs_by_any_spelling() {
        assert_eq!(find("git").map(|spec| spec.name.as_str()), Some("git"));
        assert_eq!(
            find("/usr/bin/git").map(|spec| spec.name.as_str()),
            Some("git")
        );
        assert_eq!(find("Git.exe").map(|spec| spec.name.as_str()), Some("git"));
        assert_eq!(
            find("sl").map(|spec| spec.name.as_str()),
            Some("Set-Location")
        );
        assert!(find("no-such-command").is_none());
    }
}
