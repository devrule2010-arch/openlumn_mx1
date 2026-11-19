//! Window management: per-monitor click-through overlays, popover, floating player, idle player.
use crate::state::AppState;
use std::{sync::atomic::{AtomicU64, Ordering}, thread, time::{Duration, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use window_vibrancy::{apply_acrylic, apply_blur};

pub static LAST_HIDE: AtomicU64 = AtomicU64::new(0);
pub fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 }

fn glass(w: &WebviewWindow) {
    if apply_acrylic(w, Some((16, 16, 20, 110))).is_err() { let _ = apply_blur(w, Some((16, 16, 20, 160))); }
}

pub fn create_popover(app: &AppHandle) -> tauri::Result<()> {
    let w = WebviewWindowBuilder::new(app, "popover", WebviewUrl::App("index.html".into()))
        .inner_size(340.0, 600.0).decorations(false).transparent(true).always_on_top(true)
        .skip_taskbar(true).resizable(false).visible(false).build()?;
    glass(&w);
    let w2 = w.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e { LAST_HIDE.store(now_ms(), Ordering::Relaxed); let _ = w2.hide(); }
    });
    Ok(())
}

pub fn toggle_popover(app: &AppHandle, at: PhysicalPosition<f64>) {
    let Some(w) = app.get_webview_window("popover") else { return };
    if w.is_visible().unwrap_or(false) { let _ = w.hide(); return; }
    if now_ms() - LAST_HIDE.load(Ordering::Relaxed) < 250 { return; } // click that caused the blur
    let size = w.outer_size().unwrap();
    let x = (at.x - size.width as f64 / 2.0).max(0.0);
    let y = if at.y > size.height as f64 + 16.0 { at.y - size.height as f64 - 12.0 } else { at.y + 12.0 };
    let _ = w.set_position(PhysicalPosition::new(x as i32, y as i32));
    let _ = w.show();
    let _ = w.set_focus();
}

fn overlay_like(app: &AppHandle, label: &str, url: &str, mon: &tauri::Monitor) {
    let b = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .decorations(false).transparent(true).always_on_top(true).skip_taskbar(true)
        .shadow(false).focused(false).resizable(false).visible(false);
    if let Ok(w) = b.build() {
        let _ = w.set_position(*mon.position());
        let _ = w.set_size(*mon.size());
        let _ = w.set_ignore_cursor_events(true);   // 100% click-through
        let _ = w.set_content_protected(true);      // WDA_EXCLUDEFROMCAPTURE: hidden from recordings
        let _ = w.show();
    }
}

/// Reconcile open windows with current settings. Call from async commands or threads only.
pub fn sync(app: &AppHandle) {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let mons = app.available_monitors().unwrap_or_default();
    for (i, m) in mons.iter().enumerate() {
        let label = format!("overlay-{i}");
        let want = s.active && !s.disabled_displays.contains(&i);
        match (app.get_webview_window(&label), want) {
            (None, true) => overlay_like(app, &label, "overlay.html", m),
            (Some(w), false) => { let _ = w.close(); }
            _ => {}
        }
    }
    for (label, w) in app.webview_windows() {
        if let Some(n) = label.strip_prefix("overlay-").and_then(|n| n.parse::<usize>().ok()) {
            if n >= mons.len() { let _ = w.close(); }
        }
    }
    match (app.get_webview_window("player"), s.floating_player) {
        (None, true) => {
            if let Ok(w) = WebviewWindowBuilder::new(app, "player", WebviewUrl::App("player.html".into()))
                .inner_size(380.0, 150.0).position(80.0, 80.0).decorations(false).transparent(true)
                .always_on_top(true).skip_taskbar(true).resizable(false).build() { glass(&w); }
        }
        (Some(w), false) => { let _ = w.close(); }
        _ => {}
    }
}

fn idle_ms() -> u32 {
    use windows::Win32::{System::SystemInformation::GetTickCount, UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO}};
    let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe { if GetLastInputInfo(&mut li).as_bool() { GetTickCount().wrapping_sub(li.dwTime) } else { 0 } }
}

/// Windows cannot draw over the secure lock screen, so the "lockscreen" player is an idle overlay
/// (shown after 2 min without input while music plays) on the primary display.
pub fn start_idle_watch(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        let st = app.state::<AppState>();
        let enabled = st.settings.lock().unwrap().lockscreen_player;
        let playing = st.playing.load(Ordering::Relaxed);
        let want = enabled && playing && idle_ms() > 120_000;
        let existing = app.get_webview_window("idle");
        match (existing, want) {
            (None, true) => if let Ok(Some(m)) = app.primary_monitor() { overlay_like(&app, "idle", "idle.html", &m); },
            (Some(w), false) => { let _ = w.close(); }
            _ => {}
        }
    });
}
