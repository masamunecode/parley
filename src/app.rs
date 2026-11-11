//! Root view: application state, input handling and the top-level layout.
use crate::data::*;
use crate::demo::{Demo, Target};
use crate::hud::Hud;
use crate::theme::*;
use crate::ui::*;
use gpui::{
    canvas, div, font, point, prelude::*, px, AnyElement, Bounds, Context, Div, FocusHandle, Font,
    FontWeight, KeyDownEvent, ListAlignment, ListOffset, ListState, MouseButton, MouseMoveEvent,
    Pixels, SharedString, UniformListScrollHandle, Window,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

pub const TITLE_H: f32 = 36.;

#[derive(Default)]
pub struct Opts {
    pub demo: bool,
    pub hud: bool,
    pub server: Option<usize>,
    pub channel: Option<usize>,
    pub at: Option<usize>,
    pub switcher: Option<String>,
}

pub struct Fonts {
    pub ui: Font,
    pub medium: Font,
    pub mono: Font,
}

pub struct SwitchItem {
    pub server: usize,
    pub channel: Option<usize>,
    pub positions: Vec<usize>,
    pub score: i32,
}

pub struct Switcher {
    pub field: TextField,
    pub sel: usize,
    pub results: Vec<SwitchItem>,
}

pub struct Parley {
    pub world: World,
    pub server: usize,
    pub channel_of: Vec<usize>,
    pub lists: HashMap<(usize, usize), ListState>,
    cache: HashMap<usize, Rc<Content>>,
    cache_key: (usize, usize),
    pub composer: TextField,
    pub switcher: Option<Switcher>,
    pub focus: FocusHandle,
    pub muted: bool,
    pub deafened: bool,
    pub show_members: bool,
    pub collapsed: HashSet<(usize, usize)>,
    pub start: Instant,
    pub t: f32,
    pub hud: Option<Hud>,
    pub demo: Option<Demo>,
    pub hover_server: Option<usize>,
    pub fonts: Fonts,
    pub scroll_drag: bool,
    pub targets: Rc<RefCell<HashMap<Target, Bounds<Pixels>>>>,
    pub typists: [Option<(u16, f32)>; 2],
    next_typist: [f32; 2],
    live_rng: Rng,
    pub members_scroll: Vec<UniformListScrollHandle>,
    /// Message row bounds recorded during paint (demo mode only), for simulated hover.
    pub msg_bounds: Rc<RefCell<Vec<(usize, Bounds<Pixels>)>>>,
    pub hot_msg: Option<usize>,
    pub hot_member: Option<usize>,
}

impl Parley {
    pub fn new(opts: Opts, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (demo, hud) = (opts.demo, opts.hud);
        let world = World::generate();
        let focus = cx.focus_handle();
        window.focus(&focus);
        let channel_of: Vec<usize> = world.servers.iter().map(|s| s.default_channel).collect();
        let server_count = world.servers.len();
        let mut medium = font(UI_FONT);
        medium.weight = FontWeight::MEDIUM;
        let mut this = Parley {
            world,
            server: 0,
            channel_of,
            lists: HashMap::new(),
            cache: HashMap::new(),
            cache_key: (usize::MAX, 0),
            composer: TextField::default(),
            switcher: None,
            focus,
            muted: false,
            deafened: false,
            show_members: true,
            collapsed: HashSet::new(),
            start: Instant::now(),
            t: 0.,
            hud: hud.then(Hud::new),
            demo: demo.then(Demo::new),
            hover_server: None,
            fonts: Fonts { ui: font(UI_FONT), medium, mono: font(MONO_FONT) },
            scroll_drag: false,
            targets: Rc::new(RefCell::new(HashMap::new())),
            typists: [None, None],
            next_typist: [2.5, 6.0],
            live_rng: Rng::new(0xBEEF),
            members_scroll: (0..server_count).map(|_| UniformListScrollHandle::new()).collect(),
            msg_bounds: Rc::new(RefCell::new(Vec::new())),
            hot_msg: None,
            hot_member: None,
        };
        if let Some(s) = opts.server.filter(|s| *s < this.world.servers.len()) {
            this.server = s;
        }
        if let Some(c) = opts.channel.filter(|c| *c < this.world.servers[this.server].channels.len()) {
            this.channel_of[this.server] = c;
        }
        if let Some(at) = opts.at {
            let ls = this.list_state();
            ls.scroll_to(ListOffset { item_ix: at.min(ls.item_count()), offset_in_item: px(0.) });
        }
        if let Some(query) = opts.switcher {
            this.open_switcher();
            if let Some(sw) = this.switcher.as_mut() {
                sw.field.insert(&query);
            }
            this.update_switcher();
        }
        if demo {
            crate::demo::schedule(cx.weak_entity(), window);
        }
        this
    }

    // ------------------------------------------------------------ state helpers

    pub fn cur(&self) -> (usize, usize) {
        (self.server, self.channel_of[self.server])
    }

    pub fn channel(&self) -> &Channel {
        let (s, c) = self.cur();
        &self.world.servers[s].channels[c]
    }

    pub fn list_state(&mut self) -> ListState {
        let key = self.cur();
        let count = self.world.servers[key.0].channels[key.1].metas.len() + 1;
        self.lists
            .entry(key)
            .or_insert_with(|| ListState::new(count, ListAlignment::Bottom, px(1000.)))
            .clone()
    }

    pub fn content(&mut self, ix: usize) -> Rc<Content> {
        let key = self.cur();
        if self.cache_key != key {
            self.cache.clear();
            self.cache_key = key;
        }
        if let Some(c) = self.cache.get(&ix) {
            return c.clone();
        }
        if self.cache.len() > 4000 {
            self.cache.clear();
        }
        let server = &self.world.servers[key.0];
        let ch = &server.channels[key.1];
        let meta = ch.metas[ix];
        let content = if meta.flags & F_CUSTOM != 0 {
            custom_content(&ch.custom[meta.seed as usize])
        } else {
            gen_content(ix, &meta, &self.world.users, &server.active, ch.dev)
        };
        let rc = Rc::new(content);
        self.cache.insert(ix, rc.clone());
        rc
    }

    pub fn select_server(&mut self, i: usize, cx: &mut Context<Self>) {
        self.server = i;
        let c = self.channel_of[i];
        self.select_channel(c, cx);
    }

    pub fn select_channel(&mut self, c: usize, cx: &mut Context<Self>) {
        let s = self.server;
        self.channel_of[s] = c;
        let ch = &mut self.world.servers[s].channels[c];
        ch.unread = false;
        ch.mentions = 0;
        self.scroll_drag = false;
        cx.notify();
    }

    pub fn jump_to_present(&mut self) {
        let ls = self.list_state();
        ls.scroll_to(ListOffset { item_ix: ls.item_count(), offset_in_item: px(0.) });
    }

    pub fn toggle_mute(&mut self) {
        if self.deafened {
            self.deafened = false;
            self.muted = false;
        } else {
            self.muted = !self.muted;
        }
    }

    /// Scroll the message list by `dy` pixels (positive reveals older messages),
    /// using the same measured-pixel space the list's wheel handler uses.
    pub fn scroll_chat_by(&mut self, dy: f32) {
        let ls = self.list_state();
        let at_bottom = ls.logical_scroll_top().item_ix >= ls.item_count();
        let cur: f32 = if at_bottom {
            f32::from(ls.max_offset_for_scrollbar().height) + 26.
        } else {
            -f32::from(ls.scroll_px_offset_for_scrollbar().y)
        };
        let target = (cur - dy).max(0.);
        ls.set_offset_from_scrollbar(point(px(0.), px(-target)));
    }

    pub fn scroll_members_by(&mut self, dy: f32) {
        let handle = self.members_scroll[self.server].0.borrow().base_handle.clone();
        let offset = handle.offset();
        let max: f32 = handle.max_offset().height.into();
        let y = (f32::from(offset.y) + dy).clamp(-max, 0.);
        handle.set_offset(point(offset.x, px(y)));
    }

    fn push_message(&mut self, s: usize, c: usize, meta: MsgMeta) {
        self.world.servers[s].channels[c].metas.push(meta);
        if let Some(ls) = self.lists.get(&(s, c)) {
            let n = ls.item_count();
            ls.splice(n..n, 1);
        }
    }

    fn now_ts(&self) -> u32 {
        (self.world.clock.now + self.start.elapsed().as_secs() as i64) as u32
    }

    pub fn send(&mut self, cx: &mut Context<Self>) {
        let text = self.composer.text.trim().to_string();
        self.composer.clear();
        if text.is_empty() || self.channel().kind == ChanKind::Voice {
            return;
        }
        let (s, c) = self.cur();
        let ts = self.now_ts();
        let ch = &mut self.world.servers[s].channels[c];
        ch.custom.push(text.into());
        let seed = (ch.custom.len() - 1) as u32;
        self.push_message(s, c, MsgMeta { ts, seed, author: 0, flags: F_CUSTOM });
        self.jump_to_present();
        cx.notify();
    }

    /// Simulated live activity in the main channel: people type, then their message lands.
    fn tick(&mut self) {
        let t = self.t;
        let main = (0, self.world.servers[0].default_channel);
        for slot in 0..2 {
            match self.typists[slot] {
                Some((user, until)) if t >= until => {
                    self.typists[slot] = None;
                    self.next_typist[slot] = t + 2.0 + self.live_rng.f() * 7.0;
                    let ts = self.now_ts();
                    let seed = self.live_rng.next() as u32;
                    self.push_message(main.0, main.1, MsgMeta { ts, seed, author: user, flags: 0 });
                    if self.cur() != main {
                        self.world.servers[main.0].channels[main.1].unread = true;
                    }
                }
                None if t >= self.next_typist[slot] => {
                    let active = &self.world.servers[0].active;
                    let mut user = active[self.live_rng.below(active.len())];
                    if user == 0 || self.typists[1 - slot].map(|(u, _)| u) == Some(user) {
                        user = active[0];
                    }
                    self.typists[slot] = Some((user, t + 2.2 + self.live_rng.f() * 4.5));
                }
                _ => {}
            }
        }
    }

    pub fn mark(&self, target: Target) -> Option<AnyElement> {
        self.demo.as_ref()?;
        let map = self.targets.clone();
        Some(
            canvas(
                move |bounds, _, _| {
                    map.borrow_mut().insert(target, bounds);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element(),
        )
    }

    // ------------------------------------------------------------ input

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        let t = self.t;
        if (m.platform || m.control) && ks.key == "k" {
            if self.switcher.is_some() {
                self.switcher = None;
            } else {
                self.open_switcher();
            }
            cx.notify();
            return;
        }
        let paste = if m.platform && ks.key == "v" {
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .map(|s| s.replace(['\n', '\r', '\t'], " "))
        } else {
            None
        };
        if self.switcher.is_some() {
            let mut accept = None;
            let mut changed = false;
            {
                let sw = self.switcher.as_mut().unwrap();
                let n = sw.results.len().max(1);
                match ks.key.as_str() {
                    "escape" => {
                        self.switcher = None;
                        cx.notify();
                        return;
                    }
                    "down" | "tab" => sw.sel = (sw.sel + 1) % n,
                    "up" => sw.sel = (sw.sel + n - 1) % n,
                    "enter" => accept = Some(sw.sel),
                    _ => {
                        if let Some(p) = &paste {
                            sw.field.insert(p);
                            changed = true;
                        } else if sw.field.handle(ks) {
                            changed = true;
                        }
                        sw.field.edited_at = t;
                    }
                }
            }
            if changed {
                self.update_switcher();
            }
            if let Some(i) = accept {
                self.switcher_accept(i, cx);
            }
            cx.notify();
            return;
        }
        match ks.key.as_str() {
            "enter" if !m.shift => self.send(cx),
            "escape" => self.jump_to_present(),
            "pageup" if self.channel().kind != ChanKind::Voice => self.scroll_chat_by(560.),
            "pagedown" if self.channel().kind != ChanKind::Voice => self.scroll_chat_by(-560.),
            _ => {
                if self.channel().kind != ChanKind::Voice {
                    if let Some(p) = &paste {
                        self.composer.insert(p);
                    } else {
                        self.composer.handle(ks);
                    }
                    self.composer.edited_at = t;
                }
            }
        }
        cx.notify();
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.scroll_drag {
            if ev.pressed_button == Some(MouseButton::Left) {
                self.scrub_to(ev.position.y);
            } else {
                self.scroll_drag = false;
            }
            cx.notify();
        }
    }

    // ------------------------------------------------------------ chrome

    fn render_titlebar(&self) -> Div {
        let server = &self.world.servers[self.server];
        div()
            .h(px(TITLE_H))
            .w_full()
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(logo_mark(18., c(ACCENT), c(TEXT_BRIGHT), c(BG_RAIL)))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(TEXT_MUTED))
                    .child(SharedString::from(format!("Parley  ·  {}", server.name))),
            )
            .children(self.demo.as_ref().filter(|d| !d.caption.is_empty()).map(|d| {
                div()
                    .ml(px(6.))
                    .h(px(22.))
                    .pl(px(4.))
                    .pr(px(10.))
                    .rounded_full()
                    .bg(ca(ACCENT, 0.14))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(c(ACCENT_SOFT))
                    .child(
                        div()
                            .h(px(15.))
                            .px(px(6.))
                            .rounded_full()
                            .bg(c(ACCENT))
                            .flex()
                            .items_center()
                            .text_size(px(9.5))
                            .line_height(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c(ACCENT_INK))
                            .child("DEMO"),
                    )
                    .child(d.caption)
            }))
            .child(
                div()
                    .absolute()
                    .right(px(14.))
                    .top_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(icon("inbox", 18., c(TEXT_MUTED)))
                    .child(icon("help", 18., c(TEXT_MUTED))),
            )
    }

    fn render_server_tooltip(&self) -> Option<Div> {
        let i = self.hover_server?;
        let name = if i == usize::MAX {
            SharedString::from("Direct Messages")
        } else {
            self.world.servers.get(i)?.name.clone()
        };
        let slot = if i == usize::MAX { 0. } else { 66. + i as f32 * 56. };
        let y = TITLE_H + 4. + slot + 24. - 17.;
        Some(
            div()
                .absolute()
                .left(px(RAIL_W + 6.))
                .top(px(y))
                .h(px(34.))
                .px(px(12.))
                .rounded(px(8.))
                .bg(c(BG_FLOAT))
                .border_1()
                .border_color(white(0.08))
                .shadow_lg()
                .flex()
                .items_center()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(TEXT_BRIGHT))
                .child(name),
        )
    }
}

/// The Parley mark: two overlapping speech bubbles.
pub fn logo_mark(s: f32, a: gpui::Hsla, b: gpui::Hsla, cut: gpui::Hsla) -> Div {
    let u = s / 48.;
    div()
        .relative()
        .size(px(s))
        .flex_none()
        .child(
            div()
                .absolute()
                .left(px(6. * u))
                .top(px(8. * u))
                .w(px(25. * u))
                .h(px(19. * u))
                .rounded(px(7. * u))
                .rounded_bl(px(1.5 * u))
                .bg(a),
        )
        .child(
            div()
                .absolute()
                .left(px(14. * u))
                .top(px(18. * u))
                .w(px(30. * u))
                .h(px(24. * u))
                .rounded(px(9.5 * u))
                .rounded_br(px(4. * u))
                .bg(cut)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(25. * u))
                        .h(px(19. * u))
                        .rounded(px(7. * u))
                        .rounded_br(px(1.5 * u))
                        .bg(b),
                ),
        )
}

impl Render for Parley {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.t = (now - self.start).as_secs_f32();
        let t = self.t;
        if let Some(h) = &mut self.hud {
            h.frame(now, t);
        }
        self.tick();
        window.request_animation_frame();
        crate::pump::frame_rendered();

        // Simulated hover for the demo cursor: find the message row under it.
        if let Some((x, y)) = self.demo_pos() {
            let rows = self.msg_bounds.clone();
            let mut rows = rows.borrow_mut();
            let p = point(px(x), px(y));
            let over_list = self.switcher.is_none()
                && self.lists.get(&self.cur()).map_or(false, |ls| ls.viewport_bounds().contains(&p));
            self.hot_msg = if over_list {
                rows.iter().find(|(_, b)| b.contains(&p)).map(|(ix, _)| *ix)
            } else {
                None
            };
            rows.clear();
            // ...and the member row, from the list's bounds and scroll offset.
            self.hot_member = None;
            if self.switcher.is_none() {
                if let Some(b) = self.targets.borrow().get(&Target::Members) {
                    if b.contains(&p) {
                        let scroll = self.members_scroll[self.server].0.borrow().base_handle.offset().y;
                        let row = f32::from(p.y - b.top() - scroll) / 44.;
                        self.hot_member = Some(row.max(0.) as usize);
                    }
                }
            }
        }

        let voice = self.channel().kind == ChanKind::Voice;
        let main: AnyElement = if voice {
            self.render_voice(cx).into_any_element()
        } else {
            self.render_chat(window, cx).into_any_element()
        };

        div()
            .id("root")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(c(BG_RAIL))
            .text_color(c(TEXT))
            .font_family(UI_FONT)
            .text_size(px(14.))
            .line_height(px(20.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.scroll_drag {
                        this.scroll_drag = false;
                        cx.notify();
                    }
                }),
            )
            .child(self.render_titlebar())
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_row()
                    .child(self.render_rail(cx))
                    .child(self.render_sidebar(cx))
                    .child(main),
            )
            .children(self.render_server_tooltip())
            .children(self.render_switcher(cx))
            .children(self.render_hud())
            .children(self.render_demo_cursor())
    }
}
