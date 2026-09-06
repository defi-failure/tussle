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

/// A non-fatal problem during a scan: the source still produced results,
/// but they are known to be incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanWarning {
    /// A configuration file was skipped because it could not be read or
    /// parsed. Everything else from the same source is still reported.
    Skipped { path: PathBuf, message: String },
    /// A running app did not answer Accessibility queries within the
    /// timeout, even on retry, so its menu shortcuts are missing or
    /// incomplete.
    Unresponsive { app: String },
    /// An `--app` pattern matched no running app. Only running apps can
    /// be scanned; `similar` lists running apps whose names come close.
    NoMatchingApp {
        pattern: String,
        similar: Vec<String>,
    },
}

impl std::fmt::Display for ScanWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanWarning::Skipped { path, message } => {
                write!(f, "skipped {}: {message}", path.display())
            }
            ScanWarning::Unresponsive { app } => write!(
                f,
                "{app} did not answer Accessibility queries in time; its menu shortcuts are missing"
            ),
            ScanWarning::NoMatchingApp { pattern, similar } => {
                write!(
                    f,
                    "no running app matches {pattern:?}; only running apps can be scanned"
                )?;
                if !similar.is_empty() {
                    write!(f, " (similar: {})", similar.join(", "))?;
                }
                Ok(())
            }
        }
    }
}
