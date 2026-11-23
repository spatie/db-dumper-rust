#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod compressors;
mod dumper;
mod process;

pub use compressors::{Bzip2Compressor, Compressor, GzipCompressor};
pub use dumper::{
    Database, DatabaseDriver, DbDumper, MariaDb, MongoDb, MySql, PostgreSql, Sqlite, Tables,
};
pub use process::DumpCommand;

/// A failure to configure, run or save a database dump.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A required setting is missing, or settings conflict.
    #[error("invalid dump configuration: {0}")]
    InvalidConfiguration(String),
    /// The database URL cannot be parsed.
    #[error("invalid database URL")]
    InvalidDatabaseUrl,
    /// An executable could not be started, or a file operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A native dump tool or compressor returned an unsuccessful status.
    /// Stderr is intentionally not retained: tools may echo passwords.
    #[error("{program} failed with exit code {code:?}")]
    ProcessFailed {
        /// Executable name, without arguments or credentials.
        program: String,
        /// Exit code, or `None` when terminated by a signal.
        code: Option<i32>,
    },
    /// The deadline expired. The direct child has been killed and reaped.
    #[error("database dump timed out")]
    Timeout,
    /// The dump tool succeeded but wrote no bytes.
    #[error("database dump was empty")]
    EmptyDump,
}

#[cfg(doctest)]
#[doc = include_str!("../docs/usage.md")]
mod usage {}
