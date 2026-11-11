//! Immediate-mode 2D drawing for the HUD and menus: rounded boxes, rings, text, buttons. Coordinates are logical (720 units tall).
use crate::gfx::UiVert;
use crate::tex::{FontAtlas, Glyph};
use std::collections::HashMap;

pub type Col = [u8; 4];
pub const WHITE: Col = [255, 255, 255, 255];
pub const PINK: Col = [255, 79, 154, 255];
pub const HOT: Col = [255, 45, 120, 255];
pub const MINT: Col = [62, 230, 207, 255];
pub const SUN: Col = [255, 210, 63, 255];
pub const INK: Col = [42, 10, 30, 255];
pub const RED: Col = [255, 59, 92, 255];
pub const GLASS: Col = [36, 9, 26, 165];
pub const LINE: Col = [255, 255, 255, 40];
pub fn fade(c: Col, a: f32) -> Col { [c[0], c[1], c[2], (c[3] as f32 * a.clamp(0.0, 1.0)) as u8] }

pub struct Ui {
    pub v: Vec<UiVert>, glyphs: HashMap<char, Glyph>, px: f32,
    pub w: f32, pub h: f32, pub s: f32, pub mx: f32, pub my: f32, pub click: bool, pub hot: bool,
}
impl Ui {
    pub fn blank() -> Ui { Ui { v: Vec::new(), glyphs: HashMap::new(), px: 1.0, w: 1280.0, h: 720.0, s: 1.0, mx: 0.0, my: 0.0, click: false, hot: false } }
    pub fn new(font: &FontAtlas) -> Ui { Ui { v: Vec::new(), glyphs: font.glyphs.clone(), px: font.px_size, w: 1280.0, h: 720.0, s: 1.0, mx: 0.0, my: 0.0, click: false, hot: false } }
    pub fn begin(&mut self, pw: u32, ph: u32, mouse_px: (f32, f32), click: bool) {
        self.v.clear();
        self.s = ph as f32 / 720.0; self.w = pw as f32 / self.s; self.h = 720.0;
        self.mx = mouse_px.0 / self.s; self.my = mouse_px.1 / self.s; self.click = click; self.hot = false;
    }
    fn quad(&mut self, cx: f32, cy: f32, hw: f32, hh: f32, rot: f32, c: Col, shape: [f32; 4], uv: Option<[f32; 4]>) {
        let (s, co, si) = (self.s, rot.cos(), rot.sin());
        let corner = |x: f32, y: f32| -> UiVert {
            let (px, py) = ((cx + x * hw * co - y * hh * si) * s, (cy + x * hw * si + y * hh * co) * s);
            let t = match uv { Some(u) => [if x < 0.0 { u[0] } else { u[2] }, if y < 0.0 { u[1] } else { u[3] }], None => [x * hw * s, y * hh * s] };
            UiVert { pos: [px, py], uv: t, color: c, shape }
        };
        let q = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
        self.v.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
    }
    /// Rounded box.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        let s = self.s;
        self.quad(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0, 0.0, c, [w / 2.0 * s, h / 2.0 * s, r.min(w / 2.0).min(h / 2.0) * s, 0.0], None);
    }
    pub fn rect_rot(&mut self, cx: f32, cy: f32, w: f32, h: f32, rot: f32, r: f32, c: Col) {
        let s = self.s;
        self.quad(cx, cy, w / 2.0, h / 2.0, rot, c, [w / 2.0 * s, h / 2.0 * s, r * s, 0.0], None);
    }
    /// Outline of a rounded box, `t` thick (pass a huge box and thickness to punch a circular hole in the screen).
    pub fn ring(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, c: Col) {
        let s = self.s;
        self.quad(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0, 0.0, c, [w / 2.0 * s, h / 2.0 * s, r.min(w / 2.0).min(h / 2.0) * s, 1.0 + (t * s).max(0.8)], None);
    }
    pub fn panel(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) { self.rect(x, y, w, h, r, GLASS); self.ring(x, y, w, h, r, 1.0, LINE); }
    pub fn text_w(&self, t: &str, size: f32) -> f32 { let k = size / self.px; t.chars().map(|ch| if ch == '·' { size * 0.42 } else { self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?')).map(|g| g.adv * k).unwrap_or(0.0) }).sum() }
    /// Left-aligned text; `y` is the top of the line. Returns the width drawn.
    pub fn text(&mut self, x: f32, y: f32, size: f32, c: Col, t: &str) -> f32 {
        let (k, base) = (size / self.px, y + size * 0.82);
        let mut pen = x;
        for ch in t.chars() {
            if ch == '·' { let (r, s) = (size * 0.075, self.s); self.quad(pen + size * 0.21, y + size * 0.52, r, r, 0.0, c, [r * s, r * s, r * s, 0.0], None); pen += size * 0.42; continue; } // drawn, not typeset: old Mac fonts map it elsewhere
            let Some(g) = self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?')).copied() else { continue };
            if g.size[0] > 0.0 {
                let (w, h) = (g.size[0] * k, g.size[1] * k);
                self.quad(pen + g.off[0] * k + w / 2.0, base + g.off[1] * k + h / 2.0, w / 2.0, h / 2.0, 0.0, c, [0.0, 0.0, 0.0, 1.0], Some(g.uv));
            }
            pen += g.adv * k;
        }
        pen - x
    }
    pub fn text_c(&mut self, cx: f32, y: f32, size: f32, c: Col, t: &str) { let w = self.text_w(t, size); self.text(cx - w / 2.0, y, size, c, t); }
    pub fn text_r(&mut self, rx: f32, y: f32, size: f32, c: Col, t: &str) { let w = self.text_w(t, size); self.text(rx - w, y, size, c, t); }
    /// Text with a soft dark drop for legibility over the world. `align`: 0 left, 1 centre, 2 right.
    pub fn label(&mut self, x: f32, y: f32, size: f32, c: Col, t: &str, align: u8) {
        let w = self.text_w(t, size);
        let x0 = match align { 1 => x - w / 2.0, 2 => x - w, _ => x };
        let d = (size * 0.06).max(1.0);
        self.text(x0, y + d, size, fade(INK, 0.75 * c[3] as f32 / 255.0), t);
        self.text(x0, y, size, c, t);
    }
    /// Word-wrapped paragraph; returns its height.
    pub fn wrap(&mut self, x: f32, y: f32, w: f32, size: f32, c: Col, t: &str) -> f32 {
        let (lh, mut line, mut yy) = (size * 1.42, String::new(), y);
        for word in t.split_whitespace() {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.text_w(&cand, size) > w && !line.is_empty() { self.text(x, yy, size, c, &line); yy += lh; line = word.to_string(); } else { line = cand; }
        }
        if !line.is_empty() { self.text(x, yy, size, c, &line); yy += lh; }
        yy - y
    }
    /// Height `wrap` would take, without drawing.
    pub fn wrap_h(&self, w: f32, size: f32, t: &str) -> f32 {
        let (mut line, mut n) = (String::new(), 0);
        for word in t.split_whitespace() {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.text_w(&cand, size) > w && !line.is_empty() { n += 1; line = word.to_string(); } else { line = cand; }
        }
        if !line.is_empty() { n += 1; }
        n as f32 * size * 1.42
    }
    pub fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool { self.mx >= x && self.mx <= x + w && self.my >= y && self.my <= y + h }
    /// A menu button. Returns true on click.
    pub fn button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, sub: &str, primary: bool, enabled: bool) -> bool {
        let hov = enabled && self.hover(x, y, w, h);
        if hov { self.hot = true; }
        let dx = if hov { 4.0 } else { 0.0 };
        if primary {
            self.rect(x + dx, y + 5.0, w, h, 14.0, [156, 17, 75, if enabled { 255 } else { 110 }]);
            self.rect(x + dx, y, w, h, 14.0, if !enabled { [255, 45, 120, 110] } else if hov { [255, 110, 165, 255] } else { HOT });
        } else {
            self.rect(x + dx, y, w, h, 14.0, [255, 255, 255, if !enabled { 12 } else if hov { 54 } else { 24 }]);
            self.ring(x + dx, y, w, h, 14.0, 1.0, LINE);
        }
        let size = f32::min(if primary { 20.0 } else { 17.0 }, h * 0.5);
        let col = fade(WHITE, if enabled { 1.0 } else { 0.45 });
        let lw = self.text(x + dx + 18.0, y + (h - size) / 2.0 - 1.0, size, col, label);
        if !sub.is_empty() { self.text(x + dx + 28.0 + lw, y + (h - 12.0) / 2.0, 12.0, fade(col, 0.7), sub); }
        hov && self.click
    }
    /// Centre-labelled button (cards, footers).
    pub fn button_c(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, primary: bool, enabled: bool) -> bool {
        let hov = enabled && self.hover(x, y, w, h);
        if hov { self.hot = true; }
        if primary { self.rect(x, y + 4.0, w, h, 12.0, [156, 17, 75, if enabled { 255 } else { 110 }]); self.rect(x, y, w, h, 12.0, if !enabled { [255, 45, 120, 110] } else if hov { [255, 110, 165, 255] } else { HOT }); }
        else { self.rect(x, y, w, h, 12.0, [255, 255, 255, if !enabled { 12 } else if hov { 60 } else { 26 }]); self.ring(x, y, w, h, 12.0, 1.0, LINE); }
        let size = (h * 0.42).min(18.0);
        self.text_c(x + w / 2.0, y + (h - size) / 2.0 - 1.0, size, fade(WHITE, if enabled { 1.0 } else { 0.45 }), label);
        hov && self.click
    }
    /// Horizontal meter.
    pub fn bar(&mut self, x: f32, y: f32, w: f32, h: f32, frac: f32, c: Col) {
        self.rect(x, y, w, h, h / 2.0, [36, 9, 26, 150]);
        let f = frac.clamp(0.0, 1.0);
        if f > 0.005 { self.rect(x, y, (w * f).max(h), h, h / 2.0, c); }
    }
    pub fn star(&mut self, cx: f32, cy: f32, r: f32, on: bool) {
        let s = self.s;
        self.quad(cx, cy, r * 1.1, r * 1.1, 0.0, if on { SUN } else { [255, 255, 255, 50] }, [r * s, r * s, 0.0, -1.0], None);
    }
}
