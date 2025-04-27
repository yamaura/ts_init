/// Creates a `tracing_subscriber::filter::EnvFilter` from the environment,
/// falling back to the given default directive string if no environment variable is set.
///
/// This first tries to load the filter configuration from the `RUST_LOG` environment variable.
/// If the environment variable is not set or is invalid, it uses the provided default directive.
///
pub fn env_filter_with_default<S: AsRef<str>>(
    default_env: S,
) -> tracing_subscriber::filter::EnvFilter {
    tracing_subscriber::filter::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new(default_env))
}
