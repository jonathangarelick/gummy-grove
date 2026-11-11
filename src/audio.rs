//! Fully synthesised audio: layered effects with reverb and panning, squeaky bear babble, an adaptive music loop.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::TAU;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy)]
enum Wave { Sine, Square, Saw, Tri, Noise }
#[derive(Clone, Copy)]
enum Filt { None, Low, High, Band }
#[derive(Clone, Copy)]
struct Voice {
    wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32, pan: f32, filt: Filt, c0: f32, c1: f32, q: f32, music: bool, pad: bool,
    age: f32, phase: f32, z1: f32, z2: f32, seed: u32,
}
enum Cmd { Voice(Voice), Music(f32), Muffle(bool), Volume(f32) }

struct Synth {
    sr: f32, voices: Vec<Voice>, volume: f32, muffle: bool, mlp: [f32; 2], intensity: f32, target: f32,
    step: u32, next: f32, clock: f32, combs: Vec<(Vec<f32>, usize, f32)>, aps: Vec<(Vec<f32>, usize)>,
}
const STEP: f32 = 60.0 / 128.0 / 4.0;
const CH: [(f32, [f32; 4]); 4] = [(110.0, [220.0, 261.63, 329.63, 440.0]), (87.31, [174.61, 220.0, 261.63, 349.23]), (130.81, [261.63, 329.63, 392.0, 523.25]), (98.0, [196.0, 246.94, 293.66, 392.0])];
const ARP: [usize; 16] = [0, 2, 1, 2, 3, 2, 1, 2, 0, 2, 1, 3, 2, 3, 1, 2];
const LEAD: [&[(u32, f32, f32)]; 4] = [
    &[(0, 880.0, 3.0), (4, 783.99, 2.0), (6, 659.25, 2.0), (10, 587.33, 2.0), (12, 659.25, 4.0)],
    &[(0, 698.46, 3.0), (4, 659.25, 2.0), (8, 523.25, 2.0), (12, 587.33, 4.0)],
    &[(0, 783.99, 2.0), (3, 659.25, 1.0), (6, 523.25, 2.0), (8, 659.25, 2.0), (12, 783.99, 4.0)],
    &[(0, 587.33, 3.0), (4, 493.88, 2.0), (8, 587.33, 2.0), (12, 783.99, 2.0), (14, 880.0, 2.0)],
];
fn v(wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32) -> Voice {
    Voice { wave, f0, f1, dur, vol, delay: 0.0, pan: 0.0, filt: Filt::None, c0: 1000.0, c1: 1000.0, q: 0.7, music: false, pad: false, age: 0.0, phase: 0.0, z1: 0.0, z2: 0.0, seed: 1 }
}
fn nz(dur: f32, filt: Filt, c0: f32, c1: f32, vol: f32) -> Voice { Voice { filt, c0, c1, ..v(Wave::Noise, 0.0, 0.0, dur, vol) } }

impl Synth {
    fn new(sr: f32) -> Synth {
        let k = sr / 44100.0;
        Synth {
            sr, voices: Vec::with_capacity(96), volume: 0.8, muffle: false, mlp: [0.0; 2], intensity: 0.0, target: 0.15, step: 0, next: 0.1, clock: 0.0,
            combs: [1557.0, 1617.0, 1491.0, 1422.0].iter().map(|d| (vec![0.0; (d * k) as usize], 0, 0.0)).collect(),
            aps: [225.0, 556.0].iter().map(|d| (vec![0.0; (d * k) as usize], 0)).collect(),
        }
    }
    fn music(&mut self, st: u32, when: f32) {
        let (bar, s) = ((st >> 4) as usize, st & 15);
        let (root, tones) = CH[bar];
        let i = self.intensity;
        let mut push = |mut x: Voice| { x.music = true; x.delay = when; self.voices.push(x); };
        if s == 0 { for t in &tones[..3] { for det in [0.9965f32, 1.0035] { push(Voice { pad: true, filt: Filt::Low, c0: 900.0, c1: 900.0, ..v(Wave::Saw, t * det, t * det, STEP * 16.0, 0.016) }); } } }
        let af = tones[ARP[s as usize]] * 2.0;
        push(v(Wave::Tri, af, af, STEP * 1.6, 0.045 + i * 0.04));
        if i > 0.3 && s % 4 == 2 { push(nz(0.04, Filt::High, 8000.0, 8000.0, 0.05 + i * 0.05)); }
        if i > 0.6 && s % 2 == 1 { push(nz(0.025, Filt::High, 9000.0, 9000.0, 0.035)); }
        if i > 0.45 {
            if s % 4 == 0 { push(v(Wave::Sine, 150.0, 42.0, 0.16, 0.6)); }
            if s == 4 || s == 12 { push(Voice { q: 0.8, ..nz(0.13, Filt::Band, 1900.0, 1200.0, 0.3) }); }
        }
        if i > 0.4 && s % 2 == 0 { let f = if s == 6 || s == 14 { root * 2.0 } else { root }; push(Voice { filt: Filt::Low, c0: 700.0, c1: 180.0, q: 3.0, ..v(Wave::Saw, f, f, STEP * 1.7, 0.22) }); }
        if i > 0.8 { for &(ls, f, len) in LEAD[bar] { if ls == s { push(v(Wave::Square, f, f, STEP * len, 0.04)); } } }
    }
    fn render(&mut self, out: &mut [f32], ch: usize, rx: &Receiver<Cmd>) {
        while let Ok(c) = rx.try_recv() {
            match c {
                Cmd::Voice(x) => { if self.voices.len() < 90 { self.voices.push(x); } }
                Cmd::Music(t) => self.target = t, Cmd::Muffle(m) => self.muffle = m, Cmd::Volume(vv) => self.volume = vv,
            }
        }
        let dt = 1.0 / self.sr;
        let frames = out.len() / ch;
        self.intensity += (self.target - self.intensity) * (frames as f32 * dt * 1.2).min(1.0);
        let block = frames as f32 * dt;
        while self.next < self.clock + block + 0.05 { let st = self.step; let when = (self.next - self.clock).max(0.0); self.music(st, when); self.next += STEP; self.step = (self.step + 1) % 64; }
        self.clock += block;
        let cut = if self.muffle { 650.0 } else { 16000.0 };
        let ma = 1.0 - (-TAU * cut / self.sr).exp();
        for f in 0..frames {
            let (mut sl, mut sr_, mut ml, mut mr) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for x in self.voices.iter_mut() {
                if x.delay > 0.0 { x.delay -= dt; continue; }
                let t = x.age / x.dur;
                if t >= 1.0 { continue; }
                let env = if x.pad { x.vol * if t < 0.3 { t / 0.3 } else { (1.0 - t) / 0.7 } } else { x.vol * (-7.13 * t).exp() };
                let mut s = match x.wave {
                    Wave::Noise => { x.seed ^= x.seed << 13; x.seed ^= x.seed >> 17; x.seed ^= x.seed << 5; (x.seed & 0xffff) as f32 / 32768.0 - 1.0 }
                    w => {
                        let fq = if x.f1 == x.f0 { x.f0 } else { x.f0 * (x.f1 / x.f0).powf(t) };
                        x.phase = (x.phase + fq * dt).fract();
                        match w { Wave::Sine => (x.phase * TAU).sin(), Wave::Square => if x.phase < 0.5 { 0.6 } else { -0.6 }, Wave::Saw => (x.phase * 2.0 - 1.0) * 0.7, _ => 1.0 - 4.0 * (x.phase - 0.5).abs() }
                    }
                };
                if !matches!(x.filt, Filt::None) { // state-variable filter with a sweeping cutoff
                    let fc = (x.c0 * (x.c1 / x.c0).powf(t)).clamp(30.0, self.sr * 0.3);
                    let g = (std::f32::consts::PI * fc / self.sr).tan();
                    let k = 1.0 / x.q.max(0.3);
                    let a1 = 1.0 / (1.0 + g * (g + k));
                    let v1 = a1 * (x.z1 + g * (s - x.z2));
                    let v2 = x.z2 + g * v1;
                    x.z1 = 2.0 * v1 - x.z1; x.z2 = 2.0 * v2 - x.z2;
                    s = match x.filt { Filt::Low => v2, Filt::Band => v1, _ => s - k * v1 - v2 };
                }
                s *= env;
                let (gl, gr) = ((1.0 - x.pan).min(1.0), (1.0 + x.pan).min(1.0));
                if x.music { ml += s * gl; mr += s * gr; } else { sl += s * gl; sr_ += s * gr; }
                x.age += dt;
            }
            // a small room for the effects
            let dry = (sl + sr_) * 0.5;
            let mut wet = 0.0;
            for (buf, pos, lp) in self.combs.iter_mut() { let y = buf[*pos]; *lp = y * 0.75 + *lp * 0.25; buf[*pos] = dry + *lp * 0.8; *pos = (*pos + 1) % buf.len(); wet += y; }
            for (buf, pos) in self.aps.iter_mut() { let b = buf[*pos]; let y = -wet * 0.5 + b; buf[*pos] = wet + b * 0.5; *pos = (*pos + 1) % buf.len(); wet = y; }
            self.mlp[0] += (ml - self.mlp[0]) * ma; self.mlp[1] += (mr - self.mlp[1]) * ma;
            let l = ((sl + wet * 0.04 + self.mlp[0] * 0.55) * self.volume).tanh();
            let r = ((sr_ + wet * 0.04 + self.mlp[1] * 0.55) * self.volume).tanh();
            for c in 0..ch { out[f * ch + c] = if c == 0 { l } else if c == 1 { r } else { 0.0 }; }
        }
        self.voices.retain(|x| x.age < x.dur);
    }
}

/// Where a sound is relative to the listener: volume and pan.
#[derive(Clone, Copy)]
pub struct Sp { pub v: f32, pub p: f32 }
pub const HERE: Sp = Sp { v: 1.0, p: 0.0 };

pub struct Audio { tx: Option<Sender<Cmd>>, _stream: Option<cpal::Stream>, pub rate: f32, pub muted: bool, volume: f32, lx: f32, lz: f32, lyaw: f32, rng: u32 }
impl Audio {
    pub fn silent() -> Audio { Audio { tx: None, _stream: None, rate: 1.0, muted: false, volume: 0.8, lx: 0.0, lz: 0.0, lyaw: 0.0, rng: 7 } }
    pub fn new() -> Audio {
        let mut a = Audio::silent();
        let Some(dev) = cpal::default_host().default_output_device() else { return a };
        let Ok(cfg) = dev.default_output_config() else { return a };
        if cfg.sample_format() != cpal::SampleFormat::F32 { return a; }
        let (tx, rx) = channel();
        let (ch, mut synth) = (cfg.channels() as usize, Synth::new(cfg.sample_rate().0 as f32));
        let stream = dev.build_output_stream(&cfg.config(), move |data: &mut [f32], _: &cpal::OutputCallbackInfo| synth.render(data, ch, &rx), |e| eprintln!("audio: {e}"), None);
        if let Ok(s) = stream { if s.play().is_ok() { a.tx = Some(tx); a._stream = Some(s); } }
        a
    }
    fn send(&self, c: Cmd) { if let Some(tx) = &self.tx { let _ = tx.send(c); } }
    fn r(&mut self) -> f32 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5; (self.rng & 0xffffff) as f32 / 16777216.0 }
    pub fn set_volume(&mut self, vol: f32) { self.volume = vol; self.send(Cmd::Volume(if self.muted { 0.0 } else { vol })); }
    pub fn toggle_mute(&mut self) { self.muted = !self.muted; self.set_volume(self.volume); }
    pub fn set_listener(&mut self, x: f32, z: f32, yaw: f32) { self.lx = x; self.lz = z; self.lyaw = yaw; }
    pub fn sp(&self, x: f32, z: f32) -> Sp {
        let (dx, dz) = (x - self.lx, z - self.lz);
        let d = dx.hypot(dz).max(1e-4);
        Sp { v: 1.0 / (1.0 + d * 0.07), p: ((dx * self.lyaw.cos() - dz * self.lyaw.sin()) / d).clamp(-1.0, 1.0) * 0.8 }
    }
    pub fn music(&self, intensity: f32) { self.send(Cmd::Music(intensity)); }
    pub fn muffle(&self, on: bool) { self.send(Cmd::Muffle(on)); }
    fn tone(&self, w: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32, sp: Sp) {
        let r = self.rate;
        self.send(Cmd::Voice(Voice { delay, pan: sp.p, ..v(w, f0 * r, f1 * r, dur / r, vol * sp.v) }));
    }
    fn noise(&self, dur: f32, f: Filt, c0: f32, c1: f32, vol: f32, delay: f32, sp: Sp, q: f32) {
        let r = self.rate;
        self.send(Cmd::Voice(Voice { delay, pan: sp.p, q, seed: (c0 as u32).wrapping_mul(2654435761) | 1, ..nz(dur / r, f, c0 * r, c1 * r, vol * sp.v) }));
    }
    /// Animal-crossing-style gibberish for bear barks.
    pub fn babble(&mut self, sp: Sp, pitch: f32) {
        let n = 3 + (self.r() * 4.0) as u32;
        for i in 0..n { let f = (520.0 + self.r() * 520.0) * pitch; let k = 0.85 + self.r() * 0.4; self.tone(if i % 2 == 1 { Wave::Square } else { Wave::Tri }, f, f * k, 0.055, 0.06, i as f32 * 0.07, sp); }
    }
}
use Filt::{Band, High, Low};
use Wave::{Saw, Sine, Square, Tri};
// The sound effects: each is a handful of oscillator sweeps and filtered noise bursts.
impl Audio {
    pub fn blaster(&self) { self.noise(0.1, Band, 3000.0, 500.0, 0.6, 0.0, HERE, 0.6); self.tone(Sine, 190.0, 48.0, 0.11, 0.55, 0.0, HERE); self.noise(0.03, High, 6500.0, 4000.0, 0.22, 0.0, HERE, 0.7); self.noise(0.22, Low, 900.0, 200.0, 0.12, 0.04, HERE, 0.7); }
    pub fn scatter(&self) { self.noise(0.32, Low, 4200.0, 200.0, 0.85, 0.0, HERE, 0.7); self.tone(Sine, 150.0, 36.0, 0.3, 0.75, 0.0, HERE); self.noise(0.05, High, 5000.0, 3000.0, 0.3, 0.0, HERE, 0.7); }
    pub fn pump(&self) { self.noise(0.05, Band, 1400.0, 900.0, 0.22, 0.0, HERE, 0.7); self.noise(0.06, Band, 2200.0, 1200.0, 0.22, 0.12, HERE, 0.7); }
    pub fn cannon(&self) { self.tone(Sine, 210.0, 46.0, 0.3, 0.7, 0.0, HERE); self.noise(0.16, Low, 1200.0, 160.0, 0.5, 0.0, HERE, 0.7); self.noise(0.05, Band, 900.0, 500.0, 0.3, 0.0, HERE, 3.0); }
    pub fn boom(&mut self, sp: Sp) { self.noise(0.9, Low, 2400.0, 50.0, 0.95, 0.0, sp, 0.7); self.tone(Sine, 95.0, 24.0, 0.7, 0.9, 0.0, sp); for _ in 0..14 { let d = 0.15 + self.r() * 0.9; self.noise(0.03, High, 6000.0, 4000.0, 0.11, d, sp, 0.7); } }
    pub fn egun(&self, kind: u8, sp: Sp) {
        match kind {
            1 => { self.noise(0.07, Band, 2600.0, 700.0, 0.5, 0.0, sp, 0.6); self.tone(Sine, 170.0, 55.0, 0.07, 0.4, 0.0, sp); }
            2 => { self.noise(0.4, Low, 7000.0, 250.0, 0.9, 0.0, sp, 0.7); self.tone(Sine, 130.0, 30.0, 0.4, 0.7, 0.0, sp); }
            3 => { self.noise(0.55, Band, 500.0, 2400.0, 0.5, 0.0, sp, 1.2); self.tone(Sine, 110.0, 50.0, 0.3, 0.5, 0.0, sp); }
            _ => { self.noise(0.09, Band, 1900.0, 600.0, 0.5, 0.0, sp, 0.7); self.tone(Sine, 210.0, 60.0, 0.08, 0.4, 0.0, sp); }
        }
    }
    pub fn whiz(&self) { self.noise(0.13, Band, 3200.0, 900.0, 0.14, 0.0, HERE, 6.0); }
    pub fn hit(&self) { self.noise(0.06, Low, 2400.0, 400.0, 0.24, 0.0, HERE, 0.7); self.tone(Sine, 520.0, 230.0, 0.06, 0.16, 0.0, HERE); }
    pub fn head(&self) { self.tone(Tri, 1250.0, 1900.0, 0.09, 0.22, 0.0, HERE); self.noise(0.05, Low, 2600.0, 500.0, 0.2, 0.0, HERE, 0.7); }
    pub fn kill(&self, sp: Sp) { self.noise(0.24, Low, 1100.0, 120.0, 0.4, 0.0, sp, 0.7); self.tone(Sine, 330.0, 55.0, 0.28, 0.34, 0.0, sp); }
    pub fn pop(&self, sp: Sp) { self.tone(Sine, 500.0, 1300.0, 0.07, 0.3, 0.0, sp); self.noise(0.05, High, 3000.0, 2000.0, 0.2, 0.0, sp, 0.7); }
    pub fn disarm(&self) { self.tone(Sine, 1000.0, 180.0, 0.38, 0.2, 0.0, HERE); self.tone(Tri, 700.0, 900.0, 0.05, 0.2, 0.0, HERE); }
    pub fn clang(&self, sp: Sp) { self.tone(Square, 1900.0, 1750.0, 0.08, 0.1, 0.0, sp); self.tone(Tri, 2950.0, 2700.0, 0.22, 0.08, 0.0, sp); }
    pub fn hurt(&self) { self.tone(Saw, 170.0, 55.0, 0.26, 0.3, 0.0, HERE); self.noise(0.14, Low, 700.0, 150.0, 0.35, 0.0, HERE, 0.7); }
    pub fn bit(&self, n: u32) { let f = 660.0 * 1.06f32.powi(n.min(14) as i32); self.tone(Tri, f, f * 1.5, 0.07, 0.15, 0.0, HERE); }
    pub fn jump(&self) { self.tone(Sine, 260.0, 430.0, 0.09, 0.09, 0.0, HERE); }
    pub fn land(&self) { self.noise(0.08, Low, 320.0, 80.0, 0.4, 0.0, HERE, 0.7); }
    pub fn reload(&self, dur: f32) { self.tone(Tri, 420.0, 640.0, 0.06, 0.18, 0.0, HERE); self.noise(0.04, Band, 1800.0, 1200.0, 0.2, dur * 0.45, HERE, 0.7); self.tone(Tri, 640.0, 980.0, 0.07, 0.2, dur * 0.88, HERE); }
    pub fn swap(&self) { self.noise(0.07, Band, 900.0, 2200.0, 0.2, 0.0, HERE, 1.5); self.tone(Tri, 500.0, 700.0, 0.05, 0.1, 0.06, HERE); }
    pub fn charge(&self, sp: Sp) { self.tone(Sine, 1500.0, 1500.0, 0.06, 0.16, 0.0, sp); }
    pub fn lock(&self, sp: Sp) { self.tone(Square, 2100.0, 2100.0, 0.05, 0.12, 0.0, sp); self.tone(Square, 2100.0, 2100.0, 0.05, 0.12, 0.09, sp); }
    pub fn wave(&self) { for (i, f) in [220.0, 277.0, 330.0, 440.0].iter().enumerate() { self.tone(Saw, *f, *f, 0.5, 0.09, i as f32 * 0.09, HERE); } self.noise(0.6, Low, 300.0, 3000.0, 0.2, 0.0, HERE, 0.7); }
    pub fn clear(&self) { for (i, f) in [523.0, 659.0, 784.0, 1047.0, 1319.0].iter().enumerate() { self.tone(Tri, *f, *f, 0.3, 0.2, i as f32 * 0.08, HERE); } }
    pub fn rush_on(&self) { self.tone(Saw, 110.0, 880.0, 0.5, 0.2, 0.0, HERE); self.noise(0.6, Band, 300.0, 6000.0, 0.3, 0.0, HERE, 2.0); for (i, f) in [523.0, 659.0, 784.0, 1047.0].iter().enumerate() { self.tone(Square, *f, *f, 0.2, 0.08, 0.3 + i as f32 * 0.05, HERE); } }
    pub fn rush_off(&self) { self.tone(Saw, 700.0, 90.0, 0.5, 0.18, 0.0, HERE); }
    pub fn heart(&self) { self.tone(Sine, 70.0, 40.0, 0.12, 0.5, 0.0, HERE); self.tone(Sine, 62.0, 36.0, 0.12, 0.4, 0.17, HERE); }
    pub fn boss(&self) { self.tone(Saw, 160.0, 70.0, 0.9, 0.3, 0.0, HERE); self.tone(Square, 320.0, 150.0, 0.9, 0.12, 0.0, HERE); self.noise(0.8, Low, 500.0, 100.0, 0.4, 0.0, HERE, 0.7); }
    pub fn over(&self) { for (i, f) in [392.0, 330.0, 262.0, 196.0].iter().enumerate() { self.tone(Saw, *f, *f * 0.97, 0.32, 0.16, i as f32 * 0.18, HERE); } }
    pub fn ui(&self) { self.tone(Tri, 700.0, 1000.0, 0.05, 0.14, 0.0, HERE); }
    pub fn deny(&self) { self.tone(Square, 200.0, 150.0, 0.12, 0.12, 0.0, HERE); }
    pub fn buy(&self) { for (i, f) in [784.0, 988.0, 1319.0].iter().enumerate() { self.tone(Tri, *f, *f, 0.12, 0.2, i as f32 * 0.05, HERE); } self.noise(0.08, High, 6000.0, 9000.0, 0.1, 0.0, HERE, 0.7); }
    pub fn levelup(&self) { for (i, f) in [523.0, 659.0, 784.0, 1047.0, 1319.0, 1568.0].iter().enumerate() { self.tone(Square, *f, *f, 0.22, 0.09, i as f32 * 0.07, HERE); } self.tone(Saw, 131.0, 262.0, 0.6, 0.15, 0.0, HERE); }
    pub fn star(&self, i: u32) { let f = 880.0 * 1.26f32.powi(i as i32); self.tone(Tri, f, f, 0.3, 0.22, 0.0, HERE); self.tone(Sine, f * 2.0, f * 2.0, 0.4, 0.1, 0.0, HERE); }
    pub fn radio(&self) { self.noise(0.05, Band, 2400.0, 2400.0, 0.14, 0.0, HERE, 3.0); self.tone(Square, 1300.0, 1300.0, 0.04, 0.05, 0.05, HERE); }
    pub fn win(&self) { for (i, f) in [392.0, 523.0, 659.0, 784.0, 1047.0].iter().enumerate() { self.tone(Saw, *f, *f, 0.5, 0.1, i as f32 * 0.1, HERE); self.tone(Tri, f * 2.0, f * 2.0, 0.5, 0.1, i as f32 * 0.1, HERE); } }
    pub fn step(&mut self, run: bool) { let c = 500.0 + self.r() * 300.0; self.noise(0.06, Low, c, 120.0, if run { 0.2 } else { 0.12 }, 0.0, HERE, 0.7); self.tone(Sine, 90.0, 55.0, 0.05, if run { 0.14 } else { 0.08 }, 0.0, HERE); }
    pub fn slide(&self) { self.noise(0.75, Band, 1400.0, 500.0, 0.4, 0.0, HERE, 0.8); self.noise(0.15, Low, 400.0, 100.0, 0.4, 0.0, HERE, 0.7); }
    pub fn mantle(&self) { self.noise(0.12, Low, 600.0, 150.0, 0.3, 0.0, HERE, 0.7); self.noise(0.1, Band, 900.0, 500.0, 0.18, 0.18, HERE, 1.5); }
    pub fn swing(&self) { self.noise(0.16, Band, 700.0, 2600.0, 0.3, 0.0, HERE, 2.0); }
    pub fn thwack(&self) { self.noise(0.09, Low, 1600.0, 200.0, 0.6, 0.0, HERE, 0.7); self.tone(Sine, 200.0, 60.0, 0.12, 0.55, 0.0, HERE); self.tone(Square, 900.0, 300.0, 0.04, 0.1, 0.0, HERE); }
    pub fn pickup(&self) { self.noise(0.06, Band, 2400.0, 1400.0, 0.25, 0.0, HERE, 2.0); self.tone(Tri, 500.0, 760.0, 0.07, 0.16, 0.05, HERE); }
    pub fn splash(&self) { self.noise(0.7, Low, 3200.0, 300.0, 0.6, 0.0, HERE, 0.7); self.noise(0.4, High, 4000.0, 6000.0, 0.14, 0.1, HERE, 0.7); }
    pub fn wind(&self) { self.noise(7.5, Band, 600.0, 260.0, 0.4, 0.0, HERE, 0.5); self.noise(7.5, Low, 300.0, 120.0, 0.28, 0.0, HERE, 0.7); }
    pub fn dive(&mut self) { self.noise(6.0, Low, 300.0, 180.0, 0.5, 0.0, HERE, 0.7); for _ in 0..14 { let (a, b, d) = (500.0 + self.r() * 500.0, 1100.0 + self.r() * 500.0, 0.4 + self.r() * 5.0); self.tone(Sine, a, b, 0.07, 0.05, d, HERE); } self.tone(Saw, 62.0, 58.0, 5.5, 0.09, 0.0, HERE); }
}
