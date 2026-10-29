//! Splits the shell's output into the bytes meant for the terminal and the
//! markers the shell integration prints: `ESC ] 6973 ; <kind> [; payload]`,
//! ended by BEL or `ESC \`. Markers never reach the terminal.

const MARK: &[u8] = b"\x1b]6973;";

/// ConPTY asks the terminal to switch to win32-input-mode, which would turn
/// every key into a sequence only ConPTY understands. Dropping the request
/// keeps keys readable, and ConPTY accepts plain VT input just as well.
const DROPPED: [&[u8]; 2] = [b"\x1b[?9001h", b"\x1b[?9001l"];

/// A marker longer than this is not one of ours, so it is cut short.
const MAX_MARK: usize = 1 << 16;

#[derive(Debug, PartialEq, Eq)]
pub enum Chunk {
    /// Bytes to pass on to the terminal.
    Output(Vec<u8>),
    /// A prompt is about to be drawn in this directory.
    Prompt(String),
    /// The PATH the shell finds commands on.
    Path(String),
    /// The prompt is drawn; the cursor sits where input starts.
    Input,
}

#[derive(Default)]
pub struct Scanner {
    /// Bytes that may be the start of a marker or dropped sequence.
    held: Vec<u8>,
    /// The body of the marker being read, once its start has been seen.
    mark: Option<Vec<u8>>,
}

impl Scanner {
    pub fn feed(&mut self, input: &[u8]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut output = Vec::new();
        for &byte in input {
            if let Some(mut mark) = self.mark.take() {
                let ended = match byte {
                    0x07 => true,
                    b'\\' if mark.last() == Some(&0x1b) => {
                        mark.pop();
                        true
                    }
                    _ => {
                        if mark.len() < MAX_MARK {
                            mark.push(byte);
                        }
                        false
                    }
                };
                if ended {
                    if !output.is_empty() {
                        chunks.push(Chunk::Output(std::mem::take(&mut output)));
                    }
                    chunks.extend(parse(&mark));
                } else {
                    self.mark = Some(mark);
                }
                continue;
            }
            if self.held.is_empty() && byte != 0x1b {
                output.push(byte);
                continue;
            }
            self.held.push(byte);
            if self.held == MARK {
                self.held.clear();
                self.mark = Some(Vec::new());
            } else if DROPPED.contains(&self.held.as_slice()) {
                self.held.clear();
            } else if !MARK.starts_with(&self.held)
                && !DROPPED.iter().any(|seq| seq.starts_with(&self.held))
            {
                // Only the first held byte can be ESC, so only this one
                // can begin a new sequence.
                self.held.pop();
                output.append(&mut self.held);
                if byte == 0x1b {
                    self.held.push(byte);
                } else {
                    output.push(byte);
                }
            }
        }
        if !output.is_empty() {
            chunks.push(Chunk::Output(output));
        }
        chunks
    }

    /// Hands back bytes held while waiting to see whether they start a
    /// marker, for when no more output is coming for now.
    pub fn flush(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.held)
    }
}

fn parse(mark: &[u8]) -> Option<Chunk> {
    let text = String::from_utf8_lossy(mark).into_owned();
    let (kind, payload) = text.split_once(';').unwrap_or((text.as_str(), ""));
    Some(match kind {
        "A" => Chunk::Prompt(payload.to_string()),
        "P" => Chunk::Path(payload.to_string()),
        "B" => Chunk::Input,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_all(parts: &[&[u8]]) -> Vec<Chunk> {
        let mut scanner = Scanner::default();
        let mut chunks: Vec<Chunk> = Vec::new();
        for part in parts {
            for chunk in scanner.feed(part) {
                match (chunks.last_mut(), chunk) {
                    (Some(Chunk::Output(last)), Chunk::Output(more)) => last.extend(more),
                    (_, chunk) => chunks.push(chunk),
                }
            }
        }
        chunks
    }

    #[test]
    fn separates_markers_from_output() {
        let chunks = feed_all(&[b"hi\x1b]6973;A;/tmp\x07$ \x1b]6973;B\x1b\\ls"]);
        assert_eq!(
            chunks,
            [
                Chunk::Output(b"hi".to_vec()),
                Chunk::Prompt("/tmp".into()),
                Chunk::Output(b"$ ".to_vec()),
                Chunk::Input,
                Chunk::Output(b"ls".to_vec()),
            ]
        );
    }

    #[test]
    fn markers_split_across_reads() {
        let chunks = feed_all(&[b"a\x1b]69", b"73;P;/bin:/usr/b", b"in\x07b"]);
        assert_eq!(
            chunks,
            [
                Chunk::Output(b"a".to_vec()),
                Chunk::Path("/bin:/usr/bin".into()),
                Chunk::Output(b"b".to_vec()),
            ]
        );
    }

    #[test]
    fn other_sequences_pass_through() {
        let input = b"\x1b[31mred\x1b]0;title\x07\x1b\x1b[0m";
        assert_eq!(feed_all(&[input]), [Chunk::Output(input.to_vec())]);
    }

    #[test]
    fn drops_win32_input_mode() {
        let chunks = feed_all(&[b"x\x1b[?90", b"01hy\x1b[?9001l"]);
        assert_eq!(chunks, [Chunk::Output(b"xy".to_vec())]);
    }
}
