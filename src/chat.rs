//! Chat column: header, virtualized message list, composer and member list.
use crate::app::Parley;
use crate::data::*;
use crate::demo::Target;
use crate::theme::*;
use crate::ui::*;
use gpui::{
    canvas, div, hsla, linear_color_stop, linear_gradient, list, prelude::*, px, uniform_list, AnyElement,
    Bounds, Context, Div, FontWeight, Hsla, ListOffset, MouseButton, MouseDownEvent, Pixels,
    SharedString, StyledText, TextRun, Window,
};
use std::ops::Range;

const GROUP_GAP_SECS: u32 = 420;

fn f(p: Pixels) -> f32 {
    p.into()
}

/// Covers the square corners of a clipped box so its children appear rounded.
fn corner_mask(radius: f32, color: Hsla) -> Div {
    let half = radius / 2.;
    div()
        .absolute()
        .top(px(-half))
        .left(px(-half))
        .right(px(-half))
        .bottom(px(-half))
        .border(px(half))
        .border_color(color)
        .rounded(px(radius * 1.5))
}

fn day_divider(label: String) -> Div {
    div()
        .mt(px(18.))
        .mb(px(-2.))
        .mx(px(16.))
        .h(px(20.))
        .flex()
        .items_center()
        .gap(px(10.))
        .child(div().flex_1().h(px(1.)).bg(white(0.07)))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(TEXT_MUTED))
                .child(SharedString::from(label)),
        )
        .child(div().flex_1().h(px(1.)).bg(white(0.07)))
}

fn tool_button(name: &'static str) -> Div {
    div()
        .size(px(28.))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(white(0.08)))
        .child(icon(name, 17., c(TEXT_MUTED)))
}

impl Parley {
    // ------------------------------------------------------------ scrollbar maths

    /// (viewport bounds, thumb height, fraction 0..1) for the message scrollbar.
    fn scroll_geom(&mut self) -> Option<(Bounds<Pixels>, f32, f32)> {
        let ls = self.list_state();
        let vp = ls.viewport_bounds();
        let h = f(vp.size.height);
        if h < 80. || f(ls.max_offset_for_scrollbar().height) <= 0. {
            return None;
        }
        let count = ls.item_count().max(1) as f32;
        let th = (h * 24. / count).clamp(40., h * 0.5);
        let top = ls.logical_scroll_top();
        let frac = (top.item_ix as f32 / count).clamp(0., 1.);
        Some((vp, th, frac))
    }

    pub fn thumb_center(&mut self) -> Option<(f32, f32)> {
        let (vp, th, frac) = self.scroll_geom()?;
        let h = f(vp.size.height);
        Some((f(vp.right()) - 7., f(vp.top()) + frac * (h - th) + th / 2.))
    }

    pub fn track_point(&mut self, frac: f32) -> Option<(f32, f32)> {
        let (vp, th, _) = self.scroll_geom()?;
        let h = f(vp.size.height);
        Some((f(vp.right()) - 7., f(vp.top()) + frac * (h - th) + th / 2.))
    }

    pub fn scrub_to(&mut self, y: Pixels) {
        let Some((vp, th, _)) = self.scroll_geom() else {
            return;
        };
        let h = f(vp.size.height);
        let frac = ((f(y) - f(vp.top()) - th / 2.) / (h - th)).clamp(0., 1.);
        let ls = self.list_state();
        let count = ls.item_count();
        if frac > 0.9995 {
            ls.scroll_to(ListOffset { item_ix: count, offset_in_item: px(0.) });
        } else {
            let ix = (frac as f64 * count as f64) as usize;
            ls.scroll_to(ListOffset { item_ix: ix.min(count), offset_in_item: px(0.) });
        }
    }

    // ------------------------------------------------------------ chat column

    pub fn render_chat(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ls = self.list_state();
        let geom = self.scroll_geom();
        let count = ls.item_count();
        let top = ls.logical_scroll_top();
        let far_from_present = top.item_ix + 30 < count;
        let dragging = self.scroll_drag;

        let list_el = list(
            ls.clone(),
            cx.processor(|this, ix: usize, window, cx| this.render_message(ix, window, cx)),
        )
        .size_full()
        .pb(px(26.));

        let scrollbar = geom.map(|(vp, th, frac)| {
            let h = f(vp.size.height);
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(14.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                        this.scroll_drag = true;
                        this.scrub_to(ev.position.y);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(frac * (h - th)))
                        .right(px(3.))
                        .w(px(8.))
                        .h(px(th))
                        .rounded_full()
                        .bg(if dragging { c(ACCENT) } else { white(0.16) })
                        .hover(|s| s.bg(white(0.3))),
                )
        });

        let scrub_label = match (dragging, geom) {
            (true, Some((vp, th, frac))) if top.item_ix >= 1 && top.item_ix <= self.channel().metas.len() => {
                let h = f(vp.size.height);
                let ts = self.channel().metas[top.item_ix - 1].ts;
                Some(
                    div()
                        .absolute()
                        .right(px(20.))
                        .top(px(frac * (h - th) + th / 2. - 15.))
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(c(BG_FLOAT))
                        .border_1()
                        .border_color(white(0.08))
                        .shadow_lg()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(TEXT_BRIGHT))
                        .child(SharedString::from(self.world.clock.long_date(ts)))
                        .child(
                            div()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(c(TEXT_MUTED))
                                .child(SharedString::from(format!(
                                    "#{}",
                                    group_digits(top.item_ix)
                                ))),
                        ),
                )
            }
            _ => None,
        };

        let jump = far_from_present.then(|| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom(px(10.))
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("jump")
                        .relative()
                        .h(px(34.))
                        .pl(px(14.))
                        .pr(px(10.))
                        .rounded_full()
                        .bg(c(ACCENT))
                        .shadow_lg()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .cursor_pointer()
                        .hover(|s| s.bg(c(0xffb255)))
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(ACCENT_INK))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.jump_to_present();
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                        .child("You're viewing older messages")
                        .child(div().w(px(1.)).h(px(14.)).bg(ca(ACCENT_INK, 0.35)))
                        .child("Jump to present")
                        .child(icon("arrow-down", 15., c(ACCENT_INK)))
                        .children(self.mark(Target::Jump)),
                )
        });

        let body = div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .relative()
                    .child(list_el)
                    .children(scrollbar)
                    .children(scrub_label)
                    .children(jump),
            )
            .child(self.render_composer(cx));

        div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .flex_col()
            .bg(c(BG_CHAT))
            .child(self.render_chat_header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_row()
                    .child(body)
                    .children(self.show_members.then(|| self.render_members(cx))),
            )
    }

    pub fn render_chat_header(&self, cx: &mut Context<Self>) -> Div {
        let ch = self.channel();
        let voice = ch.kind == ChanKind::Voice;
        let members_on = self.show_members && !voice;
        let header_icon = |name: &'static str, on: bool| {
            div()
                .size(px(30.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(white(0.06)))
                .child(icon(name, 20., c(if on { TEXT_BRIGHT } else { TEXT_MUTED })))
        };
        div()
            .h(px(HEADER_H))
            .flex_none()
            .px(px(16.))
            .flex()
            .items_center()
            .gap(px(8.))
            .border_b_1()
            .border_color(black(0.28))
            .child(icon(
                match ch.kind {
                    ChanKind::Text => "hash",
                    ChanKind::Announce => "megaphone",
                    ChanKind::Voice => "volume",
                },
                22.,
                c(TEXT_FAINT),
            ))
            .child(
                div()
                    .flex_none()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(TEXT_BRIGHT))
                    .child(ch.name.clone()),
            )
            .child(div().w(px(1.)).h(px(22.)).mx(px(8.)).flex_none().bg(white(0.1)))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(13.5))
                    .text_color(c(TEXT_MUTED))
                    .child(if voice {
                        SharedString::from("Voice channel · 3 connected")
                    } else {
                        ch.topic.clone()
                    }),
            )
            .child(header_icon("threads", false))
            .child(header_icon("bell", false))
            .child(header_icon("pin", false))
            .child(
                header_icon("users", members_on)
                    .relative()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.show_members = !this.show_members;
                            cx.notify();
                        }),
                    )
                    .children(self.mark(Target::MembersToggle)),
            )
            .child(
                div()
                    .ml(px(4.))
                    .w(px(190.))
                    .h(px(30.))
                    .flex_none()
                    .rounded(px(7.))
                    .bg(c(BG_RAIL))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.5))
                    .text_color(c(TEXT_FAINT))
                    .child(SharedString::from(format!(
                        "Search {}",
                        self.world.servers[self.server].name
                    )))
                    .child(icon("search", 15., c(TEXT_FAINT))),
            )
    }

    // ------------------------------------------------------------ messages

    fn text_runs(&self, content: &Content) -> Vec<TextRun> {
        content
            .spans
            .iter()
            .map(|&(len, span)| {
                let (font, color, bg) = match span {
                    Span::Plain => (&self.fonts.ui, c(TEXT), None),
                    Span::Mention => (&self.fonts.medium, c(ACCENT_SOFT), Some(ca(ACCENT, 0.2))),
                    Span::Code => (&self.fonts.mono, c(0xe9ebf0), Some(c(BG_CODE))),
                    Span::Link => (&self.fonts.ui, c(LINK), None),
                    Span::Muted => (&self.fonts.ui, c(TEXT_FAINT), None),
                };
                TextRun {
                    len,
                    font: font.clone(),
                    color,
                    background_color: bg,
                    underline: None,
                    strikethrough: None,
                }
            })
            .collect()
    }

    fn code_block(&self, code: &Code) -> Div {
        let runs: Vec<TextRun> = code
            .spans
            .iter()
            .map(|&(len, tok)| TextRun {
                len,
                font: self.fonts.mono.clone(),
                color: c(match tok {
                    Tok::Plain => 0xd5d8e0,
                    Tok::Keyword => 0xf6a03c,
                    Tok::Str => 0x9fd68b,
                    Tok::Comment => 0x6b7080,
                    Tok::Num => 0x7fc8f8,
                    Tok::Type => 0x7edbc9,
                }),
                background_color: None,
                underline: None,
                strikethrough: None,
            })
            .collect();
        div()
            .mt(px(6.))
            .mb(px(2.))
            .max_w(px(680.))
            .relative()
            .rounded(px(8.))
            .bg(c(BG_CODE))
            .border_1()
            .border_color(white(0.07))
            .px(px(14.))
            .py(px(10.))
            .text_size(px(13.))
            .line_height(px(19.5))
            .child(StyledText::new(code.text.clone()).with_runs(runs))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .right(px(10.))
                    .text_size(px(10.5))
                    .line_height(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(TEXT_FAINT))
                    .child(code.lang),
            )
    }

    fn image_block(&self, img: &Image) -> Div {
        let a = hsla(img.hue, 0.5, 0.46, 1.);
        let b = hsla((img.hue + 0.13).fract(), 0.55, 0.2, 1.);
        let (w, h) = (img.w, img.h);
        let scene = match img.style {
            0 => div()
                .child(
                    div()
                        .absolute()
                        .right(px(w * 0.16))
                        .top(px(h * 0.15))
                        .size(px(h * 0.22))
                        .rounded_full()
                        .bg(white(0.28)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(-w * 0.25))
                        .top(px(h * 0.58))
                        .size(px(w * 0.95))
                        .rounded_full()
                        .bg(black(0.2)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(w * 0.38))
                        .top(px(h * 0.7))
                        .size(px(w * 0.9))
                        .rounded_full()
                        .bg(black(0.3)),
                ),
            1 => div()
                .child(
                    div()
                        .absolute()
                        .left(px(w * 0.08))
                        .top(px(h * 0.14))
                        .w(px(w * 0.52))
                        .h(px(h * 0.5))
                        .rounded(px(8.))
                        .bg(black(0.22)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(w * 0.36))
                        .top(px(h * 0.34))
                        .w(px(w * 0.54))
                        .h(px(h * 0.5))
                        .rounded(px(8.))
                        .bg(white(0.16)),
                ),
            _ => div().children((0..5).map(|i| {
                let bar_h = h * (0.25 + 0.13 * ((i * 7 + 3) % 5) as f32);
                div()
                    .absolute()
                    .left(px(w * (0.12 + 0.16 * i as f32)))
                    .top(px(h * 0.86 - bar_h))
                    .w(px(w * 0.1))
                    .h(px(bar_h))
                    .rounded(px(4.))
                    .bg(white(0.12 + 0.05 * i as f32))
            })),
        };
        div()
            .mt(px(6.))
            .mb(px(2.))
            .w(px(w))
            .h(px(h))
            .relative()
            .overflow_hidden()
            .bg(linear_gradient(165., linear_color_stop(a, 0.), linear_color_stop(b, 1.)))
            .child(scene.absolute().top_0().left_0().size_full())
            .child(
                div()
                    .absolute()
                    .left(px(10.))
                    .bottom(px(10.))
                    .h(px(22.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(black(0.5))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .line_height(px(14.))
                    .text_color(white(0.85))
                    .child(icon("image", 13., white(0.8)))
                    .child(img.name.clone()),
            )
            .child(corner_mask(8., c(BG_CHAT)))
    }

    fn reactions_row(&self, reactions: &[Reaction]) -> Div {
        div()
            .mt(px(5.))
            .mb(px(2.))
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(4.))
            .children(reactions.iter().map(|r| {
                div()
                    .h(px(26.))
                    .px(px(7.))
                    .rounded(px(9.))
                    .border_1()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .map(|d| {
                        if r.mine {
                            d.bg(ca(ACCENT, 0.16)).border_color(ca(ACCENT, 0.65))
                        } else {
                            d.bg(white(0.05))
                                .border_color(white(0.0))
                                .hover(|s| s.border_color(white(0.18)))
                        }
                    })
                    .child(div().text_size(px(15.)).line_height(px(20.)).child(r.emoji))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(if r.mine { ACCENT_SOFT } else { TEXT_MUTED }))
                            .child(SharedString::from(r.count.to_string())),
                    )
            }))
    }

    fn render_welcome(&self) -> AnyElement {
        let ch = self.channel();
        div()
            .w_full()
            .px(px(16.))
            .pt(px(48.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .size(px(68.))
                    .rounded_full()
                    .bg(white(0.08))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("hash", 40., c(TEXT_BRIGHT))),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_size(px(30.))
                    .line_height(px(38.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(TEXT_BRIGHT))
                    .child(SharedString::from(format!("Welcome to #{}!", ch.name))),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .text_color(c(TEXT_MUTED))
                    .child(SharedString::from(format!(
                        "This is the start of the #{} channel. {}",
                        ch.name, ch.topic
                    ))),
            )
            .into_any_element()
    }

    pub fn render_message(
        &mut self,
        item: usize,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        if item == 0 {
            return self.render_welcome();
        }
        let ix = item - 1;
        let (si, ci) = self.cur();
        let (meta, prev) = {
            let metas = &self.world.servers[si].channels[ci].metas;
            let Some(meta) = metas.get(ix).copied() else {
                return div().into_any_element();
            };
            (meta, if ix > 0 { Some(metas[ix - 1]) } else { None })
        };
        let clock = self.world.clock;
        let content = self.content(ix);
        let new_day = prev.map_or(true, |p| clock.day(p.ts) != clock.day(meta.ts));
        let starts = new_day
            || content.reply_to.is_some()
            || prev.map_or(true, |p| {
                p.author != meta.author || meta.ts.saturating_sub(p.ts) > GROUP_GAP_SECS
            });
        let reply = content.reply_to.map(|r| {
            let author = self.world.servers[si].channels[ci].metas[r].author as usize;
            (author, self.content(r).preview.clone())
        });
        let user = &self.world.users[meta.author as usize];

        let mut col = div().flex_1().min_w(px(0.)).flex().flex_col();

        if let Some((rauthor, preview)) = &reply {
            let ru = &self.world.users[*rauthor];
            col = col.child(
                div()
                    .relative()
                    .h(px(20.))
                    .mb(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .child(
                        div()
                            .absolute()
                            .left(px(-37.))
                            .top(px(9.))
                            .w(px(31.))
                            .h(px(13.))
                            .border_l_2()
                            .border_t_2()
                            .border_color(white(0.16))
                            .rounded_tl(px(7.)),
                    )
                    .child(avatar(ru, 16.))
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(ca(ru.name_color(), 0.85))
                            .child(SharedString::from(format!("@{}", ru.name))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_color(c(TEXT_MUTED))
                            .child(preview.clone()),
                    ),
            );
        }

        if starts {
            col = col.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .h(px(22.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(user.name_color()))
                            .child(user.name.clone()),
                    )
                    .children(user.bot.then(|| {
                        div()
                            .h(px(15.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(c(ACCENT))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .text_size(px(10.))
                            .line_height(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c(ACCENT_INK))
                            .child(icon("bolt", 9., c(ACCENT_INK)))
                            .child("BOT")
                    }))
                    .child(
                        div()
                            .pt(px(2.))
                            .text_size(px(12.))
                            .text_color(c(TEXT_FAINT))
                            .child(SharedString::from(clock.stamp(meta.ts))),
                    ),
            );
        }

        if !content.text.is_empty() {
            col = col.child(
                div()
                    .text_size(px(15.))
                    .line_height(px(22.))
                    .child(StyledText::new(content.text.clone()).with_runs(self.text_runs(&content))),
            );
        }
        if let Some(code) = &content.code {
            col = col.child(self.code_block(code));
        }
        if let Some(img) = &content.image {
            col = col.child(self.image_block(img));
        }
        if !content.reactions.is_empty() {
            col = col.child(self.reactions_row(&content.reactions));
        }

        let gutter = if starts {
            div()
                .absolute()
                .left(px(16.))
                .top(px(if reply.is_some() { 26. } else { 4. }))
                .child(avatar(user, 40.))
        } else {
            div()
                .absolute()
                .left_0()
                .top(px(2.))
                .w(px(72.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(c(TEXT_FAINT))
                .invisible()
                .group_hover("msg", |s| s.visible())
                .child(SharedString::from(clock.time(meta.ts)))
        };

        let toolbar = div()
            .absolute()
            .right(px(24.))
            .top(px(-16.))
            .h(px(32.))
            .px(px(2.))
            .rounded(px(7.))
            .bg(c(BG_CHAT))
            .border_1()
            .border_color(white(0.09))
            .shadow_md()
            .flex()
            .items_center()
            .invisible()
            .group_hover("msg", |s| s.visible())
            .child(tool_button("smile"))
            .child(tool_button("reply"))
            .child(tool_button("more"));

        // Demo mode: record this row's bounds and honour the simulated hover.
        let hot = self.hot_msg == Some(item);
        let (gutter, toolbar) = if hot {
            (gutter.visible(), toolbar.visible())
        } else {
            (gutter, toolbar)
        };
        let recorder = self.demo.is_some().then(|| {
            let rows = self.msg_bounds.clone();
            canvas(move |bounds, _, _| rows.borrow_mut().push((item, bounds)), |_, _, _, _| {})
                .absolute()
                .top_0()
                .left_0()
                .size_full()
        });

        let mentions_me = content.mentions_me;
        let row = div()
            .group("msg")
            .relative()
            .w_full()
            .flex()
            .flex_row()
            .pl(px(72.))
            .pr(px(48.))
            .py(px(1.5))
            .when(starts, |d| d.mt(px(14.)))
            .map(|d| {
                if mentions_me {
                    d.bg(ca(ACCENT, if hot { 0.12 } else { 0.085 }))
                        .hover(|s| s.bg(ca(ACCENT, 0.12)))
                        .child(div().absolute().left_0().top_0().bottom_0().w(px(2.)).bg(c(ACCENT)))
                } else if hot {
                    d.bg(black(0.085))
                } else {
                    d.hover(|s| s.bg(black(0.085)))
                }
            })
            .children(recorder)
            .child(gutter)
            .child(col)
            .child(toolbar);

        if new_day {
            div()
                .w_full()
                .child(day_divider(clock.long_date(meta.ts)))
                .child(row)
                .into_any_element()
        } else {
            row.into_any_element()
        }
    }

    // ------------------------------------------------------------ composer

    fn render_typing(&self) -> Option<Div> {
        if self.cur() != (0, self.world.servers[0].default_channel) {
            return None;
        }
        let names: Vec<SharedString> = self
            .typists
            .iter()
            .flatten()
            .map(|(u, _)| self.world.users[*u as usize].name.clone())
            .collect();
        if names.is_empty() {
            return None;
        }
        let t = self.t;
        let dots = div().flex().items_center().gap(px(3.)).h(px(12.)).children((0..3).map(|i| {
            let p = (t * 6.5 - i as f32 * 0.95).sin() * 0.5 + 0.5;
            div()
                .relative()
                .top(px(1.5 - 3. * p))
                .size(px(5.))
                .rounded_full()
                .bg(white(0.35 + 0.6 * p))
        }));
        let bold = |s: SharedString| {
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(TEXT_BRIGHT))
                .child(s)
        };
        let mut row = div()
            .absolute()
            .left(px(18.))
            .bottom(px(4.))
            .h(px(18.))
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(12.5))
            .line_height(px(18.))
            .text_color(c(TEXT_MUTED))
            .child(dots.mr(px(3.)));
        if names.len() == 1 {
            row = row.child(bold(names[0].clone())).child("is typing…");
        } else {
            row = row
                .child(bold(names[0].clone()))
                .child("and")
                .child(bold(names[1].clone()))
                .child("are typing…");
        }
        Some(row)
    }

    fn render_composer(&self, cx: &mut Context<Self>) -> Div {
        let ch = self.channel();
        let placeholder = SharedString::from(format!("Message #{}", ch.name));
        let focused = self.switcher.is_none();
        div()
            .flex_none()
            .relative()
            .px(px(16.))
            .pb(px(24.))
            .child(
                div()
                    .id("composer")
                    .relative()
                    .h(px(46.))
                    .rounded(px(10.))
                    .bg(c(BG_INPUT))
                    .border_1()
                    .border_color(white(0.06))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .cursor_text()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            window.focus(&this.focus);
                            this.composer.edited_at = this.t;
                            cx.notify();
                        }),
                    )
                    .child(icon("plus-circle", 22., c(TEXT_MUTED)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .child(self.composer.render(placeholder, self.t, focused)),
                    )
                    .child(icon("gift", 21., c(TEXT_MUTED)))
                    .child(icon("sticker", 21., c(TEXT_MUTED)))
                    .child(icon("smile", 21., c(TEXT_MUTED)))
                    .children(self.mark(Target::Composer)),
            )
            .children(self.render_typing())
    }

    // ------------------------------------------------------------ members

    fn render_members(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.world.servers[self.server].rows.len();
        div()
            .w(px(MEMBERS_W))
            .flex_none()
            .h_full()
            .relative()
            .bg(c(BG_SIDEBAR))
            .child(
                uniform_list(
                    ("members", self.server),
                    count,
                    cx.processor(|this, range: Range<usize>, _window, _cx| {
                        range.map(|i| this.member_row(i)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(self.members_scroll[self.server].clone())
                .size_full(),
            )
            .children(self.mark(Target::Members))
    }

    fn member_row(&self, i: usize) -> Div {
        let row = &self.world.servers[self.server].rows[i];
        match row {
            MemberRow::Header(label) => div()
                .h(px(44.))
                .px(px(16.))
                .pb(px(5.))
                .flex()
                .items_end()
                .text_size(px(11.5))
                .font_weight(FontWeight::BOLD)
                .text_color(c(TEXT_FAINT))
                .child(SharedString::from(label.to_uppercase())),
            MemberRow::User(u) => {
                let user = &self.world.users[*u as usize];
                let offline = user.status == Status::Offline;
                div().h(px(44.)).px(px(8.)).child(
                    div()
                        .h(px(42.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .cursor_pointer()
                        .when(offline, |d| d.opacity(0.42))
                        .hover(|s| s.bg(white(0.05)).opacity(1.))
                        .child(avatar_with_status(user, 32., c(BG_SIDEBAR)))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(15.))
                                                .line_height(px(19.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(c(user.name_color()))
                                                .child(user.name.clone()),
                                        )
                                        .children(user.bot.then(|| {
                                            div()
                                                .h(px(14.))
                                                .px(px(4.))
                                                .rounded(px(4.))
                                                .bg(c(ACCENT))
                                                .flex()
                                                .items_center()
                                                .text_size(px(9.5))
                                                .line_height(px(14.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(c(ACCENT_INK))
                                                .child("BOT")
                                        })),
                                )
                                .children(user.activity.clone().map(|a| {
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .line_height(px(15.))
                                        .text_color(c(TEXT_MUTED))
                                        .child(a)
                                })),
                        ),
                )
            }
        }
    }
}

pub fn group_digits(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
