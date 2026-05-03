use std::path::PathBuf;

/// Errors a source parser can produce while scanning hotkey configurations.
///
/// New variants are added as parsers introduce new failure modes (e.g. SQLite
/// errors when the Raycast parser comes online).
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("io error reading {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The file parsed at the syntax level but its structure didn't match what
    /// the parser expects (missing key, wrong type, etc.). Used when we can
    /// read bytes but can't interpret them as a meaningful binding set.
    #[error("unexpected schema in {path}: {message}")]
    Schema { path: PathBuf, message: String },
}
