//! Stroke icons (paths from the IsoCity web UI) served to GPUI's `svg()`.

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

const ICONS: &[(&str, &str)] = &[
    ("play", r#"<path d="M8 5v14l10-7z"/>"#),
    ("pause", r#"<path d="M8 5v14M16 5v14"/>"#),
    ("close", r#"<path d="M6 6l12 12M18 6l-12 12"/>"#),
    ("fire", r#"<path d="M12 3s2 3 2 5-2 4-2 4-2-2-2-4 2-5 2-5z"/><path d="M12 12c-2 1-3 3-3 4.5a3 3 0 0 0 6 0C15 15 14 13 12 12z"/>"#),
    ("power", r#"<path d="M13 2l-4 10h4l-2 10 8-12h-5l3-8z"/>"#),
    ("water", r#"<path d="M12 3s6 7 6 11a6 6 0 0 1-12 0c0-4 6-11 6-11z"/><path d="M9 15c.5.8 1.5 1.2 3 1.2 1.5 0 2.5-.4 3-1.2"/>"#),
    ("population", r#"<circle cx="9" cy="8" r="3"/><path d="M4 19c0-3 2.5-5 5-5s5 2 5 5"/><circle cx="17" cy="10" r="2"/><path d="M15 19c0-2 1.2-3.3 2.5-3.8"/>"#),
    ("happy", r#"<circle cx="12" cy="12" r="9"/><path d="M9 10h.01M15 10h.01M8 14c1 1 2.5 1.5 4 1.5s3-.5 4-1.5"/>"#),
    ("health", r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 9v6M9 12h6"/>"#),
    ("education", r#"<path d="M3 9l9-5 9 5-9 5-9-5z"/><path d="M12 14v7"/><path d="M7 12v4c0 1 2 2 5 2s5-1 5-2v-4"/>"#),
    ("safety", r#"<path d="M12 3l8 4v5c0 4.5-3.5 7.8-8 9-4.5-1.2-8-4.5-8-9V7z"/><path d="M9 12l2 2 4-4"/>"#),
    ("environment", r#"<path d="M12 3c3 3 6 6 6 10a6 6 0 0 1-12 0c0-4 3-7 6-10z"/><path d="M12 10v6M9 14h6"/>"#),
    ("budget", r#"<path d="M4 7h16v10H4z"/><path d="M8 11h4M8 15h8"/><circle cx="16" cy="9" r="1.2"/>"#),
    ("chart", r#"<path d="M4 19h16"/><path d="M7 15l3-4 4 3 3-5"/><path d="M7 10h.01M10 11h.01M14 14h.01M17 9h.01"/>"#),
    ("advisor", r#"<path d="M12 3l7 4-7 4-7-4z"/><path d="M5 11v5l7 4 7-4v-5"/>"#),
    ("settings", r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>"#),
    ("alert", r#"<path d="M12 4l8 14H4z"/><path d="M12 10v4M12 17h.01"/>"#),
    ("info", r#"<circle cx="12" cy="12" r="9"/><path d="M12 10v6M12 7h.01"/>"#),
    ("check", r#"<path d="M5 12l5 5L20 7"/>"#),
    ("subway", r#"<rect x="4" y="8" width="16" height="10" rx="3"/><rect x="6" y="10" width="4" height="4" rx="1"/><rect x="14" y="10" width="4" height="4" rx="1"/><circle cx="8" cy="19" r="1.5"/><circle cx="16" cy="19" r="1.5"/><path d="M10 8V5M14 8V5M10 5h4"/>"#),
    ("search", r#"<path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>"#),
    ("exit", r#"<path d="M7 16l-4-4m0 0l4-4m-4 4h14m-6 4v1a3 3 0 003 3h4a3 3 0 003-3V7a3 3 0 00-3-3h-4a3 3 0 00-3 3v1"/>"#),
    ("chevron", r#"<path d="M9 5l7 7-7 7"/>"#),
    ("sun", r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#),
    ("moon", r#"<path d="M21 13A9 9 0 1 1 11 3a7 7 0 0 0 10 10z"/>"#),
    ("save", r#"<path d="M5 3h11l3 3v15H5z"/><path d="M8 3v5h7V3M8 21v-7h8v7"/>"#),
    ("trash", r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>"#),
    ("users", r#"<circle cx="9" cy="8" r="3"/><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/><path d="M16 4a3 3 0 0 1 0 6M21 20c0-2.6-1.6-4.9-4-5.7"/>"#),
    ("upgrade", r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
];

/// Full SVG document for a named icon.
pub fn svg_document(name: &str) -> Option<String> {
    let body = ICONS.iter().find(|(n, _)| *n == name)?.1;
    Some(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
    ))
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let name = path.trim_start_matches("icons/").trim_end_matches(".svg");
        Ok(svg_document(name).map(|s| Cow::Owned(s.into_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(n, _)| SharedString::from(format!("icons/{n}.svg")))
            .filter(|p| p.starts_with(path))
            .collect())
    }
}

pub fn path(name: &str) -> SharedString {
    format!("icons/{name}.svg").into()
}
