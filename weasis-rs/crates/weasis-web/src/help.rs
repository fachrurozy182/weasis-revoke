//! Links to the online documentation.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/GuiUtils.java`
//! (`getOnlineHelpUrl`, `createHelpButton`).

/// Online tutorials (`weasis.help.online` of the launcher preferences).
// TODO(7.6): read `weasis.help.online` from the preferences.
const ONLINE_HELP_URL: &str = "https://nroduit.github.io/en/tutorials/";

/// Page of the online tutorials about `topic` (the index when empty).
///
/// NOTE: the desktop application adds its version (`?v=4.8.0`) for the documentation of that
/// version; the version of this rewrite does not number Weasis releases, so none is added.
pub fn online_help_url(topic: &str) -> String {
    format!("{ONLINE_HELP_URL}{topic}")
}
