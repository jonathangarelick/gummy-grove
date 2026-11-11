//! Vertex format and a small modelling kit: push primitives into one merged mesh, material carried per vertex.
use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};
use std::f32::consts::{PI, TAU};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4], // sRGB tint; alpha below 255 = lights up at night
    pub mat: [u8; 4],   // texture layer, roughness (0 = from texture), metallic, emissive
}

/// A surface description stamped onto every vertex of a primitive.
#[derive(Copy, Clone, Debug)]
pub struct Mat {
    pub layer: u8,
    pub color: [u8; 4],
    pub rough: u8,
    pub metal: u8,
    pub emissive: u8,
    pub tile: f32, // texture repeats per metre
}
pub const fn mat(layer: u8, hex: u32, rough: f32, metal: f32) -> Mat {
    Mat { layer, color: [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255], rough: (rough * 255.0) as u8, metal: (metal * 255.0) as u8, emissive: 0, tile: 0.5 }
}
impl Mat {
    pub const fn glow(mut self, e: f32) -> Mat { self.emissive = (e * 255.0) as u8; self }
    pub const fn night(mut self, n: f32) -> Mat { self.color[3] = 255 - (n * 255.0) as u8; self }
    pub const fn tile(mut self, t: f32) -> Mat { self.tile = t; self }
    pub const fn tint(mut self, hex: u32) -> Mat { self.color = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, self.color[3]]; self }
    pub fn shade(mut self, k: f32) -> Mat { for c in 0..3 { self.color[c] = (self.color[c] as f32 * k).clamp(0.0, 255.0) as u8; } self }
}

#[derive(Default, Clone)]
pub struct Mesh {
    pub v: Vec<Vertex>,
    pub i: Vec<u32>,
}
impl Mesh {
    pub fn append(&mut self, o: &Mesh, xf: Mat4) {
        let base = self.v.len() as u32;
        for v in &o.v {
            let mut w = *v;
            w.pos = xf.transform_point3(Vec3::from(v.pos)).to_array();
            w.nrm = xf.transform_vector3(Vec3::from(v.nrm)).normalize_or_zero().to_array();
            self.v.push(w);
        }
        self.i.extend(o.i.iter().map(|i| i + base));
    }
    pub fn scaled(mut self, s: f32) -> Mesh { for v in &mut self.v { for c in 0..3 { v.pos[c] *= s; } } self }
}

pub struct Kit {
    pub m: Mesh,
    pub base: Mat4,
}
fn perp(a: Vec3) -> (Vec3, Vec3) {
    let u = if a.y.abs() > 0.99 { Vec3::X } else { Vec3::Y.cross(a).normalize() };
    (u, a.cross(u))
}
impl Kit {
    pub fn new() -> Self { Kit { m: Mesh::default(), base: Mat4::IDENTITY } }
    /// Place everything that follows relative to a point and heading.
    pub fn at(&mut self, x: f32, y: f32, z: f32, ry: f32) -> &mut Self { self.base = Mat4::from_translation(Vec3::new(x, y, z)) * Mat4::from_rotation_y(ry); self }
    pub fn origin(&mut self) -> &mut Self { self.base = Mat4::IDENTITY; self }
    pub fn push(&mut self, local: Mat4) -> Mat4 { let old = self.base; self.base = old * local; old }
    pub fn pop(&mut self, old: Mat4) { self.base = old; }

    #[inline]
    fn v(&mut self, mt: &Mat, p: Vec3, n: Vec3, uv: Vec2) -> u32 {
        let wp = self.base.transform_point3(p);
        let wn = self.base.transform_vector3(n).normalize_or_zero();
        self.m.v.push(Vertex { pos: wp.to_array(), nrm: wn.to_array(), uv: (uv * mt.tile).to_array(), color: mt.color, mat: [mt.layer, mt.rough, mt.metal, mt.emissive] });
        (self.m.v.len() - 1) as u32
    }
    #[inline]
    fn tri(&mut self, a: u32, b: u32, c: u32) { self.m.i.extend_from_slice(&[a, b, c]); }

    /// Quad with explicit UVs (already in tiles: the material's tile factor is not applied twice by callers that pass atlas UVs).
    pub fn quad4(&mut self, mt: Mat, p: [Vec3; 4], uv: [Vec2; 4]) {
        let n = (p[1] - p[0]).cross(p[3] - p[0]).normalize_or_zero();
        let i = [self.v(&mt, p[0], n, uv[0]), self.v(&mt, p[1], n, uv[1]), self.v(&mt, p[2], n, uv[2]), self.v(&mt, p[3], n, uv[3])];
        self.tri(i[0], i[1], i[2]);
        self.tri(i[0], i[2], i[3]);
    }
    /// Quad a→b→c→d, counter-clockwise seen from the front; UVs in metres.
    pub fn quad(&mut self, mt: Mat, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let (w, h) = ((b - a).length(), (d - a).length());
        self.quad4(mt, [a, b, c, d], [Vec2::ZERO, Vec2::new(w, 0.0), Vec2::new(w, h), Vec2::new(0.0, h)]);
    }
    /// Rectangle in the local XY plane at depth z facing +Z, UVs continuous in wall space.
    pub fn rect_xy(&mut self, mt: Mat, x0: f32, x1: f32, y0: f32, y1: f32, z: f32) {
        self.quad4(mt, [Vec3::new(x0, y0, z), Vec3::new(x1, y0, z), Vec3::new(x1, y1, z), Vec3::new(x0, y1, z)], [Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)]);
    }
    pub fn boxm(&mut self, mt: Mat, sz: Vec3, xf: Mat4) {
        let h = sz * 0.5;
        let old = self.push(xf);
        let faces = [(Vec3::X, Vec3::NEG_Z, Vec3::Y), (Vec3::NEG_X, Vec3::Z, Vec3::Y), (Vec3::Y, Vec3::X, Vec3::NEG_Z), (Vec3::NEG_Y, Vec3::X, Vec3::Z), (Vec3::Z, Vec3::X, Vec3::Y), (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y)];
        for (n, u, v) in faces {
            let (c, hu, hv) = (n * h, u.abs().dot(h), v.abs().dot(h));
            self.quad(mt, c - u * hu - v * hv, c + u * hu - v * hv, c + u * hu + v * hv, c - u * hu + v * hv);
        }
        self.pop(old);
    }
    pub fn bx(&mut self, mt: Mat, w: f32, h: f32, d: f32, x: f32, y: f32, z: f32) -> &mut Self {
        self.boxm(mt, Vec3::new(w, h, d), Mat4::from_translation(Vec3::new(x, y, z)));
        self
    }
    /// Box with an XYZ Euler rotation (same order as the web version).
    pub fn bxr(&mut self, mt: Mat, w: f32, h: f32, d: f32, x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32) -> &mut Self {
        self.boxm(mt, Vec3::new(w, h, d), Mat4::from_rotation_translation(Quat::from_euler(EulerRot::XYZ, rx, ry, rz), Vec3::new(x, y, z)));
        self
    }
    /// Cylinder or cone between two points.
    pub fn cyl(&mut self, mt: Mat, r0: f32, r1: f32, a: Vec3, b: Vec3, seg: u32, caps: bool) -> &mut Self {
        let len = (b - a).length().max(1e-6);
        let ax = (b - a) / len;
        let (u, w) = perp(ax);
        let start = self.m.v.len() as u32;
        let circ = TAU * (r0 + r1) * 0.5;
        for j in 0..=seg {
            let t = j as f32 / seg as f32 * TAU;
            let rad = u * t.cos() + w * t.sin();
            let n = (rad * len + ax * (r0 - r1)).normalize_or_zero();
            self.v(&mt, a + rad * r0, n, Vec2::new(j as f32 / seg as f32 * circ, 0.0));
            self.v(&mt, b + rad * r1, n, Vec2::new(j as f32 / seg as f32 * circ, len));
        }
        for j in 0..seg { let k = start + j * 2; self.tri(k, k + 2, k + 3); self.tri(k, k + 3, k + 1); }
        if caps {
            for (c, r, n) in [(a, r0, -ax), (b, r1, ax)] {
                if r < 1e-5 { continue; }
                let ci = self.v(&mt, c, n, Vec2::ZERO);
                let s = self.m.v.len() as u32;
                for j in 0..=seg { let t = j as f32 / seg as f32 * TAU; let rad = u * t.cos() + w * t.sin(); self.v(&mt, c + rad * r, n, Vec2::new(t.cos() * r, t.sin() * r)); }
                for j in 0..seg { self.tri(ci, s + j, s + j + 1); }
            }
        }
        self
    }
    /// Ellipsoid (or just its upper half).
    pub fn ellipsoid(&mut self, mt: Mat, r: Vec3, c: Vec3, seg: u32, dome: bool) -> &mut Self {
        let (rings, start) = (if dome { seg / 4 } else { seg / 2 }.max(2), self.m.v.len() as u32);
        let span = if dome { PI * 0.5 } else { PI };
        let rm = r.max_element();
        for i in 0..=rings {
            let th = i as f32 / rings as f32 * span;
            for j in 0..=seg {
                let ph = j as f32 / seg as f32 * TAU;
                let d = Vec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
                self.v(&mt, c + d * r, (d / r).normalize_or_zero(), Vec2::new(j as f32 / seg as f32 * TAU * rm, i as f32 / rings as f32 * span * rm));
            }
        }
        for i in 0..rings { for j in 0..seg { let a = start + i * (seg + 1) + j; let b = a + seg + 1; self.tri(a, a + 1, b + 1); self.tri(a, b + 1, b); } }
        self
    }
    pub fn ball(&mut self, mt: Mat, r: f32, x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32) -> &mut Self { self.ellipsoid(mt, Vec3::new(r * sx, r * sy, r * sz), Vec3::new(x, y, z), 14, false) }
    pub fn capsule(&mut self, mt: Mat, r: f32, a: Vec3, b: Vec3, seg: u32) -> &mut Self {
        let len = (b - a).length().max(1e-6);
        let ax = (b - a) / len;
        let (u, w) = perp(ax);
        let (rings, start) = (6u32, self.m.v.len() as u32);
        let mut rows = 0;
        for half in 0..2 {
            for i in 0..=rings {
                let th = (i as f32 / rings as f32 + half as f32) * PI * 0.5;
                let base = if half == 0 { b } else { a };
                for j in 0..=seg {
                    let ph = j as f32 / seg as f32 * TAU;
                    let d = (u * ph.cos() + w * ph.sin()) * th.sin() + ax * th.cos();
                    self.v(&mt, base + d * r, d, Vec2::new(j as f32 / seg as f32 * TAU * r, th * r + if half == 1 { len } else { 0.0 }));
                }
                rows += 1;
            }
        }
        for i in 0..rows - 1 { for j in 0..seg { let a0 = start + i * (seg + 1) + j; let b0 = a0 + seg + 1; self.tri(a0, a0 + 1, b0 + 1); self.tri(a0, b0 + 1, b0); } }
        self
    }
    /// Torus around `c`, lying in the plane perpendicular to `axis`.
    pub fn torus(&mut self, mt: Mat, big: f32, small: f32, c: Vec3, axis: Vec3, seg: u32) -> &mut Self {
        let (u, w) = perp(axis.normalize());
        let (ts, start) = (8u32, self.m.v.len() as u32);
        for i in 0..=seg {
            let a = i as f32 / seg as f32 * TAU;
            let rad = u * a.cos() + w * a.sin();
            for j in 0..=ts { let b = j as f32 / ts as f32 * TAU; let n = rad * b.cos() + axis.normalize() * b.sin(); self.v(&mt, c + rad * big + n * small, n, Vec2::new(a * big, b * small)); }
        }
        for i in 0..seg { for j in 0..ts { let a = start + i * (ts + 1) + j; let b = a + ts + 1; self.tri(a, b, b + 1); self.tri(a, b + 1, a + 1); } }
        self
    }

    // ── helpers named after the web kit, so the gun models port line for line (everything along +Z) ──
    pub fn tube(&mut self, mt: Mat, r: f32, z0: f32, z1: f32, x: f32, y: f32, seg: u32, r_front: f32) -> &mut Self {
        let (ra, rb) = if z1 >= z0 { (r, r_front) } else { (r_front, r) };
        self.cyl(mt, ra, rb, Vec3::new(x, y, z0.min(z1)), Vec3::new(x, y, z0.max(z1)), seg, true)
    }
    pub fn rod(&mut self, mt: Mat, r: f32, a: [f32; 3], b: [f32; 3], seg: u32, cap: bool) -> &mut Self {
        if cap { self.capsule(mt, r, a.into(), b.into(), seg) } else { self.cyl(mt, r, r, a.into(), b.into(), seg, true) }
    }
    pub fn rail(&mut self, mt: Mat, z0: f32, z1: f32, y: f32) -> &mut Self {
        self.bx(mt, 0.017, 0.006, z1 - z0, 0.0, y, (z0 + z1) * 0.5);
        let mut z = z0 + 0.005;
        while z < z1 { self.bx(mt, 0.021, 0.005, 0.0052, 0.0, y + 0.0045, z); z += 0.01; }
        self
    }
    /// Side profile: points are [z, y], or [cz, cy, z, y] for a quadratic curve; extruded across X to thickness `t` with a chamfer.
    pub fn prof(&mut self, mt: Mat, pts: &[&[f32]], t: f32, x: f32, bevel: f32) -> &mut Self {
        let mut p: Vec<Vec2> = Vec::new();
        for q in pts {
            if q.len() == 4 {
                let (p0, c, e) = (*p.last().unwrap(), Vec2::new(q[0], q[1]), Vec2::new(q[2], q[3]));
                for s in 1..=5 { let k = s as f32 / 5.0; p.push(p0 * (1.0 - k) * (1.0 - k) + c * 2.0 * k * (1.0 - k) + e * k * k); }
            } else { p.push(Vec2::new(q[0], q[1])); }
        }
        if p.len() > 2 && (p[0] - *p.last().unwrap()).length() < 1e-5 { p.pop(); }
        let area: f32 = (0..p.len()).map(|i| { let (a, b) = (p[i], p[(i + 1) % p.len()]); a.x * b.y - b.x * a.y }).sum();
        if area < 0.0 { p.reverse(); }
        let n = p.len();
        let en: Vec<Vec2> = (0..n).map(|i| { let e = p[(i + 1) % n] - p[i]; Vec2::new(e.y, -e.x).normalize_or_zero() }).collect(); // outward edge normals
        let b = bevel.min(t * 0.45);
        let inset: Vec<Vec2> = (0..n).map(|i| { let (n0, n1) = (en[(i + n - 1) % n], en[i]); let bis = (n0 + n1).normalize_or_zero(); p[i] - bis * (b / bis.dot(n1).max(0.35)) }).collect();
        let at = |q: Vec2, xx: f32| Vec3::new(x + xx, q.y, q.x);
        let tris = triangulate(&inset);
        for side in [-1.0f32, 1.0] {
            let nrm = Vec3::X * side;
            let s = self.m.v.len() as u32;
            for q in &inset { self.v(&mt, at(*q, side * t * 0.5), nrm, *q); }
            for tr in &tris { self.tri(s + tr[0] as u32, s + tr[1] as u32, s + tr[2] as u32); }
        }
        let h = t * 0.5 - b;
        let mut run = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let (el, nn) = ((p[j] - p[i]).length(), Vec3::new(0.0, en[i].y, en[i].x));
            let s = self.m.v.len() as u32;
            for (q, u) in [(p[i], run), (p[j], run + el)] { self.v(&mt, at(q, -h), nn, Vec2::new(u, 0.0)); self.v(&mt, at(q, h), nn, Vec2::new(u, 2.0 * h)); }
            self.tri(s, s + 2, s + 3); self.tri(s, s + 3, s + 1);
            if b > 1e-5 {
                for side in [-1.0f32, 1.0] {
                    let cn = (nn + Vec3::X * side).normalize();
                    let s = self.m.v.len() as u32;
                    self.v(&mt, at(p[i], side * h), cn, Vec2::new(run, 0.0)); self.v(&mt, at(p[j], side * h), cn, Vec2::new(run + el, 0.0));
                    self.v(&mt, at(inset[j], side * t * 0.5), cn, Vec2::new(run + el, b)); self.v(&mt, at(inset[i], side * t * 0.5), cn, Vec2::new(run, b));
                    self.tri(s, s + 1, s + 2); self.tri(s, s + 2, s + 3);
                }
            }
            run += el;
        }
        self
    }
}

/// Ear clipping for small, simple, counter-clockwise polygons.
fn triangulate(p: &[Vec2]) -> Vec<[usize; 3]> {
    let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
    let inside = |q: Vec2, a: Vec2, b: Vec2, c: Vec2| cross(b - a, q - a) > 0.0 && cross(c - b, q - b) > 0.0 && cross(a - c, q - c) > 0.0;
    let mut idx: Vec<usize> = (0..p.len()).collect();
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < 4096 {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            if cross(p[b] - p[a], p[c] - p[b]) <= 1e-10 { continue; }
            if idx.iter().any(|&j| j != a && j != b && j != c && inside(p[j], p[a], p[b], p[c])) { continue; }
            out.push([a, b, c]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped { idx.remove(0); }
    }
    if idx.len() == 3 { out.push([idx[0], idx[1], idx[2]]); }
    out
}
