// Fully synthesised audio: layered SFX with reverb + panning, squeaky bear babble, and an adaptive music loop.
let ac, master, sfxBus, musicBus, musicFilter, noiseBuf;
const A = { rate: 1, volume: 0.8, muted: false, lx: 0, lz: 0, lyaw: 0 };
const clamp = (v, a, b) => (v < a ? a : v > b ? b : v);

function init() {
  if (!ac) {
    const AC = window.AudioContext || window.webkitAudioContext; if (!AC) return;
    ac = new AC();
    const comp = ac.createDynamicsCompressor(); comp.threshold.value = -14; comp.ratio.value = 5; comp.connect(ac.destination);
    master = ac.createGain(); master.gain.value = A.muted ? 0 : A.volume; master.connect(comp);
    sfxBus = ac.createGain(); sfxBus.connect(master);
    const len = Math.floor(ac.sampleRate * 1.4), ir = ac.createBuffer(2, len, ac.sampleRate);
    for (let ch = 0; ch < 2; ch++) { const d = ir.getChannelData(ch); for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / len, 2.6); }
    const conv = ac.createConvolver(); conv.buffer = ir; const wet = ac.createGain(); wet.gain.value = 0.15; sfxBus.connect(conv); conv.connect(wet); wet.connect(master);
    musicFilter = ac.createBiquadFilter(); musicFilter.type = 'lowpass'; musicFilter.frequency.value = 18000; musicFilter.connect(master);
    musicBus = ac.createGain(); musicBus.gain.value = 0.55; musicBus.connect(musicFilter);
    noiseBuf = ac.createBuffer(1, ac.sampleRate, ac.sampleRate); const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    music.next = ac.currentTime + 0.1; setInterval(musicTick, 50);
  }
  if (ac.state === 'suspended') ac.resume();
}

function dest(sp) { if (!sp || !ac.createStereoPanner) return sfxBus; const p = ac.createStereoPanner(); p.pan.value = sp.p; p.connect(sfxBus); return p; }
function osc(out, type, f0, f1, t, dur, vol) {
  const o = ac.createOscillator(), g = ac.createGain();
  o.type = type; o.frequency.setValueAtTime(f0, t); if (f1 !== f0) o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
  g.gain.setValueAtTime(vol, t); g.gain.exponentialRampToValueAtTime(0.0008, t + dur);
  o.connect(g); g.connect(out); o.start(t); o.stop(t + dur + 0.03);
}
function nz(out, t, dur, ftype, f0, f1, vol, q = 0.7) {
  const s = ac.createBufferSource(), f = ac.createBiquadFilter(), g = ac.createGain();
  s.buffer = noiseBuf; s.loop = true; f.type = ftype; f.Q.value = q;
  f.frequency.setValueAtTime(f0, t); f.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + dur);
  g.gain.setValueAtTime(vol, t); g.gain.exponentialRampToValueAtTime(0.0008, t + dur);
  s.connect(f); f.connect(g); g.connect(out); s.start(t, Math.random() * 0.5); s.stop(t + dur + 0.03);
}
// SFX wrappers: apply the slow-mo pitch rate and spatialisation
function tone(type, f0, f1, dur, vol, delay = 0, sp) { if (!ac) return; const r = A.rate; osc(dest(sp), type, f0 * r, f1 * r, ac.currentTime + delay, dur / r, vol * (sp ? sp.v : 1)); }
function noise(dur, ftype, f0, f1, vol, delay = 0, sp, q) { if (!ac) return; const r = A.rate; nz(dest(sp), ac.currentTime + delay, dur / r, ftype, f0 * r, f1 * r, vol * (sp ? sp.v : 1), q); }

const sfx = {
  blaster() { noise(0.1, 'bandpass', 3000, 500, 0.6, 0, null, 0.6); tone('sine', 190, 48, 0.11, 0.55); noise(0.03, 'highpass', 6500, 4000, 0.22); noise(0.22, 'lowpass', 900, 200, 0.12, 0.04); },
  scatter() { noise(0.32, 'lowpass', 4200, 200, 0.85); tone('sine', 150, 36, 0.3, 0.75); noise(0.05, 'highpass', 5000, 3000, 0.3); },
  pump() { noise(0.05, 'bandpass', 1400, 900, 0.22); noise(0.06, 'bandpass', 2200, 1200, 0.22, 0.12); },
  cannon() { tone('sine', 210, 46, 0.3, 0.7); noise(0.16, 'lowpass', 1200, 160, 0.5); noise(0.05, 'bandpass', 900, 500, 0.3, 0, null, 3); },
  boom(sp) { noise(0.9, 'lowpass', 2400, 50, 0.95, 0, sp); tone('sine', 95, 24, 0.7, 0.9, 0, sp); for (let i = 0; i < 14; i++) noise(0.03, 'highpass', 6000, 4000, 0.11, 0.15 + Math.random() * 0.9, sp); },
  egun(kind, sp) {
    if (kind === 'rifle' || kind === 'minigun') { noise(0.07, 'bandpass', 2600, 700, 0.5, 0, sp, 0.6); tone('sine', 170, 55, 0.07, 0.4, 0, sp); }
    else if (kind === 'sniper') { noise(0.4, 'lowpass', 7000, 250, 0.9, 0, sp); tone('sine', 130, 30, 0.4, 0.7, 0, sp); }
    else if (kind === 'rpg') { noise(0.55, 'bandpass', 500, 2400, 0.5, 0, sp, 1.2); tone('sine', 110, 50, 0.3, 0.5, 0, sp); }
    else { noise(0.09, 'bandpass', 1900, 600, 0.5, 0, sp, 0.7); tone('sine', 210, 60, 0.08, 0.4, 0, sp); }
  },
  whiz() { noise(0.13, 'bandpass', 3200, 900, 0.14, 0, null, 6); },
  hit() { noise(0.06, 'lowpass', 2400, 400, 0.24); tone('sine', 520, 230, 0.06, 0.16); },
  head() { tone('triangle', 1250, 1900, 0.09, 0.22); noise(0.05, 'lowpass', 2600, 500, 0.2); },
  kill(sp) { noise(0.24, 'lowpass', 1100, 120, 0.4, 0, sp); tone('sine', 330, 55, 0.28, 0.34, 0, sp); },
  pop(sp) { tone('sine', 500, 1300, 0.07, 0.3, 0, sp); noise(0.05, 'highpass', 3000, 2000, 0.2, 0, sp); },
  disarm() { tone('sine', 1000, 180, 0.38, 0.2); tone('triangle', 700, 900, 0.05, 0.2); },
  clang(sp) { tone('square', 1900, 1750, 0.08, 0.1, 0, sp); tone('triangle', 2950, 2700, 0.22, 0.08, 0, sp); },
  hurt() { tone('sawtooth', 170, 55, 0.26, 0.3); noise(0.14, 'lowpass', 700, 150, 0.35); },
  bit(n) { const f = 660 * Math.pow(1.06, Math.min(n, 14)); tone('triangle', f, f * 1.5, 0.07, 0.15); },
  dash() { noise(0.22, 'bandpass', 500, 2600, 0.3, 0, null, 2); },
  jump() { tone('sine', 260, 430, 0.09, 0.09); },
  land() { noise(0.08, 'lowpass', 320, 80, 0.4); },
  reload(dur) { tone('triangle', 420, 640, 0.06, 0.18); noise(0.04, 'bandpass', 1800, 1200, 0.2, dur * 0.45); tone('triangle', 640, 980, 0.07, 0.2, dur * 0.88); },
  swap() { noise(0.07, 'bandpass', 900, 2200, 0.2, 0, null, 1.5); tone('triangle', 500, 700, 0.05, 0.1, 0.06); },
  charge(sp) { tone('sine', 1500, 1500, 0.06, 0.16, 0, sp); },
  lock(sp) { tone('square', 2100, 2100, 0.05, 0.12, 0, sp); tone('square', 2100, 2100, 0.05, 0.12, 0.09, sp); },
  wave() { [220, 277, 330, 440].forEach((f, i) => tone('sawtooth', f, f, 0.5, 0.09, i * 0.09)); noise(0.6, 'lowpass', 300, 3000, 0.2); },
  clear() { [523, 659, 784, 1047, 1319].forEach((f, i) => tone('triangle', f, f, 0.3, 0.2, i * 0.08)); },
  rushOn() { tone('sawtooth', 110, 880, 0.5, 0.2); noise(0.6, 'bandpass', 300, 6000, 0.3, 0, null, 2); [523, 659, 784, 1047].forEach((f, i) => tone('square', f, f, 0.2, 0.08, 0.3 + i * 0.05)); },
  rushOff() { tone('sawtooth', 700, 90, 0.5, 0.18); },
  heart() { tone('sine', 70, 40, 0.12, 0.5); tone('sine', 62, 36, 0.12, 0.4, 0.17); },
  boss() { tone('sawtooth', 160, 70, 0.9, 0.3); tone('square', 320, 150, 0.9, 0.12); noise(0.8, 'lowpass', 500, 100, 0.4); },
  over() { [392, 330, 262, 196].forEach((f, i) => tone('sawtooth', f, f * 0.97, 0.32, 0.16, i * 0.18)); },
  ui() { tone('triangle', 700, 1000, 0.05, 0.14); },
  step(run) { noise(0.06, 'lowpass', 500 + Math.random() * 300, 120, run ? 0.2 : 0.12); tone('sine', 90, 55, 0.05, run ? 0.14 : 0.08); },
  slide() { noise(0.75, 'bandpass', 1400, 500, 0.4, 0, null, 0.8); noise(0.15, 'lowpass', 400, 100, 0.4); },
  mantle() { noise(0.12, 'lowpass', 600, 150, 0.3); noise(0.1, 'bandpass', 900, 500, 0.18, 0.18, null, 1.5); },
  swing() { noise(0.16, 'bandpass', 700, 2600, 0.3, 0, null, 2); },
  thwack() { noise(0.09, 'lowpass', 1600, 200, 0.6); tone('sine', 200, 60, 0.12, 0.55); tone('square', 900, 300, 0.04, 0.1); },
  pickup() { noise(0.06, 'bandpass', 2400, 1400, 0.25, 0, null, 2); tone('triangle', 500, 760, 0.07, 0.16, 0.05); },
  splash() { noise(0.7, 'lowpass', 3200, 300, 0.6); noise(0.4, 'highpass', 4000, 6000, 0.14, 0.1); },
  wind() { noise(7.5, 'bandpass', 600, 260, 0.4, 0, null, 0.5); noise(7.5, 'lowpass', 300, 120, 0.28); },
  dive() { noise(6, 'lowpass', 300, 180, 0.5); for (let i = 0; i < 14; i++) tone('sine', 500 + Math.random() * 500, 1100 + Math.random() * 500, 0.07, 0.05, 0.4 + Math.random() * 5); tone('sawtooth', 62, 58, 5.5, 0.09); },
  deny() { tone('square', 200, 150, 0.12, 0.12); },
  buy() { [784, 988, 1319].forEach((f, i) => tone('triangle', f, f, 0.12, 0.2, i * 0.05)); noise(0.08, 'highpass', 6000, 9000, 0.1); },
  levelup() { [523, 659, 784, 1047, 1319, 1568].forEach((f, i) => tone('square', f, f, 0.22, 0.09, i * 0.07)); tone('sawtooth', 131, 262, 0.6, 0.15); },
  star(i) { const f = 880 * Math.pow(1.26, i); tone('triangle', f, f, 0.3, 0.22); tone('sine', f * 2, f * 2, 0.4, 0.1); },
  radio() { noise(0.05, 'bandpass', 2400, 2400, 0.14, 0, null, 3); tone('square', 1300, 1300, 0.04, 0.05, 0.05); },
  win() { [392, 523, 659, 784, 1047].forEach((f, i) => { tone('sawtooth', f, f, 0.5, 0.1, i * 0.1); tone('triangle', f * 2, f * 2, 0.5, 0.1, i * 0.1); }); },
};

// Animal-crossing-style gibberish for bear barks
function babble(sp, pitch = 1) {
  if (!ac) return;
  const n = 3 + ((Math.random() * 4) | 0);
  for (let i = 0; i < n; i++) { const f = (520 + Math.random() * 520) * pitch; tone(i % 2 ? 'square' : 'triangle', f, f * (0.85 + Math.random() * 0.4), 0.055, 0.06, i * 0.07, sp); }
}

// ───────────────────────── adaptive music ─────────────────────────
const STEP = 60 / 128 / 4;
const CH = [[110, [220, 261.63, 329.63, 440]], [87.31, [174.61, 220, 261.63, 349.23]], [130.81, [261.63, 329.63, 392, 523.25]], [98, [196, 246.94, 293.66, 392]]];
const ARP = [0, 2, 1, 2, 3, 2, 1, 2, 0, 2, 1, 3, 2, 3, 1, 2];
const LEAD = [
  [[0, 880, 3], [4, 783.99, 2], [6, 659.25, 2], [10, 587.33, 2], [12, 659.25, 4]],
  [[0, 698.46, 3], [4, 659.25, 2], [8, 523.25, 2], [12, 587.33, 4]],
  [[0, 783.99, 2], [3, 659.25, 1], [6, 523.25, 2], [8, 659.25, 2], [12, 783.99, 4]],
  [[0, 587.33, 3], [4, 493.88, 2], [8, 587.33, 2], [12, 783.99, 2], [14, 880, 2]],
];
const music = { I: 0, target: 0.15, step: 0, next: 0 };
function musicTick() {
  if (!ac || ac.state !== 'running') return;
  music.I += (music.target - music.I) * 0.06;
  if (music.next < ac.currentTime) music.next = ac.currentTime + 0.05;
  while (music.next < ac.currentTime + 0.2) { schedStep(music.step, music.next); music.next += STEP; music.step = (music.step + 1) % 64; }
}
function pad(f, t, dur, vol) {
  const g = ac.createGain(), fl = ac.createBiquadFilter(); fl.type = 'lowpass'; fl.frequency.value = 900;
  g.gain.setValueAtTime(0.0001, t); g.gain.linearRampToValueAtTime(vol, t + dur * 0.3); g.gain.linearRampToValueAtTime(0.0001, t + dur);
  for (const det of [-6, 6]) { const o = ac.createOscillator(); o.type = 'sawtooth'; o.frequency.value = f; o.detune.value = det; o.connect(fl); o.start(t); o.stop(t + dur + 0.05); }
  fl.connect(g); g.connect(musicBus);
}
function bass(f, t, dur, vol) {
  const o = ac.createOscillator(), fl = ac.createBiquadFilter(), g = ac.createGain();
  o.type = 'sawtooth'; o.frequency.value = f; fl.type = 'lowpass'; fl.Q.value = 5; fl.frequency.setValueAtTime(700, t); fl.frequency.exponentialRampToValueAtTime(180, t + dur);
  g.gain.setValueAtTime(vol, t); g.gain.exponentialRampToValueAtTime(0.001, t + dur);
  o.connect(fl); fl.connect(g); g.connect(musicBus); o.start(t); o.stop(t + dur + 0.03);
}
function schedStep(st, t) {
  const bar = st >> 4, s = st & 15, [root, tones] = CH[bar], I = music.I;
  if (s === 0) for (let i = 0; i < 3; i++) pad(tones[i], t, STEP * 16, 0.03);
  const af = tones[ARP[s]] * 2; osc(musicBus, 'triangle', af, af, t, STEP * 1.6, 0.045 + I * 0.04);
  if (I > 0.3 && s % 4 === 2) nz(musicBus, t, 0.04, 'highpass', 8000, 8000, 0.05 + I * 0.05);
  if (I > 0.6 && s % 2 === 1) nz(musicBus, t, 0.025, 'highpass', 9000, 9000, 0.035);
  if (I > 0.45) {
    if (s % 4 === 0) osc(musicBus, 'sine', 150, 42, t, 0.16, 0.6);
    if (s === 4 || s === 12) nz(musicBus, t, 0.13, 'bandpass', 1900, 1200, 0.3, 0.8);
  }
  if (I > 0.4 && s % 2 === 0) bass(s === 6 || s === 14 ? root * 2 : root, t, STEP * 1.7, 0.22);
  if (I > 0.8) for (const [ls, f, len] of LEAD[bar]) if (ls === s) osc(musicBus, 'square', f, f, t, STEP * len, 0.04);
}

export const audio = {
  init, sfx, babble,
  setVolume(v) { A.volume = v; if (master) master.gain.value = A.muted ? 0 : v; },
  toggleMute() { A.muted = !A.muted; if (master) master.gain.value = A.muted ? 0 : A.volume; return A.muted; },
  setRate(r) { A.rate = r; },
  setListener(x, z, yaw) { A.lx = x; A.lz = z; A.lyaw = yaw; },
  sp(x, z) {
    const dx = x - A.lx, dz = z - A.lz, d = Math.hypot(dx, dz) || 1e-4;
    return { v: 1 / (1 + d * 0.07), p: clamp((dx * Math.cos(A.lyaw) - dz * Math.sin(A.lyaw)) / d, -1, 1) * 0.8 };
  },
  musicIntensity(v) { music.target = v; },
  muffle(on) { if (musicFilter) musicFilter.frequency.setTargetAtTime(on ? 650 : 18000, ac.currentTime, 0.12); },
};
