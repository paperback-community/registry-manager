use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*};

pub fn new() -> Result<(), ()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("LOG_LEVEL")
        .from_env_lossy();

    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(std::io::stdout)
                .with_filter(env_filter),
        )
        .try_init();

    if let Err(err) = subscriber {
        eprintln!(
            "Something went wrong while initializing the logger: {}",
            &err
        );
        return Err(());
    }

    Ok(())
}
