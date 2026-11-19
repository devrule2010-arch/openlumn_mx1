//! Now-playing via Windows SMTC (Spotify, Apple Music, browsers, anything that registers a media session).
use crate::{color, state::{AppState, Track}};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::{sync::atomic::Ordering, thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager_,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Storage::Streams::{DataReader, IRandomAccessStreamReference},
    Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
};

#[derive(Serialize, Clone)]
struct Playback { playing: bool, position: f64, duration: f64 }

fn session() -> windows::core::Result<Session> {
    Manager_::RequestAsync()?.get()?.GetCurrentSession()
}

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        unsafe { let _ = RoInitialize(RO_INIT_MULTITHREADED); }
        let mut last = String::new();
        loop {
            if let Err(e) = poll(&app, &mut last) { eprintln!("media: {e}"); }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

fn read_thumb(t: &IRandomAccessStreamReference) -> windows::core::Result<Vec<u8>> {
    let stream = t.OpenReadAsync()?.get()?;
    let size = stream.Size()? as u32;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    reader.LoadAsync(size)?.get()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

fn poll(app: &AppHandle, last: &mut String) -> windows::core::Result<()> {
    let st = app.state::<AppState>();
    let s = match session() {
        Ok(s) => s,
        Err(_) => {
            st.playing.store(false, Ordering::Relaxed);
            let _ = app.emit("playback", Playback { playing: false, position: 0.0, duration: 0.0 });
            return Ok(());
        }
    };
    let props = s.TryGetMediaPropertiesAsync()?.get()?;
    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();
    let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    let tl = s.GetTimelineProperties()?;
    let position = tl.Position()?.Duration as f64 / 1e7;
    let duration = (tl.EndTime()?.Duration - tl.StartTime()?.Duration) as f64 / 1e7;
    st.playing.store(playing, Ordering::Relaxed);

    let key = format!("{title}\u{1}{artist}");
    if key != *last {
        *last = key;
        let bytes = props.Thumbnail().ok().and_then(|t| read_thumb(&t).ok());
        let colors = bytes.as_deref().and_then(color::extract).unwrap_or_default();
        let art = bytes.map(|b| format!("data:image/png;base64,{}", STANDARD.encode(b)));
        let track = Track { title, artist, art, colors, playing, position, duration };
        *st.track.lock().unwrap() = Some(track.clone());
        let _ = app.emit("track", &track);
    } else if let Some(t) = st.track.lock().unwrap().as_mut() {
        t.playing = playing; t.position = position; t.duration = duration;
    }
    let _ = app.emit("playback", Playback { playing, position, duration });
    Ok(())
}

pub fn control(action: &str) {
    let action = action.to_string();
    thread::spawn(move || {
        unsafe { let _ = RoInitialize(RO_INIT_MULTITHREADED); }
        if let Ok(s) = session() {
            let _ = match action.as_str() {
                "toggle" => s.TryTogglePlayPauseAsync().and_then(|o| o.get()),
                "next" => s.TrySkipNextAsync().and_then(|o| o.get()),
                "prev" => s.TrySkipPreviousAsync().and_then(|o| o.get()),
                _ => Ok(false),
            };
        }
    });
}
