//! The session: runs the shell in a pseudo terminal, passes every byte
//! through, keeps a model of the screen, and draws suggestions over the
//! prompt while you type.
//!
//! Suggestions show as a list under the cursor. Its first row is what you
//! typed; the rows after it are the suggestions.
//! - Tab / Shift+Tab: move down or up the list. The highlighted suggestion
//!   is put on the command line as you go; the first row puts back what you
//!   typed.
//! - Up / Down: move through the list once you have tabbed into it.
//! - Enter: runs the command line as it stands, as usual.
//! - Esc: put back what you typed and hide the list until you type again.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use portable_pty::{native_pty_system, MasterPty, PtySize};
use unicode_width::UnicodeWidthStr;

use crate::complete::{self, Completion, Context};
use crate::log::Log;
use crate::markers::{Chunk, Scanner};
use crate::overlay::{Overlay, View};
use crate::screen::{nonzero, Prompt};
use crate::shell::{self, Shell};
use crate::terminal::{self, RawMode};

/// How long to wait for the shell to echo typed keys before acting on Tab,
/// so the list acted on matches what is on screen.
const ECHO_WAIT: Duration = Duration::from_millis(50);
/// How long to wait for the terminal to report the cursor position.
const CURSOR_WAIT: Duration = Duration::from_secs(1);
/// How often to check for a resized terminal when nothing else happens.
const TICK: Duration = Duration::from_millis(50);

enum Event {
    Input(Vec<u8>),
    Output(Vec<u8>),
    Exit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Tab,
    BackTab,
    Up,
    Down,
    Escape,
}

impl Key {
    /// A key press arrives as one read, so a read holding exactly one of
    /// these sequences is that key. Pastes and fast typing pass through.
    fn parse(bytes: &[u8]) -> Option<Key> {
        Some(match bytes {
            b"\t" => Key::Tab,
            b"\x1b[Z" => Key::BackTab,
            b"\x1b[A" | b"\x1bOA" => Key::Up,
            b"\x1b[B" | b"\x1bOB" => Key::Down,
            b"\x1b" => Key::Escape,
            _ => return None,
        })
    }
}

/// The suggestions for the current command line, shown as a list whose
/// first row is what was typed.
#[derive(Default)]
struct Menu {
    /// The command line as typed when the suggestions were made; `None` when
    /// the cursor is not at the end of a command line.
    line: Option<String>,
    completion: Completion,
    /// The highlighted row: 0 is what was typed, `n` is `items[n - 1]`.
    selected: usize,
    /// A row was typed into the shell and its echo has not shown yet.
    pending: bool,
    /// Hidden after Esc, until more input.
    suppressed: bool,
}

impl Menu {
    fn has_items(&self) -> bool {
        self.line.is_some() && !self.completion.items.is_empty()
    }

    fn visible(&self) -> bool {
        self.has_items() && !self.suppressed
    }

    /// The token the highlighted row puts on the command line.
    fn shown(&self) -> &str {
        match self.selected {
            0 => &self.completion.raw,
            n => &self.completion.items[n - 1].text,
        }
    }

    /// The whole command line the highlighted row puts on screen.
    fn expected(&self) -> Option<String> {
        let line = self.line.as_deref()?;
        let base = line.strip_suffix(self.completion.raw.as_str())?;
        Some(format!("{base}{}", self.shown()))
    }
}

pub fn run(shell: Shell, exe: Option<PathBuf>, login: bool) -> Result<()> {
    let (rows, cols) = terminal::size()?;
    let pty = native_pty_system()
        .openpty(pty_size(rows, cols))
        .context("opening a pseudo terminal")?;
    let command = shell::command(shell, exe, login)?;
    let _raw = RawMode::enable()?;
    // Ask where the cursor is, so the screen model starts where the
    // terminal really is. The answer arrives as input.
    let mut stdout = std::io::stdout();
    stdout.write_all(b"\x1b[6n")?;
    stdout.flush()?;
    let mut child = pty
        .slave
        .spawn_command(command)
        .with_context(|| format!("starting {}", shell.name()))?;
    drop(pty.slave);
    let reader = pty.master.try_clone_reader()?;
    let writer = pty.master.take_writer()?;

    let mut log = Log::open();
    log.line(format_args!("started {} in a {cols}x{rows} terminal", shell.name()));

    let (tx, rx) = mpsc::channel();
    read_stdin(tx.clone());
    read_output(reader, tx.clone());
    thread::spawn(move || {
        let _ = child.wait();
        let _ = tx.send(Event::Exit);
    });

    Session {
        master: pty.master,
        pty: writer,
        rx,
        backlog: VecDeque::new(),
        parser: vt100::Parser::new(nonzero(rows), nonzero(cols), 0),
        scanner: Scanner::default(),
        ctx: Context::new(shell, std::env::current_dir()?),
        prompt: None,
        menu: Menu::default(),
        overlay: Overlay::default(),
        size: (rows, cols),
        cursor_query: Some(Instant::now()),
        stale: false,
        redraw: false,
        out: Vec::new(),
        log,
    }
    .run()
}

fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn read_stdin(tx: Sender<Event>) {
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                // The Windows console reports Ctrl+Z as an empty read.
                Ok(0) if cfg!(windows) => thread::sleep(Duration::from_millis(10)),
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(Event::Input(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

fn read_output(mut reader: Box<dyn Read + Send>, tx: Sender<Event>) {
    thread::spawn(move || {
        let mut buf = [0u8; 16384];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 || tx.send(Event::Output(buf[..n].to_vec())).is_err() {
                break;
            }
        }
        let _ = tx.send(Event::Exit);
    });
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    pty: Box<dyn Write + Send>,
    rx: Receiver<Event>,
    /// Events set aside while waiting for the shell's echo.
    backlog: VecDeque<Event>,
    parser: vt100::Parser,
    scanner: Scanner,
    ctx: Context,
    prompt: Option<Prompt>,
    menu: Menu,
    overlay: Overlay,
    size: (u16, u16),
    /// When the cursor position was asked for, until the answer arrives.
    cursor_query: Option<Instant>,
    /// Input went to the shell and its echo has not come back yet.
    stale: bool,
    redraw: bool,
    /// Bytes for the terminal, written once per loop.
    out: Vec<u8>,
    log: Log,
}

impl Session {
    fn run(mut self) -> Result<()> {
        loop {
            let event = match self.backlog.pop_front() {
                Some(event) => Ok(event),
                None => self.rx.recv_timeout(TICK),
            };
            match event {
                Ok(Event::Output(bytes)) => self.output(&bytes),
                Ok(Event::Input(bytes)) => self.input(&bytes)?,
                Ok(Event::Exit) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => self.idle(),
            }
            // Draw only once everything already waiting is handled.
            if self.backlog.is_empty() {
                if let Ok(event) = self.rx.try_recv() {
                    self.backlog.push_back(event);
                }
            }
            if self.backlog.is_empty() {
                self.check_size()?;
                if self.redraw {
                    self.draw();
                }
            }
            self.flush();
        }
        // Show what the shell printed on its way out.
        let deadline = Instant::now() + Duration::from_millis(200);
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.rx.recv_timeout(left) {
                Ok(Event::Output(bytes)) => self.output(&bytes),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        self.overlay.clear(self.parser.screen(), &mut self.out);
        self.flush();
        Ok(())
    }

    fn output(&mut self, bytes: &[u8]) {
        let chunks = self.scanner.feed(bytes);
        self.overlay.clear(self.parser.screen(), &mut self.out);
        for chunk in chunks {
            match chunk {
                Chunk::Output(data) => {
                    self.parser.process(&data);
                    self.out.extend_from_slice(&data);
                }
                Chunk::Prompt(cwd) => {
                    self.log.line(format_args!("prompt in {cwd}"));
                    self.ctx.cwd = PathBuf::from(cwd);
                    self.prompt = None;
                    self.menu = Menu::default();
                }
                Chunk::Path(path) => self.ctx.set_path(&path),
                Chunk::Input => {
                    let (row, col) = self.parser.screen().cursor_position();
                    self.log.line(format_args!("input starts at row {row}, column {col}"));
                    self.prompt = Some(Prompt::capture(self.parser.screen()));
                }
            }
        }
        self.stale = false;
        self.redraw = true;
    }

    fn input(&mut self, bytes: &[u8]) -> Result<()> {
        let mut bytes = bytes.to_vec();
        if self.cursor_query.is_some() {
            if let Some((row, col, rest)) = take_cursor_report(&bytes) {
                self.cursor_query = None;
                self.parser.process(format!("\x1b[{row};{col}H").as_bytes());
                bytes = rest;
            }
        }
        if bytes.is_empty() {
            return Ok(());
        }
        let key = Key::parse(&bytes);
        if key.is_some() {
            if self.stale {
                self.settle();
            }
            self.update();
        }
        let handled = match key {
            Some(Key::Tab | Key::BackTab) if self.menu.has_items() => {
                self.menu.suppressed = false;
                self.step(key == Some(Key::Tab))?;
                true
            }
            Some(Key::Up | Key::Down) if self.menu.visible() && self.menu.selected > 0 => {
                self.step(key == Some(Key::Down))?;
                true
            }
            Some(Key::Escape) if self.menu.visible() => {
                if self.menu.selected > 0 {
                    self.select(0)?;
                }
                self.menu.suppressed = true;
                true
            }
            _ => false,
        };
        if key.is_some() {
            let menu = &self.menu;
            let rows = menu.completion.items.len();
            let text = String::from_utf8_lossy(&bytes);
            self.log.line(format_args!(
                "key {text:?}: handled {handled}, {rows} suggestions, row {}",
                menu.selected
            ));
        }
        if !handled {
            self.forward(&bytes)?;
        }
        self.redraw = true;
        Ok(())
    }

    /// Moves the highlight one row down or up, wrapping around.
    fn step(&mut self, forward: bool) -> Result<()> {
        let rows = self.menu.completion.items.len() + 1;
        let row = if forward {
            (self.menu.selected + 1) % rows
        } else {
            (self.menu.selected + rows - 1) % rows
        };
        self.select(row)
    }

    /// Highlights `row` and puts what it stands for on the command line,
    /// erasing only the part that differs from what is there now.
    fn select(&mut self, row: usize) -> Result<()> {
        let from = self.menu.shown().to_string();
        self.menu.selected = row;
        let to = self.menu.shown();
        let common = from.chars().zip(to.chars()).take_while(|(a, b)| a == b).count();
        let erase = "\x7f".repeat(from.chars().count() - common);
        let keys = erase + &to.chars().skip(common).collect::<String>();
        self.pty.write_all(keys.as_bytes())?;
        self.pty.flush()?;
        self.menu.pending = true;
        self.stale = true;
        Ok(())
    }

    fn forward(&mut self, bytes: &[u8]) -> Result<()> {
        self.pty.write_all(bytes)?;
        self.pty.flush()?;
        if bytes.contains(&b'\r') {
            // The line runs; suggestions wait for the next prompt.
            self.overlay.clear(self.parser.screen(), &mut self.out);
            self.prompt = None;
            self.menu = Menu::default();
        } else {
            self.menu.suppressed = false;
            self.menu.pending = false;
        }
        self.stale = true;
        Ok(())
    }

    /// Waits briefly for the echo of keys already sent, handling output and
    /// setting other events aside.
    fn settle(&mut self) {
        let deadline = Instant::now() + ECHO_WAIT;
        while self.stale {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            match self.rx.recv_timeout(left) {
                Ok(Event::Output(bytes)) => self.output(&bytes),
                Ok(event) => self.backlog.push_back(event),
                Err(_) => break,
            }
        }
    }

    /// Reads the command line off the screen and recomputes the suggestions
    /// when it changed. While a highlighted row is on the line, the list
    /// stays as it is.
    fn update(&mut self) {
        let screen = self.parser.screen();
        let Some(line) = self.prompt.as_mut().and_then(|prompt| prompt.read(screen)) else {
            if self.menu.line.is_some() && !self.menu.pending {
                self.log.line(format_args!("cursor is not at the end of the input"));
                self.menu.line = None;
            }
            return;
        };
        if self.menu.selected > 0 || self.menu.pending {
            if self.menu.expected().as_deref() == Some(line.as_str()) {
                self.menu.pending = false;
                return;
            }
            if self.menu.pending {
                return;
            }
        }
        if self.menu.line.as_ref() != Some(&line) {
            self.menu.completion = complete::complete(&line, &mut self.ctx);
            self.menu.selected = 0;
            let count = self.menu.completion.items.len();
            self.log.line(format_args!("line {line:?}: {count} suggestions"));
            self.menu.line = Some(line);
        }
    }

    fn draw(&mut self) {
        self.redraw = false;
        self.update();
        let screen = self.parser.screen();
        self.overlay.clear(screen, &mut self.out);
        if self.prompt.is_none() || !self.menu.visible() || self.menu.pending {
            return;
        }
        let view = View {
            typed: &self.menu.completion.raw,
            items: &self.menu.completion.items,
            selected: self.menu.selected,
            token_width: self.menu.shown().width(),
        };
        self.overlay.draw(screen, &view, &mut self.out);
    }

    fn idle(&mut self) {
        let held = self.scanner.flush();
        if !held.is_empty() {
            self.overlay.clear(self.parser.screen(), &mut self.out);
            self.parser.process(&held);
            self.out.extend_from_slice(&held);
            self.redraw = true;
        }
        if self
            .cursor_query
            .is_some_and(|asked| asked.elapsed() > CURSOR_WAIT)
        {
            self.cursor_query = None;
        }
    }

    fn check_size(&mut self) -> Result<()> {
        let Ok(size) = terminal::size() else {
            return Ok(());
        };
        if size == self.size || size.0 == 0 || size.1 == 0 {
            return Ok(());
        }
        self.size = size;
        self.parser.set_size(nonzero(size.0), nonzero(size.1));
        self.overlay.clear(self.parser.screen(), &mut self.out);
        self.master.resize(pty_size(size.0, size.1))?;
        self.redraw = true;
        Ok(())
    }

    fn flush(&mut self) {
        if self.out.is_empty() {
            return;
        }
        let mut stdout = std::io::stdout().lock();
        // Terminals that support synchronized output show the overlay change
        // in one frame instead of flickering through it.
        let synced = self.overlay.take_touched();
        if synced {
            let _ = stdout.write_all(b"\x1b[?2026h");
        }
        let _ = stdout.write_all(&self.out);
        if synced {
            let _ = stdout.write_all(b"\x1b[?2026l");
        }
        let _ = stdout.flush();
        self.out.clear();
    }
}

/// Pulls a cursor position report (`ESC [ row ; col R`) out of input,
/// returning the position and the rest of the input.
fn take_cursor_report(bytes: &[u8]) -> Option<(u16, u16, Vec<u8>)> {
    let start = bytes.windows(2).position(|pair| pair == b"\x1b[")?;
    let body = &bytes[start + 2..];
    let end = body.iter().position(|&byte| byte == b'R')?;
    let (row, col) = std::str::from_utf8(&body[..end]).ok()?.split_once(';')?;
    let (row, col) = (row.parse().ok()?, col.parse().ok()?);
    let mut rest = bytes[..start].to_vec();
    rest.extend_from_slice(&body[end + 1..]);
    Some((row, col, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keys() {
        assert!(matches!(Key::parse(b"\t"), Some(Key::Tab)));
        assert!(matches!(Key::parse(b"\x1b[Z"), Some(Key::BackTab)));
        assert!(Key::parse(b"ls\r").is_none());
    }

    #[test]
    fn takes_the_cursor_report_out_of_input() {
        assert_eq!(take_cursor_report(b"a\x1b[12;40Rb"), Some((12, 40, b"ab".to_vec())));
        assert_eq!(take_cursor_report(b"\x1b[A"), None);
    }
}
