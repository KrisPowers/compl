//! Splits a command line into words the way the shell would, and quotes
//! suggestions so the shell reads them back unchanged.

use crate::shell::Quoting;

/// Characters POSIX shells treat specially in an unquoted word.
const POSIX_SPECIAL: &str = " \t'\"\\$`&|;<>()*?[]{}!#";
/// Characters that make PowerShell need a quoted word.
#[rustfmt::skip]
const PWSH_SPECIAL: &[char] = &[
    ' ', '\t', '\'', '"', '`', '$', ';', '&', '|', '(', ')', '{', '}', '@', '#', ',', '<', '>',
];
/// Characters that make cmd need a quoted word.
#[rustfmt::skip]
const CMD_SPECIAL: &[char] = &[
    ' ', '&', '(', ')', '[', ']', '{', '}', '^', '=', ';', '!', '\'', '+', ',', '`', '~',
];

/// A word on the command line, as typed and as the shell reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Token {
    pub raw: String,
    pub value: String,
    /// The quote the word opened with, kept when it is completed.
    pub quote: Option<char>,
}

#[derive(Debug, Default)]
pub struct Parsed {
    /// Finished words of the command being typed, the one after the last
    /// `|`, `;` or `&`.
    pub words: Vec<String>,
    /// The word at the cursor; empty when the line ends in a space.
    pub current: Token,
}

pub fn parse(line: &str, quoting: Quoting) -> Parsed {
    let escape = match quoting {
        Quoting::Posix => '\\',
        Quoting::Pwsh => '`',
        Quoting::Cmd => '^',
    };
    let mut parsed = Parsed::default();
    let mut token = Token::default();
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(open) = quote {
            token.raw.push(c);
            if c == open {
                // PowerShell writes a quote inside single quotes as ''.
                if quoting == Quoting::Pwsh && open == '\'' && chars.peek() == Some(&'\'') {
                    chars.next();
                    token.raw.push('\'');
                    token.value.push('\'');
                } else {
                    quote = None;
                }
            } else if open == '"' && c == escape && quoting != Quoting::Cmd {
                if let Some(next) = chars.next() {
                    token.raw.push(next);
                    token.value.push(next);
                }
            } else {
                token.value.push(c);
            }
            continue;
        }
        match c {
            c if c.is_whitespace() => finish(&mut parsed, &mut token),
            '|' | ';' | '&' => {
                finish(&mut parsed, &mut token);
                parsed.words.clear();
            }
            '"' | '\'' if c == '"' || quoting != Quoting::Cmd => {
                if token.raw.is_empty() {
                    token.quote = Some(c);
                }
                token.raw.push(c);
                quote = Some(c);
            }
            c if c == escape => {
                token.raw.push(c);
                if let Some(next) = chars.next() {
                    token.raw.push(next);
                    token.value.push(next);
                }
            }
            c => {
                token.raw.push(c);
                token.value.push(c);
            }
        }
    }
    parsed.current = token;
    parsed
}

fn finish(parsed: &mut Parsed, token: &mut Token) {
    if !token.raw.is_empty() {
        parsed.words.push(std::mem::take(token).value);
    }
}

/// Writes `value` as a word the shell reads back as `value`. `quote` keeps
/// the quote the user started with; `open` leaves the quote unclosed so more
/// can be typed after a directory.
pub fn quote(value: &str, quote: Option<char>, quoting: Quoting, open: bool) -> String {
    let (mark, body) = match (quoting, quote) {
        (Quoting::Posix, None) => return escape_with(value, POSIX_SPECIAL, '\\'),
        (Quoting::Posix, Some('\'')) => ('\'', value.replace('\'', "'\\''")),
        (Quoting::Posix, Some(_)) => ('"', escape_with(value, "\\\"$`", '\\')),
        (Quoting::Pwsh, None) if !value.contains(PWSH_SPECIAL) => return value.to_string(),
        (Quoting::Pwsh, Some('"')) => ('"', escape_with(value, "`\"$", '`')),
        (Quoting::Pwsh, _) => ('\'', value.replace('\'', "''")),
        (Quoting::Cmd, None) if !value.contains(CMD_SPECIAL) => return value.to_string(),
        (Quoting::Cmd, _) => ('"', value.to_string()),
    };
    let mut out = String::with_capacity(body.len() + 2);
    out.push(mark);
    out.push_str(&body);
    if !open {
        out.push(mark);
    }
    out
}

fn escape_with(value: &str, special: &str, escape: char) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if special.contains(c) {
            out.push(escape);
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The finished words followed by the word at the cursor.
    fn words(line: &str, quoting: Quoting) -> Vec<String> {
        let mut parsed = parse(line, quoting);
        parsed.words.push(parsed.current.value);
        parsed.words
    }

    fn current(line: &str, quoting: Quoting) -> String {
        parse(line, quoting).current.value
    }

    #[test]
    fn splits_words() {
        assert_eq!(words("git commit -m", Quoting::Posix), ["git", "commit", "-m"]);
        assert_eq!(words("cd ", Quoting::Posix), ["cd", ""]);
        assert_eq!(words("ls | gr", Quoting::Posix), ["gr"]);
    }

    #[test]
    fn reads_quotes_and_escapes() {
        assert_eq!(current(r"cd My\ Mus", Quoting::Posix), "My Mus");
        assert_eq!(current("cd 'My Mus", Quoting::Posix), "My Mus");
        assert_eq!(current(r#"cd "a\"b"#, Quoting::Posix), "a\"b");
        assert_eq!(current(r"cd D:\Work\src", Quoting::Pwsh), r"D:\Work\src");
        assert_eq!(current("cd 'it''s", Quoting::Pwsh), "it's");
        assert_eq!(parse("cd 'My", Quoting::Posix).current.quote, Some('\''));
    }

    #[test]
    fn quotes_values() {
        assert_eq!(quote("My Music/", None, Quoting::Posix, true), r"My\ Music/");
        assert_eq!(quote("My Music/", Some('\''), Quoting::Posix, true), "'My Music/");
        assert_eq!(quote("it's", Some('\''), Quoting::Posix, false), r"'it'\''s'");
        assert_eq!(quote("My Music", None, Quoting::Pwsh, false), "'My Music'");
        assert_eq!(quote("plain", None, Quoting::Pwsh, false), "plain");
        assert_eq!(quote("a b", None, Quoting::Cmd, false), "\"a b\"");
    }

    #[test]
    fn quoted_values_parse_back() {
        for value in ["My Music", "it's", "a$b", "x&y"] {
            for quoting in [Quoting::Posix, Quoting::Pwsh, Quoting::Cmd] {
                let line = format!("cat {}", quote(value, None, quoting, false));
                assert_eq!(parse(&line, quoting).current.value, value, "{line}");
            }
        }
    }
}
