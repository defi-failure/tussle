//! Bootstrap the default macOS source set and surface permission caveats.

use anyhow::{Context, Result, bail};
use tussle_core::Source;
use tussle_core::sources::accessibility::{self, Accessibility};
use tussle_core::sources::nsuserkeyequivalents::AppMenuOverrides;
use tussle_core::sources::symbolichotkeys::SymbolicHotkeys;

/// Build the default macOS source set, optionally restricted to the
/// names in `only`.
///
/// Each source is constructed with paths/configuration the CLI looks up via
/// `dirs`; `tussle-core` itself stays filesystem-agnostic. An unknown name
/// in `only` is an error that lists what is available.
pub(super) fn default_sources(
    ax_timeout: f32,
    ax_concurrency: usize,
    app_filter: Vec<String>,
    only: &[String],
) -> Result<Vec<Box<dyn Source>>> {
    let prefs = dirs::preference_dir().context("could not locate user preferences directory")?;
    let mut sources: Vec<Box<dyn Source>> = vec![
        Box::new(SymbolicHotkeys::new(
            prefs.join("com.apple.symbolichotkeys.plist"),
        )),
        Box::new(AppMenuOverrides::new(prefs.clone())),
        Box::new(Accessibility::new(ax_timeout, ax_concurrency).with_bundle_filter(app_filter)),
    ];
    if only.is_empty() {
        return Ok(sources);
    }
    let available: Vec<&'static str> = sources.iter().map(|s| s.name()).collect();
    for name in only {
        if !available.contains(&name.as_str()) {
            bail!(
                "unknown source {name:?}; available: {}",
                available.join(", ")
            );
        }
    }
    sources.retain(|s| only.iter().any(|n| n == s.name()));
    Ok(sources)
}

/// Print a one-line stderr note when the Accessibility source is about to
/// run without permission, since that silently truncates per-app menu
/// enumeration.
pub(super) fn warn_if_no_accessibility(sources: &[Box<dyn Source>]) {
    let uses_accessibility = sources.iter().any(|s| s.name() == "accessibility");
    if uses_accessibility && !accessibility::is_trusted() {
        eprintln!(
            "note: tussle does not currently have Accessibility permission, \
             so app menu shortcuts will be missing. Grant access in \
             System Settings → Privacy & Security → Accessibility, then re-run."
        );
    }
}
