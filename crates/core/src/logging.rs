//! A small stderr logger for the `log` facade.
//!
//! Lines look like `[  12.345s] INFO  [renderer] selected GPU driver: vulkan`. The bracketed
//! context is the log target, which every Lantern crate sets to its subsystem name
//! (`log::info!(target: "renderer", ...)`).
//! `LANTERN_LOG=debug` (or trace, info, warn, error) changes the level at startup.

use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

struct StderrLogger {
    start: Instant,
    level: log::LevelFilter,
}

static LOGGER: OnceLock<StderrLogger> = OnceLock::new();

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let seconds = self.start.elapsed().as_secs_f64();
        let mut stderr = std::io::stderr().lock();
        // A failed write to stderr has nowhere better to be reported.
        let _ = writeln!(stderr, "[{seconds:8.3}s] {:<5} [{}] {}", record.level(), record.target(), record.args());
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Installs the logger once. Later calls do nothing.
pub fn init(default_level: log::LevelFilter) {
    let level = std::env::var("LANTERN_LOG")
        .ok()
        .and_then(|text| text.parse::<log::LevelFilter>().ok())
        .unwrap_or(default_level);
    let logger = LOGGER.get_or_init(|| StderrLogger { start: Instant::now(), level });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(logger.level);
    }
}
