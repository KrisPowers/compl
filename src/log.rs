//! A short log of the current session, kept in the cache directory as
//! `session.log` so a bug report can include what compl saw.

use std::fmt::Arguments;
use std::fs::{self, File};
use std::io::Write;

/// The log stops growing past this, so a long session cannot fill the disk.
const MAX_BYTES: usize = 1 << 20;

pub struct Log {
    file: Option<File>,
    written: usize,
}

impl Log {
    /// Starts a fresh log, replacing the last session's. Logging is skipped
    /// when the file cannot be created.
    pub fn open() -> Log {
        let dir = crate::shell::cache_dir();
        let file = fs::create_dir_all(&dir)
            .and_then(|()| File::create(dir.join("session.log")))
            .ok();
        Log { file, written: 0 }
    }

    pub fn line(&mut self, args: Arguments) {
        let Some(file) = &mut self.file else {
            return;
        };
        if self.written < MAX_BYTES {
            let text = format!("{args}\n");
            self.written += text.len();
            let _ = file.write_all(text.as_bytes());
        }
    }
}
