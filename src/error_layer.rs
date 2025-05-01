#[derive(Debug)]
#[must_use]
pub struct ErrorLayer;

impl ErrorLayer {
    pub fn setup(&self) -> Result<(), color_eyre::eyre::Error> {
        use tracing_error::ErrorLayer;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{EnvFilter, fmt};

        color_eyre::install()?;

        let format_layer = fmt::layer()
            .pretty()
            .without_time()
            .with_writer(std::io::stderr);

        let crate_name = env!("CARGO_CRATE_NAME");
        let default_rust_log = format!("{crate_name}=trace,tower_http=debug,axum::rejection=trace");
        let filter_layer =
            EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(default_rust_log))?;

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(format_layer)
            .with(ErrorLayer::default())
            .try_init()?;

        Ok(())
    }
}
