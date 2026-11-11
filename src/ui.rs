//! Small shared building blocks: icons, avatars, status badges, text field.
use crate::data::{Status, User};
use crate::icons::icon_path;
use crate::theme::*;
use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, svg, Div, FontWeight, Hsla, Keystroke,
    SharedString, Svg,
};

pub fn icon(name: &'static str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(icon_path(name))
        .size(px(size))
        .flex_none()
        .text_color(color)
}

pub fn gradient(hue: f32) -> gpui::Background {
    let (a, b) = avatar_colors(hue);
    linear_gradient(150., linear_color_stop(a, 0.), linear_color_stop(b, 1.))
}

pub fn avatar(user: &User, size: f32) -> Div {
    div()
        .size(px(size))
        .flex_none()
        .rounded_full()
        .bg(gradient(user.hue))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px((size * 0.4).round()))
        .line_height(px(size))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(white(0.95))
        .child(user.initials.clone())
}

pub fn status_color(status: Status) -> Hsla {
    match status {
        Status::Online => c(GREEN),
        Status::Idle => c(YELLOW),
        Status::Dnd => c(RED),
        Status::Offline => c(0x7a7e8a),
    }
}

/// Status dot with a ring in the colour of the surface it sits on.
pub fn status_badge(status: Status, d: f32, ring: Hsla) -> Div {
    let dot = div()
        .size(px(d))
        .rounded_full()
        .bg(status_color(status))
        .relative()
        .flex()
        .items_center()
        .justify_center();
    let dot = match status {
        Status::Online => dot,
        Status::Idle => dot.child(
            div()
                .absolute()
                .top(px(-d * 0.12))
                .left(px(-d * 0.12))
                .size(px(d * 0.62))
                .rounded_full()
                .bg(ring),
        ),
        Status::Dnd => dot.child(div().w(px(d * 0.6)).h(px(d * 0.24)).rounded_full().bg(ring)),
        Status::Offline => dot.child(div().size(px(d * 0.5)).rounded_full().bg(ring)),
    };
    div().p(px(3.)).rounded_full().bg(ring).child(dot)
}

pub fn avatar_with_status(user: &User, size: f32, ring: Hsla) -> Div {
    div()
        .relative()
        .size(px(size))
        .flex_none()
        .child(avatar(user, size))
        .child(
            div()
                .absolute()
                .right(px(-4.))
                .bottom(px(-4.))
                .child(status_badge(user.status, 10., ring)),
        )
}

pub fn pill_badge(count: u32, ring: Hsla) -> Div {
    div().p(px(3.)).rounded_full().bg(ring).child(
        div()
            .h(px(16.))
            .min_w(px(16.))
            .px(px(4.5))
            .rounded_full()
            .bg(c(RED))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.))
            .line_height(px(16.))
            .font_weight(FontWeight::BOLD)
            .text_color(white(1.))
            .child(SharedString::from(count.to_string())),
    )
}

// ---------------------------------------------------------------- text field

#[derive(Default)]
pub struct TextField {
    pub text: String,
    pub cursor: usize,
    pub edited_at: f32,
}

impl TextField {
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn insert(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn prev(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map(|ch| self.cursor + ch.len_utf8())
            .unwrap_or(self.text.len())
    }

    fn word_left(&self) -> usize {
        let before = &self.text[..self.cursor];
        let trimmed = before.trim_end();
        trimmed
            .char_indices()
            .rev()
            .find(|(_, ch)| ch.is_whitespace())
            .map(|(i, ch)| i + ch.len_utf8())
            .unwrap_or(0)
    }

    fn word_right(&self) -> usize {
        let after = &self.text[self.cursor..];
        let skipped = after.len() - after.trim_start().len();
        after[skipped..]
            .find(char::is_whitespace)
            .map(|i| self.cursor + skipped + i)
            .unwrap_or(self.text.len())
    }

    /// Apply an editing keystroke. Returns true if it was consumed.
    pub fn handle(&mut self, ks: &Keystroke) -> bool {
        let m = &ks.modifiers;
        match ks.key.as_str() {
            "backspace" => {
                let to = if m.platform {
                    0
                } else if m.alt {
                    self.word_left()
                } else {
                    self.prev()
                };
                self.text.replace_range(to..self.cursor, "");
                self.cursor = to;
                true
            }
            "delete" => {
                let to = self.next();
                self.text.replace_range(self.cursor..to, "");
                true
            }
            "left" => {
                self.cursor = if m.platform {
                    0
                } else if m.alt {
                    self.word_left()
                } else {
                    self.prev()
                };
                true
            }
            "right" => {
                self.cursor = if m.platform {
                    self.text.len()
                } else if m.alt {
                    self.word_right()
                } else {
                    self.next()
                };
                true
            }
            "home" => {
                self.cursor = 0;
                true
            }
            "end" => {
                self.cursor = self.text.len();
                true
            }
            _ => {
                if m.platform || m.control {
                    return false;
                }
                match ks.key_char.as_deref() {
                    Some(s) if !s.is_empty() && !s.chars().any(|c| c.is_control()) => {
                        self.insert(s);
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Single-line rendering with a zero-width caret between the two halves.
    pub fn render(&self, placeholder: SharedString, t: f32, focused: bool) -> Div {
        let blink = (t - self.edited_at) < 0.6 || (((t - self.edited_at) * 1.9) as i32) % 2 == 0;
        let caret = div().w(px(0.)).h(px(20.)).relative().flex_none().child(
            div()
                .absolute()
                .left(px(-0.5))
                .top_0()
                .w(px(1.5))
                .h_full()
                .rounded_full()
                .bg(if focused && blink { c(TEXT_BRIGHT) } else { white(0.) }),
        );
        let row = div()
            .flex()
            .flex_row()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap();
        if self.text.is_empty() {
            return row
                .child(caret)
                .child(div().text_color(c(TEXT_FAINT)).child(placeholder));
        }
        // Keep the caret in view for long lines by trimming the leading part.
        let mut start = 0;
        let before_chars = self.text[..self.cursor].chars().count();
        if before_chars > 96 {
            start = self.text[..self.cursor]
                .char_indices()
                .nth(before_chars - 96)
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        let nb = |s: &str| SharedString::from(s.replace(' ', "\u{00a0}"));
        row.child(div().flex_none().child(nb(&self.text[start..self.cursor])))
            .child(caret)
            .child(div().flex_none().child(nb(&self.text[self.cursor..])))
    }
}

// ---------------------------------------------------------------- fuzzy

/// Case-insensitive subsequence match. Returns a score and matched byte offsets.
pub fn fuzzy(query: &str, target: &str) -> Option<(i32, Vec<usize>)> {
    let q: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).flat_map(|c| c.to_lowercase()).collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let t: Vec<(usize, char)> = target
        .char_indices()
        .map(|(i, c)| (i, c.to_lowercase().next().unwrap_or(c)))
        .collect();
    let mut best: Option<(i32, Vec<usize>)> = None;
    for start in 0..t.len() {
        if t[start].1 != q[0] {
            continue;
        }
        let mut score = 0i32;
        let mut pos = Vec::with_capacity(q.len());
        let mut ti = start;
        let mut last: Option<usize> = None;
        let mut ok = true;
        for &qc in &q {
            let mut found = None;
            while ti < t.len() {
                if t[ti].1 == qc {
                    found = Some(ti);
                    break;
                }
                ti += 1;
            }
            let Some(f) = found else {
                ok = false;
                break;
            };
            let word_start = f == 0 || !t[f - 1].1.is_alphanumeric();
            score += 10;
            if word_start {
                score += 14;
            }
            if f == 0 {
                score += 10;
            }
            match last {
                Some(l) if f == l + 1 => score += 18,
                Some(l) => score -= (f - l) as i32,
                None => {}
            }
            pos.push(t[f].0);
            last = Some(f);
            ti = f + 1;
        }
        if !ok {
            break;
        }
        score -= t.len() as i32 / 3;
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, pos));
        }
    }
    best
}
