//! WASAPI loopback (via cpal) -> FFT -> bass/high/bands. No microphone is ever opened:
//! building an *input* stream on the default *output* device is WASAPI loopback.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use serde::Serialize;
use std::{
    sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const N: usize = 2048;

#[derive(Serialize, Clone)]
struct Frame { bass: f32, high: f32, level: f32, beat: bool, bands: [f32; 16] }

pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<Frame>();
    thread::spawn(move || { for f in rx { let _ = app.emit("audio", &f); } });
    thread::spawn(move || loop {
        if let Err(e) = run(&tx) { eprintln!("audio: {e}"); }
        thread::sleep(Duration::from_secs(2));
    });
}

fn run(tx: &mpsc::Sender<Frame>) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let dev = host.default_output_device().ok_or("no output device")?;
    let name = dev.name().unwrap_or_default();
    let cfg = dev.default_output_config()?;
    if cfg.sample_format() != cpal::SampleFormat::F32 { return Err("unsupported sample format".into()); }
    let ch = cfg.channels() as usize;
    let sr = cfg.sample_rate().0 as f32;
    let sc: cpal::StreamConfig = cfg.into();
    let mut an = Analyzer::new(sr);
    let tx = tx.clone();
    let failed = Arc::new(AtomicBool::new(false));
    let f2 = failed.clone();
    let stream = dev.build_input_stream(
        &sc,
        move |data: &[f32], _| {
            for fr in data.chunks(ch) { an.push(fr.iter().sum::<f32>() / ch as f32, &tx); }
        },
        move |e| { eprintln!("audio stream: {e}"); f2.store(true, Ordering::Relaxed); },
        None,
    )?;
    stream.play()?;
    loop {
        thread::sleep(Duration::from_secs(2));
        if failed.load(Ordering::Relaxed) { break; }
        let cur = host.default_output_device().and_then(|d| d.name().ok()).unwrap_or_default();
        if cur != name { break; } // default device changed: re-acquire
    }
    Ok(())
}

struct Analyzer {
    sr: f32, buf: Vec<f32>, win: Vec<f32>, fft: Arc<dyn Fft<f32>>,
    avg: f32, peak: f32, hpeak: f32, bpeak: f32, last_beat: Instant,
}
impl Analyzer {
    fn new(sr: f32) -> Self {
        let win = (0..N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / N as f32).cos()).collect();
        Self {
            sr, buf: Vec::with_capacity(N * 2), win, fft: FftPlanner::new().plan_fft_forward(N),
            avg: 0.0, peak: 1e-4, hpeak: 1e-4, bpeak: 1e-4, last_beat: Instant::now(),
        }
    }
    fn push(&mut self, x: f32, tx: &mpsc::Sender<Frame>) {
        self.buf.push(x);
        if self.buf.len() >= N {
            self.process(tx);
            self.buf.drain(..N / 2); // 50% overlap, ~47 frames/s at 48 kHz
        }
    }
    fn process(&mut self, tx: &mpsc::Sender<Frame>) {
        let level = (self.buf[..N].iter().map(|x| x * x).sum::<f32>() / N as f32).sqrt();
        if level < 1e-5 { return; } // silence: JS decays on its own
        let mut c: Vec<Complex<f32>> =
            self.buf[..N].iter().zip(&self.win).map(|(x, w)| Complex::new(x * w, 0.0)).collect();
        self.fft.process(&mut c);
        let mag: Vec<f32> = c[..N / 2].iter().map(|z| z.norm() / N as f32 * 2.0).collect();
        let bw = self.sr / N as f32;
        let range = |lo: f32, hi: f32| {
            let a = ((lo / bw).ceil() as usize).max(1);
            let b = ((hi / bw).floor() as usize).min(N / 2 - 1).max(a);
            mag[a..=b].iter().sum::<f32>() / (b - a + 1) as f32
        };
        let bass_raw = range(20.0, 150.0);
        let high_raw = range(2000.0, 12000.0);
        self.peak = (self.peak * 0.999).max(bass_raw).max(1e-4);
        self.hpeak = (self.hpeak * 0.999).max(high_raw).max(1e-4);
        let bass = (bass_raw / self.peak).min(1.0);
        let high = (high_raw / self.hpeak).min(1.0);
        let beat = bass_raw > self.avg * 1.4 && bass > 0.25 && self.last_beat.elapsed() > Duration::from_millis(120);
        if beat { self.last_beat = Instant::now(); }
        self.avg = self.avg * 0.9 + bass_raw * 0.1;

        let mut raw = [0f32; 16];
        for i in 0..16 {
            let lo = 40.0 * 400f32.powf(i as f32 / 16.0);
            let hi = (40.0 * 400f32.powf((i + 1) as f32 / 16.0)).min(self.sr / 2.0 - bw);
            raw[i] = if lo < hi { range(lo, hi) } else { 0.0 };
        }
        self.bpeak = (self.bpeak * 0.999).max(raw.iter().cloned().fold(0.0, f32::max)).max(1e-4);
        let mut bands = [0f32; 16];
        for i in 0..16 { bands[i] = (raw[i] / self.bpeak).min(1.0).sqrt(); }
        let _ = tx.send(Frame { bass, high, level, beat, bands });
    }
}
