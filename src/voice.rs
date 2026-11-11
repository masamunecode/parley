//! Voice channel stage: participant tiles with animated speaking rings.
use crate::app::Parley;
use crate::sidebar::speak_amp;
use crate::theme::*;
use crate::ui::*;
use gpui::{
    div, hsla, linear_color_stop, linear_gradient, prelude::*, px, Context, Div, FontWeight,
    MouseButton, SharedString,
};

impl Parley {
    pub fn render_voice(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.t;
        let ch = self.channel();
        let muted = self.muted || self.deafened;

        let tiles = div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .px(px(32.))
            .children(ch.voice.iter().enumerate().map(|(slot, &u)| {
                let user = &self.world.users[u];
                let amp = speak_amp(t, slot);
                let speaking = amp > 0.12;
                // Expanding pulse that restarts while the participant keeps talking.
                let phase = (t * 1.25 + slot as f32 * 0.37).fract();
                let pulse_d = 104. + phase * 60.;
                let pulse_a = if speaking { (1. - phase) * 0.5 * amp.max(0.4) } else { 0. };
                let ring_w = 3. + amp * 3.;
                let (h1, h2) = (
                    hsla(user.hue, 0.42, 0.24, 1.),
                    hsla((user.hue + 0.08).fract(), 0.5, 0.13, 1.),
                );
                div()
                    .flex_1()
                    .max_w(px(340.))
                    .h(px(250.))
                    .relative()
                    .rounded(px(14.))
                    .bg(linear_gradient(160., linear_color_stop(h1, 0.), linear_color_stop(h2, 1.)))
                    .border_2()
                    .border_color(ca(GREEN, if speaking { 0.35 + 0.65 * amp } else { 0. }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        // Fixed-size stage so the rings never shift the layout.
                        div()
                            .relative()
                            .size(px(170.))
                            .child(
                                div()
                                    .absolute()
                                    .left(px((170. - pulse_d) / 2.))
                                    .top(px((170. - pulse_d) / 2.))
                                    .size(px(pulse_d))
                                    .rounded_full()
                                    .border_2()
                                    .border_color(ca(GREEN, pulse_a)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(35. - ring_w - 3.))
                                    .top(px(35. - ring_w - 3.))
                                    .size(px(100. + 2. * (ring_w + 3.)))
                                    .rounded_full()
                                    .border(px(ring_w))
                                    .border_color(ca(GREEN, if speaking { 0.5 + 0.5 * amp } else { 0. })),
                            )
                            .child(div().absolute().left(px(35.)).top(px(35.)).child(avatar(user, 100.))),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(10.))
                            .bottom(px(10.))
                            .h(px(26.))
                            .px(px(9.))
                            .rounded(px(7.))
                            .bg(black(0.5))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(TEXT_BRIGHT))
                            .child(icon(
                                "mic",
                                13.,
                                if speaking { c(GREEN) } else { c(TEXT_MUTED) },
                            ))
                            .child(user.name.clone()),
                    )
                    .children((slot == 2).then(|| {
                        div()
                            .absolute()
                            .right(px(10.))
                            .top(px(10.))
                            .h(px(18.))
                            .px(px(6.))
                            .rounded(px(9.))
                            .bg(c(RED))
                            .flex()
                            .items_center()
                            .text_size(px(10.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(white(1.))
                            .child("LIVE")
                    }))
                    .child(level_meter(t, slot, amp))
            }));

        let controls = div()
            .h(px(96.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(14.))
            .child(
                stage_button(if muted { "mic-off" } else { "mic" }, muted, false).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.muted = !this.muted;
                        cx.notify();
                    }),
                ),
            )
            .child(stage_button("video", false, false))
            .child(stage_button("screen", false, false))
            .child(stage_button("phone-off", false, true));

        let status = div()
            .flex_none()
            .pb(px(18.))
            .flex()
            .justify_center()
            .items_center()
            .gap(px(8.))
            .text_size(px(13.))
            .text_color(c(TEXT_MUTED))
            .child(icon("signal", 15., c(GREEN)))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(GREEN))
                    .child("Voice connected"),
            )
            .child(SharedString::from(format!(
                "{} / {}  ·  24 ms",
                ch.name, self.world.servers[self.server].name
            )));

        div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .flex_col()
            .bg(c(BG_STAGE))
            .child(self.render_chat_header(cx))
            .child(tiles)
            .child(controls)
            .child(status)
    }
}

/// Small animated audio level bars in the corner of a tile.
fn level_meter(t: f32, slot: usize, amp: f32) -> Div {
    div()
        .absolute()
        .right(px(12.))
        .bottom(px(12.))
        .h(px(22.))
        .flex()
        .items_end()
        .gap(px(2.5))
        .children((0..5).map(|i| {
            let wobble = ((t * (7. + i as f32 * 1.7) + slot as f32 * 2. + i as f32).sin() * 0.5 + 0.5) * amp;
            div()
                .w(px(3.))
                .h(px(3. + 19. * wobble))
                .rounded_full()
                .bg(ca(GREEN, 0.35 + 0.65 * amp))
        }))
}

fn stage_button(name: &'static str, off: bool, hangup: bool) -> Div {
    div()
        .size(px(56.))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .map(|d| {
            if hangup {
                d.bg(c(RED)).hover(|s| s.bg(c(0xff6a66)))
            } else if off {
                d.bg(c(TEXT_BRIGHT)).hover(|s| s.bg(white(0.85)))
            } else {
                d.bg(white(0.09)).hover(|s| s.bg(white(0.15)))
            }
        })
        .child(icon(
            name,
            24.,
            if hangup {
                white(1.)
            } else if off {
                c(RED)
            } else {
                c(TEXT_BRIGHT)
            },
        ))
}
