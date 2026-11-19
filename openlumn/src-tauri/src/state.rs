use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::{atomic::AtomicBool, Mutex}};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub active: bool,
    pub mode: String, // sync | default | idle | none
    pub override_album: bool,
    pub primary: String,
    pub secondary: String,
    pub notch: bool,
    pub thickness: f32,
    pub glow: f32,
    pub disabled_displays: Vec<usize>,
    pub floating_player: bool,
    pub lockscreen_player: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            active: true, mode: "sync".into(), override_album: false,
            primary: "#ff3d81".into(), secondary: "#3d7bff".into(), notch: false,
            thickness: 24.0, glow: 50.0, disabled_displays: vec![],
            floating_player: false, lockscreen_player: false,
        }
    }
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub art: Option<String>,
    pub colors: Vec<String>,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub track: Mutex<Option<Track>>,
    pub playing: AtomicBool,
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}
pub fn load(app: &AppHandle) {
    if let Some(p) = path(app) {
        if let Ok(t) = std::fs::read_to_string(p) {
            if let Ok(s) = serde_json::from_str::<Settings>(&t) {
                *app.state::<AppState>().settings.lock().unwrap() = s;
            }
        }
    }
}
pub fn save(app: &AppHandle) {
    if let Some(p) = path(app) {
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        let s = app.state::<AppState>().settings.lock().unwrap().clone();
        let _ = std::fs::write(p, serde_json::to_string_pretty(&s).unwrap());
    }
}
