//! Raw mode and size for the real terminal compl runs in.

use anyhow::Result;

/// Keeps the terminal in raw mode until dropped. On Windows this also turns on
/// VT input and output, so keys arrive as the same byte sequences as on Unix.
pub struct RawMode {
    #[cfg(windows)]
    saved: console::Saved,
}

#[cfg(unix)]
impl RawMode {
    pub fn enable() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self {})
    }
}

#[cfg(unix)]
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// The terminal's size as (rows, columns).
#[cfg(unix)]
pub fn size() -> Result<(u16, u16)> {
    let (cols, rows) = crossterm::terminal::size()?;
    Ok((rows, cols))
}

#[cfg(windows)]
impl RawMode {
    pub fn enable() -> Result<Self> {
        Ok(Self {
            saved: console::enable()?,
        })
    }
}

#[cfg(windows)]
impl Drop for RawMode {
    fn drop(&mut self) {
        console::restore(&self.saved);
    }
}

#[cfg(windows)]
pub use console::size;

#[cfg(windows)]
mod console {
    use anyhow::{bail, Result};
    use windows_sys::Win32::System::Console::{
        GetConsoleCP, GetConsoleMode, GetConsoleOutputCP, GetConsoleScreenBufferInfo, GetStdHandle,
        SetConsoleCP, SetConsoleMode, SetConsoleOutputCP, CONSOLE_SCREEN_BUFFER_INFO,
        DISABLE_NEWLINE_AUTO_RETURN, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
        ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    const UTF8: u32 = 65001;

    pub struct Saved {
        input: u32,
        output: u32,
        input_cp: u32,
        output_cp: u32,
    }

    pub fn enable() -> Result<Saved> {
        // SAFETY: plain console API calls on this process's standard handles.
        unsafe {
            let input = GetStdHandle(STD_INPUT_HANDLE);
            let output = GetStdHandle(STD_OUTPUT_HANDLE);
            let (mut input_mode, mut output_mode) = (0, 0);
            if GetConsoleMode(input, &mut input_mode) == 0
                || GetConsoleMode(output, &mut output_mode) == 0
            {
                bail!("compl has to run in a console");
            }
            let saved = Saved {
                input: input_mode,
                output: output_mode,
                input_cp: GetConsoleCP(),
                output_cp: GetConsoleOutputCP(),
            };
            let raw =
                input_mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
            SetConsoleMode(input, raw | ENABLE_VIRTUAL_TERMINAL_INPUT);
            SetConsoleMode(
                output,
                output_mode
                    | ENABLE_PROCESSED_OUTPUT
                    | ENABLE_VIRTUAL_TERMINAL_PROCESSING
                    | DISABLE_NEWLINE_AUTO_RETURN,
            );
            SetConsoleCP(UTF8);
            SetConsoleOutputCP(UTF8);
            Ok(saved)
        }
    }

    pub fn restore(saved: &Saved) {
        // SAFETY: as above.
        unsafe {
            SetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), saved.input);
            SetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), saved.output);
            SetConsoleCP(saved.input_cp);
            SetConsoleOutputCP(saved.output_cp);
        }
    }

    pub fn size() -> Result<(u16, u16)> {
        // SAFETY: the info struct is plain data, filled in by the call.
        unsafe {
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &mut info) == 0 {
                bail!("cannot read the console size");
            }
            let window = info.srWindow;
            let rows = window.Bottom - window.Top + 1;
            let cols = window.Right - window.Left + 1;
            Ok((rows.max(1) as u16, cols.max(1) as u16))
        }
    }
}
