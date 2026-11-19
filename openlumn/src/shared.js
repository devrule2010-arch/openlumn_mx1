const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const OL = window.OL = {
  invoke, listen, settings: null, track: null, audio: null, audioAt: 0,
  async init() {
    listen('settings', e => { OL.settings = e.payload; OL.onSettings?.(); });
    listen('track', e => { OL.track = e.payload; OL.onTrack?.(); });
    listen('playback', e => { OL.track = Object.assign(OL.track || {}, e.payload, { _at: performance.now() }); OL.onPlayback?.(); });
    listen('audio', e => { OL.audio = e.payload; OL.audioAt = performance.now(); OL.onAudio?.(); });
    OL.settings = await invoke('get_settings');
    OL.track = await invoke('get_track');
    return OL;
  },
  colors() {
    const s = OL.settings, t = OL.track;
    return (!s.overrideAlbum && t?.colors?.length === 2) ? t.colors : [s.primary, s.secondary];
  },
  save() { return invoke('set_settings', { settings: OL.settings }); },
  // Shared Now Playing binding for the floating and idle players
  bindPlayer() {
    const $ = id => document.getElementById(id), fmt = s => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
    let base = 0, at = performance.now();
    const paint = () => {
      const t = OL.track; if (!t || !t.title) { $('title').textContent = 'Nothing playing'; $('artist').textContent = ''; return; }
      $('title').textContent = t.title; $('artist').textContent = t.artist || '';
      if (t.art) { $('art').src = t.art; document.documentElement.style.setProperty('--art', `url(${t.art})`); }
      $('pp').textContent = t.playing ? '⏸' : '▶';
    };
    OL.onTrack = paint;
    OL.onPlayback = () => { base = OL.track.position || 0; at = performance.now(); paint(); };
    setInterval(() => {
      const t = OL.track; if (!t?.duration) return;
      const p = Math.min(t.duration, base + (t.playing ? (performance.now() - at) / 1000 : 0));
      $('prog').style.width = (100 * p / t.duration) + '%';
      if ($('time')) $('time').textContent = `${fmt(p)} / ${fmt(t.duration)}`;
    }, 500);
    document.querySelectorAll('[data-act]').forEach(b => b.onclick = () => invoke('media_control', { action: b.dataset.act }));
    const cv = $('wave');
    if (cv) {
      const g = cv.getContext('2d');
      OL.onAudio = () => {
        const r = devicePixelRatio; cv.width = cv.clientWidth * r; cv.height = cv.clientHeight * r;
        g.clearRect(0, 0, cv.width, cv.height); g.fillStyle = 'rgba(255,255,255,.7)';
        const n = 16, w = cv.width / n;
        OL.audio.bands.forEach((v, i) => { const h = Math.max(2 * r, v * cv.height); g.fillRect(i * w + w * .2, (cv.height - h) / 2, w * .6, h); });
      };
    }
    paint();
  },
};
