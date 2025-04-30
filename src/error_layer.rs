#[derive(Debug)]
#[must_use]
pub struct ErrorLayer;

impl ErrorLayer {
    #[expect(clippy::missing_errors_doc)]
    pub fn setup(&self) -> Result<(), color_eyre::eyre::Error> {
        use tracing_error::ErrorLayer;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{EnvFilter, fmt};

        color_eyre::install()?;

        let format_layer = fmt::layer()
            .pretty()
            .without_time()
            .with_target(true)
            .with_writer(std::io::stderr);
        let filter_layer =
            EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(format_layer)
            .with(ErrorLayer::default())
            .try_init()?;

        Ok(())
    }
}
