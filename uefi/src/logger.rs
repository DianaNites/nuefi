//! Logging helpers for UEFI
use core::fmt::Write;

use log::{Metadata, Record};

use crate::get_boot_table;

static LOGGER: UefiLogger = UefiLogger::new();

/// UEFI Logger
///
/// This logs to the UEFI `stdout`,
/// if ExitBootServices has not been called, otherwise it does nothing.
pub struct UefiLogger {
    //
}

impl UefiLogger {
    const fn new() -> Self {
        Self {}
    }

    pub(crate) fn init() {
        // This cannot error, because we set it before user code is called.
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(log::STATIC_MAX_LEVEL);
    }
}

impl log::Log for UefiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        //&& metadata.level() <= Level::Info
        target == "uefi_stub" || target == "uefi"
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
