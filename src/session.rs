//! The session: runs the shell in a pseudo terminal, passes every byte
//! through, keeps a model of the screen, and draws suggestions over the
//! prompt while you type.
//!
//! Keys while a suggestion shows:
//! - Tab / Shift+Tab: open the list, then move through it.
//! - Up / Down: move through the open list.
//! - Enter: put the selected suggestion on the command line. Enter again runs
//!   it. When the line already matches the suggestion, Enter runs it at once.
//! - Esc: hide suggestions until you type again.

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
use crate::markers::{Chunk, Scanner};
use crate::overlay::{Overlay, View};
use crate::screen::Prompt;
use crate::shell::{self, Shell};
use crate::terminal::{self, RawMode};

/// How long to wait for the shell to echo typed keys before acting on Tab or
/// Enter, so the suggestion acted on matches what is on screen.
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
    Enter,
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
            b"\r" => Key::Enter,
            b"\x1b" => Key::Escape,
            _ => return None,
        })
    }
}

/// The suggestions for the current command line and how they are shown.
#[derive(Default)]
struct Menu {
    /// The command line the suggestions were made for; `None` when the
    /// cursor is not at the end of a command line.
    line: Option<String>,
    completion: Completion,
    selected: usize,
    open: bool,
    /// Hidden after Esc or after a suggestion was accepted, until more input.
    suppressed: bool,
}

impl Menu {
    fn has_items(&self) -> bool {
        self.line.is_some() && !self.completion.items.is_empty()
    }

    fn visible(&self) -> bool {
        self.has_items() && !self.suppressed
    }

    fn step(&mut self, forward: bool) {
        let len = self.completion.items.len();
        if len > 0 {
            self.selected = if forward {
                (self.selected + 1) % len
            } else {
                (self.selected + len - 1) % len
            };
        }
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
        parser: vt100::Parser::new(rows, cols, 0),
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
                    self.ctx.cwd = PathBuf::from(cwd);
                    self.prompt = None;
                    self.menu = Menu::default();
                }
                Chunk::Path(path) => self.ctx.set_path(&path),
                Chunk::Input => self.prompt = Some(Prompt::capture(self.parser.screen())),
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
                if self.menu.suppressed || !self.menu.open {
                    self.menu.suppressed = false;
                } else {
                    self.menu.step(key == Some(Key::Tab));
                }
                self.menu.open = true;
                true
            }
            Some(Key::Up | Key::Down) if self.menu.visible() && self.menu.open => {
                self.menu.step(key == Some(Key::Down));
                true
            }
            Some(Key::Enter) if self.menu.visible() => self.accept()?,
            Some(Key::Escape) if self.menu.visible() => {
                self.menu.suppressed = true;
                self.menu.open = false;
                true
            }
            _ => false,
        };
        if !handled {
            self.forward(&bytes)?;
        }
        self.redraw = true;
        Ok(())
    }

    /// Types the selected suggestion into the shell. Returns false, leaving
    /// Enter to run the line, when the line already says the same thing.
    fn accept(&mut self) -> Result<bool> {
        let Completion { raw, items } = &self.menu.completion;
        let item = &items[self.menu.selected];
        if item.matches(raw) {
            return Ok(false);
        }
        let mut keys = match item.text.strip_prefix(raw.as_str()) {
            Some(rest) => rest.to_string(),
            // The case or quoting differs: erase the token and retype it.
            None => "\x7f".repeat(raw.chars().count()) + &item.text,
        };
        if !item.open {
            keys.push(' ');
        }
        self.pty.write_all(keys.as_bytes())?;
        self.pty.flush()?;
        self.menu.suppressed = true;
        self.menu.open = false;
        self.stale = true;
        Ok(true)
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
    /// when it changed.
    fn update(&mut self) {
        let screen = self.parser.screen();
        let line = self.prompt.as_mut().and_then(|prompt| prompt.read(screen));
        match line {
            None => self.menu.line = None,
            Some(line) => {
                if self.menu.line.as_ref() != Some(&line) {
                    self.menu.completion = complete::complete(&line, &mut self.ctx);
                    self.menu.selected = 0;
                    self.menu.line = Some(line);
                }
            }
        }
    }

    fn draw(&mut self) {
        self.redraw = false;
        self.update();
        let screen = self.parser.screen();
        self.overlay.clear(screen, &mut self.out);
        let Some(prompt) = &self.prompt else {
            return;
        };
        if !self.menu.visible() {
            return;
        }
        let Completion { raw, items } = &self.menu.completion;
        let view = View {
            ghost: items[self.menu.selected]
                .text
                .chars()
                .skip(raw.chars().count())
                .collect(),
            items,
            selected: self.menu.selected,
            open: self.menu.open,
            token_width: raw.width(),
            limit: prompt.limit(screen.size().1),
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
        self.parser.set_size(size.0, size.1);
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
        assert!(matches!(Key::parse(b"\r"), Some(Key::Enter)));
        assert!(Key::parse(b"ls\r").is_none());
    }

    #[test]
    fn takes_the_cursor_report_out_of_input() {
        assert_eq!(take_cursor_report(b"a\x1b[12;40Rb"), Some((12, 40, b"ab".to_vec())));
        assert_eq!(take_cursor_report(b"\x1b[A"), None);
    }
}
