const VS = `#version 300 es
in vec2 p; void main(){ gl_Position = vec4(p, 0., 1.); }`;
const FS = `#version 300 es
precision highp float;
uniform vec2 uRes, uNotch; uniform float uThick, uSpread, uTime, uPhase, uPulse, uHigh;
uniform vec3 uC1, uC2;
out vec4 o;
float sdBox(vec2 p, vec2 c, vec2 h){ vec2 d = abs(p - c) - h; return length(max(d, 0.)) + min(max(d.x, d.y), 0.); }
void main(){
  vec2 p = vec2(gl_FragCoord.x, uRes.y - gl_FragCoord.y);
  float d = min(min(p.x, p.y), min(uRes.x - p.x, uRes.y - p.y));          // distance to nearest bezel
  if (uNotch.x > 0.) d = min(d, max(sdBox(p, vec2(uRes.x * .5, uNotch.y * .5), uNotch * .5), 0.)); // glow wraps the notch
  float th = uThick * (1. + uPulse * .6);
  float band = 1. - smoothstep(0., th, d);
  float halo = exp(-d / (uSpread * (1. + uPulse * .5)) * 2.5);
  float I = clamp(band * .9 + halo * (.55 + uPulse * .8), 0., 1.);
  vec2 c = uRes * .5;
  float a = atan(p.y - c.y, p.x - c.x);
  float tri = abs(fract(a / 6.28318 + uPhase) * 2. - 1.);                 // seamless around the perimeter
  float m = smoothstep(.45, .75, tri);                                     // ~60% primary / 40% secondary
  vec3 col = mix(uC1, uC2, m) * (1. + uHigh * .25 * sin(uTime * 8. + a * 6.));
  o = vec4(clamp(col, 0., 1.) * I, I);
}`;
const cv = document.getElementById('c');
const gl = cv.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: false, powerPreference: 'low-power' });
const mk = (t, s) => { const x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) console.error(gl.getShaderInfoLog(x)); return x; };
const pr = gl.createProgram(); gl.attachShader(pr, mk(gl.VERTEX_SHADER, VS)); gl.attachShader(pr, mk(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(pr); gl.useProgram(pr);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW); // one oversized triangle
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
const U = n => gl.getUniformLocation(pr, n);
const hex = h => [1, 3, 5].map(i => parseInt(h.substr(i, 2), 16) / 255);
const spring = (s, target, dt) => { const k = 220, c = 2 * Math.sqrt(k) * .85; s.v += (k * (target - s.x) - c * s.v) * dt; s.x += s.v * dt; };

let cur = null, phase = 0, last = performance.now(), lastDraw = 0, tAcc = 0;
const pulse = { x: 0, v: 0 }, hi = { x: 0, v: 0 }; let kick = 0;

function frame(now) {
  requestAnimationFrame(frame);
  const s = OL.settings; if (!s) return;
  const mode = s.mode, want = OL.colors().map(hex);
  if (!cur) cur = want.map(c => c.slice());
  const fresh = now - OL.audioAt < 250;
  const converging = cur.some((c, i) => c.some((v, j) => Math.abs(v - want[i][j]) > .003));
  const minGap = mode === 'sync' ? 0 : (mode === 'none' && !converging ? 250 : 33);   // idle modes render at ~30 fps or less
  if (now - lastDraw < minGap) return;
  const dt = Math.min((now - last) / 1000, .1); last = now; lastDraw = now; tAcc += dt;

  const k = 1 - Math.exp(-dt / .5);                                                    // ~1.5 s crossfade
  cur.forEach((c, i) => c.forEach((_, j) => c[j] += (want[i][j] - c[j]) * k));

  let target = 0, speed = 0, h = 0;
  if (mode === 'sync') {
    if (fresh && OL.audio) { kick = Math.max(kick * .9, OL.audio.beat ? .35 : 0); target = Math.min(1, OL.audio.bass * .7 + kick); h = OL.audio.high; }
    speed = .02 + (fresh ? OL.audio.level * .6 + h * .05 : 0);
  } else if (mode === 'default') { target = .12 + .1 * Math.sin(tAcc * .8); speed = .05; }
  else if (mode === 'idle') { target = .05 + .05 * Math.sin(tAcc * .3); speed = .012; }
  else { target = .05; speed = 0; }
  spring(pulse, target, dt); spring(hi, h, dt); phase = (phase + speed * dt) % 1;

  const r = devicePixelRatio, W = Math.round(cv.clientWidth * r), H = Math.round(cv.clientHeight * r);
  if (cv.width !== W || cv.height !== H) { cv.width = W; cv.height = H; gl.viewport(0, 0, W, H); }
  gl.uniform2f(U('uRes'), W, H);
  gl.uniform2f(U('uNotch'), s.notch ? 200 * r : 0, s.notch ? 32 * r : 0);
  gl.uniform1f(U('uThick'), s.thickness * r);
  gl.uniform1f(U('uSpread'), (20 + s.glow * 8) * r);
  gl.uniform1f(U('uTime'), tAcc); gl.uniform1f(U('uPhase'), phase);
  gl.uniform1f(U('uPulse'), Math.max(0, pulse.x)); gl.uniform1f(U('uHigh'), Math.max(0, hi.x));
  gl.uniform3fv(U('uC1'), cur[0]); gl.uniform3fv(U('uC2'), cur[1]);
  gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
}
OL.init().then(() => requestAnimationFrame(frame));
