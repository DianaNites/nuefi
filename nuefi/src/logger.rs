//! Logging helpers for UEFI
use core::fmt::Write;

use log::{Log, Metadata, Record};

use crate::{
    get_boot_table,
    proto::console::{TextBackground, TextForeground},
};

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
    excludes: Option<&'static [&'static str]>,
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
            excludes: None,
        }
    }

    /// Like [`UefiLogger::new`], but filters nothing, allowing all logs
    /// through.
    pub const fn all() -> Self {
        Self {
            targets: None,
            excludes: None,
        }
    }

    /// Add excludes
    pub const fn exclude(self, excludes: &'static [&'static str]) -> Self {
        Self {
            targets: self.targets,
            excludes: Some(excludes),
        }
    }

    /// Add colorful output
    pub const fn color(self) -> UefiColorLogger {
        UefiColorLogger(self)
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
        let exclude = if let Some(excludes) = self.excludes {
            excludes.iter().any(|s| {
                s == &target
                    || (target.starts_with(s)
                        && target.as_bytes().get(s.len()).copied().unwrap_or_default() == b':')
            })
        } else {
            false
        };
        let include = if let Some(targets) = self.targets {
            targets.iter().any(|s| {
                s == &target
                    || (target.starts_with(s)
                        && target.as_bytes().get(s.len()).copied().unwrap_or_default() == b':')
            })
        } else {
            true
        };
        !exclude && include
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

    #[inline]
    fn flush(&self) {}
}

/// Like [UefiLogger], but colors its output based on the level
///
/// See [`UefiLogger::color`]
pub struct UefiColorLogger(UefiLogger);

impl Log for UefiColorLogger {
    #[inline]
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.0.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if let Some(table) = get_boot_table() {
            if self.enabled(record.metadata()) {
                let stdout = table.stdout();
                let attr = match record.level() {
                    log::Level::Error => TextForeground::RED,
                    log::Level::Warn => TextForeground::YELLOW,
                    log::Level::Info => TextForeground::LIGHT_CYAN,
                    // log::Level::Info => OutputAttribute::LIGHT_GRAY,
                    log::Level::Debug => TextForeground::BLUE,
                    log::Level::Trace => TextForeground::MAGENTA,
                };
                let _ = stdout.with_attributes(attr, TextBackground::BLACK, || self.0.log(record));
            }
        }
    }

    #[inline]
    fn flush(&self) {
        self.0.flush()
    }
}
