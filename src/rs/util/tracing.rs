pub fn init_stdout_logger() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .pretty()
        .init();
}
