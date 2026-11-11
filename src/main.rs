//! Parley — an offline, Discord-style community chat client built with GPUI.
//!
//!   parley            interactive app
//!   parley --demo     self-playing ~45 s tour (loops)
//!   parley --hud      fps / frame time / memory overlay
mod app;
mod chat;
mod data;
mod demo;
mod hud;
mod icons;
mod pump;
mod sidebar;
mod switcher;
mod theme;
mod ui;
mod voice;

use gpui::{
    actions, point, prelude::*, px, size, App, Application, Bounds, KeyBinding, Menu, MenuItem,
    TitlebarOptions, WindowBounds, WindowOptions,
};

actions!(parley, [Quit]);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");
    let hud = args.iter().any(|a| a == "--hud");
    // Extra start-up state flags, handy for screenshots:
    //   --server=N --channel=N --at=MESSAGE_INDEX --switcher=QUERY
    let value = |name: &str| {
        args.iter()
            .find_map(|a| a.strip_prefix(name).and_then(|r| r.strip_prefix('=')).map(str::to_string))
    };
    let opts = app::Opts {
        demo,
        hud,
        server: value("--server").and_then(|v| v.parse().ok()),
        channel: value("--channel").and_then(|v| v.parse().ok()),
        at: value("--at").and_then(|v| v.parse().ok()),
        switcher: value("--switcher"),
    };
    // SNAP=1: open without activating the app or stealing focus (used by snap.sh).
    let snap = std::env::var("SNAP").map(|v| v == "1").unwrap_or(false);

    Application::new().with_assets(icons::Assets).run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Parley".into(),
            items: vec![MenuItem::action("Quit Parley", Quit)],
        }]);

        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Parley".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(13.), px(12.))),
                }),
                focus: !snap,
                show: true,
                window_min_size: Some(size(px(1000.), px(620.))),
                ..Default::default()
            },
            |window, cx| {
                if snap || demo {
                    pump::start(window, cx);
                }
                cx.new(|cx| app::Parley::new(opts, window, cx))
            },
        )
        .expect("failed to open window");

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        if !snap {
            cx.activate(true);
        }
    });
}
