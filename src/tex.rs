//! Everything painted: procedural PBR material layers, the graffiti/sign decal atlas, and font atlases.
use std::collections::HashMap;
use std::f32::consts::TAU;

pub const TEX: usize = 512;
pub const LAYERS: usize = 16;
pub const L_FLAT: u8 = 0;
pub const L_PLASTER: u8 = 1;
pub const L_BRICK: u8 = 2;
pub const L_CONC: u8 = 3;
pub const L_GROUND: u8 = 4;
pub const L_CORR: u8 = 5;
pub const L_WOOD: u8 = 6;
pub const L_SAND: u8 = 7;
pub const L_METAL: u8 = 8;
pub const L_RUST: u8 = 9;
pub const L_FABRIC: u8 = 10;
pub const L_HILL: u8 = 11;
pub const L_DOOR: u8 = 12;
pub const L_SHUTTER: u8 = 13;
pub const L_GRASS: u8 = 14;

// ───────────────────────── noise ─────────────────────────
fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(0x9E3779B9);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffffff) as f32 / 16777216.0
}
/// Value noise that tiles every `per` cells.
fn vn(x: f32, y: f32, per: i32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let w = |a: i32| a.rem_euclid(per.max(1));
    let (a, b, c, d) = (hash(w(xi), w(yi), seed), hash(w(xi + 1), w(yi), seed), hash(w(xi), w(yi + 1), seed), hash(w(xi + 1), w(yi + 1), seed));
    a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy
}
/// Tiling fractal noise over the unit square, 0..1.
fn fbm(u: f32, v: f32, freq: i32, oct: u32, seed: u32) -> f32 {
    let (mut s, mut amp, mut norm, mut f) = (0.0, 0.5, 0.0, freq);
    for o in 0..oct { s += amp * vn(u * f as f32, v * f as f32, f, seed + o * 17); norm += amp; amp *= 0.5; f *= 2; }
    s / norm
}
fn sstep(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }
fn mix(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] { [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)] }
fn mul3(a: [f32; 3], k: f32) -> [f32; 3] { [a[0] * k, a[1] * k, a[2] * k] }

struct Px { rgb: [f32; 3], h: f32, rough: f32, a: f32 }

/// One texel of a material layer. `u` runs across, `v` runs up; each tile covers roughly two metres of wall.
fn layer_px(layer: u8, u: f32, v: f32) -> Px {
    let speck = hash((u * 512.0) as i32, (v * 512.0) as i32, 7);
    match layer {
        L_PLASTER => {
            let stain = fbm(u, v, 4, 5, 1);
            let streak = fbm(u, v * 0.12, 24, 3, 2);
            let crack = (fbm(u, v, 6, 4, 3) * 2.0 - 1.0).abs();
            let cr = sstep(0.018, 0.0, crack) * sstep(0.5, 0.62, fbm(u, v, 2, 2, 4));
            let c = 0.94 - stain * 0.16 - streak * 0.1 * (1.0 - v) - cr * 0.35 + (speck - 0.5) * 0.04;
            Px { rgb: [c, c, c * 0.985], h: fbm(u, v, 16, 4, 5) * 0.5 + fbm(u, v, 64, 2, 6) * 0.2 - cr * 0.6, rough: 0.86 + stain * 0.08, a: 0.0 }
        }
        L_BRICK => {
            let ry = v * 10.0;
            let row = ry.floor();
            let fx = u * 5.0 + if (row as i32) % 2 == 1 { 0.5 } else { 0.0 };
            let (bx, ly) = (fx.floor(), ry - row);
            let lx = fx - bx;
            let edge = (lx.min(1.0 - lx) * 0.4).min(ly.min(1.0 - ly) * 0.2);
            let brick = sstep(0.008, 0.016, edge);
            let id = hash(bx as i32 % 5, row as i32, 11);
            let groove = sstep(0.05, 0.0, (ly - 0.34).abs()) + sstep(0.05, 0.0, (ly - 0.66).abs());
            let grain = fbm(u, v, 48, 3, 12);
            let base = mul3([0.72 + id * 0.1, 0.33 + id * 0.08, 0.19 + id * 0.04], 0.74 + id * 0.36 + (grain - 0.5) * 0.2);
            let mortar = mul3([0.5, 0.47, 0.43], 0.8 + grain * 0.3);
            Px { rgb: mix3(mortar, mul3(base, 1.0 - groove * 0.22), brick), h: brick * (0.7 + grain * 0.15 - groove * 0.22) + (1.0 - brick) * grain * 0.2, rough: 0.9, a: 0.0 }
        }
        L_CONC => {
            let blotch = fbm(u, v, 3, 5, 21);
            let pore = sstep(0.93, 0.97, hash((u * 256.0) as i32, (v * 256.0) as i32, 22));
            let seam = sstep(0.012, 0.0, ((v * 4.0).fract() - 0.5).abs() - 0.488).min(1.0) * 0.0 + sstep(0.01, 0.0, (v * 4.0).fract().min(1.0 - (v * 4.0).fract()));
            let c = 0.6 + (blotch - 0.5) * 0.26 - pore * 0.2 - seam * 0.14 + (speck - 0.5) * 0.05;
            Px { rgb: [c, c * 0.985, c * 0.96], h: fbm(u, v, 24, 4, 23) * 0.4 - pore * 0.5 - seam * 0.5, rough: 0.8 - blotch * 0.1, a: 0.0 }
        }
        L_GROUND => {
            let worn = sstep(0.5, 0.68, fbm(u, v, 3, 4, 31));
            let crack = (fbm(u, v, 5, 4, 32) * 2.0 - 1.0).abs();
            let cr = sstep(0.03, 0.0, crack) * sstep(0.42, 0.55, fbm(u, v, 2, 2, 33));
            let wet = sstep(0.67, 0.75, fbm(u, v, 3, 3, 34));
            let oil = sstep(0.7, 0.8, fbm(u, v, 5, 2, 35));
            let agg = (speck - 0.5) * 0.12 + (fbm(u, v, 96, 2, 36) - 0.5) * 0.1;
            let mut c = mix(0.25, 0.4, worn) + agg - cr * 0.16 - oil * 0.07;
            c *= 1.0 - wet * 0.3;
            Px { rgb: [c, c * 0.975, c * 0.94], h: (fbm(u, v, 48, 3, 37) * 0.35 - cr * 0.9) * (1.0 - wet), rough: mix(0.9, 0.05, wet) - oil * 0.25 * (1.0 - wet), a: 0.0 }
        }
        L_CORR => {
            let wave = 0.5 + 0.5 * (u * TAU * 12.0).sin();
            let rust = sstep(0.52, 0.7, fbm(u, v, 5, 4, 41) + (0.5 - v) * 0.15);
            let streak = fbm(u, v * 0.1, 32, 2, 42);
            let zinc = mul3([0.6, 0.62, 0.64], 0.8 + streak * 0.3);
            Px { rgb: mix3(zinc, mul3([0.44, 0.2, 0.09], 0.7 + speck * 0.5), rust), h: wave + rust * 0.05 * speck, rough: mix(0.38, 0.92, rust), a: 0.0 }
        }
        L_WOOD => {
            let pv = v * 6.0;
            let id = pv.floor();
            let grain = fbm(u + id * 0.37, v, 4, 2, 51) * 0.6 + fbm(u * 0.05 + id * 0.11, v, 96, 2, 52) * 0.4;
            let gap = sstep(0.06, 0.0, (pv - id).min(1.0 - (pv - id)));
            let k = (0.75 + hash(id as i32, 0, 53) * 0.35) * (0.8 + grain * 0.4) * (1.0 - gap * 0.75);
            Px { rgb: mul3([0.58, 0.43, 0.26], k), h: grain * 0.3 + (1.0 - gap) * 0.7, rough: 0.74, a: 0.0 }
        }
        L_SAND => {
            let n = fbm(u, v, 8, 4, 61);
            Px { rgb: mul3([0.72, 0.63, 0.46], 0.8 + n * 0.3 + (speck - 0.5) * 0.14), h: n * 0.6 + speck * 0.25, rough: 1.0, a: 0.0 }
        }
        L_METAL => { let n = fbm(u, v, 32, 3, 71); Px { rgb: [0.96 + n * 0.04; 3], h: n * 0.15 + speck * 0.05, rough: 0.4, a: 0.0 } }
        L_RUST => { let n = fbm(u, v, 8, 5, 81); Px { rgb: mix3([0.3, 0.14, 0.07], [0.62, 0.31, 0.12], n), h: n * 0.6 + speck * 0.2, rough: 0.9, a: 0.0 } }
        L_FABRIC => { let w = (u * TAU * 80.0).sin() * (v * TAU * 80.0).sin(); let n = fbm(u, v, 8, 3, 91); Px { rgb: [0.86 + w * 0.06 + n * 0.08; 3], h: 0.5 + w * 0.5, rough: 0.95, a: 0.0 } }
        L_HILL => {
            // a storey of a far-away house: wall, two windows, grime
            let n = fbm(u, v, 6, 4, 101);
            let (cu, cv) = ((u * 2.0).fract(), v);
            let win = (cu > 0.28 && cu < 0.72 && cv > 0.34 && cv < 0.74) as i32 as f32;
            let frame = (cu > 0.24 && cu < 0.76 && cv > 0.3 && cv < 0.78) as i32 as f32 - win;
            let lit = (u < 0.5) as i32 as f32;
            let wall = 0.9 - n * 0.2 - sstep(0.3, 0.0, v) * 0.25 - sstep(0.94, 1.0, v) * 0.3;
            let c = mix(wall, 0.12, win) * (1.0 - frame * 0.25);
            Px { rgb: [c, c, c], h: 0.5 - win * 0.5, rough: mix(0.9, 0.1, win), a: win * lit }
        }
        L_DOOR => {
            let inset = |cx: f32, cy: f32, w: f32, h: f32| sstep(0.0, 0.02, w - (u - cx).abs()) * sstep(0.0, 0.02, h - (v - cy).abs());
            let p = inset(0.5, 0.72, 0.32, 0.18) + inset(0.5, 0.28, 0.32, 0.2);
            let knob = sstep(0.035, 0.025, ((u - 0.86).powi(2) + (v - 0.5).powi(2)).sqrt());
            let n = fbm(u, v, 12, 3, 111);
            let c = 0.88 - p * 0.1 + (n - 0.5) * 0.12 - sstep(0.12, 0.0, v) * 0.25;
            Px { rgb: mix3([c, c, c], [0.75, 0.7, 0.55], knob), h: 0.6 - p * 0.4 + knob * 0.4, rough: mix(0.6, 0.3, knob), a: 0.0 }
        }
        L_SHUTTER => { let s = (v * 26.0).fract(); let n = fbm(u, v, 16, 3, 121); Px { rgb: [0.62 + s * 0.14 - n * 0.14; 3], h: s, rough: 0.5 + n * 0.2, a: 0.0 } }
        L_GRASS => { let n = fbm(u, v, 6, 5, 131); let m = fbm(u, v, 24, 3, 132); Px { rgb: mix3([0.2, 0.3, 0.12], [0.42, 0.36, 0.2], n * 0.8 + m * 0.2), h: m, rough: 1.0, a: 0.0 } }
        _ => Px { rgb: [1.0; 3], h: 0.5, rough: 0.5, a: 0.0 },
    }
}
fn bump(layer: u8) -> f32 {
    match layer { L_BRICK => 3.8, L_CORR => 2.6, L_GROUND => 1.7, L_WOOD => 3.0, L_PLASTER => 1.2, L_CONC => 1.8, L_SAND => 3.0, L_SHUTTER => 3.5, L_DOOR => 3.0, L_HILL => 2.0, L_FABRIC => 1.2, L_FLAT => 0.0, _ => 1.5 }
}

pub struct Materials { pub albedo: Vec<Vec<u8>>, pub nr: Vec<Vec<u8>> }
fn srgb(c: f32) -> u8 { (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 }

/// Albedo (sRGB, alpha = night window mask) and normal.xy / roughness / height for every layer. One thread per layer.
pub fn build_materials() -> Materials {
    let n = TEX;
    let layers: Vec<(Vec<u8>, Vec<u8>)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..LAYERS as u8).map(|l| s.spawn(move || {
            let (mut al, mut nr, mut ht) = (vec![0u8; n * n * 4], vec![0u8; n * n * 4], vec![0f32; n * n]);
            for y in 0..n { for x in 0..n {
                let p = layer_px(l, (x as f32 + 0.5) / n as f32, 1.0 - (y as f32 + 0.5) / n as f32);
                let i = (y * n + x) * 4;
                al[i] = srgb(p.rgb[0]); al[i + 1] = srgb(p.rgb[1]); al[i + 2] = srgb(p.rgb[2]); al[i + 3] = srgb(p.a);
                nr[i + 2] = srgb(p.rough); nr[i + 3] = srgb(p.h);
                ht[y * n + x] = p.h;
            } }
            let k = bump(l);
            for y in 0..n { for x in 0..n {
                let g = |xx: usize, yy: usize| ht[(yy % n) * n + (xx % n)];
                let (dx, dy) = ((g(x + 1, y) - g(x + n - 1, y)) * k, (g(x, y + n - 1) - g(x, y + 1)) * k); // image y runs down, v runs up
                let inv = 1.0 / (dx * dx + dy * dy + 1.0).sqrt();
                let i = (y * n + x) * 4;
                nr[i] = srgb(-dx * inv * 0.5 + 0.5); nr[i + 1] = srgb(-dy * inv * 0.5 + 0.5);
            } }
            (al, nr)
        })).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let (albedo, nr) = layers.into_iter().unzip();
    Materials { albedo, nr }
}

// ───────────────────────── fonts and a tiny canvas ─────────────────────────
pub struct Fonts { pub ui: fontdue::Font, pub sign: fontdue::Font, pub tag: fontdue::Font }
fn load_font(paths: &[&str]) -> fontdue::Font {
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            if let Ok(f) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings { scale: 64.0, ..Default::default() }) { return f; }
        }
    }
    panic!("no usable system font found (looked for {paths:?})");
}
pub fn load_fonts() -> Fonts {
    let sup = "/System/Library/Fonts/Supplemental/";
    let fallback = [&format!("{sup}Arial Bold.ttf")[..], "/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/SFNS.ttf"].map(String::from);
    let with = |first: &[&str]| -> fontdue::Font { let mut v: Vec<&str> = first.to_vec(); v.extend(fallback.iter().map(|s| s.as_str())); load_font(&v) };
    Fonts {
        ui: with(&[&format!("{sup}Arial Rounded Bold.ttf")]),
        sign: with(&[&format!("{sup}Impact.ttf")]),
        tag: with(&["/System/Library/Fonts/MarkerFelt.ttc", &format!("{sup}Chalkduster.ttf")]),
    }
}

pub struct Canvas { pub w: usize, pub h: usize, pub px: Vec<u8> }
impl Canvas {
    pub fn new(w: usize, h: usize) -> Self { Canvas { w, h, px: vec![0; w * h * 4] } }
    /// Source-over blend of straight-alpha colour `c` with coverage `cov`.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: [u8; 4], cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 { return; }
        let i = (y as usize * self.w + x as usize) * 4;
        let sa = c[3] as f32 / 255.0 * cov.clamp(0.0, 1.0);
        let da = self.px[i + 3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        if oa <= 0.0 { return; }
        for k in 0..3 { self.px[i + k] = ((c[k] as f32 * sa + self.px[i + k] as f32 * da * (1.0 - sa)) / oa) as u8; }
        self.px[i + 3] = (oa * 255.0) as u8;
    }
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [u8; 4]) {
        for yy in y.floor() as i32..(y + h).ceil() as i32 { for xx in x.floor() as i32..(x + w).ceil() as i32 { self.blend(xx, yy, c, 1.0); } }
    }
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: [u8; 4]) {
        for yy in (cy - ry - 1.0) as i32..=(cy + ry + 1.0) as i32 { for xx in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
            let d = (((xx as f32 + 0.5 - cx) / rx).powi(2) + ((yy as f32 + 0.5 - cy) / ry).powi(2)).sqrt();
            self.blend(xx, yy, c, (1.0 - d) * rx.min(ry) + 0.5);
        } }
    }
    pub fn text_w(font: &fontdue::Font, s: &str, size: f32) -> f32 { s.chars().map(|ch| font.metrics(ch, size).advance_width).sum() }
    /// Draws text with its baseline at `y`; returns the advance.
    pub fn text(&mut self, font: &fontdue::Font, s: &str, x: f32, y: f32, size: f32, c: [u8; 4]) -> f32 {
        let mut pen = x;
        for ch in s.chars() {
            let (m, bmp) = font.rasterize(ch, size);
            let (ox, oy) = (pen.round() as i32 + m.xmin, y.round() as i32 - m.ymin - m.height as i32);
            for gy in 0..m.height { for gx in 0..m.width { let cov = bmp[gy * m.width + gx]; if cov > 0 { self.blend(ox + gx as i32, oy + gy as i32, c, cov as f32 / 255.0); } } }
            pen += m.advance_width;
        }
        pen - x
    }
    /// Spray-can lettering: dark outline, colour fill, a few drips.
    pub fn tag(&mut self, font: &fontdue::Font, s: &str, cx: f32, y: f32, size: f32, c: [u8; 4], seed: u32) {
        let w = Self::text_w(font, s, size);
        let (x, r) = (cx - w * 0.5, size * 0.09);
        for k in 0..12 { let a = k as f32 / 12.0 * TAU; self.text(font, s, x + a.cos() * r, y + a.sin() * r, size, [22, 17, 15, 255]); }
        self.text(font, s, x, y, size, c);
        for k in 0..5 { let dx = hash(k, 0, seed) * w; self.rect(x + dx, y + 1.0, 2.5, 6.0 + hash(k, 1, seed) * size * 0.6, c); }
    }
    pub fn premultiply(&mut self) { for p in self.px.chunks_exact_mut(4) { let a = p[3] as u32; for k in 0..3 { p[k] = (p[k] as u32 * a / 255) as u8; } } }
    pub fn blit(&mut self, src: &Canvas, dx: usize, dy: usize) {
        for y in 0..src.h { let (s, d) = (y * src.w * 4, ((dy + y) * self.w + dx) * 4); self.px[d..d + src.w * 4].copy_from_slice(&src.px[s..s + src.w * 4]); }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Rect { pub u0: f32, pub v0: f32, pub u1: f32, pub v1: f32, pub aspect: f32 }

pub struct Atlas {
    pub size: usize, pub px: Vec<u8>,
    pub tags: Vec<Rect>, pub signs: Vec<Rect>,
    pub bear: Rect, pub king: Rect, pub mural: Rect, pub crate_label: Rect, pub shield: Rect, pub splat: Rect, pub white: Rect,
}
pub const TAGS: [&str; 9] = ["VIVA O REI GUMMY", "GUMMY LIVRE", "DOCE OU MORTE", "S.W.E.E.T. FORA!", "URSO NÃO DORME", "JUJUBA 4EVER", "AQUI MANDA O URSO", "CUIDADO: URSO BRAVO", "O AÇÚCAR É NOSSO"];
pub const SIGNS: [&str; 8] = ["BAR DO GUMMY", "MERCADINHO JUJUBA", "DOCERIA DO REI", "AÇAÍ & BALAS", "BARBEARIA DO URSO", "LAN HOUSE GELATINA", "PADARIA DOCE VIDA", "BORRACHARIA URSÃO"];
pub const SPRAY: [u32; 7] = [0xff3b6b, 0x35d6c8, 0xffd23f, 0x9b4dff, 0xffffff, 0x3fd45c, 0xff8a1f];
fn rgba(hex: u32) -> [u8; 4] { [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255] }

fn stencil_bear(c: &mut Canvas, cx: f32, cy: f32, s: f32, fill: [u8; 4], crown: bool) {
    let parts: [[f32; 4]; 8] = [[-0.33, 0.75, 0.22, 0.3], [0.33, 0.75, 0.22, 0.3], [-0.62, 0.05, 0.2, 0.3], [0.62, 0.05, 0.2, 0.3], [0.0, 0.3, 0.55, 0.62], [-0.3, -0.82, 0.16, 0.16], [0.3, -0.82, 0.16, 0.16], [0.0, -0.5, 0.42, 0.4]];
    for p in parts { c.ellipse(cx + p[0] * s, cy + p[1] * s, (p[2] + 0.07) * s, (p[3] + 0.07) * s, [22, 17, 15, 255]); }
    for p in parts { c.ellipse(cx + p[0] * s, cy + p[1] * s, p[2] * s, p[3] * s, fill); }
    c.rect(cx - 0.24 * s, cy - 0.6 * s, 0.14 * s, 0.1 * s, [22, 17, 15, 255]);
    c.rect(cx + 0.1 * s, cy - 0.6 * s, 0.14 * s, 0.1 * s, [22, 17, 15, 255]);
    c.ellipse(cx - 0.2 * s, cy + 0.05 * s, 0.09 * s, 0.2 * s, [255, 255, 255, 110]);
    if crown { for k in 0..3 { let px = cx + (k as f32 - 1.0) * 0.2 * s; for y in 0..(0.36 * s) as i32 { let half = 0.11 * s * (y as f32 / (0.36 * s)); c.rect(px - half, cy - 1.32 * s + y as f32, half * 2.0, 1.0, [255, 210, 63, 255]); } } c.rect(cx - 0.32 * s, cy - 0.98 * s, 0.64 * s, 0.1 * s, [255, 210, 63, 255]); }
    for k in 0..3 { c.rect(cx + (hash(k, 3, s as u32) - 0.5) * s, cy + 0.95 * s, 0.05 * s, (0.2 + hash(k, 4, s as u32) * 0.5) * s, fill); }
}

/// The decal atlas: spray tags, shop signs, stencils, the mural, labels and a goo splat. Premultiplied alpha.
pub fn build_atlas(f: &Fonts) -> Atlas {
    let size = 2048;
    let mut atlas = Canvas::new(size, size);
    let (mut sx, mut sy, mut row_h) = (2usize, 2usize, 0usize);
    let mut place = |atlas: &mut Canvas, c: &Canvas| -> Rect {
        if sx + c.w + 2 > size { sx = 2; sy += row_h + 4; row_h = 0; }
        atlas.blit(c, sx, sy);
        let r = Rect { u0: (sx as f32 + 0.5) / size as f32, v0: (sy as f32 + 0.5) / size as f32, u1: (sx + c.w) as f32 / size as f32 - 0.5 / size as f32, v1: (sy + c.h) as f32 / size as f32 - 0.5 / size as f32, aspect: c.w as f32 / c.h as f32 };
        sx += c.w + 4; row_h = row_h.max(c.h);
        r
    };
    let mut tags = Vec::new();
    for (i, t) in TAGS.iter().enumerate() {
        let w = (Canvas::text_w(&f.tag, t, 64.0) + 40.0) as usize;
        let mut c = Canvas::new(w, 128);
        c.tag(&f.tag, t, w as f32 * 0.5, 76.0, 64.0, [255, 255, 255, 255], i as u32 + 1);
        tags.push(place(&mut atlas, &c));
    }
    let mut signs = Vec::new();
    for (i, t) in SIGNS.iter().enumerate() {
        let mut c = Canvas::new(448, 84);
        c.rect(0.0, 0.0, 448.0, 84.0, [30, 22, 18, 255]);
        c.rect(4.0, 4.0, 440.0, 76.0, rgba(SPRAY[i % SPRAY.len()]));
        let w = Canvas::text_w(&f.sign, t, 44.0).min(420.0);
        let sz = if Canvas::text_w(&f.sign, t, 44.0) > 420.0 { 44.0 * 420.0 / Canvas::text_w(&f.sign, t, 44.0) } else { 44.0 };
        for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0), (2.0, 2.0), (-2.0, -2.0)] { c.text(&f.sign, t, 224.0 - w * 0.5 + dx, 58.0 + dy, sz, [28, 21, 18, 255]); }
        c.text(&f.sign, t, 224.0 - w * 0.5, 58.0, sz, [255, 255, 255, 255]);
        signs.push(place(&mut atlas, &c));
    }
    let mut bear_c = Canvas::new(160, 256); stencil_bear(&mut bear_c, 80.0, 138.0, 66.0, [255, 255, 255, 255], false);
    let mut king_c = Canvas::new(160, 256); stencil_bear(&mut king_c, 80.0, 150.0, 62.0, [255, 255, 255, 255], true);
    let mut mural = Canvas::new(1024, 512);
    mural.rect(0.0, 0.0, 1024.0, 512.0, rgba(0x2f9c95));
    for i in 0..60 { let (x, y) = (hash(i, 0, 5) * 1024.0, hash(i, 1, 5) * 512.0); mural.ellipse(x, y, 30.0 + hash(i, 2, 5) * 60.0, 20.0 + hash(i, 3, 5) * 40.0, [20, 30, 30, 22]); }
    stencil_bear(&mut mural, 250.0, 290.0, 175.0, rgba(0xff3b4e), true);
    mural.tag(&f.tag, "REI GUMMY", 690.0, 230.0, 112.0, rgba(0xffd23f), 9);
    mural.tag(&f.tag, "TE VIGIA", 700.0, 370.0, 84.0, [255, 255, 255, 255], 10);
    let mut label = Canvas::new(256, 128);
    let lw = Canvas::text_w(&f.sign, "DOCES", 72.0); label.text(&f.sign, "DOCES", 128.0 - lw * 0.5, 74.0, 72.0, [22, 14, 8, 215]);
    let lw2 = Canvas::text_w(&f.sign, "FRÁGIL · 20 KG", 26.0); label.text(&f.sign, "FRÁGIL · 20 KG", 128.0 - lw2 * 0.5, 110.0, 26.0, [22, 14, 8, 215]);
    let mut shield = Canvas::new(320, 80);
    let sw = Canvas::text_w(&f.sign, "S.W.E.E.T.", 56.0); shield.text(&f.sign, "S.W.E.E.T.", 160.0 - sw * 0.5, 60.0, 56.0, [255, 255, 255, 255]);
    let mut splat = Canvas::new(256, 256);
    splat.ellipse(128.0, 128.0, 68.0, 68.0, [255, 255, 255, 255]);
    for i in 0..14 { let a = hash(i, 0, 77) * TAU; let d = 52.0 + hash(i, 1, 77) * 40.0; let r = 12.0 + hash(i, 2, 77) * 20.0; splat.ellipse(128.0 + a.cos() * d, 128.0 + a.sin() * d, r, r, [255, 255, 255, 255]); }
    let mut white = Canvas::new(16, 16); white.rect(0.0, 0.0, 16.0, 16.0, [255, 255, 255, 255]);
    let (bear, king, mural_r, crate_label, shield_r, splat_r, white_r) = (place(&mut atlas, &bear_c), place(&mut atlas, &king_c), place(&mut atlas, &mural), place(&mut atlas, &label), place(&mut atlas, &shield), place(&mut atlas, &splat), place(&mut atlas, &white));
    atlas.premultiply();
    Atlas { size, px: atlas.px, tags, signs, bear, king, mural: mural_r, crate_label, shield: shield_r, splat: splat_r, white: white_r }
}

#[derive(Copy, Clone)]
pub struct Glyph { pub uv: [f32; 4], pub size: [f32; 2], pub off: [f32; 2], pub adv: f32 }
/// Single-channel glyph atlas for the HUD and menus.
pub struct FontAtlas { pub w: usize, pub h: usize, pub px: Vec<u8>, pub glyphs: HashMap<char, Glyph>, pub px_size: f32 }
pub fn build_font_atlas(font: &fontdue::Font, px_size: f32) -> FontAtlas {
    let (w, h) = (1024usize, 1024usize);
    let mut px = vec![0u8; w * h];
    let mut glyphs = HashMap::new();
    let (mut x, mut y, mut row) = (4usize, 4usize, 0usize);
    let chars: Vec<char> = (32u8..127).map(|c| c as char).chain("·…×’–áãâçéêíóõúÁÃÇÉÍÓÚ".chars()).collect();
    for ch in chars {
        let (m, bmp) = font.rasterize(ch, px_size);
        if x + m.width + 10 > w { x = 4; y += row + 10; row = 0; }
        if y + m.height + 4 > h { break; }
        for gy in 0..m.height { px[(y + gy) * w + x..(y + gy) * w + x + m.width].copy_from_slice(&bmp[gy * m.width..(gy + 1) * m.width]); }
        glyphs.insert(ch, Glyph { uv: [x as f32 / w as f32, y as f32 / h as f32, (x + m.width) as f32 / w as f32, (y + m.height) as f32 / h as f32], size: [m.width as f32, m.height as f32], off: [m.xmin as f32, -(m.ymin as f32) - m.height as f32], adv: m.advance_width });
        x += m.width + 10; row = row.max(m.height);
    }
    FontAtlas { w, h, px, glyphs, px_size }
}
