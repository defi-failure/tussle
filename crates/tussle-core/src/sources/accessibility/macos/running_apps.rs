//! Enumerate running apps that plausibly own a menu bar.
//!
//! `NSWorkspace.runningApplications` returns every process that has an
//! `NSRunningApplication` proxy — including XPC services, daemons, and
//! helper processes that have no menu bar but still respond to AX queries
//! by stalling until the messaging timeout. Two filters trim them out:
//!
//!   - `activationPolicy == Prohibited` — anything explicitly declared as
//!     "shouldn't have UI" (most XPC services, *PrivateProvider helpers).
//!   - executable path contains `/XPCServices/` — Apple-style XPC bundles
//!     that nonetheless declare `Regular` activation policy so they can
//!     host UI in their own process (notably `WebKit.WebContent` and the
//!     surrounding `WebKit.GPU`/`WebKit.Networking` siblings).

use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

pub(super) struct RunningApp {
    pub(super) pid: i32,
    pub(super) bundle_id: Option<String>,
    pub(super) app_name: Option<String>,
}

pub(super) fn list_running_apps() -> Vec<RunningApp> {
    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();
    let mut out = Vec::with_capacity(apps.len());
    for app in apps.iter() {
        if app.activationPolicy() == NSApplicationActivationPolicy::Prohibited {
            continue;
        }
        if let Some(url) = app.executableURL()
            && let Some(path) = url.path()
            && path.to_string().contains("/XPCServices/")
        {
            continue;
        }
        out.push(RunningApp {
            pid: app.processIdentifier(),
            bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
            app_name: app.localizedName().map(|s| s.to_string()),
        });
    }
    out
}
