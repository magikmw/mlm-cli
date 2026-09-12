//! `mlm` as a library, so `src/main.rs` and dev tooling (`examples/`,
//! `tests/`) can both reuse the real parsing/storage/rendering logic
//! instead of re-deriving it. No behavior lives here — every module is
//! exactly what the binary already used, just made `pub` and moved out
//! from under `main.rs`'s own `mod` list.

pub mod cli;
pub mod commands;
pub mod date;
pub mod db;
pub mod render;
pub mod status;
pub mod stint;
pub mod storage;
pub mod time;
pub mod week;
pub mod week_target;
pub mod week_view;

/// One nonzero exit code for every hard error (§6.3 only requires
/// "nonzero"); a single code keeps the surface small. `Ok` is always 0.
pub fn exit_code(result: &anyhow::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
