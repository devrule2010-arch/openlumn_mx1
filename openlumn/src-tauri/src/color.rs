//! Album-art palette: filter to vibrant pixels, k-means (k=6), rank by population x saturation.
fn hsv(c: [f32; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] / 255.0, c[1] / 255.0, c[2] / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d == 0.0 { 0.0 }
        else if mx == r { 60.0 * (((g - b) / d) % 6.0) }
        else if mx == g { 60.0 * ((b - r) / d + 2.0) }
        else { 60.0 * ((r - g) / d + 4.0) };
    (if h < 0.0 { h + 360.0 } else { h }, if mx == 0.0 { 0.0 } else { d / mx }, mx)
}
fn rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 % 6 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}
fn vivid(c: [f32; 3]) -> String {
    let (h, s, v) = hsv(c);
    let o = rgb(h, s.max(0.55), v.max(0.85));
    format!("#{:02x}{:02x}{:02x}", o[0] as u8, o[1] as u8, o[2] as u8)
}
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub fn extract(bytes: &[u8]) -> Option<Vec<String>> {
    let img = image::load_from_memory(bytes).ok()?.thumbnail(64, 64).to_rgb8();
    let px: Vec<[f32; 3]> = img.pixels()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .filter(|&c| { let (_, s, v) = hsv(c); s > 0.25 && v > 0.3 })
        .collect();
    if px.len() < 20 { return None; } // greyscale / muddy art: keep user colors
    let k = 6;
    let mut cent: Vec<[f32; 3]> = (0..k).map(|i| px[i * px.len() / k]).collect();
    let mut cnt = vec![0usize; k];
    for _ in 0..12 {
        let mut sum = vec![[0f32; 3]; k];
        cnt = vec![0; k];
        for p in &px {
            let j = (0..k).min_by(|&a, &b| dist(*p, cent[a]).partial_cmp(&dist(*p, cent[b])).unwrap()).unwrap();
            for i in 0..3 { sum[j][i] += p[i]; }
            cnt[j] += 1;
        }
        for j in 0..k {
            if cnt[j] > 0 { for i in 0..3 { cent[j][i] = sum[j][i] / cnt[j] as f32; } }
        }
    }
    let score = |j: usize| cnt[j] as f32 * (0.4 + hsv(cent[j]).1);
    let mut order: Vec<usize> = (0..k).filter(|&j| cnt[j] > 0).collect();
    order.sort_by(|&a, &b| score(b).partial_cmp(&score(a)).unwrap());
    let first = order[0];
    let second = order.iter().copied().find(|&j| dist(cent[j], cent[first]) > 90.0);
    let c1 = vivid(cent[first]);
    let c2 = match second {
        Some(j) => vivid(cent[j]),
        None => { let (h, s, v) = hsv(cent[first]); vivid(rgb((h + 40.0) % 360.0, s, v)) }
    };
    Some(vec![c1, c2])
}
