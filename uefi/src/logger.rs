//! Logging helpers for UEFI
use core::fmt::Write;

use log::{Log, Metadata, Record};

use crate::get_boot_table;

/// Log from within the logger
#[allow(dead_code)]
fn debug_log(args: core::fmt::Arguments) {
    if let Some(table) = get_boot_table() {
        let mut stdout = table.stdout();
        let _ = writeln!(stdout, "{args}",);
    }
}

/// UEFI Logger
///
/// This logs to the UEFI `stdout`,
/// if ExitBootServices has not been called, otherwise it does nothing.
///
/// This filters out logs from crates other than this one
/// or the provided `target` in [`UefiLogger::new`]
pub struct UefiLogger {
    targets: Option<&'static [&'static str]>,
}

impl UefiLogger {
    /// Create a new [UefiLogger]
    ///
    /// Filters out logs from crates that are not in `targets`.
    ///
    /// Note that if this is empty then all logs will be filtered.
    ///
    /// You will need to include your own crates name.
    pub const fn new(targets: &'static [&'static str]) -> Self {
        Self {
            targets: Some(targets),
        }
    }

    /// Like [`UefiLogger::new`], but filters nothing, allowing all logs
    /// through.
    pub const fn all() -> Self {
        Self { targets: None }
    }

    /// Initialize the logger with [log]
    ///
    /// This will set the max log level to [`log::STATIC_MAX_LEVEL`]
    ///
    /// Calling this more than once has no effect, including on the max level.
    pub fn init(logger: &'static dyn Log) {
        if log::set_logger(logger).is_ok() {
            log::set_max_level(log::STATIC_MAX_LEVEL);
        }
    }
}

impl Log for UefiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        if let Some(targets) = self.targets {
            targets.iter().any(|s| {
                target.starts_with(s)
                    && target.as_bytes().get(s.len()).copied().unwrap_or_default() == b':'
            })
        } else {
            true
        }
    }

    fn log(&self, record: &Record) {
        if let Some(table) = get_boot_table() {
            if self.enabled(record.metadata()) {
                let mut stdout = table.stdout();
                let _ = writeln!(
                    stdout,
                    "[{}] {} - {}",
                    record.target(),
                    record.level(),
                    record.args()
                );
            }
        }
    }

    fn flush(&self) {}
}
