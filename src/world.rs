//! The favela: static neighbourhood geometry, per-mission street furniture, lighting palettes and collision.
use crate::mesh::{mat, Kit, Mat, Mesh};
use crate::tex::*;
use glam::{Mat4, Vec2, Vec3};
use std::f32::consts::{PI, TAU};

pub const ARENA_R: f32 = 90.0;
/// The stepped rooftops in the middle of the square: (half-size, height).
pub const CAKE: [(f32, f32); 3] = [(6.5, 1.3), (4.2, 2.6), (2.2, 3.9)];
pub const WATER_Y: f32 = -0.4;
pub const SPAWN: (f32, f32) = (0.0, ARENA_R - 9.0);
const BAY: f32 = 0.72; // half-angle of the waterfront, centred on +Z
fn in_bay(x: f32, z: f32) -> bool { x.atan2(z).abs() < BAY + 0.06 }

/// mulberry32, so every mission's layout is the same each time.
pub struct Rng(pub u32);
impl Rng {
    pub fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64 / 4294967296.0) as f32
    }
    pub fn r(&mut self, a: f32, b: f32) -> f32 { a + self.f() * (b - a) }
    pub fn chance(&mut self, p: f32) -> bool { self.f() < p }
    pub fn pick<T: Copy>(&mut self, a: &[T]) -> T { a[((self.f() * a.len() as f32) as usize).min(a.len() - 1)] }
    pub fn sign(&mut self) -> f32 { if self.f() < 0.5 { -1.0 } else { 1.0 } }
}

pub fn lin(hex: u32) -> Vec3 {
    let f = |c: u32| ((c & 255) as f32 / 255.0).powf(2.2);
    Vec3::new(f(hex >> 16), f(hex >> 8), f(hex))
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub top: Vec3, pub mid: Vec3, pub bot: Vec3, pub fog: Vec3, pub fog_density: f32, pub sun_color: Vec3, pub sun_i: f32, pub sun_dir: Vec3,
    pub ambient: Vec3, pub exposure: f32, pub night: f32, pub bloom: f32, pub clouds: f32, pub water: Vec3, pub uw: Vec3,
}
pub fn palette(name: &str) -> Palette {
    let d = |x: f32, y: f32, z: f32| Vec3::new(x, y, z).normalize();
    match name {
        "sunset" => Palette { top: lin(0x2b3f7c), mid: lin(0xff9a55), bot: lin(0x6b5a50), fog: lin(0xd9b3a0), fog_density: 0.0034, sun_color: Vec3::new(1.0, 0.62, 0.36), sun_i: 5.0, sun_dir: d(75.0, 24.0, -30.0), ambient: lin(0xc0b0cc) * 0.95, exposure: 1.1, night: 0.45, bloom: 0.5, clouds: 0.5, water: lin(0x6a4a44), uw: lin(0x3a4a4a) },
        "overcast" => Palette { top: lin(0x6f7c8c), mid: lin(0xb9c0c6), bot: lin(0x8a8f92), fog: lin(0xa9b0b6), fog_density: 0.011, sun_color: Vec3::new(0.9, 0.93, 1.0), sun_i: 1.7, sun_dir: d(-30.0, 80.0, 40.0), ambient: lin(0xc8d0dc) * 1.0, exposure: 1.0, night: 0.3, bloom: 0.35, clouds: 0.97, water: lin(0x3c4a4e), uw: lin(0x2c4444) },
        "night" => Palette { top: lin(0x04060e), mid: lin(0x18223c), bot: lin(0x10121a), fog: lin(0x121a2e), fog_density: 0.007, sun_color: Vec3::new(0.6, 0.72, 1.0), sun_i: 0.95, sun_dir: d(-45.0, 60.0, -35.0), ambient: lin(0x4a5a8a) * 0.85, exposure: 1.7, night: 1.0, bloom: 0.8, clouds: 0.35, water: lin(0x0a1a26), uw: lin(0x061418) },
        _ => Palette { top: lin(0x2a6fd0), mid: lin(0xbfd6ea), bot: lin(0x8f9aa0), fog: lin(0xb8cbd9), fog_density: 0.0032, sun_color: Vec3::new(1.0, 0.95, 0.84), sun_i: 5.2, sun_dir: d(40.0, 70.0, 28.0), ambient: lin(0x9fc4ee) * 0.62, exposure: 1.0, night: 0.0, bloom: 0.28, clouds: 0.45, water: lin(0x1f5560), uw: lin(0x1c5a5e) },
    }
}

// ───────────────────────── materials ─────────────────────────
const SLAB: Mat = mat(L_CONC, 0xc9c5bc, 0.0, 0.0);
const CONC: Mat = mat(L_CONC, 0xdcd8d0, 0.0, 0.0);
const BRICK: Mat = mat(L_BRICK, 0xffffff, 0.0, 0.0);
const CORR: Mat = mat(L_CORR, 0xffffff, 0.0, 0.5);
const CRATE: Mat = mat(L_WOOD, 0xd9b27a, 0.0, 0.0);
const WOODP: Mat = mat(L_WOOD, 0x9a7048, 0.0, 0.0);
const SAND: Mat = mat(L_SAND, 0xffffff, 0.0, 0.0).tile(0.4);
const RUST: Mat = mat(L_RUST, 0xffffff, 0.0, 0.4);
const GREY: Mat = mat(L_METAL, 0x8a8d92, 0.5, 0.6);
const BLUE: Mat = mat(L_FLAT, 0x1f5fbf, 0.42, 0.0);
const POLE: Mat = mat(L_CONC, 0x9a958c, 0.0, 0.0);
const WIRE: Mat = mat(L_FLAT, 0x0e0e0e, 0.7, 0.0);
const TIRE: Mat = mat(L_FLAT, 0x141414, 0.9, 0.0);
const GLASS: Mat = mat(L_FLAT, 0x10161e, 0.05, 0.0);
const FRAME: Mat = mat(L_FLAT, 0xd4d0c6, 0.55, 0.0);
const BARS: Mat = mat(L_METAL, 0x45484d, 0.5, 0.8);
const LAMP: Mat = mat(L_FLAT, 0xfff0d0, 0.4, 0.0).night(1.0);
const GROUND: Mat = mat(L_GROUND, 0xffffff, 0.0, 0.0).tile(1.0 / 6.0);
const GRASS: Mat = mat(L_GRASS, 0xffffff, 0.0, 0.0).tile(0.04);
const PL: [u32; 12] = [0x2f9c95, 0xd9a441, 0xd4705f, 0x4f8fca, 0x86c5a4, 0xc45f42, 0xe6dfd0, 0x9a82bd, 0xdf82a6, 0x7aa957, 0xf0c24a, 0x5bb0c0];
const DOORS: [u32; 5] = [0x3d6f9c, 0x4d7a52, 0x8a4a32, 0x6b6f76, 0xd8d2c4];
const CARS: [u32; 6] = [0xd8d2c4, 0x9c2b22, 0x2d5c8a, 0xc9b06a, 0x3a3d40, 0x5a2a16];
fn cloth(hex: u32) -> Mat { mat(L_FABRIC, hex, 0.0, 0.0) }
fn paint(hex: u32) -> Mat { mat(L_PLASTER, hex, 0.0, 0.0) }
fn v3(x: f32, y: f32, z: f32) -> Vec3 { Vec3::new(x, y, z) }

// ───────────────────────── collision ─────────────────────────
/// A circle or a rotated box with a height. `top` means you can stand (and climb) on it.
#[derive(Clone, Copy, Debug)]
pub struct Collider { pub boxy: bool, pub x: f32, pub z: f32, pub r: f32, pub br: f32, pub hx: f32, pub hz: f32, pub cos: f32, pub sin: f32, pub h: f32, pub top: bool }
#[derive(Clone, Copy)]
pub struct Statue { pub pos: Vec3, pub scale: f32, pub yaw: f32, pub color: u32, pub pose: u8, pub crown: bool }
struct Step { x: f32, z: f32, hx: f32, hz: f32, h: f32, base: f32 }

pub struct World {
    pub static_mesh: Mesh, pub static_decals: Mesh, pub arena_mesh: Mesh, pub arena_decals: Mesh,
    pub colliders: Vec<Collider>, pub last_hit: Option<usize>,
    pub lamps: Vec<Vec3>, static_lamps: Vec<Vec3>, pub statues: Vec<Statue>, static_statues: Vec<Statue>,
    steps: Vec<Step>,
}

impl World {
    fn add_c(&mut self, x: f32, z: f32, r: f32, h: f32, top: bool) { self.colliders.push(Collider { boxy: false, x, z, r, br: r, hx: 0.0, hz: 0.0, cos: 1.0, sin: 0.0, h, top }); }
    fn add_b(&mut self, x: f32, z: f32, hx: f32, hz: f32, rot: f32, h: f32, top: bool) { self.colliders.push(Collider { boxy: true, x, z, r: 0.0, br: hx.hypot(hz), hx, hz, cos: rot.cos(), sin: rot.sin(), h, top }); }

    /// Push a point of radius `rad` out of everything taller than its feet. Remembers what it bumped into.
    pub fn push_out(&mut self, p: &mut Vec3, rad: f32, feet_y: f32) {
        self.last_hit = None;
        for (i, c) in self.colliders.iter().enumerate() {
            if feet_y >= c.h - 0.4 { continue; }
            let (wx, wz) = (p.x - c.x, p.z - c.z);
            if !c.boxy {
                let (min, d2) = (c.r + rad, wx * wx + wz * wz);
                if d2 < min * min {
                    let d = d2.sqrt().max(1e-4);
                    p.x = c.x + wx / d * min; p.z = c.z + wz / d * min;
                    if self.last_hit.is_none() || c.top { self.last_hit = Some(i); }
                }
            } else {
                if wx * wx + wz * wz > (c.br + rad) * (c.br + rad) { continue; }
                let (lx, lz) = (wx * c.cos - wz * c.sin, wx * c.sin + wz * c.cos);
                let (qx, qz) = (lx.clamp(-c.hx, c.hx), lz.clamp(-c.hz, c.hz));
                let (ox, oz) = (lx - qx, lz - qz);
                let d2 = ox * ox + oz * oz;
                let (nx, nz);
                if d2 > 1e-10 { if d2 >= rad * rad { continue; } let d = d2.sqrt(); nx = qx + ox / d * rad; nz = qz + oz / d * rad; }
                else if c.hx - lx.abs() < c.hz - lz.abs() { nx = lx.signum() * (c.hx + rad); nz = lz; }
                else { nx = lx; nz = lz.signum() * (c.hz + rad); }
                p.x = c.x + nx * c.cos + nz * c.sin; p.z = c.z - nx * c.sin + nz * c.cos;
                if self.last_hit.is_none() || c.top { self.last_hit = Some(i); }
            }
        }
    }
    pub fn floor_at(&self, x: f32, z: f32, feet_y: f32) -> f32 {
        let mut f = 0.0;
        for c in &self.colliders {
            if !c.top || c.h <= f || feet_y < c.h - 0.4 { continue; }
            let (wx, wz) = (x - c.x, z - c.z);
            if !c.boxy { if wx * wx + wz * wz < c.r * c.r { f = c.h; } }
            else { let (lx, lz) = (wx * c.cos - wz * c.sin, wx * c.sin + wz * c.cos); if lx.abs() < c.hx && lz.abs() < c.hz { f = c.h; } }
        }
        f
    }
    /// First point (0..1) where the segment a→b enters something solid, or -1. Bullets and line of sight.
    pub fn seg_blocked(&self, a: Vec3, b: Vec3) -> f32 {
        let (dx, dz, dy) = (b.x - a.x, b.z - a.z, b.y - a.y);
        let len2 = dx * dx + dz * dz;
        if len2 < 1e-9 { return -1.0; }
        let mut best = -1.0f32;
        for c in &self.colliders {
            let (fx, fz) = (a.x - c.x, a.z - c.z);
            let tc = (-(fx * dx + fz * dz) / len2).clamp(0.0, 1.0);
            let (cx, cz) = (fx + dx * tc, fz + dz * tc);
            if cx * cx + cz * cz > c.br * c.br { continue; }
            let (mut te, mut tx);
            if !c.boxy {
                let bq = 2.0 * (fx * dx + fz * dz);
                let disc = bq * bq - 4.0 * len2 * (fx * fx + fz * fz - c.r * c.r);
                if disc < 0.0 { continue; }
                let sq = disc.sqrt();
                te = (-bq - sq) / (2.0 * len2); tx = (-bq + sq) / (2.0 * len2);
            } else {
                let (ox, oz, ex, ez) = (fx * c.cos - fz * c.sin, fx * c.sin + fz * c.cos, dx * c.cos - dz * c.sin, dx * c.sin + dz * c.cos);
                te = f32::NEG_INFINITY; tx = f32::INFINITY;
                if ex.abs() < 1e-9 { if ox.abs() > c.hx { continue; } } else { let (t1, t2) = ((-c.hx - ox) / ex, (c.hx - ox) / ex); te = te.max(t1.min(t2)); tx = tx.min(t1.max(t2)); }
                if ez.abs() < 1e-9 { if oz.abs() > c.hz { continue; } } else { let (t1, t2) = ((-c.hz - oz) / ez, (c.hz - oz) / ez); te = te.max(t1.min(t2)); tx = tx.min(t1.max(t2)); }
                if te > tx { continue; }
            }
            if tx < 0.0 || te > 1.0 { continue; }
            te = te.max(0.0); tx = tx.min(1.0);
            let (y0, y1) = (a.y + dy * te, a.y + dy * tx);
            let t = if y0 < c.h { te } else if y1 < c.h { te + (tx - te) * (y0 - c.h) / (y0 - y1) } else { continue };
            if best < 0.0 || t < best { best = t; }
        }
        best
    }
}

// ───────────────────────── building blocks ─────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Op { Window, Door, Shop }
#[derive(Clone, Copy)]
struct Opening { x0: f32, x1: f32, y0: f32, y1: f32, kind: Op }

fn decal(dk: &mut Kit, r: Rect, cx: f32, cy: f32, z: f32, w: f32, h: f32, tint: u32, alpha: f32, rot: f32, rough: f32) {
    let m = Mat { layer: 0, color: [(tint >> 16) as u8, (tint >> 8) as u8, tint as u8, (alpha * 255.0) as u8], rough: (rough * 255.0) as u8, metal: 0, emissive: 0, tile: 1.0 };
    let (c, s) = (rot.cos(), rot.sin());
    let p = |x: f32, y: f32| v3(cx + x * c - y * s, cy + x * s + y * c, z);
    dk.quad4(m, [p(-w / 2.0, -h / 2.0), p(w / 2.0, -h / 2.0), p(w / 2.0, h / 2.0), p(-w / 2.0, h / 2.0)], [Vec2::new(r.u0, r.v1), Vec2::new(r.u1, r.v1), Vec2::new(r.u1, r.v0), Vec2::new(r.u0, r.v0)]);
}
/// A wall in the local XY plane facing +Z with real openings cut into it: recessed, framed, glazed, barred.
fn wall(k: &mut Kit, rng: &mut Rng, mt: Mat, band: Option<Mat>, x0: f32, x1: f32, y0: f32, y1: f32, ops: &[Opening], bars: bool) {
    let (mut xs, mut ys) = (vec![x0, x1], vec![y0, y1]);
    for o in ops { xs.push(o.x0); xs.push(o.x1); ys.push(o.y0); ys.push(o.y1); }
    if band.is_some() { ys.push(y0 + 0.95); }
    xs.sort_by(|a, b| a.total_cmp(b)); ys.sort_by(|a, b| a.total_cmp(b));
    for xi in 0..xs.len() - 1 { for yi in 0..ys.len() - 1 {
        let (xa, xb, ya, yb) = (xs[xi], xs[xi + 1], ys[yi], ys[yi + 1]);
        if xb - xa < 1e-4 || yb - ya < 1e-4 { continue; }
        let (cx, cy) = ((xa + xb) * 0.5, (ya + yb) * 0.5);
        if ops.iter().any(|o| cx > o.x0 && cx < o.x1 && cy > o.y0 && cy < o.y1) { continue; }
        let m = match band { Some(b) if cy < y0 + 0.95 => b, _ => mt };
        k.rect_xy(m, xa, xb, ya, yb, 0.0);
    } }
    let jamb = mt.shade(0.78);
    for o in ops {
        let dp = match o.kind { Op::Window => 0.17, Op::Door => 0.11, Op::Shop => 0.14 };
        k.quad(jamb, v3(o.x0, o.y0, 0.0), v3(o.x0, o.y0, -dp), v3(o.x0, o.y1, -dp), v3(o.x0, o.y1, 0.0));
        k.quad(jamb, v3(o.x1, o.y0, -dp), v3(o.x1, o.y0, 0.0), v3(o.x1, o.y1, 0.0), v3(o.x1, o.y1, -dp));
        k.quad(jamb, v3(o.x0, o.y0, 0.0), v3(o.x1, o.y0, 0.0), v3(o.x1, o.y0, -dp), v3(o.x0, o.y0, -dp));
        k.quad(jamb, v3(o.x0, o.y1, -dp), v3(o.x1, o.y1, -dp), v3(o.x1, o.y1, 0.0), v3(o.x0, o.y1, 0.0));
        let (w, h, cx, cy) = (o.x1 - o.x0, o.y1 - o.y0, (o.x0 + o.x1) * 0.5, (o.y0 + o.y1) * 0.5);
        match o.kind {
            Op::Window => {
                let lit = if rng.chance(0.5) { rng.r(0.5, 1.0) } else { 0.0 };
                k.rect_xy(GLASS.night(lit), o.x0, o.x1, o.y0, o.y1, -dp);
                let fz = -dp + 0.03;
                k.bx(FRAME, w, 0.05, 0.05, cx, o.y0 + 0.025, fz).bx(FRAME, w, 0.05, 0.05, cx, o.y1 - 0.025, fz).bx(FRAME, 0.05, h, 0.05, o.x0 + 0.025, cy, fz).bx(FRAME, 0.05, h, 0.05, o.x1 - 0.025, cy, fz).bx(FRAME, 0.04, h, 0.04, cx, cy, fz);
                if rng.chance(0.5) { let c = rng.pick(&PL); k.rect_xy(cloth(c), o.x0 + 0.05, o.x0 + w * rng.r(0.3, 0.5), o.y0 + 0.05, o.y1 - 0.05, -dp + 0.012); }
                if bars { let n = (w / 0.24) as i32; for b in 1..n { k.bx(BARS, 0.018, h, 0.018, o.x0 + w * b as f32 / n as f32, cy, -0.05); } k.bx(BARS, w, 0.018, 0.018, cx, cy, -0.05); }
                k.bx(SLAB, w + 0.16, 0.06, 0.16, cx, o.y0 - 0.03, 0.03);
            }
            Op::Door => {
                let m = Mat { tile: 1.0, ..mat(L_DOOR, rng.pick(&DOORS), 0.0, 0.0) };
                k.quad4(m, [v3(o.x0, o.y0, -dp), v3(o.x1, o.y0, -dp), v3(o.x1, o.y1, -dp), v3(o.x0, o.y1, -dp)], [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)]);
                k.bx(SLAB, w + 0.3, 0.14, 0.34, cx, o.y0 + 0.07 - 0.14, 0.12);
            }
            Op::Shop => {
                if rng.chance(0.6) { k.rect_xy(mat(L_SHUTTER, rng.pick(&[0x9aa0a6, 0x6f8fae, 0xb8a06a]), 0.0, 0.6), o.x0, o.x1, o.y0, o.y1, -dp); }
                else { k.rect_xy(mat(L_FLAT, 0x0a0a0c, 0.9, 0.0).night(0.35), o.x0, o.x1, o.y0, o.y1, -0.9); k.bx(WOODP, w * 0.8, 0.9, 0.4, cx, o.y0 + 0.45, -0.3); }
            }
        }
    }
}
fn tank(k: &mut Kit, x: f32, y: f32, z: f32, r: f32, h: f32) {
    k.cyl(BLUE, r, r, v3(x, y, z), v3(x, y + h, z), 18, true);
    for t in [0.3, 0.62] { k.cyl(BLUE, r * 1.04, r * 1.04, v3(x, y + h * t, z), v3(x, y + h * t + 0.06, z), 18, true); }
    k.ellipsoid(BLUE, v3(r * 0.98, r * 0.22, r * 0.98), v3(x, y + h, z), 16, false).cyl(BLUE, r * 0.25, r * 0.25, v3(x, y + h, z), v3(x, y + h + r * 0.32, z), 10, true);
}
fn roof_junk(k: &mut Kit, rng: &mut Rng, w: f32, d: f32, y: f32, xo: f32, zo: f32) {
    let (cx, cz) = (xo + (w / 2.0 - 0.9) * rng.sign(), zo + (d / 2.0 - 0.9) * rng.sign());
    if rng.chance(0.6) { k.bx(CONC, 1.3, 0.5, 1.3, cx, y + 0.25, cz); tank(k, cx, y + 0.5, cz, 0.55, 0.9); }
    if rng.chance(0.55) { for sx in [-1.0, 1.0] { for sz in [-1.0, 1.0] { let (px, pz) = (xo + sx * (w / 2.0 - 0.2), zo + sz * (d / 2.0 - 0.2)); k.cyl(RUST, 0.03, 0.03, v3(px, y, pz), v3(px + rng.r(-0.1, 0.1), y + rng.r(0.5, 1.2), pz), 5, false); } } }
    if rng.chance(0.4) { let (px, pz) = (2.0 * xo - cx, 2.0 * zo - cz); k.cyl(GREY, 0.03, 0.03, v3(px, y, pz), v3(px, y + 0.7, pz), 6, false).ellipsoid(GREY, v3(0.42, 0.42, 0.1), v3(px, y + 0.85, pz + 0.1), 12, false); }
    if rng.chance(0.4) { // washing line
        let (x0, x1, lz) = (xo - w / 2.0 + 0.4, xo + w / 2.0 - 0.4, zo + rng.r(-d / 4.0, d / 4.0));
        k.cyl(GREY, 0.025, 0.025, v3(x0, y, lz), v3(x0, y + 1.7, lz), 5, false).cyl(GREY, 0.025, 0.025, v3(x1, y, lz), v3(x1, y + 1.7, lz), 5, false).cyl(WIRE, 0.008, 0.008, v3(x0, y + 1.6, lz), v3(x1, y + 1.6, lz), 4, false);
        let mut px = x0 + 0.5;
        while px < x1 - 0.3 { let (h, c) = (rng.r(0.5, 0.8), rng.pick(&PL)); k.quad(cloth(c), v3(px - 0.25, y + 1.6 - h, lz), v3(px + 0.25, y + 1.6 - h, lz), v3(px + 0.25, y + 1.6, lz), v3(px - 0.25, y + 1.6, lz)); px += rng.r(0.6, 1.0); }
    }
}
/// One storey: four walls with openings on the front (and maybe the sides), a concrete slab on top.
fn storey(k: &mut Kit, dk: &mut Kit, at: &Atlas, rng: &mut Rng, w: f32, d: f32, h: f32, y: f32, xo: f32, zo: f32, ground: bool, sides: bool, roof: Mat) {
    let brick = rng.chance(0.3);
    let wall_m = if brick { BRICK } else { paint(rng.pick(&PL)) };
    let band = if !brick && rng.chance(0.35) { Some(paint(rng.pick(&PL)).shade(0.75)) } else { None };
    let frames = [(v3(xo, 0.0, zo + d / 2.0), 0.0, w, true), (v3(xo, 0.0, zo - d / 2.0), PI, w, false), (v3(xo + w / 2.0, 0.0, zo), PI / 2.0, d, false), (v3(xo - w / 2.0, 0.0, zo), -PI / 2.0, d, false)];
    for (pos, rot, ww, front) in frames {
        let old = k.push(Mat4::from_translation(pos) * Mat4::from_rotation_y(rot));
        let dold = dk.push(Mat4::from_translation(pos) * Mat4::from_rotation_y(rot));
        let mut ops: Vec<Opening> = Vec::new();
        let win = |cx: f32, ww2: f32| Opening { x0: cx - ww2 / 2.0, x1: cx + ww2 / 2.0, y0: y + 1.0, y1: y + 2.15, kind: Op::Window };
        if front {
            if ground && rng.chance(0.3) && ww > 6.0 {
                let cx = ww * 0.16;
                ops.push(Opening { x0: cx - 1.4, x1: cx + 1.4, y0: y, y1: y + 2.35, kind: Op::Shop });
                ops.push(Opening { x0: -ww * 0.34 - 0.45, x1: -ww * 0.34 + 0.45, y0: y, y1: y + 2.05, kind: Op::Door });
                k.bx(mat(L_FLAT, 0x2a2420, 0.6, 0.0), 2.9, 0.56, 0.08, cx, y + 2.72, 0.05);
                let s = rng.pick(&at.signs); decal(dk, s, cx, y + 2.72, 0.095, 2.8, 2.8 / s.aspect, 0xffffff, 1.0, 0.0, 0.5);
                if rng.chance(0.6) { let c = rng.pick(&PL); k.bxr(cloth(c), 3.2, 0.05, 1.3, cx, y + 2.38, 0.62, 0.32, 0.0, 0.0); }
            } else if ground {
                ops.push(Opening { x0: -ww * 0.28 - 0.45, x1: -ww * 0.28 + 0.45, y0: y, y1: y + 2.05, kind: Op::Door });
                ops.push(win(ww * 0.2, 1.3));
            } else if ww > 5.0 { ops.push(win(-ww * 0.24, 1.2)); ops.push(win(ww * 0.24, 1.2)); } else { ops.push(win(0.0, 1.2)); }
        } else if sides && ww > 3.6 && rng.chance(0.6) { ops.push(win(rng.r(-ww * 0.15, ww * 0.15), 1.1)); }
        let bars = ground || rng.chance(0.5);
        wall(k, rng, wall_m, band, -ww / 2.0, ww / 2.0, y, y + h, &ops, bars);
        if front {
            if rng.chance(0.45) { let t = rng.pick(&at.tags); let tw = rng.r(2.0, 3.2).min(ww * 0.5); decal(dk, t, rng.r(-ww * 0.1, ww * 0.3), y + rng.r(0.45, 0.8), 0.012, tw, tw / t.aspect, rng.pick(&SPRAY), 0.92, rng.r(-0.07, 0.07), 0.8); }
            if rng.chance(0.3) { let king = rng.chance(0.3); let b = if king { at.king } else { at.bear }; decal(dk, b, rng.r(ww * 0.3, ww * 0.42), y + 0.8, 0.012, 0.9, 0.9 / b.aspect, rng.pick(&SPRAY), 0.95, 0.0, 0.8); }
            if rng.chance(0.5) { k.bx(GREY, 0.8, 0.5, 0.45, rng.r(-ww / 3.0, ww / 3.0), y + rng.r(2.3, 2.7), 0.22); }
            if rng.chance(0.4) { let px = ww / 2.0 - 0.25; k.cyl(mat(L_FLAT, 0xb8b4aa, 0.6, 0.0), 0.05, 0.05, v3(px, y, 0.06), v3(px, y + h, 0.06), 6, false); }
        }
        k.pop(old); dk.pop(dold);
    }
    k.bx(roof, w + 0.3, 0.16, d + 0.3, xo, y + h + 0.08, zo);
}
/// A ring sector of flat ground with world-space UVs. Angles are measured from +Z.
fn fan(k: &mut Kit, mt: Mat, r0: f32, r1: f32, a0: f32, a1: f32, seg: u32, y: f32) {
    for i in 0..seg {
        let (ta, tb) = (a0 + (a1 - a0) * i as f32 / seg as f32, a0 + (a1 - a0) * (i + 1) as f32 / seg as f32);
        let p = |a: f32, r: f32| v3(a.sin() * r, y, a.cos() * r);
        let pts = [p(ta, r0), p(ta, r1), p(tb, r1), p(tb, r0)];
        k.quad4(mt, pts, pts.map(|q| Vec2::new(q.x, q.z)));
    }
}

impl World {
    pub fn new(at: &Atlas) -> World {
        let mut w = World { static_mesh: Mesh::default(), static_decals: Mesh::default(), arena_mesh: Mesh::default(), arena_decals: Mesh::default(), colliders: Vec::new(), last_hit: None, lamps: Vec::new(), static_lamps: Vec::new(), statues: Vec::new(), static_statues: Vec::new(), steps: Vec::new() };
        for i in 0..3 {
            let f = i as f32;
            w.steps.push(Step { x: 0.0, z: 6.8 + (2.0 - f) * 0.6, hx: 1.6, hz: 0.3, h: 0.325 * (f + 1.0), base: 0.0 });
            w.steps.push(Step { x: -(4.5 + (2.0 - f) * 0.6), z: 0.0, hx: 0.3, hz: 1.4, h: 1.3 + 0.325 * (f + 1.0), base: 1.3 });
            w.steps.push(Step { x: 2.5 + (2.0 - f) * 0.6, z: 0.0, hx: 0.3, hz: 1.1, h: 2.6 + 0.325 * (f + 1.0), base: 2.6 });
        }
        w.build_static(at);
        w.build_arena(at, 1, &[]);
        w
    }

    fn build_static(&mut self, at: &Atlas) {
        let (mut k, mut dk, mut rng, r) = (Kit::new(), Kit::new(), Rng(90210), ARENA_R);
        // ground: the square, the land around it, the seabed and quay of the bay to the south
        fan(&mut k, GROUND, 0.05, 30.0, 0.0, TAU, 32, 0.0); fan(&mut k, GROUND, 30.0, r + 0.5, 0.0, TAU, 64, 0.0);
        fan(&mut k, GROUND, r + 0.5, 150.0, BAY, TAU - BAY, 48, 0.0); fan(&mut k, GRASS, 150.0, 700.0, BAY, TAU - BAY, 48, 0.0);
        k.quad(SAND, v3(-420.0, -5.2, r + 600.0), v3(420.0, -5.2, r + 600.0), v3(420.0, -5.2, r - 60.0), v3(-420.0, -5.2, r - 60.0));
        for i in 0..32 { let (a0, a1) = (-BAY + 2.0 * BAY * i as f32 / 32.0, -BAY + 2.0 * BAY * (i + 1) as f32 / 32.0); let p = |a: f32, y: f32| v3(a.sin() * (r + 0.5), y, a.cos() * (r + 0.5)); k.quad(CONC, p(a1, -5.4), p(a0, -5.4), p(a0, 0.0), p(a1, 0.0)); }
        for (a0, a1) in [(-BAY, -0.05), (0.05, BAY)] { for i in 0..24 { // sea wall with a gap for the boat ramp
            let (ta, tb) = (a0 + (a1 - a0) * i as f32 / 24.0, a0 + (a1 - a0) * (i + 1) as f32 / 24.0);
            let (pa, pb) = (v3(ta.sin() * r, 0.0, ta.cos() * r), v3(tb.sin() * r, 0.0, tb.cos() * r));
            let mid = (pa + pb) * 0.5;
            k.boxm(CONC, v3((pb - pa).length() + 0.05, 0.9, 0.35), Mat4::from_translation(mid + Vec3::Y * 0.45) * Mat4::from_rotation_y(-(pb.z - pa.z).atan2(pb.x - pa.x)));
        } }
        k.bxr(CONC, 8.4, 0.4, 17.0, 0.0, -1.72, r + 8.3, 0.2, 0.0, 0.0); // boat ramp
        k.bx(WOODP, 3.0, 0.25, 24.0, -20.0, 0.15, r + 11.0);
        let mut z = r + 1.0; while z < r + 23.0 { for x in [-21.3, -18.7] { k.cyl(WOODP, 0.16, 0.16, v3(x, -5.0, z), v3(x, 0.6, z), 6, true); } z += 4.0; }
        for (bx, bz, c) in [(-26.0, r + 14.0, 0x2f6fb0u32), (22.0, r + 26.0, 0xc8553a), (-9.0, r + 34.0, 0xe6dfd0)] { // fishing boats
            let old = k.push(Mat4::from_translation(v3(bx, WATER_Y, bz)) * Mat4::from_rotation_y(rng.r(0.0, TAU)));
            k.ellipsoid(mat(L_WOOD, c, 0.0, 0.0), v3(1.3, 0.7, 3.4), v3(0.0, 0.05, 0.0), 16, false).bx(paint(0xe6dfd0), 1.4, 1.1, 1.6, 0.0, 0.9, -0.8);
            k.pop(old);
        }
        for _ in 0..1000 { // litter and candy wrappers
            let (a, d) = (rng.r(0.0, TAU), rng.f().sqrt() * (r - 2.0));
            let c = if rng.chance(0.6) { rng.pick(&SPRAY) } else { rng.pick(&[0xd8d2c4, 0x6a665e, 0xb0a890]) };
            k.bxr(mat(L_FLAT, c, 0.6, 0.0), rng.r(0.12, 0.3), 0.012, rng.r(0.08, 0.2), a.cos() * d, 0.012, a.sin() * d, 0.0, rng.r(0.0, TAU), 0.0);
        }
        for i in 0..60 { // dirt and old asphalt patches, puddles
            let (a, d, s) = (rng.r(0.0, TAU), rng.r(10.0, r - 4.0), rng.r(2.0, 7.0));
            let wet = i % 3 == 0;
            let old = dk.push(Mat4::from_translation(v3(a.cos() * d, 0.012 + i as f32 * 0.0002, a.sin() * d)) * Mat4::from_rotation_x(-PI / 2.0));
            decal(&mut dk, at.splat, 0.0, 0.0, 0.0, s * if wet { 0.6 } else { 1.6 }, s * rng.r(0.4, 1.0), if wet { 0x1a1f24 } else { rng.pick(&[0x4f4034, 0x3b3530, 0x5d4f3f]) }, if wet { 0.7 } else { rng.r(0.3, 0.55) }, rng.r(0.0, TAU), if wet { 0.03 } else { 0.95 });
            dk.pop(old);
        }

        // the ring of stacked houses that walls the square in
        let n = 54; let w_house = TAU * (r + 5.0) / n as f32;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU; let (cx, cz) = (a.cos(), a.sin());
            if in_bay(cx, cz) { continue; }
            let alley = i % 9 == 4;
            let d = rng.r(7.0, 10.0); let rc = r + 0.8 + d / 2.0 + if alley { 6.0 } else { 0.0 };
            let rot = (-cx).atan2(-cz);
            k.at(cx * rc, 0.0, cz * rc, rot); dk.at(cx * rc, 0.0, cz * rc, rot);
            if alley { k.bx(CORR, w_house, 2.4, 0.08, 0.0, 1.2, d / 2.0 - 5.4); }
            let floors = if alley { 2 } else { 2 + (rng.f() * 3.0) as i32 };
            let (mut w, mut dd, mut xo, mut zo, mut y) = (w_house * rng.r(0.96, 1.04), d, 0.0, 0.0, 0.0);
            for f in 0..floors {
                let h = rng.r(2.9, 3.3);
                let roof = if f == floors - 1 && rng.chance(0.4) { CORR } else { SLAB };
                storey(&mut k, &mut dk, at, &mut rng, w, dd, h, y, xo, zo, f == 0 && !alley, f > 0, roof);
                y += h + 0.16;
                if f == floors - 1 { roof_junk(&mut k, &mut rng, w, dd, y, xo, zo); }
                let (nw, nd) = (w * rng.r(0.55, 1.0), dd * rng.r(0.6, 1.0));
                xo += rng.r(-(w - nw) / 2.0, (w - nw) / 2.0); zo += (dd - nd) / 2.0 * rng.r(0.2, 1.0); w = nw; dd = nd;
            }
        }
        for i in 0..40 { // a taller second row behind, for the skyline
            let a = i as f32 / 40.0 * TAU + 0.05; let (cx, cz) = (a.cos(), a.sin());
            if in_bay(cx, cz) { continue; }
            let rc = r + rng.r(17.0, 27.0); let rot = (-cx).atan2(-cz) + rng.r(-0.3, 0.3);
            k.at(cx * rc, 0.0, cz * rc, rot); dk.at(cx * rc, 0.0, cz * rc, rot);
            let (mut y, mut w, mut dd) = (0.0, rng.r(8.0, 13.0), rng.r(7.0, 10.0));
            let nf = 3 + (rng.f() * 4.0) as i32;
            for f in 0..nf { let h = rng.r(2.9, 3.3); storey(&mut k, &mut dk, at, &mut rng, w, dd, h, y, 0.0, 0.0, false, true, SLAB); y += h + 0.16; w *= rng.r(0.7, 1.0); dd *= rng.r(0.7, 1.0); if f == nf - 1 { roof_junk(&mut k, &mut rng, w, dd, y, 0.0, 0.0); } }
        }
        k.origin(); dk.origin();
        // street poles round the edge, sagging wires, party lights strung in to the middle
        let mut poles: Vec<(f32, f32)> = Vec::new();
        for i in 0..14 { let a = i as f32 / 14.0 * TAU + 0.2; let (cx, cz) = (a.cos(), a.sin()); if !in_bay(cx, cz) { poles.push((cx * (r + 0.3), cz * (r + 0.3))); } }
        for (i, &(x, z)) in poles.iter().enumerate() {
            let face = x.atan2(z);
            k.cyl(POLE, 0.17, 0.14, v3(x, 0.0, z), v3(x, 9.0, z), 8, true).bxr(POLE, 0.14, 0.14, 2.2, x, 8.4, z, 0.0, face + PI / 2.0, 0.0).cyl(GREY, 0.3, 0.3, v3(x, 6.6, z + 0.45), v3(x, 7.4, z + 0.45), 10, true);
            let (lx, lz) = (x * 0.975, z * 0.975);
            k.cyl(POLE, 0.05, 0.05, v3(x, 7.8, z), v3(lx, 8.0, lz), 5, false).bxr(LAMP, 0.5, 0.12, 0.3, lx, 7.95, lz, 0.0, face, 0.0);
            self.static_lamps.push(v3(lx, 7.6, lz));
            let (nx, nz) = poles[(i + 1) % poles.len()];
            if (nx - x).hypot(nz - z) < 60.0 { for hgt in [8.3, 8.6] { let (mx, mz) = ((x + nx) / 2.0, (z + nz) / 2.0); k.cyl(WIRE, 0.018, 0.018, v3(x, hgt, z), v3(mx, hgt - 0.9, mz), 4, false).cyl(WIRE, 0.018, 0.018, v3(mx, hgt - 0.9, mz), v3(nx, hgt, nz), 4, false); } }
            if i % 2 == 0 { // festa lights
                let pt = |t: f32| v3(x * (1.0 - t), 8.6 * (1.0 - t) + 5.6 * t - (t * PI).sin() * 2.2, z * (1.0 - t));
                for j in 1..26 { let p = pt(j as f32 / 26.0); k.ellipsoid(mat(L_FLAT, SPRAY[j % SPRAY.len()], 0.3, 0.0).glow(0.12).night(0.6), Vec3::splat(0.11), p, 8, false); if j < 25 { k.cyl(WIRE, 0.012, 0.012, p + Vec3::Y * 0.1, pt((j + 1) as f32 / 26.0) + Vec3::Y * 0.1, 3, false); } }
            }
            for _ in 0..2 { // gummies slung over the wire, where the trainers usually go
                let t = rng.r(0.2, 0.8); let (mx, mz) = (x + (nx - x) * t, z + (nz - z) * t); let c = mat(L_FLAT, rng.pick(&SPRAY), 0.25, 0.0);
                k.cyl(WIRE, 0.008, 0.008, v3(mx, 7.5, mz), v3(mx, 6.9, mz), 3, false).ellipsoid(c, v3(0.13, 0.19, 0.13), v3(mx - 0.12, 6.8, mz), 8, false).ellipsoid(c, v3(0.13, 0.19, 0.13), v3(mx + 0.14, 6.7, mz), 8, false);
            }
        }
        // the middle of the square: a half-built house you can climb, floor by floor
        let mut y0 = 0.0;
        for (i, &(half, h)) in CAKE.iter().enumerate() {
            let s = half * 2.0;
            let m = if i == 1 { paint(0x2f9c95) } else { BRICK };
            k.bx(m, s, h - y0 - 0.16, s, 0.0, (h + y0 - 0.16) / 2.0, 0.0).bx(SLAB, s + 0.3, 0.16, s + 0.3, 0.0, h - 0.08, 0.0);
            if i == 1 { decal(&mut dk, at.mural, 0.0, (h + y0 - 0.16) / 2.0, half + 0.012, s * 0.96, (h - y0 - 0.16) * 0.96, 0xffffff, 1.0, 0.0, 0.85); }
            for sx in [-1.0, 1.0] { for sz in [-1.0, 1.0] { let (px, pz) = (sx * (half - 0.25), sz * (half - 0.25)); k.cyl(RUST, 0.035, 0.035, v3(px, h, pz), v3(px + rng.r(-0.1, 0.1), h + rng.r(0.7, 1.4), pz), 5, false); } }
            y0 = h;
        }
        for s in &self.steps { k.bx(SLAB, s.hx * 2.0, s.h - s.base, s.hz * 2.0, s.x, (s.h + s.base) / 2.0, s.z); }
        k.bx(CONC, 1.6, 0.4, 1.6, 0.0, 4.1, 0.0); tank(&mut k, 0.0, 4.3, 0.0, 0.8, 1.2);
        self.static_lamps.push(v3(0.0, 6.6, 0.0));

        // hills of houses on three sides, and the statue everyone can see
        let mut hills: Vec<(f32, f32, f32, f32)> = Vec::new();
        for i in 0..7 { let a = BAY + 0.35 + i as f32 / 6.0 * (TAU - 2.0 * BAY - 0.7); let d = rng.r(190.0, 270.0); hills.push((a.sin() * d, a.cos() * d, rng.r(70.0, 110.0), rng.r(38.0, 70.0))); }
        for &(hx, hz, hr, hh) in &hills { k.ellipsoid(GRASS, v3(hr, hh, hr), v3(hx, -1.0, hz), 28, true); }
        for i in 0..2200 {
            let (hx, hz, hr, hh) = hills[i % hills.len()];
            let (a, rr) = (rng.r(0.0, TAU), rng.f().sqrt() * hr * 0.93);
            let sy = rng.r(2.7, 3.3) * if rng.chance(0.35) { 2.0 } else { 1.0 };
            let c = if rng.chance(0.3) { 0xb5603a } else { rng.pick(&PL) };
            let pos = v3(hx + a.cos() * rr, hh * (1.0 - (rr / hr).powi(2)).max(0.0).sqrt() - 1.0 + sy / 2.0 - 0.6, hz + a.sin() * rr);
            k.boxm(mat(L_HILL, c, 0.0, 0.0).tile(1.0 / 3.0), v3(rng.r(3.5, 7.0), sy, rng.r(3.5, 7.0)), Mat4::from_translation(pos) * Mat4::from_rotation_y(rng.r(0.0, TAU)));
        }
        let top = *hills.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        k.bx(CONC, 12.0, 8.0, 12.0, top.0, top.3 - 1.0, top.1);
        self.static_statues.push(Statue { pos: v3(top.0, top.3 + 3.0, top.1), scale: 13.0, yaw: (-top.0).atan2(-top.1), color: 0xff3b4e, pose: 1, crown: false });
        self.static_statues.push(Statue { pos: v3(17.0, WATER_Y + 0.9, r + 15.0), scale: 3.2, yaw: 0.6, color: 0xff5fb8, pose: 2, crown: false }); // the inflatable in the bay
        self.static_mesh = k.m; self.static_decals = dk.m;
    }

    fn place(&self, rng: &mut Rng, r: f32, min_r: f32) -> Option<(f32, f32)> {
        let max_r = ARENA_R - 8.0;
        for _ in 0..90 {
            let (a, d) = (rng.r(0.0, TAU), rng.r(min_r * min_r, max_r * max_r).sqrt());
            let (x, z) = (a.cos() * d, a.sin() * d);
            if (x - SPAWN.0).hypot(z - SPAWN.1) < r + 8.0 { continue; }
            if self.colliders.iter().all(|c| (c.x - x).hypot(c.z - z) > c.br + r + 4.2) { return Some((x, z)); }
        }
        None
    }

    /// Street furniture for one mission, from a seed: shacks, walls, cars, stalls, crates, tanks, rubble, poles, statues.
    pub fn build_arena(&mut self, at: &Atlas, seed: u32, layout: &[(&str, u32)]) {
        self.colliders.clear();
        for &(half, h) in CAKE.iter() { self.add_b(0.0, 0.0, half, half, 0.0, h, true); }
        for i in 0..self.steps.len() { let s = &self.steps[i]; let (x, z, hx, hz, h) = (s.x, s.z, s.hx, s.hz, s.h); self.add_b(x, z, hx, hz, 0.0, h, true); }
        self.add_c(0.0, 0.0, 0.9, 5.8, false);
        self.lamps = self.static_lamps.clone(); self.statues = self.static_statues.clone();
        let count = |key: &str, def: u32| layout.iter().find(|(k2, _)| *k2 == key).map(|(_, v)| (*v as f32 * 2.2).round() as u32).unwrap_or(def);
        let (mut k, mut dk, mut rng) = (Kit::new(), Kit::new(), Rng(seed.wrapping_mul(7919).wrapping_add(13)));

        for _ in 0..count("shacks", 22) { // one-room houses: hard cover with a roof to fight from
            let (w, d) = (rng.r(4.6, 6.4), rng.r(3.8, 5.0));
            let Some((x, z)) = self.place(&mut rng, w.hypot(d) / 2.0, 20.0) else { continue };
            let rot = rng.r(0.0, TAU);
            k.at(x, 0.0, z, rot); dk.at(x, 0.0, z, rot);
            let roof = if rng.chance(0.5) { CORR } else { SLAB };
            storey(&mut k, &mut dk, at, &mut rng, w, d, 2.55, 0.0, 0.0, 0.0, true, true, roof);
            roof_junk(&mut k, &mut rng, w, d, 2.71, 0.0, 0.0);
            self.add_b(x, z, w / 2.0 + 0.1, d / 2.0 + 0.1, rot, 2.71, true);
        }
        for _ in 0..count("statues", 2) {
            let Some((x, z)) = self.place(&mut rng, 2.4, 22.0) else { continue };
            k.at(x, 0.0, z, rng.r(0.0, TAU)).bx(CONC, 3.6, 1.2, 3.6, 0.0, 0.6, 0.0);
            self.statues.push(Statue { pos: v3(x, 1.2, z), scale: 3.6, yaw: (-x).atan2(-z), color: rng.pick(&[0xff3b4e, 0x3fd45c, 0xffc21f, 0xff8a1f]), pose: 0, crown: true });
            self.add_c(x, z, 2.3, 8.4, false);
        }
        for i in 0..count("walls", 22) {
            let hx = rng.r(2.2, 3.6);
            let Some((x, z)) = self.place(&mut rng, hx, 16.0) else { continue };
            let rot = rng.r(0.0, TAU);
            k.at(x, 0.0, z, rot); dk.at(x, 0.0, z, rot);
            let m = if i % 2 == 0 { BRICK } else { paint(rng.pick(&[0xb9b4aa, 0xc9bfa6, 0x9fb3b0])) };
            k.bx(m, hx * 2.0, 2.3, 0.9, 0.0, 1.15, 0.0).bx(SLAB, hx * 2.0 + 0.1, 0.1, 1.0, 0.0, 2.3, 0.0);
            for side in [1.0f32, -1.0] {
                let old = dk.push(Mat4::from_rotation_y(if side > 0.0 { 0.0 } else { PI }));
                let t = rng.pick(&at.tags); let tw = (hx * 1.5).min(4.2);
                decal(&mut dk, t, rng.r(-0.3, 0.3), rng.r(1.1, 1.4), 0.462, tw, tw / t.aspect, rng.pick(&SPRAY), 0.95, rng.r(-0.06, 0.06), 0.8);
                if rng.chance(0.7) { let b = if rng.chance(0.3) { at.king } else { at.bear }; decal(&mut dk, b, -hx + 0.7, 1.1, 0.462, 1.0, 1.0 / b.aspect, rng.pick(&SPRAY), 0.95, 0.0, 0.8); }
                dk.pop(old);
            }
            self.add_b(x, z, hx, 0.45, rot, 2.35, true);
        }
        for _ in 0..count("cars", 10) {
            let Some((x, z)) = self.place(&mut rng, 2.4, 16.0) else { continue };
            let rot = rng.r(0.0, TAU);
            let body = mat(L_METAL, rng.pick(&CARS), 0.38, 0.35);
            k.at(x, 0.0, z, rot);
            k.bx(body, 4.2, 0.6, 1.75, 0.0, 0.62, 0.0).bx(body, 2.2, 0.52, 1.6, -0.25, 1.16, 0.0).bx(GLASS, 2.24, 0.36, 1.62, -0.25, 1.17, 0.0).bx(GLASS, 1.9, 0.38, 1.64, -0.25, 1.17, 0.0).bx(GREY, 0.14, 0.2, 1.8, 2.14, 0.5, 0.0).bx(GREY, 0.14, 0.2, 1.8, -2.14, 0.5, 0.0);
            for wx in [-1.35, 1.35] { k.cyl(TIRE, 0.34, 0.34, v3(wx, 0.34, -0.92), v3(wx, 0.34, 0.92), 12, true); }
            self.add_b(x, z, 2.15, 0.9, rot, 1.42, true);
        }
        for _ in 0..count("stalls", 8) { // market stalls, selling exactly what you think
            let Some((x, z)) = self.place(&mut rng, 1.8, 16.0) else { continue };
            let rot = rng.r(0.0, TAU);
            k.at(x, 0.0, z, rot).bx(WOODP, 2.8, 0.1, 1.3, 0.0, 0.95, 0.0);
            for sx in [-1.3, 1.3] { for sz in [-0.55, 0.55] { k.cyl(WOODP, 0.05, 0.05, v3(sx, 0.0, sz), v3(sx, 2.3, sz), 5, false); } }
            let c = rng.pick(&PL); k.bxr(cloth(c), 3.1, 0.05, 1.7, 0.0, 2.34, 0.0, 0.12, 0.0, 0.0);
            for j in 0..5 { let c = mat(L_FLAT, SPRAY[j % SPRAY.len()], 0.2, 0.0).glow(0.03); for _ in 0..7 { k.ellipsoid(c, v3(0.07, 0.11, 0.07), v3(-1.1 + j as f32 * 0.55 + rng.r(-0.12, 0.12), 1.06 + rng.r(0.0, 0.12), rng.r(-0.3, 0.3)), 8, false); } }
            self.add_b(x, z, 1.4, 0.65, rot, 1.0, true);
        }
        for _ in 0..count("crates", 20) {
            let Some((x, z)) = self.place(&mut rng, 1.4, 16.0) else { continue };
            let rot = rng.r(0.0, TAU);
            k.at(x, 0.0, z, rot).bx(CRATE, 2.3, 1.25, 2.3, 0.0, 0.625, 0.0); dk.at(x, 0.0, z, rot);
            decal(&mut dk, at.crate_label, 0.0, 0.62, 1.162, 1.5, 0.75, 0xffffff, 0.9, 0.0, 0.8);
            self.add_b(x, z, 1.15, 1.15, rot, 1.25, true);
            if rng.chance(0.4) { k.bxr(CRATE, 1.5, 1.1, 1.5, 0.0, 1.8, 0.0, 0.0, 0.5, 0.0); self.add_c(x, z, 0.75, 2.35, true); }
        }
        k.origin(); dk.origin();
        for _ in 0..count("mallows", 12) { let (rr, h) = (rng.r(1.0, 1.5), rng.r(1.1, 1.5)); let Some((x, z)) = self.place(&mut rng, rr, 16.0) else { continue }; tank(&mut k, x, 0.0, z, rr, h - rr * 0.22); self.add_c(x, z, rr, h, true); }
        for _ in 0..count("drops", 10) { // sand and rubble waiting for the next floor to be built
            let rr = rng.r(1.6, 3.2);
            let Some((x, z)) = self.place(&mut rng, rr, 16.0) else { continue };
            k.ellipsoid(SAND, v3(rr, rr * 0.55, rr), v3(x, 0.0, z), 20, true);
            for _ in 0..5 { k.bxr(BRICK, 0.5, 0.25, 0.28, x + rng.r(-rr, rr), 0.13, z + rng.r(-rr, rr), 0.0, rng.r(0.0, TAU), 0.0); }
            self.add_c(x, z, rr * 0.9, rr * 0.5, false);
        }
        let mut poles: Vec<(f32, f32)> = Vec::new();
        for _ in 0..count("pops", 14) {
            let Some((x, z)) = self.place(&mut rng, 0.4, 16.0) else { continue };
            k.cyl(POLE, 0.17, 0.14, v3(x, 0.0, z), v3(x, 8.6, z), 8, true).bxr(POLE, 0.14, 0.14, 2.0, x, 8.1, z, 0.0, rng.r(0.0, TAU), 0.0).cyl(POLE, 0.05, 0.05, v3(x, 7.6, z), v3(x + 0.9, 7.8, z), 5, false).bx(LAMP, 0.5, 0.12, 0.3, x + 1.0, 7.74, z);
            self.lamps.push(v3(x + 1.0, 7.4, z));
            self.add_c(x, z, 0.3, 8.6, false); poles.push((x, z));
        }
        for i in 1..poles.len() { let (a, b) = (poles[i - 1], poles[i]); if (a.0 - b.0).hypot(a.1 - b.1) < 55.0 { let (mx, mz) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0); k.cyl(WIRE, 0.018, 0.018, v3(a.0, 8.2, a.1), v3(mx, 7.2, mz), 4, false).cyl(WIRE, 0.018, 0.018, v3(mx, 7.2, mz), v3(b.0, 8.2, b.1), 4, false); } }
        self.arena_mesh = k.m; self.arena_decals = dk.m;
    }
}
