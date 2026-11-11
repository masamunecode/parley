//! Server rail, channel list and the user panel.
use crate::app::{logo_mark, Parley};
use crate::data::*;
use crate::demo::Target;
use crate::theme::*;
use crate::ui::*;
use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, AnyElement, Context, Div, FontWeight,
    MouseButton, SharedString,
};

/// Smooth 0..1 speaking amplitude for a voice participant.
pub fn speak_amp(t: f32, slot: usize) -> f32 {
    let s = slot as f32;
    let gate = (t * 0.55 + s * 2.1).sin() + (t * 0.23 + s * 4.7).sin() * 0.7;
    let on = ((gate + 0.35) / 0.4).clamp(0., 1.);
    let syllables = 0.5 + 0.5 * ((t * 9.0 + s * 1.3).sin() * (t * 5.3 + s).sin()).abs();
    on * syllables
}

impl Parley {
    // ------------------------------------------------------------ rail

    pub fn render_rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let home_hover = self.hover_server == Some(usize::MAX);
        let mut col = div()
            .id("rail")
            .w(px(RAIL_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.))
            .pt(px(4.))
            .pb(px(8.))
            .overflow_y_scroll()
            .child(
                div()
                    .id("home")
                    .size(px(48.))
                    .flex_none()
                    .rounded(px(if home_hover { 16. } else { 24. }))
                    .bg(linear_gradient(
                        150.,
                        linear_color_stop(c(0xffb85c), 0.),
                        linear_color_stop(c(0xee7f2b), 1.),
                    ))
                    .cursor_pointer()
                    .on_hover(cx.listener(|this, h: &bool, _, cx| {
                        if *h {
                            this.hover_server = Some(usize::MAX);
                        } else if this.hover_server == Some(usize::MAX) {
                            this.hover_server = None;
                        }
                        cx.notify();
                    }))
                    .child(logo_mark(48., c(ACCENT_INK), c(TEXT_BRIGHT), c(0xf59638))),
            )
            .child(div().w(px(32.)).h(px(2.)).flex_none().rounded_full().bg(white(0.09)));

        for i in 0..self.world.servers.len() {
            col = col.child(self.server_button(i, cx));
        }

        col.child(rail_action("plus", "add-server"))
            .child(rail_action("compass", "discover"))
    }

    fn server_button(&self, i: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.world.servers[i];
        let selected = i == self.server;
        let hovered = self.hover_server == Some(i);
        let unread = s.unread();
        let mentions = s.mentions();
        let pill_h = if selected {
            40.
        } else if hovered {
            20.
        } else if unread {
            8.
        } else {
            0.
        };
        div()
            .id(("server", i))
            .relative()
            .w_full()
            .h(px(48.))
            .flex_none()
            .flex()
            .justify_center()
            .on_hover(cx.listener(move |this, h: &bool, _, cx| {
                if *h {
                    this.hover_server = Some(i);
                } else if this.hover_server == Some(i) {
                    this.hover_server = None;
                }
                cx.notify();
            }))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px((48. - pill_h) / 2.))
                    .w(px(4.))
                    .h(px(pill_h))
                    .rounded_r(px(4.))
                    .bg(c(TEXT_BRIGHT)),
            )
            .child(
                div()
                    .size(px(48.))
                    .relative()
                    .rounded(px(if selected || hovered { 16. } else { 24. }))
                    .bg(gradient(s.hue))
                    .when(!selected && !hovered, |d| d.opacity(0.88))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(white(0.96))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.select_server(i, cx)),
                    )
                    .child(s.initials.clone())
                    .children(self.mark(Target::Server(i))),
            )
            .children((mentions > 0).then(|| {
                div()
                    .absolute()
                    .right(px(7.))
                    .bottom(px(-5.))
                    .child(pill_badge(mentions, c(BG_RAIL)))
            }))
    }

    // ------------------------------------------------------------ channel list

    pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let si = self.server;
        let server = &self.world.servers[si];

        let header = div()
            .h(px(HEADER_H))
            .flex_none()
            .px(px(16.))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(black(0.35))
            .cursor_pointer()
            .hover(|s| s.bg(white(0.03)))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(15.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(TEXT_BRIGHT))
                    .child(server.name.clone()),
            )
            .child(icon("chevron-down", 16., c(TEXT_MUTED)));

        let mut list = div()
            .id(("channels", si))
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .pb(px(16.));

        for (cat_ix, cat) in server.cats.iter().enumerate() {
            let collapsed = self.collapsed.contains(&(si, cat_ix));
            list = list.child(
                div()
                    .id(("cat", cat_ix))
                    .mt(px(14.))
                    .h(px(24.))
                    .pl(px(6.))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            let key = (this.server, cat_ix);
                            if !this.collapsed.remove(&key) {
                                this.collapsed.insert(key);
                            }
                            cx.notify();
                        }),
                    )
                    .child(icon(
                        if collapsed { "chevron-right" } else { "chevron-down" },
                        11.,
                        c(TEXT_FAINT),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c(TEXT_FAINT))
                            .child(SharedString::from(cat.to_uppercase())),
                    )
                    .child(icon("plus", 14., c(TEXT_FAINT))),
            );
            for (ci, ch) in server.channels.iter().enumerate() {
                if ch.cat != cat_ix {
                    continue;
                }
                let selected = self.channel_of[si] == ci;
                if collapsed && !selected && !ch.unread {
                    continue;
                }
                list = list.child(self.channel_row(ci, cx));
                if ch.kind == ChanKind::Voice && !collapsed {
                    for (slot, &u) in ch.voice.iter().enumerate() {
                        list = list.child(self.voice_user_row(u, slot));
                    }
                }
            }
        }

        div()
            .w(px(SIDEBAR_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(c(BG_SIDEBAR))
            .rounded_tl(px(10.))
            .overflow_hidden()
            .child(header)
            .child(list)
            .child(self.render_user_panel(cx))
    }

    fn channel_row(&self, ci: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let si = self.server;
        let ch = &self.world.servers[si].channels[ci];
        let selected = self.channel_of[si] == ci;
        let unread = ch.unread && !selected;
        let icon_name = match ch.kind {
            ChanKind::Text => "hash",
            ChanKind::Announce => "megaphone",
            ChanKind::Voice => "volume",
        };
        let hot = self.hot(Target::Channel(ci));
        let name_color = if selected || unread || hot { TEXT_BRIGHT } else { TEXT_MUTED };
        div()
            .id(("chan", ci))
            .relative()
            .mx(px(8.))
            .mt(px(2.))
            .h(px(32.))
            .px(px(8.))
            .rounded(px(6.))
            .flex()
            .items_center()
            .gap(px(7.))
            .cursor_pointer()
            .map(|d| {
                if selected {
                    d.bg(white(0.095))
                } else if hot {
                    d.bg(white(0.045))
                } else {
                    d.hover(|s| s.bg(white(0.045)))
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_channel(ci, cx)),
            )
            .children(unread.then(|| {
                div()
                    .absolute()
                    .left(px(-8.))
                    .top(px(12.))
                    .w(px(4.))
                    .h(px(8.))
                    .rounded_r(px(4.))
                    .bg(c(TEXT_BRIGHT))
            }))
            .child(icon(icon_name, 18., c(if selected { TEXT } else { TEXT_FAINT })))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(15.))
                    .font_weight(if selected || unread { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                    .text_color(c(name_color))
                    .child(ch.name.clone()),
            )
            .children((ch.mentions > 0 && !selected).then(|| {
                div()
                    .h(px(16.))
                    .min_w(px(16.))
                    .px(px(5.))
                    .rounded_full()
                    .bg(c(RED))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(white(1.))
                    .child(SharedString::from(ch.mentions.to_string()))
            }))
            .children(self.mark(Target::Channel(ci)))
    }

    fn voice_user_row(&self, u: usize, slot: usize) -> impl IntoElement {
        let user = &self.world.users[u];
        let amp = speak_amp(self.t, slot);
        let speaking = amp > 0.12;
        div()
            .ml(px(34.))
            .mr(px(8.))
            .mt(px(1.))
            .h(px(32.))
            .px(px(6.))
            .rounded(px(6.))
            .flex()
            .items_center()
            .gap(px(8.))
            .hover(|s| s.bg(white(0.045)))
            .child(
                div()
                    .size(px(28.))
                    .flex_none()
                    .rounded_full()
                    .border_2()
                    .border_color(ca(GREEN, if speaking { 0.55 + 0.45 * amp } else { 0. }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(avatar(user, 22.)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(14.))
                    .text_color(c(if speaking { TEXT_BRIGHT } else { TEXT_MUTED }))
                    .child(user.name.clone()),
            )
            .children((slot == 1).then(|| icon("video", 14., c(TEXT_FAINT))))
            .children((slot == 2).then(|| {
                div()
                    .px(px(5.))
                    .h(px(15.))
                    .rounded(px(8.))
                    .bg(c(RED))
                    .flex()
                    .items_center()
                    .text_size(px(9.5))
                    .line_height(px(15.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(white(1.))
                    .child("LIVE")
            }))
    }

    // ------------------------------------------------------------ user panel

    fn render_user_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let me = &self.world.users[0];
        let muted = self.muted || self.deafened;
        div()
            .h(px(56.))
            .flex_none()
            .bg(c(BG_PANEL))
            .border_t_1()
            .border_color(black(0.3))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .h(px(42.))
                    .px(px(4.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .hover(|s| s.bg(white(0.06)))
                    .child(avatar_with_status(me, 32., c(BG_PANEL)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(14.))
                                    .line_height(px(18.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(c(TEXT_BRIGHT))
                                    .child(me.name.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(15.))
                                    .text_color(c(TEXT_MUTED))
                                    .child(if self.deafened {
                                        "Deafened"
                                    } else if self.muted {
                                        "Muted"
                                    } else {
                                        "Online"
                                    }),
                            ),
                    ),
            )
            .child(
                panel_button("btn-mic", if muted { "mic-off" } else { "mic" }, muted)
                    .when(self.hot(Target::Mic), |d| {
                        d.bg(if muted { ca(RED, 0.22) } else { white(0.08) })
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_mute();
                            cx.notify();
                        }),
                    )
                    .children(self.mark(Target::Mic)),
            )
            .child(
                panel_button(
                    "btn-deafen",
                    if self.deafened { "headphones-off" } else { "headphones" },
                    self.deafened,
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.deafened = !this.deafened;
                        cx.notify();
                    }),
                )
                .children(self.mark(Target::Deafen)),
            )
            .child(panel_button("btn-settings", "settings", false))
    }
}

fn panel_button(id: &'static str, icon_name: &'static str, danger: bool) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .relative()
        .size(px(32.))
        .flex_none()
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .map(|d| {
            if danger {
                d.bg(ca(RED, 0.14)).hover(|s| s.bg(ca(RED, 0.22)))
            } else {
                d.hover(|s| s.bg(white(0.08)))
            }
        })
        .child(icon(icon_name, 19., c(if danger { RED } else { TEXT_MUTED })))
}

fn rail_action(icon_name: &'static str, id: &'static str) -> AnyElement {
    div()
        .id(id)
        .group(id)
        .size(px(48.))
        .flex_none()
        .rounded(px(24.))
        .bg(c(BG_CHAT))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(c(GREEN)).rounded(px(16.)))
        .child(icon(icon_name, 22., c(GREEN)).group_hover(id, |s| s.text_color(white(1.))))
        .into_any_element()
}
