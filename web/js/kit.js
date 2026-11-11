// A tiny modelling kit: push primitives, then merge everything that shares a material into one mesh.
import { THREE } from './core.js';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';

const _m = new THREE.Matrix4(), _q = new THREE.Quaternion(), _e = new THREE.Euler(), _v = new THREE.Vector3(), _one = new THREE.Vector3(1, 1, 1), _up = new THREE.Vector3(0, 1, 0), _d = new THREE.Vector3();

export class Kit {
  constructor() { this.map = new Map(); this.base = null; }
  // place everything that follows relative to a point and heading (call with no arguments to reset)
  at(x, y = 0, z = 0, ry = 0) { this.base = x === undefined ? null : new THREE.Matrix4().compose(new THREE.Vector3(x, y, z), new THREE.Quaternion().setFromAxisAngle(_up, ry), _one); return this; }
  push(mat, geo, x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0) {
    if (geo.index) geo = geo.toNonIndexed();
    if (x || y || z || rx || ry || rz) geo.applyMatrix4(_m.compose(_v.set(x, y, z), _q.setFromEuler(_e.set(rx, ry, rz)), _one));
    if (this.base) geo.applyMatrix4(this.base);
    if (!this.map.has(mat)) this.map.set(mat, []);
    this.map.get(mat).push(geo); return this;
  }
  box(mat, w, h, d, x, y, z, rx, ry, rz) { return this.push(mat, new THREE.BoxGeometry(w, h, d), x, y, z, rx, ry, rz); }
  plane(mat, w, h, x, y, z, rx, ry, rz) { return this.push(mat, new THREE.PlaneGeometry(w, h), x, y, z, rx, ry, rz); }
  // cylinder along Z from z0 (radius r) to z1 (radius rFront)
  tube(mat, r, z0, z1, x = 0, y = 0, seg = 14, rFront = r) {
    const lo = Math.min(z0, z1), hi = Math.max(z0, z1), g = new THREE.CylinderGeometry(z1 >= z0 ? rFront : r, z1 >= z0 ? r : rFront, hi - lo, seg);
    g.rotateX(Math.PI / 2); return this.push(mat, g, x, y, (lo + hi) / 2);
  }
  // cylinder (or capsule) between two points
  rod(mat, r, ax, ay, az, bx, by, bz, seg = 10, cap = false) {
    const len = _d.set(bx - ax, by - ay, bz - az).length();
    let g = cap ? new THREE.CapsuleGeometry(r, len, 4, seg) : new THREE.CylinderGeometry(r, r, len, seg);
    if (g.index) g = g.toNonIndexed();
    g.applyMatrix4(_m.compose(_v.set((ax + bx) / 2, (ay + by) / 2, (az + bz) / 2), _q.setFromUnitVectors(_up, _d.normalize()), _one));
    return this.push(mat, g);
  }
  ball(mat, r, x, y, z, sx = 1, sy = 1, sz = 1) { const g = new THREE.SphereGeometry(r, 14, 10); g.scale(sx, sy, sz); return this.push(mat, g, x, y, z); }
  // side profile: points are [z, y] (or [cz, cy, z, y] for a curve), extruded across X to thickness t
  prof(mat, pts, t, x = 0, bevel = 0.004, hole = null) {
    const trace = (s, p) => p.forEach((q, i) => { if (q.length === 4) s.quadraticCurveTo(q[0], q[1], q[2], q[3]); else if (i) s.lineTo(q[0], q[1]); else s.moveTo(q[0], q[1]); });
    const s = new THREE.Shape(); trace(s, pts);
    if (hole) { const h = new THREE.Path(); trace(h, hole); s.holes.push(h); }
    const depth = Math.max(0.001, t - bevel * 2);
    const g = new THREE.ExtrudeGeometry(s, { depth, bevelEnabled: bevel > 0, bevelThickness: bevel, bevelSize: bevel, bevelOffset: -bevel, bevelSegments: 1, curveSegments: 6 });
    g.translate(0, 0, -depth / 2); g.rotateY(-Math.PI / 2); return this.push(mat, g, x, 0, 0);
  }
  rail(mat, z0, z1, y, w = 0.021) {
    this.box(mat, w * 0.8, 0.006, z1 - z0, 0, y, (z0 + z1) / 2);
    for (let z = z0 + 0.005; z < z1; z += 0.01) this.box(mat, w, 0.005, 0.0052, 0, y + 0.0045, z);
    return this;
  }
  parts(scale = 1) {
    const out = [];
    for (const [mat, geos] of this.map) { const g = mergeGeometries(geos); geos.forEach(x => x.dispose()); if (scale !== 1) g.scale(scale, scale, scale); out.push([g, mat]); }
    return out;
  }
  build(scale = 1) { return inst(this.parts(scale)); }
}
export const inst = parts => { const g = new THREE.Group(); for (const [geo, mat] of parts) { const m = new THREE.Mesh(geo, mat); m.castShadow = true; g.add(m); } return g; };
