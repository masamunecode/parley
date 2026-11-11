//! Hand-written stroke icons, embedded as SVG and served through GPUI's asset source.
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

macro_rules! stroke_icon {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

macro_rules! fill_icon {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="black" stroke="none">"#,
            $body,
            "</svg>"
        )
    };
}

const ICONS: &[(&str, &str)] = &[
    ("hash", stroke_icon!(r#"<path d="M4.5 9h15M4.5 15h15M10.5 3.5l-2 17M15.5 3.5l-2 17"/>"#)),
    ("volume", stroke_icon!(r#"<path d="M11 5.5L6.5 9H3v6h3.5l4.5 3.5z"/><path d="M15 9a4.2 4.2 0 0 1 0 6"/><path d="M18 6a8.5 8.5 0 0 1 0 12"/>"#)),
    ("mic", stroke_icon!(r#"<rect x="9" y="2.5" width="6" height="11.5" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0"/><path d="M12 17.5V21"/><path d="M8.5 21h7"/>"#)),
    ("mic-off", stroke_icon!(r#"<rect x="9" y="2.5" width="6" height="11.5" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0"/><path d="M12 17.5V21"/><path d="M8.5 21h7"/><path d="M3.5 3.5l17 17"/>"#)),
    ("headphones", stroke_icon!(r#"<path d="M4 17v-5a8 8 0 0 1 16 0v5"/><rect x="3.5" y="14" width="4.5" height="6.5" rx="1.8"/><rect x="16" y="14" width="4.5" height="6.5" rx="1.8"/>"#)),
    ("headphones-off", stroke_icon!(r#"<path d="M4 17v-5a8 8 0 0 1 16 0v5"/><rect x="3.5" y="14" width="4.5" height="6.5" rx="1.8"/><rect x="16" y="14" width="4.5" height="6.5" rx="1.8"/><path d="M3.5 3.5l17 17"/>"#)),
    ("settings", stroke_icon!(r#"<circle cx="12" cy="12" r="6.2"/><circle cx="12" cy="12" r="2.2"/><path d="M12 2.2v3.2M12 18.6v3.2M2.2 12h3.2M18.6 12h3.2M5.1 5.1l2.2 2.2M16.7 16.7l2.2 2.2M5.1 18.9l2.2-2.2M16.7 7.3l2.2-2.2"/>"#)),
    ("plus", stroke_icon!(r#"<path d="M12 5v14M5 12h14"/>"#)),
    ("plus-circle", stroke_icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 8v8M8 12h8"/>"#)),
    ("compass", stroke_icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M15.8 8.2l-2.1 5.5-5.5 2.1 2.1-5.5z"/>"#)),
    ("chevron-down", stroke_icon!(r#"<path d="M6 9.5l6 6 6-6"/>"#)),
    ("chevron-right", stroke_icon!(r#"<path d="M9.5 6l6 6-6 6"/>"#)),
    ("search", stroke_icon!(r#"<circle cx="10.8" cy="10.8" r="6.8"/><path d="M16 16l4.8 4.8"/>"#)),
    ("bell", stroke_icon!(r#"<path d="M6 16.5V11a6 6 0 0 1 12 0v5.5l1.8 2H4.2z"/><path d="M10 21.2h4"/>"#)),
    ("pin", stroke_icon!(r#"<path d="M9 3.5h6l-1 6.5 3.5 3.5h-11L10 10z"/><path d="M12 13.5V21"/>"#)),
    ("users", stroke_icon!(r#"<circle cx="9" cy="8" r="3.6"/><path d="M2.5 20a6.5 6.5 0 0 1 13 0"/><path d="M16 4.6a3.6 3.6 0 0 1 0 6.8"/><path d="M18.5 14.4a6.5 6.5 0 0 1 3 5.6"/>"#)),
    ("inbox", stroke_icon!(r#"<path d="M3.5 13.5L6 5h12l2.5 8.5V19h-17z"/><path d="M3.5 13.5H9a3 3 0 0 0 6 0h5.5"/>"#)),
    ("help", stroke_icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M9.3 9.3a2.8 2.8 0 1 1 4.2 2.4c-.9.6-1.5 1.1-1.5 2.3"/><path d="M12 17.2v.1"/>"#)),
    ("threads", stroke_icon!(r#"<path d="M4.5 8.5h15M4.5 13.5h7"/><path d="M9.5 3.5l-1.5 13"/><path d="M15 3.5l-.7 6"/><path d="M13.5 17.5a4 4 0 0 1 4-4h3v4a3 3 0 0 1-3 3h-4z"/>"#)),
    ("smile", stroke_icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M8.2 14a4.4 4.4 0 0 0 7.6 0"/><path d="M9 9.5v.2M15 9.5v.2"/>"#)),
    ("gift", stroke_icon!(r#"<rect x="3.5" y="8" width="17" height="4.5"/><path d="M5 12.5V20.5h14V12.5"/><path d="M12 8v12.5"/><path d="M12 8C10 3 5.5 4.5 7.5 8M12 8c2-5 6.5-3.500 4.5 0"/>"#)),
    ("sticker", stroke_icon!(r#"<path d="M4 4h16v10l-6 6H4z"/><path d="M20 14h-6v6"/><path d="M8.5 9.500v.2M13.5 9.500v.2"/>"#)),
    ("reply", stroke_icon!(r#"<path d="M9.500 6L4 11.5 9.500 17"/><path d="M4 11.500h9a7 7 0 0 1 7 7v1"/>"#)),
    ("more", fill_icon!(r#"<circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/>"#)),
    ("image", stroke_icon!(r#"<rect x="3" y="4" width="18" height="16" rx="2.500"/><circle cx="8.500" cy="9.500" r="1.800"/><path d="M4 18l5.500-5.500 3.500 3.500 3-3 4.500 4.500"/>"#)),
    ("x", stroke_icon!(r#"<path d="M6 6l12 12M18 6L6 18"/>"#)),
    ("arrow-down", stroke_icon!(r#"<path d="M12 4.500v15M5.500 13l6.500 6.500 6.500-6.500"/>"#)),
    ("signal", stroke_icon!(r#"<path d="M5 19v-3M10 19v-7M15 19v-11M20 19V4"/>"#)),
    ("phone-off", stroke_icon!(r#"<path d="M3 14.500c0-1.500 4-3.500 9-3.500s9 2 9 3.500v1.500c0 .8-.6 1.300-1.400 1.100l-2.800-.7c-.6-.2-1-.7-1-1.300v-1.300c-2.500-.7-5.100-.7-7.600 0v1.300c0 .6-.4 1.100-1 1.300l-2.800.7C3.600 17.300 3 16.800 3 16z"/>"#)),
    ("video", stroke_icon!(r#"<rect x="2.500" y="6" width="13" height="12" rx="2.500"/><path d="M15.500 10.500l6-3.500v10l-6-3.500z"/>"#)),
    ("screen", stroke_icon!(r#"<rect x="2.500" y="4" width="19" height="12.500" rx="2"/><path d="M8 20.500h8M12 16.500v4"/>"#)),
    ("at", stroke_icon!(r#"<circle cx="12" cy="12" r="3.800"/><path d="M15.800 8.500v5a2.600 2.600 0 0 0 5.200 0V12a9 9 0 1 0-3.600 7.200"/>"#)),
    ("bolt", fill_icon!(r#"<path d="M13.500 2L5 13.500h5.500L9.500 22 19 10h-6z"/>"#)),
    ("megaphone", stroke_icon!(r#"<path d="M4 9.500v5l3 .5 1.500 5h2.500l-1-4.700L19.500 18V6L7 9z"/>"#)),
    ("cursor", fill_icon!(r#"<path d="M4 2l0 17.500 4.700-4.300 3.100 7 2.900-1.300-3.100-6.900 6.400-.3z"/>"#)),
    ("cursor-outline", stroke_icon!(r#"<path d="M4 2l0 17.500 4.700-4.300 3.100 7 2.900-1.300-3.100-6.900 6.400-.3z"/>"#)),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let name = path
            .strip_prefix("icons/")
            .and_then(|p| p.strip_suffix(".svg"))
            .unwrap_or(path);
        Ok(ICONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, svg)| Cow::Borrowed(svg.as_bytes())))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(n, _)| SharedString::from(format!("icons/{n}.svg")))
            .collect())
    }
}

/// Asset path for a named icon.
pub fn icon_path(name: &'static str) -> SharedString {
    // Paths are tiny; interning via a static table keeps this allocation-free.
    for (n, p) in PATHS.iter() {
        if *n == name {
            return SharedString::new_static(p);
        }
    }
    SharedString::from(format!("icons/{name}.svg"))
}

macro_rules! paths {
    ($($n:literal),* $(,)?) => {
        const PATHS: &[(&str, &str)] = &[$(($n, concat!("icons/", $n, ".svg"))),*];
    };
}

paths!(
    "hash", "volume", "mic", "mic-off", "headphones", "headphones-off", "settings", "plus",
    "plus-circle", "compass", "chevron-down", "chevron-right", "search", "bell", "pin", "users",
    "inbox", "help", "threads", "smile", "gift", "sticker", "reply", "more", "image", "x",
    "arrow-down", "signal", "phone-off", "video", "screen", "at", "bolt", "megaphone", "cursor",
    "cursor-outline",
);
