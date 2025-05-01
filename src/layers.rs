use axum::body::Body;
use axum::extract::{MatchedPath, Request};
use tower_http::trace::TraceLayer;
use tracing::Span;

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
            .with_thread_names(true)
            .with_writer(std::io::stderr);

        let crate_name = env!("CARGO_CRATE_NAME");
        let default_rust_log = format!("{crate_name}=trace,axum::rejection=trace");
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

type Classifier =
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>;

#[must_use]
pub fn trace_layer() -> TraceLayer<Classifier, impl Clone + Fn(&Request<Body>) -> Span> {
    TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str);
        tracing::info_span!(
            "http_request",
            method = ?request.method(),
            matched_path,
            version = ?request.version(),
        )
    })
}
