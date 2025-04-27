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
//! ```rust
//! // Output info log of current crate to stderr
//! ts_init::init(ts_init::env_filter_directive!("info"));
//! ```

pub mod layer;
pub mod prelude;

pub use tracing;
pub use tracing_subscriber;

use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::layer::Layered;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::{
    fmt::{
        self,
        format::{DefaultFields, Format, Full},
    },
    registry::LookupSpan,
};

#[deprecated(
    since = "0.1.2",
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

/// Creates a default subscriber instance ready to be registered.
///
/// This is effectively the same as calling `builder().finish()`.
pub fn subscriber() -> tracing_subscriber::fmt::Subscriber<
    DefaultFields,
    Format<Full>,
    tracing_subscriber::filter::LevelFilter,
    fn() -> std::io::Stderr,
> {
    builder().finish()
}

pub fn try_init<S: AsRef<str>>(
    default_env: S,
) -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::util::SubscriberInitExt;

    builder()
        .finish()
        .with_env_filter_or(default_env.as_ref())
        .try_init()
}

pub fn init<S: AsRef<str>>(default_env: S) {
    try_init(default_env).expect("Failed to initialize logging")
}

#[derive(Clone)]
pub struct FileMakeWriter {
    path: PathBuf,
}

impl FileMakeWriter {
    fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().into(),
        }
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileMakeWriter {
    type Writer = fs::File;

    fn make_writer(&'a self) -> Self::Writer {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("unable to open log file")
    }
}

pub trait TiSubscriberExt: tracing_core::subscriber::Subscriber {
    /// Adds an `EnvFilter` layer to this subscriber.
    ///
    /// This first attempts to read the filter from the `RUST_LOG` environment
    /// variable. If that fails, it falls back to the provided directive string `s`.
    ///
    /// # Arguments
    ///
    /// * `default_env` – a default directive string, e.g. `"info,my_crate=debug"`.
    ///
    /// # Example
    ///
    /// ```
    /// use ts_init::prelude::*;
    ///
    /// let subscriber = ts_init::subscriber()
    ///     .with_env_filter_or("info,my_crate=debug")
    ///     .init();
    /// ```
    fn with_env_filter_or<S: AsRef<str>>(
        self,
        default_env: S,
    ) -> Layered<tracing_subscriber::EnvFilter, Self>
    where
        Self: Sized,
    {
        self.with(layer::env_filter_with_default(default_env))
    }

    /// Adds a `fmt::Layer` to the `Subscriber` that writes logs to a specified file path.
    ///
    /// # Arguments
    ///
    /// - `path`: The path to the log file where logs will be written.
    ///
    /// # Returns
    ///
    /// - A `Layered` type: a new `Subscriber` composed of the original subscriber and the added `fmt::Layer`.
    ///
    /// # Details
    ///
    /// - Uses `fmt::Layer` to log events to a file instead of stdout/stderr.
    /// - ANSI color output is disabled (`with_ansi(false)`).
    /// - The file is opened in `append` mode, so new logs are added to the end.
    /// - A new file handle is opened each time logs are written (based on `MakeWriter` behavior).
    ///
    /// # Example
    ///
    /// ```
    /// use ts_init::prelude::*;
    ///
    /// ts_init::subscriber()
    ///     .with_file("app.log")
    ///     .init();
    /// ```
    fn with_file<P>(
        self,
        path: P,
    ) -> Layered<fmt::Layer<Self, DefaultFields, Format<Full>, FileMakeWriter>, Self>
    where
        Self: Sized,
        for<'span> Self: LookupSpan<'span>,
        P: AsRef<Path>,
    {
        let path_buf = path.as_ref().to_owned();

        self.with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(FileMakeWriter::new(path_buf)),
        )
    }

    fn with_journald(self) -> Layered<tracing_journald::Layer, Self>
    where
        Self: Sized,
        for<'span> Self: tracing_subscriber::registry::LookupSpan<'span>,
    {
        self.with(tracing_journald::layer().unwrap())
    }
}

impl<S: tracing_core::subscriber::Subscriber> TiSubscriberExt for S {}

/// Generates the directive string for `tracing_subscriber::filter::EnvFilter`
/// based on CARGO_PKG_NAME and CARGO_BIN_NAME at the specified log level.
///
/// # Example
/// ```
/// use ts_init::prelude::*;
/// let directive = env_filter_directive!("info");
/// assert_eq!(directive, "ts_init=info");
/// ```
pub use ts_init_macros::env_filter_directive;

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
