//! Initializes logging based on the specified environment and output configurations.
//!
//! This function configures the global logging behavior according to the specified outputs
//! and the environment string provided. It supports conditional logging to `stderr`, files,
//! or `journald` based on the inputs.
//!
//! # Arguments
//! * `outputs` - A vector of `Option<String>` where each element represents an optional
//!   output destination. Supported values are file paths and "journald".
//! * `env` - A string slice that represents the logging environment. It can be a simple
//!   level string like "debug" or a detailed filter like "my_crate=info,my_crate::module=debug".
//!
//! # Examples
//! ```
//! ts_init::init_logging(vec![None, Some("log.log".to_string())], "debug");
//! //ts_init::init_logging(vec![None, Some("journald".to_string())], "debug");
//! //ts_init::init_logging(vec![Some("journald".to_string())], "debug");
//! ```
//!
//! # Panics
//! This function panics if the `outputs` vector has more than two elements or if the specified
//! logging configuration is invalid.
//!
//! # Errors
//! This function sets the global default logger and may return an error if logging initialization
//! fails due to system-level constraints or invalid configurations.

pub mod prelude;

pub use tracing;
pub use tracing_subscriber;

use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    fmt::format::{DefaultFields, Format, Full},
    EnvFilter,
};

#[deprecated(
    since = "0.2.0",
    note = "This function is deprecated. Use `ts_init::builder()` instead."
)]
pub fn init_logging<S: AsRef<str>>(outputs: Vec<Option<String>>, env: S) {
    use tracing::subscriber::set_global_default;
    //use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};
    use tracing_subscriber::{layer::SubscriberExt, *};

    let default_env = env.as_ref();

    fn file_appender(path: impl AsRef<str>) -> std::fs::File {
        std::fs::File::options()
            .append(true)
            .create(true)
            .open(path.as_ref())
            .unwrap_or_else(|e| panic!("{:?}: {}", path.as_ref(), e))
    }

    let t = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::builder().parse(default_env).unwrap()),
        )
        .with_writer(std::io::stderr);

    match outputs.len() {
        0 => set_global_default(t.finish()),
        1 => match outputs[0].as_ref() {
            None => set_global_default(t.finish()),
            Some(p) => match p.as_str() {
                "journald" => {
                    set_global_default(Registry::default().with(tracing_journald::layer().unwrap()))
                }
                _ => set_global_default(t.with_writer(file_appender(p)).with_ansi(false).finish()),
            },
        },
        2 => match (outputs[0].as_ref(), outputs[1].as_ref()) {
            (None, Some(p)) => match p.as_str() {
                "journald" => {
                    set_global_default(t.finish().with(tracing_journald::layer().unwrap()))
                }
                _ => set_global_default(
                    t.finish().with(
                        fmt::Layer::default()
                            .with_writer(file_appender(p))
                            .with_ansi(false),
                    ),
                ),
            },
            _ => panic!("Invalid output"),
        },
        _ => panic!("Too many outputs"),
    }
    .unwrap();
}

/// Creates a default subscriber builder that outputs logs to `stderr`.
///
/// Unlike `tracing_subscriber::fmt::SubscriberBuilder`, which writes to `stdout` by default,
/// this builder is configured to write to `stderr`.
///
/// The `tracing-log` feature is enabled by default, allowing you to seamlessly use both
/// `tracing` and `log` crates together.
///
/// # Example
///
/// ```rust
/// ts_init::builder().init();
/// ```
pub fn builder() -> tracing_subscriber::fmt::SubscriberBuilder<
    DefaultFields,
    Format<Full>,
    tracing_subscriber::filter::LevelFilter,
    fn() -> std::io::Stderr,
> {
    tracing_subscriber::fmt().with_writer(std::io::stderr)
}

/// Generates the directive string for `tracing_subscriber::filter::EnvFilter`
/// based on CARGO_PKG_NAME and CARGO_BIN_NAME at the specified log level.
///
/// # Example
/// ```
/// use ts_init::prelude::*;
/// let directive = env_filter_directive!("info");
/// assert_eq!(directive, "ts_init=info");
/// ```
#[macro_export]
macro_rules! env_filter_directive {
    ($level:expr) => {{
        let PKG: &str = option_env!("CARGO_PKG_NAME").unwrap();
        let BIN: &str = option_env!("CARGO_BIN_NAME").unwrap_or(PKG);

        let pkg = PKG.replace('-', "_");
        let bin = BIN.replace('-', "_");

        if pkg == bin {
            format!("{}={}", pkg, $level)
        } else {
            format!(
                "{pkg}={lvl},{bin}={lvl}",
                pkg = pkg,
                bin = bin,
                lvl = $level
            )
        }
    }};
}

#[deprecated(
    since = "0.2.0",
    note = "The `crate_env!` macro has been renamed to `env_filter_directive!`. Please update your usage."
)]
#[macro_export]
macro_rules! crate_env {
    ($env:expr) => {
        $crate::generate_log_env!($env)
    };
}
