

pub fn setup_logging() {
          // Initialize with ANSI colors enabled, regardless of terminal detection
    let env = pretty_env_logger::env_logger::Env::default()
        .filter_or("RUST_LOG", "info");

    pretty_env_logger::env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .write_style(pretty_env_logger::env_logger::WriteStyle::Always)
        .init();
}