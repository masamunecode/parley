//! `--hud`: frame statistics and process memory overlay.
use crate::app::{Parley, TITLE_H};
use crate::data::ChanKind;
use crate::theme::*;
use gpui::{canvas, div, fill, point, prelude::*, px, size, Bounds, Div, FontWeight, SharedString};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Instant;

const SAMPLES: usize = 120;

pub struct Hud {
    last: Option<Instant>,
    intervals: VecDeque<f32>,
    render_start: Rc<Cell<Instant>>,
    draw_ms: Rc<Cell<f32>>,
    draw_acc: (f32, u32),
    mem_mb: f32,
    mem_at: f32,
    shown_at: f32,
    fps: f32,
    frame_ms: f32,
    worst_ms: f32,
    draw_avg: f32,
}

/// Resident memory of this process in MiB (same number `ps -o rss` reports).
pub fn memory_mb() -> f32 {
    unsafe {
        let mut info: libc::rusage_info_v2 = std::mem::zeroed();
        let rc = libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V2,
            &mut info as *mut libc::rusage_info_v2 as *mut libc::rusage_info_t,
        );
        if rc == 0 {
            info.ri_resident_size as f32 / (1024. * 1024.)
        } else {
            0.
        }
    }
}

impl Hud {
    pub fn new() -> Self {
        Hud {
            last: None,
            intervals: VecDeque::with_capacity(SAMPLES + 1),
            render_start: Rc::new(Cell::new(Instant::now())),
            draw_ms: Rc::new(Cell::new(0.)),
            draw_acc: (0., 0),
            mem_mb: memory_mb(),
            mem_at: 0.,
            shown_at: -1.,
            fps: 0.,
            frame_ms: 0.,
            worst_ms: 0.,
            draw_avg: 0.,
        }
    }

    pub fn frame(&mut self, now: Instant, t: f32) {
        if let Some(last) = self.last {
            let dt = (now - last).as_secs_f32() * 1000.;
            self.intervals.push_back(dt);
            if self.intervals.len() > SAMPLES {
                self.intervals.pop_front();
            }
        }
        self.last = Some(now);
        self.render_start.set(now);
        self.draw_acc.0 += self.draw_ms.get();
        self.draw_acc.1 += 1;

        if t - self.shown_at > 0.25 && !self.intervals.is_empty() {
            self.shown_at = t;
            let recent: Vec<f32> = self.intervals.iter().rev().take(40).copied().collect();
            let avg = recent.iter().sum::<f32>() / recent.len() as f32;
            self.frame_ms = avg;
            self.fps = if avg > 0. { 1000. / avg } else { 0. };
            self.worst_ms = self.intervals.iter().copied().fold(0., f32::max);
            self.draw_avg = self.draw_acc.0 / self.draw_acc.1.max(1) as f32;
            self.draw_acc = (0., 0);
        }
        if t - self.mem_at > 0.5 {
            self.mem_at = t;
            self.mem_mb = memory_mb();
        }
    }
}

fn stat(label: &'static str, value: String, unit: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(9.5))
                .line_height(px(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(TEXT_FAINT))
                .child(label),
        )
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(3.))
                .child(
                    div()
                        .font_family(MONO_FONT)
                        .text_size(px(14.))
                        .line_height(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(TEXT_BRIGHT))
                        .child(SharedString::from(value)),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .line_height(px(15.))
                        .text_color(c(TEXT_MUTED))
                        .child(unit),
                ),
        )
}

impl Parley {
    pub fn render_hud(&self) -> Option<Div> {
        let hud = self.hud.as_ref()?;
        let samples: Vec<f32> = hud.intervals.iter().copied().collect();
        let start = hud.render_start.clone();
        let draw = hud.draw_ms.clone();
        let graph = canvas(
            |_, _, _| (),
            move |bounds: Bounds<gpui::Pixels>, _, window, _| {
                let w: f32 = bounds.size.width.into();
                let h: f32 = bounds.size.height.into();
                let bar = w / SAMPLES as f32;
                let x0: f32 = bounds.origin.x.into();
                let y0: f32 = bounds.origin.y.into();
                // 16.7 ms guide line
                let guide = h - (16.7f32 / 34.) * h;
                window.paint_quad(fill(
                    Bounds::new(point(px(x0), px(y0 + guide)), size(px(w), px(1.))),
                    white(0.1),
                ));
                let offset = SAMPLES - samples.len();
                for (i, ms) in samples.iter().enumerate() {
                    let bh = (ms / 34. * h).clamp(1., h);
                    let color = if *ms < 12. {
                        c(GREEN)
                    } else if *ms < 20. {
                        c(YELLOW)
                    } else {
                        c(RED)
                    };
                    window.paint_quad(fill(
                        Bounds::new(
                            point(px(x0 + (offset + i) as f32 * bar), px(y0 + h - bh)),
                            size(px((bar - 0.5).max(0.5)), px(bh)),
                        ),
                        color,
                    ));
                }
                // Everything before this point was rendered, laid out and painted this frame.
                draw.set(start.get().elapsed().as_secs_f32() * 1000.);
            },
        )
        .w(px(264.))
        .h(px(30.));

        let voice = self.channel().kind == ChanKind::Voice;
        let right = if self.show_members && !voice { MEMBERS_W + 16. } else { 16. };
        Some(
            div()
                .absolute()
                .top(px(TITLE_H + HEADER_H + 10.))
                .right(px(right))
                .p(px(12.))
                .rounded(px(10.))
                .bg(ca(BG_FLOAT, 0.92))
                .border_1()
                .border_color(white(0.1))
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .gap(px(18.))
                        .child(stat("FPS", format!("{:.0}", hud.fps), ""))
                        .child(stat("FRAME", format!("{:.2}", hud.frame_ms), "ms"))
                        .child(stat("DRAW", format!("{:.2}", hud.draw_avg), "ms"))
                        .child(stat("WORST", format!("{:.1}", hud.worst_ms), "ms"))
                        .child(stat("MEMORY", format!("{:.0}", hud.mem_mb), "MB")),
                )
                .child(graph),
        )
    }
}
