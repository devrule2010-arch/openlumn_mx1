#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio; mod color; mod media; mod state; mod wins;

use serde::Serialize;
use state::{AppState, Settings, Track};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

#[tauri::command]
async fn get_settings(app: AppHandle) -> Settings { app.state::<AppState>().settings.lock().unwrap().clone() }

#[tauri::command]
async fn set_settings(app: AppHandle, settings: Settings) {
    *app.state::<AppState>().settings.lock().unwrap() = settings.clone();
    state::save(&app);
    let _ = app.emit("settings", &settings);
    wins::sync(&app);
}

#[derive(Serialize)]
struct Mon { index: usize, name: String, width: u32, height: u32 }

#[tauri::command]
async fn get_monitors(app: AppHandle) -> Vec<Mon> {
    app.available_monitors().unwrap_or_default().iter().enumerate().map(|(i, m)| Mon {
        index: i,
        name: m.name().cloned().unwrap_or_else(|| format!("Display {}", i + 1)),
        width: m.size().width, height: m.size().height,
    }).collect()
}

#[tauri::command]
async fn get_track(app: AppHandle) -> Option<Track> { app.state::<AppState>().track.lock().unwrap().clone() }

#[tauri::command]
async fn media_control(action: String) { media::control(&action) }

#[tauri::command]
async fn quit(app: AppHandle) { app.exit(0) }

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![get_settings, set_settings, get_monitors, get_track, media_control, quit])
        .setup(|app| {
            let h = app.handle().clone();
            state::load(&h);
            wins::create_popover(&h)?;
            let menu = Menu::with_items(app, &[&MenuItem::with_id(app, "quit", "Quit OpenLumn", true, None::<&str>)?])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("OpenLumn")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| if ev.id.as_ref() == "quit" { app.exit(0) })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, position, .. } = ev {
                        wins::toggle_popover(tray.app_handle(), position);
                    }
                })
                .build(app)?;
            audio::start(h.clone());
            media::start(h.clone());
            wins::start_idle_watch(h.clone());
            let h2 = h.clone();
            std::thread::spawn(move || wins::sync(&h2)); // build overlays off the setup path
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running OpenLumn");
}
