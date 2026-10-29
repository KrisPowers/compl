//! Draws suggestions straight onto the terminal and puts back whatever they
//! covered, using the screen model as the record of what belongs there.

use std::io::Write as _;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use vt100::Screen;

use crate::complete::Item;
use crate::screen::screen_size;

/// The most list rows shown at once; the list scrolls past this.
const MAX_ROWS: usize = 8;
const MAX_LABEL: usize = 40;
const MAX_DESCRIPTION: usize = 50;

const ITEM: &str = "\x1b[0;38;5;252;48;5;236m";
const ITEM_DESCRIPTION: &str = "\x1b[0;38;5;245;48;5;236m";
const SELECTED: &str = "\x1b[0;1;38;5;231;48;5;25m";
const SELECTED_DESCRIPTION: &str = "\x1b[0;38;5;153;48;5;25m";

/// What to draw for the current command line: a list whose first row is
/// what was typed, followed by the suggestions.
pub struct View<'a> {
    pub typed: &'a str,
    pub items: &'a [Item],
    /// The highlighted row; 0 is what was typed.
    pub selected: usize,
    /// Width of the token on the line, so the list lines up under it.
    pub token_width: usize,
}

impl View<'_> {
    /// The label and description of list row `index`.
    fn row(&self, index: usize) -> (&str, Option<&str>) {
        match index {
            0 => (self.typed, None),
            n => {
                let item = &self.items[n - 1];
                (item.label.as_str(), item.description.as_deref())
            }
        }
    }
}

#[derive(Default)]
pub struct Overlay {
    /// Rows drawn over, to restore on the next clear.
    rows: Vec<u16>,
    /// Whether anything was drawn or restored since the last check.
    touched: bool,
}

impl Overlay {
    /// Redraws the rows the overlay covered from the screen model, and puts
    /// the cursor and colors back where the shell left them.
    pub fn clear(&mut self, screen: &Screen, out: &mut Vec<u8>) {
        if self.rows.is_empty() {
            return;
        }
        let formatted: Vec<String> = screen.rows_formatted(0, screen_size(screen).1).collect();
        self.rows.sort_unstable();
        self.rows.dedup();
        for &row in &self.rows {
            if let Some(content) = formatted.get(usize::from(row)) {
                let _ = write!(out, "\x1b[{};1H\x1b[0m\x1b[2K", row + 1);
                out.extend_from_slice(content.as_bytes());
            }
        }
        self.rows.clear();
        restore_cursor(screen, out);
        self.touched = true;
    }

    /// Draws the list below the cursor, or above it when there is no room.
    pub fn draw(&mut self, screen: &Screen, view: &View, out: &mut Vec<u8>) {
        let (rows, cols) = screen_size(screen);
        let (row, col) = screen.cursor_position();
        let total = view.items.len() + 1;
        let below = usize::from(rows.saturating_sub(row + 1));
        let wanted = total.min(MAX_ROWS);
        let (first, count) = if below >= wanted {
            (row + 1, wanted)
        } else if usize::from(row) >= wanted {
            (row - wanted as u16, wanted)
        } else {
            (row + 1, below)
        };
        if count == 0 {
            return;
        }
        let top = (view.selected + 1).saturating_sub(count);
        let cols = usize::from(cols);
        let label_width = (0..total)
            .map(|index| view.row(index).0.width())
            .max()
            .unwrap_or(0)
            .clamp(1, MAX_LABEL)
            .min(cols.saturating_sub(2));
        let description_width = (0..total)
            .filter_map(|index| view.row(index).1)
            .map(str::width)
            .max()
            .unwrap_or(0)
            .min(MAX_DESCRIPTION)
            .min(cols.saturating_sub(label_width + 3));
        let width = label_width + 2 + description_width + usize::from(description_width > 0);
        let left = usize::from(col)
            .saturating_sub(view.token_width)
            .min(cols.saturating_sub(width));
        out.extend_from_slice(b"\x1b[?25l");
        for index in top..top + count {
            let (style, description_style) = if index == view.selected {
                (SELECTED, SELECTED_DESCRIPTION)
            } else {
                (ITEM, ITEM_DESCRIPTION)
            };
            let (label, description) = view.row(index);
            let line = first + (index - top) as u16;
            let label = fit(label, label_width);
            let _ = write!(out, "\x1b[{};{}H{style} {label} ", line + 1, left + 1);
            if description_width > 0 {
                let description = fit(description.unwrap_or(""), description_width);
                let _ = write!(out, "{description_style}{description} ");
            }
            self.rows.push(line);
        }
        restore_cursor(screen, out);
        self.touched = true;
    }

    /// Whether the overlay wrote anything since the last call.
    pub fn take_touched(&mut self) -> bool {
        std::mem::take(&mut self.touched)
    }
}

fn restore_cursor(screen: &Screen, out: &mut Vec<u8>) {
    let (row, col) = screen.cursor_position();
    let _ = write!(out, "\x1b[0m\x1b[{};{}H", row + 1, col + 1);
    out.extend_from_slice(screen.attributes_formatted().as_bytes());
    if !screen.hide_cursor() {
        out.extend_from_slice(b"\x1b[?25h");
    }
}

/// Cuts or pads `text` to exactly `width` columns. Control characters are
/// shown as `?` so they cannot move the cursor.
fn fit(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let c = if c.is_control() { '?' } else { c };
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out.push_str(&" ".repeat(width - used));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_text_to_a_width() {
        assert_eq!(fit("Documents/", 4), "Docu");
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("a\nb", 3), "a?b");
        assert_eq!(fit("日本語", 5), "日本 ");
    }
}
