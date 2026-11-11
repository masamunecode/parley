//! Cmd+K quick switcher with fuzzy search over channels and servers.
use crate::app::{Parley, SwitchItem, Switcher};
use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, HighlightStyle, MouseButton, SharedString,
    StyledText,
};

const MAX_RESULTS: usize = 9;

impl Parley {
    pub fn open_switcher(&mut self) {
        let mut field = TextField::default();
        field.edited_at = self.t;
        self.switcher = Some(Switcher { field, sel: 0, results: Vec::new() });
        self.update_switcher();
    }

    pub fn update_switcher(&mut self) {
        let Some(query) = self.switcher.as_ref().map(|s| s.field.text.trim().to_string()) else {
            return;
        };
        let mut results: Vec<SwitchItem> = Vec::new();
        if query.is_empty() {
            // Suggestions: unread channels everywhere, then the current server's channels.
            for (si, server) in self.world.servers.iter().enumerate() {
                for (ci, ch) in server.channels.iter().enumerate() {
                    if ch.unread && (si, ci) != self.cur() {
                        let score = 100 + ch.mentions as i32 * 10 - si as i32;
                        results.push(SwitchItem { server: si, channel: Some(ci), positions: vec![], score });
                    }
                }
            }
            results.sort_by(|a, b| b.score.cmp(&a.score));
            results.truncate(5);
            for (ci, ch) in self.world.servers[self.server].channels.iter().enumerate() {
                if !ch.unread && (self.server, ci) != self.cur() && results.len() < MAX_RESULTS {
                    results.push(SwitchItem { server: self.server, channel: Some(ci), positions: vec![], score: 0 });
                }
            }
        } else {
            for (si, server) in self.world.servers.iter().enumerate() {
                if let Some((score, positions)) = fuzzy(&query, &server.name) {
                    results.push(SwitchItem { server: si, channel: None, positions, score: score + 2 });
                }
                for (ci, ch) in server.channels.iter().enumerate() {
                    if let Some((score, positions)) = fuzzy(&query, &ch.name) {
                        let bonus = if si == self.server { 6 } else { 0 };
                        results.push(SwitchItem { server: si, channel: Some(ci), positions, score: score + bonus });
                    }
                }
            }
            results.sort_by(|a, b| b.score.cmp(&a.score).then(a.server.cmp(&b.server)));
            results.truncate(MAX_RESULTS);
        }
        if let Some(sw) = self.switcher.as_mut() {
            sw.sel = sw.sel.min(results.len().saturating_sub(1));
            if !query.is_empty() {
                sw.sel = 0;
            }
            sw.results = results;
        }
    }

    pub fn switcher_accept(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(sw) = self.switcher.take() else {
            return;
        };
        if let Some(item) = sw.results.get(i) {
            self.server = item.server;
            let ch = item.channel.unwrap_or(self.channel_of[item.server]);
            self.select_channel(ch, cx);
        }
        cx.notify();
    }

    pub fn render_switcher(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let sw = self.switcher.as_ref()?;
        let empty_query = sw.field.text.trim().is_empty();

        let mut rows = div().flex().flex_col().px(px(8.)).pb(px(8.));
        for (i, item) in sw.results.iter().enumerate() {
            let server = &self.world.servers[item.server];
            let selected = i == sw.sel;
            let (name, lead, trail): (SharedString, Div, SharedString) = match item.channel {
                Some(ci) => {
                    let ch = &server.channels[ci];
                    let icon_name = match ch.kind {
                        ChanKind::Text => "hash",
                        ChanKind::Announce => "megaphone",
                        ChanKind::Voice => "volume",
                    };
                    (
                        ch.name.clone(),
                        div().child(icon(icon_name, 19., c(if selected { TEXT_BRIGHT } else { TEXT_MUTED }))),
                        server.name.clone(),
                    )
                }
                None => (
                    server.name.clone(),
                    div()
                        .size(px(20.))
                        .rounded(px(6.))
                        .bg(gradient(server.hue))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(white(0.95))
                        .child(server.initials.clone()),
                    SharedString::from("Server"),
                ),
            };
            let highlights: Vec<_> = item
                .positions
                .iter()
                .filter_map(|&p| {
                    let len = name[p..].chars().next()?.len_utf8();
                    Some((
                        p..p + len,
                        HighlightStyle {
                            color: Some(c(ACCENT_SOFT)),
                            font_weight: Some(FontWeight::BOLD),
                            ..Default::default()
                        },
                    ))
                })
                .collect();
            let unread = item.channel.map_or(false, |ci| server.channels[ci].unread);
            let mentions = item.channel.map_or(0, |ci| server.channels[ci].mentions);
            rows = rows.child(
                div()
                    .id(("switch", i))
                    .h(px(40.))
                    .px(px(12.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .map(|d| {
                        if selected {
                            d.bg(white(0.09))
                        } else {
                            d.hover(|s| s.bg(white(0.04)))
                        }
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.switcher_accept(i, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(lead.w(px(20.)).flex_none().flex().justify_center())
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(15.5))
                            .font_weight(if unread { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(c(if selected || unread { TEXT_BRIGHT } else { TEXT }))
                            .child(StyledText::new(name).with_highlights(highlights)),
                    )
                    .children((mentions > 0).then(|| pill_badge(mentions, white(0.))))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.5))
                            .text_color(c(TEXT_FAINT))
                            .child(trail),
                    )
                    .children(selected.then(|| keycap("↵"))),
            );
        }

        if sw.results.is_empty() {
            rows = rows.child(
                div()
                    .h(px(72.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.))
                    .text_color(c(TEXT_MUTED))
                    .child(SharedString::from(format!(
                        "No channels or servers match “{}”",
                        sw.field.text.trim()
                    ))),
            );
        }

        let panel = div()
            .mt(px(118.))
            .w(px(600.))
            .flex()
            .flex_col()
            .rounded(px(14.))
            .bg(c(0x24262e))
            .border_1()
            .border_color(white(0.09))
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .m(px(14.))
                    .h(px(52.))
                    .px(px(14.))
                    .rounded(px(10.))
                    .bg(c(BG_RAIL))
                    .border_1()
                    .border_color(ca(ACCENT, 0.55))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(icon("search", 20., c(TEXT_MUTED)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .text_size(px(18.))
                            .line_height(px(26.))
                            .text_color(c(TEXT_BRIGHT))
                            .child(sw.field.render("Where would you like to go?".into(), self.t, true)),
                    )
                    .child(keycap("esc")),
            )
            .child(
                div()
                    .px(px(20.))
                    .pb(px(6.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(TEXT_FAINT))
                    .child(if empty_query { "SUGGESTED" } else { "CHANNELS & SERVERS" }),
            )
            .child(rows)
            .child(
                div()
                    .h(px(38.))
                    .px(px(18.))
                    .bg(black(0.22))
                    .border_t_1()
                    .border_color(white(0.05))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(c(TEXT_MUTED))
                    .child(keycap("↑"))
                    .child(keycap("↓"))
                    .child("navigate")
                    .child(div().w(px(10.)))
                    .child(keycap("↵"))
                    .child("open")
                    .child(div().flex_1())
                    .child(div().font_weight(FontWeight::BOLD).text_color(c(ACCENT)).child("TIP"))
                    .child("fuzzy matching works: try “gnrl”"),
            );

        Some(
            div()
                .id("switcher-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .bg(black(0.62))
                .flex()
                .justify_center()
                .items_start()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.switcher = None;
                        cx.notify();
                    }),
                )
                .child(panel),
        )
    }
}

fn keycap(label: &'static str) -> Div {
    div()
        .h(px(20.))
        .min_w(px(20.))
        .px(px(6.))
        .rounded(px(5.))
        .bg(white(0.07))
        .border_1()
        .border_color(white(0.08))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .line_height(px(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(TEXT_MUTED))
        .child(label)
}
