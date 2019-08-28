use slog::*;
use slog_async::Async;

fn main() {

    let debug_drain = new_drain(Level::Debug);
    let info_drain = new_drain(Level::Info);
    let warn_drain = new_drain(Level::Warning);

    // init AtomicSwitch
    let drain = slog_atomic::AtomicSwitch::new(debug_drain);
    let log_ctrl = drain.ctrl();

    // Init root logger
    // Here the global log level is DEBUG
    let log = slog::Logger::root(drain, o!());

    println!("------------------");
    debug!(log, "debug log 1"); // this is logged
    info!(log, "info log 1");   // this is logged
    warn!(log, "warn log 1");   // this is logged

    // Change the log level at runtime.
    // Now the global log level is INFO
    log_ctrl.set(info_drain);

    println!("------------------");
    debug!(log, "debug log 2"); // this is NOT logged anymore
    info!(log, "info log 2");   // this is logged
    warn!(log, "warn log 2");   // this is logged

    // Change the log level at runtime.
    // Now the global log level is WARN
    log_ctrl.set(warn_drain);

    println!("------------------");
    debug!(log, "debug log 3"); // this is NOT logged anymore
    info!(log, "info log 3");   // this is NOT logged anymore
    warn!(log, "warn log 3");   // this is logged
}

fn new_drain(level: Level) -> Fuse<LevelFilter<Fuse<Async>>> {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    drain.filter_level(level).fuse()

}