use slog::{Drain, Level, Logger};
use slog_scope::GlobalLoggerGuard;

use crate::error::SkrdResult;

pub struct LoggerGuard(GlobalLoggerGuard);

impl LoggerGuard {
    pub fn init(name: &'static str, level: Level) -> SkrdResult<Self> {
        let decorator = slog_term::TermDecorator::new().stdout().build();
        let drain = slog_term::CompactFormat::new(decorator)
            .use_custom_timestamp(timestamp_local_ymdhms)
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain)
            .chan_size(1024)
            .build()
            .filter_level(level)
            .fuse();

        let logger = Logger::root(drain, o!(name => env!("CARGO_PKG_VERSION")));

        let guard = slog_scope::set_global_logger(logger);
        slog_stdlog::init()?;

        debug!("Logger initialized.");

        Ok(LoggerGuard(guard))
    }
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        debug!("Logger uninitialized.");
    }
}

fn timestamp_local_ymdhms(io: &mut std::io::Write) -> std::io::Result<()> {
    write!(
        io,
        "{}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f")
    )
}
