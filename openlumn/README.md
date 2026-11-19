# OpenLumn (Windows)

Music-reactive rim lighting for your display edges. Tauri v2 (Rust + WebView2), WebGL2 shader overlay, WASAPI loopback FFT, SMTC now-playing.

## Build
1. Install Rust (MSVC toolchain) and the "Desktop development with C++" Build Tools. WebView2 ships with Windows 10/11.
2. `cargo install tauri-cli --version "^2"`
3. From `openlumn/src-tauri`: `cargo tauri icon path\to\square-1024.png` (creates `icons/icon.ico`, needed for the tray and installer).
4. `cargo tauri dev` to run, `cargo tauri build` for an installer.

## Use
Click the tray icon for the control panel; right-click to quit.

## Layout
- `src-tauri/src/audio.rs` loopback capture and FFT (bass 20-150 Hz, highs 2-12 kHz, 16 bands, beat detect)
- `src-tauri/src/media.rs` SMTC track, artwork, playback and transport controls
- `src-tauri/src/color.rs` album-art palette (k-means on vibrant pixels)
- `src-tauri/src/wins.rs` click-through overlays per monitor, popover, player, idle watcher
- `src/overlay.js` GLSL shader, spring physics, color crossfade
- `src/index.html`, `popover.js`, `player.html`, `idle.html`, `styles.css`, `shared.js` UI

## Known limits
- Written without a Rust toolchain or network available here, so it has not been compiled. Expect a few small API fixes on first `cargo build`.
- Windows cannot draw over the secure lock screen. The "Idle player" shows after 2 minutes without input while music plays.
- Exclusive-fullscreen games hide any overlay. Borderless-windowed works.
- Each overlay is a WebView2 process, so RAM will likely exceed the 60 MB target. A native wgpu overlay is the way to hit that budget.
- Windows laptops have no notch API. The notch toggle cuts out a fixed 200x32 px area at top center.
- Overlays are excluded from screen capture. Everything runs locally; the only network use would be none, since artwork comes from SMTC.
