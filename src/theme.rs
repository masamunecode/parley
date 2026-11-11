//! Parley's dark theme: cool ink surfaces with a warm amber accent.
use gpui::{hsla, rgb, Hsla};

pub fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub fn ca(hex: u32, a: f32) -> Hsla {
    let mut h: Hsla = rgb(hex).into();
    h.a = a;
    h
}

pub fn white(a: f32) -> Hsla {
    hsla(0., 0., 1., a)
}

pub fn black(a: f32) -> Hsla {
    hsla(0., 0., 0., a)
}

// Surfaces
pub const BG_RAIL: u32 = 0x111216;
pub const BG_SIDEBAR: u32 = 0x191a20;
pub const BG_PANEL: u32 = 0x141519;
pub const BG_CHAT: u32 = 0x21232a;
pub const BG_INPUT: u32 = 0x2b2d36;
pub const BG_FLOAT: u32 = 0x0e0f12;
pub const BG_CODE: u32 = 0x17181d;
pub const BG_STAGE: u32 = 0x0b0c0f;

// Text
pub const TEXT: u32 = 0xdcdee4;
pub const TEXT_BRIGHT: u32 = 0xf5f6f8;
pub const TEXT_MUTED: u32 = 0x989ca8;
pub const TEXT_FAINT: u32 = 0x6b6f7b;

// Brand + semantic
pub const ACCENT: u32 = 0xf6a03c;
pub const ACCENT_SOFT: u32 = 0xffc37d;
pub const ACCENT_INK: u32 = 0x2a1802;
pub const GREEN: u32 = 0x3fcf7f;
pub const YELLOW: u32 = 0xf2c14e;
pub const RED: u32 = 0xf0524f;
pub const LINK: u32 = 0x5fb6ff;

pub const UI_FONT: &str = ".SystemUIFont";
pub const MONO_FONT: &str = "Menlo";

// Layout
pub const RAIL_W: f32 = 72.;
pub const SIDEBAR_W: f32 = 240.;
pub const MEMBERS_W: f32 = 244.;
pub const HEADER_H: f32 = 48.;

/// Avatar gradient pair derived from a hue in 0..1.
pub fn avatar_colors(hue: f32) -> (Hsla, Hsla) {
    (
        hsla(hue, 0.62, 0.60, 1.),
        hsla((hue + 0.09).fract(), 0.68, 0.44, 1.),
    )
}
