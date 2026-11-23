use std::{ffi::OsString, path::PathBuf};

/// An external compressor reading stdin and writing compressed bytes to stdout.
/// The program and each argument are passed literally, without a shell.
pub trait Compressor: Send + Sync {
    /// Executable name or path.
    fn program(&self) -> PathBuf;
    /// Individual command arguments.
    fn args(&self) -> Vec<OsString> {
        Vec::new()
    }
    /// Conventional extension without a leading dot.
    fn use_extension(&self) -> &str;
}

/// Compress dumps with `gzip`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GzipCompressor;

impl Compressor for GzipCompressor {
    fn program(&self) -> PathBuf {
        "gzip".into()
    }

    fn use_extension(&self) -> &str {
        "gz"
    }
}

/// Compress dumps with `bzip2`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bzip2Compressor;

impl Compressor for Bzip2Compressor {
    fn program(&self) -> PathBuf {
        "bzip2".into()
    }

    fn use_extension(&self) -> &str {
        "bz2"
    }
}
