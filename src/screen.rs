//! Reads the command line back off the screen model.
//!
//! Every shell draws its input line differently, but all of them draw it on
//! screen. The prompt marker records where input starts; from then on the
//! input is whatever sits between that spot and the cursor.

use std::num::NonZeroU16;

use vt100::Screen;

/// Where the shell's input starts, captured when the prompt finished drawing.
pub struct Prompt {
    /// Cells from the start of the logical (unwrapped) line to the input.
    offset: usize,
    /// The prompt text on that line, to tell the line is still ours.
    prefix: String,
    /// The column a right-hand prompt starts at, once checked.
    right: Option<Option<u16>>,
}

impl Prompt {
    pub fn capture(screen: &Screen) -> Prompt {
        let (row, col) = screen.cursor_position();
        let start = line_start(screen, row);
        let offset = cell_index(screen, start, row, col);
        Prompt {
            offset,
            prefix: text(screen, start, 0, offset),
            right: None,
        }
    }

    /// The input typed so far, or `None` when the cursor is not at the end of
    /// the input on the prompt line (a program is running, the cursor moved
    /// left, or the screen changed under us).
    pub fn read(&mut self, screen: &Screen) -> Option<String> {
        if screen.alternate_screen() {
            return None;
        }
        let (row, col) = screen.cursor_position();
        let start = line_start(screen, row);
        let end = cell_index(screen, start, row, col);
        if end < self.offset || text(screen, start, 0, self.offset) != self.prefix {
            return None;
        }
        let input = text(screen, start, self.offset, end);
        // Text right of the cursor is more input, unless it is a right-hand
        // prompt, which shows up on the empty line right after the prompt.
        let right = *self.right.get_or_insert_with(|| {
            if end == self.offset {
                first_filled(screen, row, col)
            } else {
                None
            }
        });
        if screen.row_wrapped(row) {
            return None;
        }
        if let Some(filled) = first_filled(screen, row, col) {
            if !matches!(right, Some(right) if filled >= right) {
                return None;
            }
        }
        Some(input)
    }
}

/// The screen's size as plain (rows, columns).
pub fn screen_size(screen: &Screen) -> (u16, u16) {
    let (rows, cols) = screen.size();
    (rows.get(), cols.get())
}

/// A screen dimension for vt100, which needs at least one row and column.
pub fn nonzero(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n).unwrap_or(NonZeroU16::MIN)
}

/// The first row of the logical line `row` belongs to.
fn line_start(screen: &Screen, row: u16) -> u16 {
    let mut start = row;
    while start > 0 && screen.row_wrapped(start - 1) {
        start -= 1;
    }
    start
}

fn cell_index(screen: &Screen, start: u16, row: u16, col: u16) -> usize {
    usize::from(row - start) * usize::from(screen_size(screen).1) + usize::from(col)
}

/// The text of cells `from..to` of the logical line starting at row `start`.
fn text(screen: &Screen, start: u16, from: usize, to: usize) -> String {
    let cols = usize::from(screen_size(screen).1);
    let mut out = String::new();
    for index in from..to {
        let row = usize::from(start) + index / cols;
        let col = index % cols;
        match screen.cell(row as u16, col as u16) {
            Some(cell) if cell.is_wide_continuation() => {}
            Some(cell) if cell.has_contents() => out.push_str(cell.contents()),
            _ => out.push(' '),
        }
    }
    out
}

/// The first column at or after `col` on `row` holding visible text.
fn first_filled(screen: &Screen, row: u16, col: u16) -> Option<u16> {
    (col..screen_size(screen).1).find(|&col| {
        screen
            .cell(row, col)
            .is_some_and(|cell| !cell.contents().trim().is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(bytes: &[u8]) -> vt100::Parser {
        let mut parser = vt100::Parser::new(nonzero(10), nonzero(20), 0);
        parser.process(bytes);
        parser
    }

    #[test]
    fn reads_input_after_the_prompt() {
        let mut parser = screen(b"$ ");
        let mut prompt = Prompt::capture(parser.screen());
        assert_eq!(prompt.read(parser.screen()).as_deref(), Some(""));
        parser.process(b"cd Doc");
        assert_eq!(prompt.read(parser.screen()).as_deref(), Some("cd Doc"));
    }

    #[test]
    fn follows_input_that_wraps() {
        let mut parser = screen(b"prompt> ");
        let mut prompt = Prompt::capture(parser.screen());
        parser.process(b"echo aaaaaaaaaaaaaaaaaaaa");
        let input = prompt.read(parser.screen());
        assert_eq!(input.as_deref(), Some("echo aaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn nothing_when_the_cursor_is_not_at_the_end() {
        let mut parser = screen(b"$ ");
        let mut prompt = Prompt::capture(parser.screen());
        prompt.read(parser.screen());
        parser.process(b"ls foo\x1b[3D");
        assert_eq!(prompt.read(parser.screen()), None);
    }

    #[test]
    fn ignores_a_right_prompt() {
        let mut parser = screen(b"$ \x1b[1;16H12:00\x1b[1;3H");
        let mut prompt = Prompt::capture(parser.screen());
        assert_eq!(prompt.read(parser.screen()).as_deref(), Some(""));
        parser.process(b"ls");
        assert_eq!(prompt.read(parser.screen()).as_deref(), Some("ls"));
    }

    #[test]
    fn nothing_once_the_prompt_scrolls_away() {
        let mut parser = screen(b"$ ");
        let mut prompt = Prompt::capture(parser.screen());
        parser.process(b"\r\nother output");
        assert_eq!(prompt.read(parser.screen()), None);
    }
}
