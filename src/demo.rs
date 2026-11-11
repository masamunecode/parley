//! `--demo`: a scripted, looping tour with its own drawn cursor.
//!
//! The script advances once per frame. Keystrokes are injected through
//! `Window::dispatch_keystroke` (the real key path); clicks, drags and flings call
//! the same handlers the mouse listeners call, and hover is simulated from the
//! drawn cursor's position against element bounds recorded during paint.
use crate::app::Parley;
use crate::theme::*;
use crate::ui::icon;
use gpui::{div, prelude::*, px, Context, Div, Keystroke, Modifiers, WeakEntity, Window};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Target {
    Server(usize),
    Channel(usize),
    Composer,
    Jump,
    Chat,
    Members,
    MembersToggle,
    Mic,
    Deafen,
    Thumb,
    /// A point on the scrollbar track, in thousandths from the top.
    Track(u16),
}

#[derive(Clone, Copy)]
enum Step {
    Caption(&'static str),
    Wait(f32),
    Move(Target, f32),
    Click,
    Press,
    Release,
    Key(&'static str),
    Type(&'static str, f32),
    /// Momentum scroll: initial velocity in px/s (positive reveals older content), duration.
    Fling(f32, f32),
    Reset,
}

pub struct Demo {
    steps: Vec<Step>,
    ix: usize,
    step_t: f32,
    typed: usize,
    pub pos: (f32, f32),
    from: (f32, f32),
    at: Option<Target>,
    pressed: bool,
    last: Option<Instant>,
    time: f32,
    click_at: f32,
    pub caption: &'static str,
}

fn script() -> Vec<Step> {
    use Step::*;
    use Target::*;
    vec![
        Caption("#general · 100,000 generated messages"),
        Wait(1.0),
        Move(Chat, 0.9),
        Caption("Scrolling back through history"),
        Fling(9000., 1.6),
        Wait(0.25),
        Caption("Fling-scrolling the virtualized list"),
        Fling(45000., 2.4),
        Fling(78000., 2.8),
        Wait(0.3),
        Caption("Scrubbing across all 100,000 messages"),
        Move(Thumb, 0.7),
        Press,
        Move(Track(520), 1.3),
        Wait(0.35),
        Move(Track(60), 1.5),
        Wait(0.5),
        Release,
        Caption("Jump to present"),
        Move(Jump, 0.7),
        Click,
        Wait(0.8),
        Caption("Composer: type, Enter sends"),
        Move(Composer, 0.7),
        Click,
        Type("this list is absurdly smooth 🚀", 0.05),
        Wait(0.35),
        Key("enter"),
        Wait(1.4),
        Caption("Voice channel · live speaking rings"),
        Move(Channel(13), 0.8),
        Click,
        Wait(3.4),
        Caption("Instant channel switching"),
        Move(Channel(5), 0.6),
        Click,
        Wait(0.9),
        Move(Channel(7), 0.45),
        Click,
        Wait(0.9),
        Move(Channel(8), 0.4),
        Click,
        Wait(0.9),
        Caption("Instant server switching"),
        Move(Server(1), 0.8),
        Click,
        Wait(0.9),
        Move(Server(3), 0.5),
        Click,
        Wait(0.9),
        Move(Server(6), 0.5),
        Click,
        Wait(0.9),
        Caption("Cmd+K quick switcher · fuzzy search"),
        Key("cmd-k"),
        Wait(0.6),
        Type("sh", 0.2),
        Wait(0.6),
        Key("down"),
        Wait(0.3),
        Key("down"),
        Wait(0.3),
        Key("up"),
        Wait(0.4),
        Key("backspace"),
        Wait(0.12),
        Key("backspace"),
        Wait(0.25),
        Type("gnrl", 0.16),
        Wait(0.8),
        Key("enter"),
        Wait(1.0),
        Caption("1,000 members grouped by role"),
        Move(Members, 0.8),
        Fling(-14000., 1.8),
        Wait(0.3),
        Fling(-30000., 1.6),
        Wait(0.5),
        Caption("Mute toggle"),
        Move(Mic, 0.9),
        Click,
        Wait(0.6),
        Click,
        Wait(0.6),
        Reset,
    ]
}

fn ease(k: f32) -> f32 {
    if k < 0.5 {
        4. * k * k * k
    } else {
        1. - (-2. * k + 2.).powi(3) / 2.
    }
}

impl Demo {
    pub fn new() -> Self {
        Demo {
            steps: script(),
            ix: 0,
            step_t: 0.,
            typed: 0,
            pos: (760., 520.),
            from: (760., 520.),
            at: None,
            pressed: false,
            last: None,
            time: 0.,
            click_at: -10.,
            caption: "",
        }
    }

    fn next(&mut self) {
        self.ix = (self.ix + 1) % self.steps.len();
        self.step_t = 0.;
        self.typed = 0;
    }

    fn advance(&mut self, app: &mut Parley, cx: &mut Context<Parley>) -> Vec<Keystroke> {
        let now = Instant::now();
        let dt = self.last.map_or(0., |l| (now - l).as_secs_f32()).min(0.05);
        self.last = Some(now);
        self.time += dt;
        let mut keys = Vec::new();

        // Run instantaneous steps until a timed one consumes this frame.
        for _ in 0..12 {
            let step = self.steps[self.ix];
            match step {
                Step::Caption(text) => {
                    self.caption = text;
                    self.next();
                }
                Step::Reset => {
                    app.reset_for_demo();
                    self.next();
                }
                Step::Wait(d) => {
                    self.step_t += dt;
                    if self.step_t >= d {
                        self.next();
                    }
                    break;
                }
                Step::Move(target, d) => {
                    if self.step_t == 0. {
                        self.from = self.pos;
                        self.at = Some(target);
                    }
                    self.step_t += dt;
                    let k = (self.step_t / d).min(1.);
                    if let Some(to) = app.resolve_target(target) {
                        let (dx, dy) = (to.0 - self.from.0, to.1 - self.from.1);
                        let len = (dx * dx + dy * dy).sqrt().max(1.);
                        let arc = if self.pressed {
                            0.
                        } else {
                            (k * std::f32::consts::PI).sin() * 0.07 * len.min(500.)
                        };
                        let e = ease(k);
                        self.pos = (
                            self.from.0 + dx * e - dy / len * arc,
                            self.from.1 + dy * e + dx / len * arc,
                        );
                    }
                    if self.pressed {
                        app.scrub_to(px(self.pos.1));
                    }
                    if k >= 1. {
                        self.next();
                    }
                    break;
                }
                Step::Click => {
                    app.demo_click(self.at, cx);
                    self.click_at = self.time;
                    self.next();
                    break;
                }
                Step::Press => {
                    if self.at == Some(Target::Thumb) {
                        app.scroll_drag = true;
                    }
                    self.pressed = true;
                    self.click_at = self.time;
                    self.next();
                    break;
                }
                Step::Release => {
                    app.scroll_drag = false;
                    self.pressed = false;
                    self.next();
                    break;
                }
                Step::Key(key) => {
                    if let Ok(ks) = Keystroke::parse(key) {
                        keys.push(ks);
                    }
                    self.next();
                    break;
                }
                Step::Type(text, per_char) => {
                    self.step_t += dt;
                    let total = text.chars().count();
                    let want = ((self.step_t / per_char) as usize).min(total);
                    if self.typed < want {
                        let ch = text.chars().nth(self.typed).unwrap();
                        let key = if ch == ' ' {
                            "space".to_string()
                        } else {
                            ch.to_lowercase().to_string()
                        };
                        keys.push(Keystroke {
                            modifiers: Modifiers::default(),
                            key,
                            key_char: Some(ch.to_string()),
                        });
                        self.typed += 1;
                    }
                    if self.typed >= total {
                        self.next();
                    }
                    break;
                }
                Step::Fling(v0, d) => {
                    self.step_t += dt;
                    let k = (self.step_t / d).min(1.);
                    let v = v0 * (-3.4 * k).exp();
                    let dy = (v * dt).clamp(-650., 650.);
                    match self.at {
                        Some(Target::Members) => app.scroll_members_by(dy),
                        _ => app.scroll_chat_by(dy),
                    }
                    if k >= 1. {
                        self.next();
                    }
                    break;
                }
            }
        }
        keys
    }
}

impl Parley {
    pub fn demo_step(&mut self, cx: &mut Context<Self>) -> Vec<Keystroke> {
        let Some(mut demo) = self.demo.take() else {
            return Vec::new();
        };
        let keys = demo.advance(self, cx);
        self.demo = Some(demo);
        self.hover_server = (0..self.world.servers.len()).find(|&i| self.hot(Target::Server(i)));
        cx.notify();
        keys
    }

    /// Is the demo cursor over the element recorded for `target`?
    pub fn hot(&self, target: Target) -> bool {
        let Some(demo) = self.demo.as_ref() else {
            return false;
        };
        let (x, y) = demo.pos;
        self.targets.borrow().get(&target).map_or(false, |b| {
            let (l, t, r, bt): (f32, f32, f32, f32) =
                (b.left().into(), b.top().into(), b.right().into(), b.bottom().into());
            x >= l && x < r && y >= t && y < bt
        })
    }

    pub fn demo_pos(&self) -> Option<(f32, f32)> {
        self.demo.as_ref().map(|d| d.pos)
    }

    fn demo_click(&mut self, at: Option<Target>, cx: &mut Context<Self>) {
        match at {
            Some(Target::Server(i)) => self.select_server(i, cx),
            Some(Target::Channel(c)) => self.select_channel(c, cx),
            Some(Target::Jump) => self.jump_to_present(),
            Some(Target::Mic) => self.toggle_mute(),
            Some(Target::Deafen) => self.deafened = !self.deafened,
            Some(Target::MembersToggle) => self.show_members = !self.show_members,
            Some(Target::Composer) => self.composer.edited_at = self.t,
            _ => {}
        }
    }

    fn reset_for_demo(&mut self) {
        self.server = 0;
        self.channel_of[0] = self.world.servers[0].default_channel;
        self.switcher = None;
        self.composer.clear();
        self.muted = false;
        self.deafened = false;
        self.show_members = true;
        self.scroll_drag = false;
        self.jump_to_present();
        self.scroll_members_by(1.0e7);
    }

    fn resolve_target(&mut self, target: Target) -> Option<(f32, f32)> {
        match target {
            Target::Chat => {
                let vp = self.list_state().viewport_bounds();
                let c = vp.center();
                let (x, y): (f32, f32) = (c.x.into(), c.y.into());
                (x > 1.).then_some((x - 40., y + 60.))
            }
            Target::Thumb => self.thumb_center(),
            Target::Track(permille) => self.track_point(permille as f32 / 1000.),
            other => self.targets.borrow().get(&other).map(|b| {
                let c = b.center();
                (c.x.into(), c.y.into())
            }),
        }
    }

    pub fn render_demo_cursor(&self) -> Option<Div> {
        let demo = self.demo.as_ref()?;
        let (x, y) = demo.pos;
        let since = demo.time - demo.click_at;
        let size = 26.;
        let scale = size / 24.;
        // The tour plays by itself: this layer swallows real mouse input so the
        // physical pointer can't cause a second set of hover effects.
        let mut layer = div().absolute().top_0().left_0().size_full().occlude();
        if since < 0.45 {
            let k = since / 0.45;
            let d = 10. + 34. * k;
            layer = layer.child(
                div()
                    .absolute()
                    .left(px(x - d / 2.))
                    .top(px(y - d / 2.))
                    .size(px(d))
                    .rounded_full()
                    .border_2()
                    .border_color(ca(ACCENT, (1. - k) * 0.9))
                    .bg(ca(ACCENT, (1. - k) * 0.18)),
            );
        }
        let press = if demo.pressed || since < 0.12 { 0.9 } else { 1. };
        let s = size * press;
        Some(
            layer.child(
                div()
                    .absolute()
                    .left(px(x - 4. * scale * press))
                    .top(px(y - 2. * scale * press))
                    .size(px(s))
                    .child(div().absolute().left(px(1.5)).top(px(2.5)).child(icon("cursor", s, black(0.35))))
                    .child(div().absolute().top_0().left_0().child(icon("cursor", s, white(1.))))
                    .child(div().absolute().top_0().left_0().child(icon("cursor-outline", s, c(0x101114)))),
            ),
        )
    }
}

/// Re-arms itself every frame: advance the script, then dispatch its keystrokes.
pub fn schedule(entity: WeakEntity<Parley>, window: &mut Window) {
    window.on_next_frame(move |window, cx| {
        let Some(strong) = entity.upgrade() else {
            return;
        };
        let keys = strong.update(cx, |this, cx| this.demo_step(cx));
        for keystroke in keys {
            window.dispatch_keystroke(keystroke, cx);
        }
        schedule(entity, window);
    });
}
