// Procedural models: the gummy bears and their riot shields. Firearms live in guns.js.
import { THREE, G, M, TAU, gummyMat, glow } from './core.js';

const mesh = (geo, mat, x = 0, y = 0, z = 0, sx = 1, sy = sx, sz = sx) => {
  const m = new THREE.Mesh(geo, mat); m.position.set(x, y, z); m.scale.set(sx, sy, sz); m.castShadow = true; return m;
};
const box = (mat, w, h, d, x = 0, y = 0, z = 0) => mesh(G.box, mat, x, y, z, w, h, d);
const tube = (mat, r, len, x = 0, y = 0, z = 0) => { const m = mesh(G.cyl, mat, x, y, z, r, len, r); m.rotation.x = Math.PI / 2; return m; }; // along Z
const torus = (mat, R, r, x = 0, y = 0, z = 0) => { const m = new THREE.Mesh(new THREE.TorusGeometry(R, r, 10, 28), mat); m.position.set(x, y, z); m.castShadow = true; return m; };


// ───────────────────────── gummy bear ─────────────────────────
// Faces +Z. ~2.05 tall at scale 1. Head and arms are pivots so they can aim — and come off.
export function makeBear(color, gear = []) {
  const mat = gummyMat(color), root = new THREE.Group(), g = new THREE.Group();
  root.add(g);
  g.add(
    mesh(G.sphere, mat, 0, 0.85, 0, 0.55, 0.62, 0.48),      // body
    mesh(G.sphere, mat, 0, 0.8, 0.14, 0.4, 0.42, 0.4),      // belly
    mesh(G.sphere, mat, -0.3, 0.26, 0.2, 0.2, 0.2, 0.32),   // legs
    mesh(G.sphere, mat, 0.3, 0.26, 0.2, 0.2, 0.2, 0.32),
  );
  const head = new THREE.Group(); head.position.set(0, 1.58, 0.02);
  head.add(
    mesh(G.sphere, mat, 0, 0, 0, 0.42, 0.4, 0.4),
    mesh(G.sphere, mat, -0.3, 0.34, -0.02, 0.16, 0.16, 0.11), mesh(G.sphere, mat, 0.3, 0.34, -0.02, 0.16, 0.16, 0.11),
    mesh(G.sphere, mat, 0, -0.08, 0.32, 0.19, 0.15, 0.15),
    mesh(G.sphere, M.dark, -0.16, 0.1, 0.34, 0.05, 0.06, 0.04), mesh(G.sphere, M.dark, 0.16, 0.1, 0.34, 0.05, 0.06, 0.04),
    mesh(G.sphere, M.dark, 0, -0.03, 0.46, 0.055, 0.04, 0.04),
  );
  for (const k of gear) {
    if (k === 'helmet') head.add(mesh(G.dome, M.olive, 0, 0.06, 0, 0.47, 0.43, 0.47), mesh(G.cyl, M.olive, 0, 0.07, 0, 0.5, 0.04, 0.5), box(M.black, 0.04, 0.3, 0.03, 0.4, -0.1, 0.1), box(M.black, 0.04, 0.3, 0.03, -0.4, -0.1, 0.1));
    if (k === 'shades') head.add(box(M.shade, 0.54, 0.12, 0.05, 0, 0.1, 0.37), box(M.shade, 0.03, 0.04, 0.4, 0.27, 0.11, 0.18), box(M.shade, 0.03, 0.04, 0.4, -0.27, 0.11, 0.18));
    if (k === 'beret') { const b = mesh(G.sphere, M.red, 0.07, 0.37, 0, 0.4, 0.1, 0.4); b.rotation.z = -0.22; head.add(b, mesh(G.sphere, M.red, 0.02, 0.47, 0, 0.04)); }
    if (k === 'bandana') { const t = torus(M.red, 0.4, 0.05, 0, 0.19, 0); t.rotation.x = Math.PI / 2; const a = box(M.red, 0.07, 0.03, 0.26, 0.06, 0.14, -0.5), b = box(M.red, 0.07, 0.03, 0.22, -0.06, 0.1, -0.48); a.rotation.x = 0.4; b.rotation.x = 0.7; head.add(t, a, b); }
    if (k === 'crown') {
      head.add(mesh(G.cyl, M.gold, 0, 0.42, 0, 0.28, 0.12, 0.28));
      for (let i = 0; i < 5; i++) { const a = (i / 5) * TAU; head.add(mesh(G.cone, M.gold, Math.cos(a) * 0.22, 0.56, Math.sin(a) * 0.22, 0.07, 0.2, 0.07)); }
      head.add(mesh(G.sphere, glow(3, 0.3, 0.5), 0, 0.5, 0.27, 0.045));
    }
  }
  g.add(head);
  const arm = side => {
    const p = new THREE.Group(); p.position.set(side * 0.48, 1.18, 0.06);
    p.add(mesh(G.sphere, mat, 0, 0, 0.2, 0.16, 0.16, 0.3));
    const hand = new THREE.Group(); hand.position.set(0, 0, 0.42); p.add(hand); p.hand = hand; g.add(p); return p;
  };
  return { root, g, mat, head, armR: arm(-1), armL: arm(1) };
}

let shieldTex;
export function makeShield() {
  if (!shieldTex) {
    const cv = document.createElement('canvas'); cv.width = 256; cv.height = 64; const x = cv.getContext('2d');
    x.fillStyle = '#1b2a4a'; x.fillRect(0, 0, 256, 64); x.fillStyle = '#fff'; x.font = '900 40px system-ui, sans-serif'; x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText('S.W.E.E.T.', 128, 34);
    shieldTex = new THREE.CanvasTexture(cv); shieldTex.colorSpace = THREE.SRGBColorSpace;
  }
  const s = new THREE.Group();
  const label = new THREE.Mesh(new THREE.PlaneGeometry(0.72, 0.18), new THREE.MeshStandardMaterial({ map: shieldTex, roughness: 0.4 })); label.position.set(0, -0.12, 0.036);
  s.add(box(M.navy, 0.95, 1.3, 0.06), box(M.steel, 1.0, 0.05, 0.07, 0, 0.65, 0), box(M.steel, 1.0, 0.05, 0.07, 0, -0.65, 0),
    box(new THREE.MeshPhysicalMaterial({ color: 0x9fd8ff, transparent: true, opacity: 0.45, roughness: 0.05 }), 0.52, 0.16, 0.07, 0, 0.36, 0), label);
  return s;
}
