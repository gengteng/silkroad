use slog::{Drain, Logger};
use slog_term;

pub struct LoggerGuard;

impl LoggerGuard {
    pub fn init(name: &'static str) -> Self {
        println!("Initializing logger...");

        let decorator = slog_term::TermDecorator::new().stdout().build();
        let drain = slog_term::CompactFormat::new(decorator)
            .use_custom_timestamp(timestamp_local_ymdhms)
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();

        let logger = Logger::root(drain, o!(name => env!("CARGO_PKG_VERSION")));

        slog_global::set_global(logger);

        info!("Logger initialized.");

        LoggerGuard
    }
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        info!("Uninitializing logger...");
        slog_global::clear_global();
        println!("Logger uninitialized.");
    }
}

fn timestamp_local_ymdhms(io: &mut std::io::Write) -> std::io::Result<()> {
    write!(
        io,
        "{}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f")
    )
}
