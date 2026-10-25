//! `mlm` as a library, so `src/main.rs` and dev tooling (`examples/`,
//! `tests/`) can both reuse the real parsing/storage/rendering logic
//! instead of re-deriving it. Apart from the exit-code and stdout-write
//! helpers below, no behavior lives here — every module is
//! exactly what the binary already used, just made `pub` and moved out
//! from under `main.rs`'s own `mod` list.

pub mod anomalies;
pub mod carry_scan;
pub mod cli;
pub mod commands;
pub mod date;
pub mod db;
pub mod export;
pub mod history;
pub mod lookback;
pub mod render;
pub mod status;
pub mod stint;
pub mod storage;
pub mod time;
pub mod week;
pub mod week_target;
pub mod week_view;

use std::io::{ErrorKind, Write};

/// The test-side exit mapping: `Ok` is 0, any `Err` is 1. `main` adds
/// exit 141 for a broken stdout pipe.
pub fn exit_code(result: &anyhow::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// A failed write to stdout. `main` exits 141 on a broken pipe and
/// prints `error: cannot write output: <reason>` for anything else.
#[derive(Debug)]
pub struct OutputError(pub std::io::Error);

impl OutputError {
    pub fn is_broken_pipe(&self) -> bool {
        self.0.kind() == ErrorKind::BrokenPipe
    }
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot write output: {}", self.0)
    }
}

impl std::error::Error for OutputError {}

/// Writes `text` to `out` and flushes. Every stdout write goes through
/// here, so a failure reaches `main` as an `OutputError`.
pub fn emit<W: Write + ?Sized>(out: &mut W, text: &str) -> Result<(), OutputError> {
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(OutputError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn emit_writes_and_flushes() {
        let mut buf = Vec::new();
        emit(&mut buf, "hello\n").unwrap();
        assert_eq!(buf, b"hello\n");
    }

    #[test]
    fn emit_maps_a_failed_write() {
        let mut full: &mut [u8] = &mut [];
        let err = emit(&mut full, "x").unwrap_err();
        assert_eq!(err.0.kind(), ErrorKind::WriteZero);
        assert!(!err.is_broken_pipe());
    }

    #[test]
    fn broken_pipe_is_detected() {
        let err = OutputError(std::io::Error::from(ErrorKind::BrokenPipe));
        assert!(err.is_broken_pipe());
    }

    #[test]
    fn output_error_display() {
        let err = OutputError(std::io::Error::other("x"));
        assert_eq!(err.to_string(), "cannot write output: x");
    }
}
