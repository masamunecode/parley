//! Fallback frame pump for occluded windows.
//!
//! GPUI stops its display link whenever macOS reports the window as occluded
//! (for example when it is opened unfocused behind other windows by `snap.sh`),
//! which freezes animations and the demo tour. While the window is not visible
//! this pump asks the view to display on a timer instead; when the window is
//! visible it does nothing and the normal vsync path is used.
use gpui::{App, Window};
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const OCCLUSION_VISIBLE: u64 = 1 << 1;

static EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_FRAME_MICROS: AtomicU64 = AtomicU64::new(0);

fn now_micros() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// Called by the root view every time it renders a frame.
pub fn frame_rendered() {
    LAST_FRAME_MICROS.store(now_micros(), Ordering::Relaxed);
}

fn since_last_frame() -> Duration {
    Duration::from_micros(now_micros().saturating_sub(LAST_FRAME_MICROS.load(Ordering::Relaxed)))
}

pub fn start(window: &mut Window, cx: &mut App) {
    let view = {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return;
        };
        appkit.ns_view.as_ptr() as usize
    };

    // Keep timers running at full rate even though the app is in the background.
    unsafe {
        let info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
        let reason: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: c"Parley playback".as_ptr()];
        let options: u64 = 0x00FF_FFFF | 0xFF_0000_0000; // user initiated | latency critical
        let token: *mut Object = msg_send![info, beginActivityWithOptions: options reason: reason];
        let _: *mut Object = msg_send![token, retain];
    }

    let mut next = Instant::now();
    cx.spawn(async move |cx| loop {
        // Pace like a 120 Hz display: sleep until the next 1/120 s boundary.
        let frame = Duration::from_micros(8333);
        let now = Instant::now();
        next = if next + frame > now { next + frame } else { now + Duration::from_millis(1) };
        cx.background_executor().timer(next.saturating_duration_since(now)).await;
        let alive = cx.update(|cx| !cx.windows().is_empty()).unwrap_or(false);
        if !alive {
            break;
        }
        unsafe {
            let view = view as *mut Object;
            let ns_window: *mut Object = msg_send![view, window];
            if ns_window.is_null() {
                break;
            }
            let occlusion: u64 = msg_send![ns_window, occlusionState];
            // Also step in if the window claims to be visible but no frame has been
            // rendered for a while (display link never started or stalled).
            let stalled = since_last_frame() > Duration::from_millis(60);
            if occlusion & OCCLUSION_VISIBLE == 0 || stalled {
                let _: () = msg_send![view, displayLayer: std::ptr::null_mut::<Object>()];
            }
        }
    })
    .detach();
}
