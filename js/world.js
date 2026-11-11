// Renderer, post-processing, lighting palettes, the arena and its collision helpers.
import { THREE, G, TAU, rand, pick, clamp, CANDY, C, settings, glow, std, gummyMat } from './core.js';
import { makeBear } from './models.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { ShaderPass } from 'three/addons/postprocessing/ShaderPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

export const ARENA_R = 56;
export const CAKE = [[7.5, 1.3], [4.6, 2.6], [2.2, 3.9]];
const HI = settings.quality === 'high';

const canvas = document.getElementById('c');
export const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, powerPreference: 'high-performance' });
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping;

export const scene = new THREE.Scene();
scene.fog = new THREE.Fog(0xffcfe6, 50, 190);
export const camera = new THREE.PerspectiveCamera(75, 1, 0.08, 800);
camera.rotation.order = 'YXZ';
export const gunScene = new THREE.Scene();
export const gunCam = new THREE.PerspectiveCamera(50, 1, 0.01, 10);

// ───────────────────────── sky, lights, environment ─────────────────────────
const skyU = { top: { value: new THREE.Color() }, mid: { value: new THREE.Color() }, bot: { value: new THREE.Color() } };
function makeSky() {
  return new THREE.Mesh(new THREE.SphereGeometry(500, 32, 16), new THREE.ShaderMaterial({
    side: THREE.BackSide, depthWrite: false, fog: false, uniforms: skyU,
    vertexShader: `varying vec3 vP; void main(){ vP = position; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
    fragmentShader: `uniform vec3 top; uniform vec3 mid; uniform vec3 bot; varying vec3 vP;
      void main(){ float h = normalize(vP).y;
        vec3 c = h > 0.0 ? mix(mid, top, pow(h, 0.55)) : mix(mid, bot, min(1.0, -h * 3.0));
        gl_FragColor = vec4(c, 1.0);
        #include <colorspace_fragment>
      }`,
  }));
}
const sky = makeSky(); scene.add(sky);
const sunDir = new THREE.Vector3(40, 70, 28).normalize();
const sunDisc = new THREE.Mesh(G.sphere, glow(6, 5, 3, { fog: false })); sunDisc.scale.setScalar(16); scene.add(sunDisc);

const hemi = new THREE.HemisphereLight(0xffffff, 0xff9ecb, 0.7); scene.add(hemi);
const sun = new THREE.DirectionalLight(0xfff1d0, 2.8);
sun.castShadow = true;
sun.shadow.mapSize.setScalar(HI ? 4096 : 2048);
Object.assign(sun.shadow.camera, { left: -64, right: 64, top: 64, bottom: -64, near: 1, far: 260 });
sun.shadow.bias = -0.0005; sun.shadow.normalBias = 0.04;
scene.add(sun);
gunScene.add(new THREE.HemisphereLight(0xffffff, 0xffb0d4, 1.5));
const gunSun = new THREE.DirectionalLight(0xffffff, 3.2); gunSun.position.set(1, 2, 1.5); gunScene.add(gunSun);
const gunRim = new THREE.DirectionalLight(0xbfd8ff, 1.6); gunRim.position.set(-2, 1, -1.5); gunScene.add(gunRim);

const pmrem = new THREE.PMREMGenerator(renderer);
const envScene = new THREE.Scene(), envSun = new THREE.Mesh(G.sphere, glow(30, 26, 18)), envFloor = new THREE.Mesh(new THREE.CircleGeometry(480, 24), new THREE.MeshBasicMaterial({ color: 0xf3a0c8 }));
envSun.scale.setScalar(34); envFloor.rotation.x = -Math.PI / 2; envFloor.position.y = -6; envScene.add(makeSky(), envSun, envFloor);
let envRT = null;

const glowMats = []; // props that light up at night
const PALETTES = {
  morning: { top: 0x6fcbff, mid: 0xffd1ea, bot: 0xfff0b8, fog: 0xffcfe6, near: 50, far: 190, sun: 0xfff1d0, sunI: 2.3, dir: [40, 70, 28], sky: 0xfff6ff, ground: 0xff9ecb, hemiI: 0.32, exp: 0.86, disc: [6, 5, 3], glow: 0, bloom: 0.38, dust: [2.2, 2, 2.4], floor: 0xf3a0c8 },
  sunset: { top: 0x4b3fc4, mid: 0xff7f66, bot: 0xffd27a, fog: 0xff9480, near: 40, far: 170, sun: 0xff9a55, sunI: 2.8, dir: [70, 26, -30], sky: 0xffb8c8, ground: 0xb85a9a, hemiI: 0.3, exp: 0.9, disc: [9, 4, 1.2], glow: 0.35, bloom: 0.5, dust: [3, 1.8, 1.2], floor: 0xd9788f },
  overcast: { top: 0x54688c, mid: 0xa493b4, bot: 0xd0b0c2, fog: 0xa493b4, near: 26, far: 125, sun: 0xdfe6ff, sunI: 1.5, dir: [-30, 80, 40], sky: 0xdde4ff, ground: 0x9a7aa8, hemiI: 0.6, exp: 0.92, disc: [1.2, 1.2, 1.4], glow: 0.6, bloom: 0.45, dust: [1.6, 1.7, 2.2], floor: 0xb88aa8 },
  night: { top: 0x070720, mid: 0x34186a, bot: 0x6a2a80, fog: 0x2b1552, near: 32, far: 150, sun: 0x9fb4ff, sunI: 1.25, dir: [-45, 60, -35], sky: 0x7a7ae0, ground: 0x6a2a8a, hemiI: 0.45, exp: 1.15, disc: [3.5, 4, 6], glow: 1.5, bloom: 0.75, dust: [2.4, 1.4, 3.4], floor: 0x6a3a8a },
};
export function setPalette(name) {
  const p = PALETTES[name] || PALETTES.morning;
  skyU.top.value.setHex(p.top); skyU.mid.value.setHex(p.mid); skyU.bot.value.setHex(p.bot);
  scene.fog.color.setHex(p.fog); scene.fog.near = p.near; scene.fog.far = p.far;
  sunDir.set(...p.dir).normalize(); sun.position.copy(sunDir).multiplyScalar(110); sun.color.setHex(p.sun); sun.intensity = p.sunI;
  hemi.color.setHex(p.sky); hemi.groundColor.setHex(p.ground); hemi.intensity = p.hemiI;
  renderer.toneMappingExposure = p.exp; sunDisc.material.color.setRGB(...p.disc); bloom.strength = p.bloom;
  dust.material.color.setRGB(...p.dust);
  for (const m of glowMats) m.emissiveIntensity = p.glow * (m.userData.glowK || 1);
  envSun.position.copy(sunDir).multiplyScalar(300); envSun.material.color.setRGB(p.disc[0] * 3, p.disc[1] * 3, p.disc[2] * 3); envFloor.material.color.setHex(p.floor);
  if (envRT) envRT.dispose();
  envRT = pmrem.fromScene(envScene, 0.02, 0.1, 1000);
  scene.environment = gunScene.environment = envRT.texture;
}

// ───────────────────────── textures ─────────────────────────
const rgb = hex => [(hex >> 16) & 255, (hex >> 8) & 255, hex & 255];
function canvasTex(w, h, draw, repeat) {
  const cv = document.createElement('canvas'); cv.width = w; cv.height = h; draw(cv.getContext('2d'), w, h);
  const t = new THREE.CanvasTexture(cv); t.colorSpace = THREE.SRGBColorSpace; t.anisotropy = 8;
  if (repeat) { t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(repeat[0], repeat[1]); }
  return t;
}
const swirlTex = (a, b) => canvasTex(256, 256, (x, s) => {
  const img = x.createImageData(s, s), ca = rgb(a), cb = rgb(b);
  for (let py = 0; py < s; py++) for (let px = 0; px < s; px++) {
    const dx = px - s / 2, dy = py - s / 2, r = Math.hypot(dx, dy) / (s / 2), c = Math.sin(Math.atan2(dy, dx) * 4 + r * 9) > 0 ? ca : cb, i = (py * s + px) * 4;
    img.data[i] = c[0]; img.data[i + 1] = c[1]; img.data[i + 2] = c[2]; img.data[i + 3] = 255;
  }
  x.putImageData(img, 0, 0);
});
const stripeTex = () => canvasTex(64, 64, (x, s) => {
  x.fillStyle = '#fff'; x.fillRect(0, 0, s, s); x.fillStyle = '#ff2d4f';
  for (let py = 0; py < s; py++) for (let px = 0; px < s; px++) if ((px + py) % 32 < 14) x.fillRect(px, py, 1, 1);
}, [2, 6]);
const groundTex = () => canvasTex(1024, 1024, (x, s) => {
  x.fillStyle = '#f3a0c8'; x.fillRect(0, 0, s, s);
  for (let i = 0; i < 130; i++) {
    const px = rand(0, s), py = rand(0, s), r = rand(50, 160), col = Math.random() < 0.5 ? '255,220,238' : '222,112,168';
    for (const ox of [-s, 0, s]) for (const oy of [-s, 0, s]) {
      const g = x.createRadialGradient(px + ox, py + oy, 0, px + ox, py + oy, r); g.addColorStop(0, `rgba(${col},0.2)`); g.addColorStop(1, `rgba(${col},0)`);
      x.fillStyle = g; x.fillRect(px + ox - r, py + oy - r, r * 2, r * 2);
    }
  }
  for (let i = 0; i < 6000; i++) { x.fillStyle = `rgba(255,255,255,${rand(0.12, 0.45)})`; x.fillRect(rand(0, s), rand(0, s), 2, 2); }
}, [15, 15]);
const chocTex = () => canvasTex(256, 128, (x, w, h) => {
  x.fillStyle = '#4a2817'; x.fillRect(0, 0, w, h);
  for (let c = 0; c < 4; c++) for (let r = 0; r < 2; r++) {
    const px = c * 64 + 5, py = r * 64 + 5;
    x.fillStyle = '#7a4a30'; x.fillRect(px, py, 54, 54); x.fillStyle = '#3a1d10'; x.fillRect(px + 5, py + 5, 49, 49); x.fillStyle = '#633823'; x.fillRect(px + 5, py + 5, 44, 44);
  }
});
const waferTex = () => canvasTex(128, 128, (x, s) => {
  x.fillStyle = '#eac27c'; x.fillRect(0, 0, s, s); x.strokeStyle = '#c9984a'; x.lineWidth = 5;
  for (let i = 0; i <= s; i += 32) { x.beginPath(); x.moveTo(i, 0); x.lineTo(i, s); x.moveTo(0, i); x.lineTo(s, i); x.stroke(); }
  x.strokeStyle = '#f6dca6'; x.lineWidth = 2; for (let i = 3; i <= s; i += 32) { x.beginPath(); x.moveTo(i, 0); x.lineTo(i, s); x.moveTo(0, i); x.lineTo(s, i); x.stroke(); }
});
const softDot = () => canvasTex(32, 32, x => { const g = x.createRadialGradient(16, 16, 0, 16, 16, 16); g.addColorStop(0, '#fff'); g.addColorStop(1, 'rgba(255,255,255,0)'); x.fillStyle = g; x.fillRect(0, 0, 32, 32); });

// ───────────────────────── collision ─────────────────────────
// Circles or rotated boxes with a height. `top` means you can stand on it.
export const colliders = [];
const addC = (x, z, r, h, top = false) => colliders.push({ kind: 'c', x, z, r, br: r, h, top });
const addB = (x, z, hx, hz, rot, h, top = false) => colliders.push({ kind: 'b', x, z, hx, hz, cos: Math.cos(rot), sin: Math.sin(rot), br: Math.hypot(hx, hz), h, top });

export function pushOut(p, rad, feetY) {
  for (const c of colliders) {
    if (feetY >= c.h - 0.4) continue;
    const wx = p.x - c.x, wz = p.z - c.z;
    if (c.kind === 'c') {
      const min = c.r + rad, d2 = wx * wx + wz * wz;
      if (d2 < min * min) { const d = Math.sqrt(d2) || 1e-4; p.x = c.x + (wx / d) * min; p.z = c.z + (wz / d) * min; }
    } else {
      if (wx * wx + wz * wz > (c.br + rad) * (c.br + rad)) continue;
      const lx = wx * c.cos - wz * c.sin, lz = wx * c.sin + wz * c.cos;
      const qx = clamp(lx, -c.hx, c.hx), qz = clamp(lz, -c.hz, c.hz), ox = lx - qx, oz = lz - qz, d2 = ox * ox + oz * oz;
      let nx, nz;
      if (d2 > 1e-10) { if (d2 >= rad * rad) continue; const d = Math.sqrt(d2); nx = qx + (ox / d) * rad; nz = qz + (oz / d) * rad; }
      else if (c.hx - Math.abs(lx) < c.hz - Math.abs(lz)) { nx = (lx < 0 ? -1 : 1) * (c.hx + rad); nz = lz; }
      else { nx = lx; nz = (lz < 0 ? -1 : 1) * (c.hz + rad); }
      p.x = c.x + nx * c.cos + nz * c.sin; p.z = c.z - nx * c.sin + nz * c.cos;
    }
  }
}
export function floorAt(x, z, feetY) {
  let f = 0;
  for (const c of colliders) {
    if (!c.top || c.h <= f || feetY < c.h - 0.4) continue;
    const wx = x - c.x, wz = z - c.z;
    if (c.kind === 'c') { if (wx * wx + wz * wz < c.r * c.r) f = c.h; }
    else { const lx = wx * c.cos - wz * c.sin, lz = wx * c.sin + wz * c.cos; if (Math.abs(lx) < c.hx && Math.abs(lz) < c.hz) f = c.h; }
  }
  return f;
}
// First point (0..1) where segment a→b enters solid geometry, or -1. Used for bullets and line of sight.
export function segBlocked(a, b) {
  const dx = b.x - a.x, dz = b.z - a.z, dy = b.y - a.y, len2 = dx * dx + dz * dz;
  if (len2 < 1e-9) return -1;
  let best = -1;
  for (const c of colliders) {
    const fx = a.x - c.x, fz = a.z - c.z;
    let tc = -(fx * dx + fz * dz) / len2; tc = tc < 0 ? 0 : tc > 1 ? 1 : tc;
    const cx = fx + dx * tc, cz = fz + dz * tc;
    if (cx * cx + cz * cz > c.br * c.br) continue;
    let te, tx;
    if (c.kind === 'c') {
      const B = 2 * (fx * dx + fz * dz), disc = B * B - 4 * len2 * (fx * fx + fz * fz - c.r * c.r);
      if (disc < 0) continue;
      const sq = Math.sqrt(disc); te = (-B - sq) / (2 * len2); tx = (-B + sq) / (2 * len2);
    } else {
      const ox = fx * c.cos - fz * c.sin, oz = fx * c.sin + fz * c.cos, ex = dx * c.cos - dz * c.sin, ez = dx * c.sin + dz * c.cos;
      te = -Infinity; tx = Infinity;
      if (Math.abs(ex) < 1e-9) { if (Math.abs(ox) > c.hx) continue; }
      else { const t1 = (-c.hx - ox) / ex, t2 = (c.hx - ox) / ex; te = Math.max(te, Math.min(t1, t2)); tx = Math.min(tx, Math.max(t1, t2)); }
      if (Math.abs(ez) < 1e-9) { if (Math.abs(oz) > c.hz) continue; }
      else { const t1 = (-c.hz - oz) / ez, t2 = (c.hz - oz) / ez; te = Math.max(te, Math.min(t1, t2)); tx = Math.min(tx, Math.max(t1, t2)); }
      if (te > tx) continue;
    }
    if (tx < 0 || te > 1) continue;
    te = Math.max(te, 0); tx = Math.min(tx, 1);
    const y0 = a.y + dy * te, y1 = a.y + dy * tx;
    let t;
    if (y0 < c.h) t = te; else if (y1 < c.h) t = te + ((tx - te) * (y0 - c.h)) / (y0 - y1); else continue;
    if (best < 0 || t < best) best = t;
  }
  return best;
}

// ───────────────────────── static world ─────────────────────────
const lit = (color, rough, k = 1) => { const m = std(color, rough, 0, { emissive: color, emissiveIntensity: 0 }); m.userData.glowK = k; glowMats.push(m); return m; };
{
  const ground = new THREE.Mesh(new THREE.CircleGeometry(270, 72), std(0xc9a3b8, 0.62, 0, { map: groundTex() }));
  ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);

  const N = 2800, sprinkles = new THREE.InstancedMesh(new THREE.CapsuleGeometry(0.055, 0.24, 2, 6), std(0xffffff, 0.35), N);
  const m = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), col = new THREE.Color(), one = new THREE.Vector3(1, 1, 1), v = new THREE.Vector3();
  for (let i = 0; i < N; i++) {
    const a = rand(0, TAU), d = 7.8 + Math.sqrt(Math.random()) * (ARENA_R + 8);
    e.set(0, rand(0, TAU), Math.PI / 2, 'YXZ'); q.setFromEuler(e); m.compose(v.set(Math.cos(a) * d, 0.05, Math.sin(a) * d), q, one);
    sprinkles.setMatrixAt(i, m); sprinkles.setColorAt(i, col.setHex(pick([...CANDY, 0xffffff, 0xffffff])));
  }
  sprinkles.receiveShadow = true; scene.add(sprinkles);

  const curb = new THREE.Mesh(new THREE.TorusGeometry(ARENA_R, 0.45, 10, 120), lit(0xff3b6b, 0.3, 0.8));
  curb.rotation.x = Math.PI / 2; curb.position.y = 0.25; curb.receiveShadow = true; scene.add(curb);
  const caneMat = std(0xffffff, 0.28, 0, { map: stripeTex() });
  const stickGeo = new THREE.CylinderGeometry(0.36, 0.36, 6, 14), hookGeo = new THREE.TorusGeometry(0.9, 0.36, 12, 20, Math.PI);
  for (let i = 0; i < 26; i++) {
    const a = (i / 26) * TAU, cane = new THREE.Group(), stick = new THREE.Mesh(stickGeo, caneMat), hook = new THREE.Mesh(hookGeo, caneMat);
    stick.position.y = 3; hook.position.set(0.9, 6, 0); stick.castShadow = hook.castShadow = true; cane.add(stick, hook);
    cane.position.set(Math.cos(a) * ARENA_R, 0, Math.sin(a) * ARENA_R); cane.rotation.y = -a + Math.PI; scene.add(cane);
  }

  // the cake: three climbable tiers and a glowing cherry
  const tierCols = [0xf3dfe8, 0xf58fbd, 0xf3dfe8], drip = [0xf06aa6, 0xf6eef2, 0xf06aa6]; let y0 = 0;
  CAKE.forEach(([r, h], i) => {
    const t = new THREE.Mesh(new THREE.CylinderGeometry(r, r, h - y0, 48), std(tierCols[i], 0.7)); t.position.y = (h + y0) / 2; t.castShadow = t.receiveShadow = true;
    const d = new THREE.Mesh(new THREE.TorusGeometry(r - 0.05, 0.3, 10, 64), std(drip[i], 0.35)); d.rotation.x = Math.PI / 2; d.position.y = h - 0.12; d.castShadow = true;
    scene.add(t, d);
    for (let k = 0; k < Math.round(r * 2.4); k++) { const a = (k / Math.round(r * 2.4)) * TAU, b = new THREE.Mesh(G.sphere, std(drip[i], 0.35)); b.scale.set(0.3, 0.42 + (k % 3) * 0.14, 0.3); b.position.set(Math.cos(a) * (r + 0.02), h - 0.42, Math.sin(a) * (r + 0.02)); scene.add(b); }
    y0 = h;
  });
  const cherry = new THREE.Mesh(G.sphere, lit(0xff1f44, 0.12, 1.4)); cherry.scale.setScalar(0.9); cherry.position.y = 4.7; cherry.castShadow = true;
  const stem = new THREE.Mesh(new THREE.CylinderGeometry(0.05, 0.05, 1, 6), std(0x4a7a2a, 0.6)); stem.position.set(0.15, 5.9, 0); stem.rotation.z = -0.3;
  scene.add(cherry, stem);

  for (let i = 0; i < 22; i++) {
    const a = (i / 22) * TAU + rand(-0.1, 0.1), d = rand(88, 155), r = rand(12, 30), hill = new THREE.Mesh(G.dome, std(pick(CANDY), 0.85));
    hill.scale.set(r, r * rand(0.8, 1.5), r); hill.position.set(Math.cos(a) * d, -1, Math.sin(a) * d); scene.add(hill);
  }
}
const clouds = new THREE.Group(); scene.add(clouds);
{
  const mats = [0xffffff, 0xffc4e4, 0xc9ecff].map(c => std(c, 1, 0, { emissive: c, emissiveIntensity: 0.18, fog: false }));
  for (let i = 0; i < 16; i++) {
    const cl = new THREE.Group(), mat = pick(mats), a = rand(0, TAU), d = rand(50, 200);
    for (let j = 0; j < 6; j++) { const puff = new THREE.Mesh(G.lowSphere, mat), s = rand(4, 8); puff.scale.set(s, s * 0.7, s); puff.position.set(rand(-9, 9), rand(-1.5, 1.5), rand(-4, 4)); cl.add(puff); }
    cl.position.set(Math.cos(a) * d, rand(42, 85), Math.sin(a) * d); clouds.add(cl);
  }
}
const dust = (() => {
  const n = 900, pos = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) { const a = rand(0, TAU), d = Math.sqrt(Math.random()) * 62; pos[i * 3] = Math.cos(a) * d; pos[i * 3 + 1] = rand(0.4, 16); pos[i * 3 + 2] = Math.sin(a) * d; }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  const p = new THREE.Points(g, new THREE.PointsMaterial({ size: 0.16, map: softDot(), transparent: true, opacity: 0.75, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false, fog: false }));
  scene.add(p); return p;
})();

// ───────────────────────── per-mission arena layout (seeded) ─────────────────────────
const arena = new THREE.Group(); scene.add(arena);
let tmp = [], rng = Math.random;
const sr = (a, b) => a + rng() * (b - a), spick = a => a[(rng() * a.length) | 0];
function mulberry32(a) { return () => { a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
function place(r, minR = 13, maxR = ARENA_R - 7) {
  for (let i = 0; i < 90; i++) {
    const a = sr(0, TAU), d = Math.sqrt(sr(minR * minR, maxR * maxR)), x = Math.cos(a) * d, z = Math.sin(a) * d;
    if (Math.hypot(x, z - 30) < r + 5) continue; // keep the player spawn clear
    if (colliders.every(c => Math.hypot(c.x - x, c.z - z) > c.br + r + 4.2)) return { x, z };
  }
  return null;
}
const matCache = new Map();
const cmat = (color, rough) => { const k = color + ':' + rough; if (!matCache.has(k)) matCache.set(k, std(color, rough)); return matCache.get(k); };
const chocMat = std(0xffffff, 0.38, 0, { map: chocTex() }), waferMat = std(0xffffff, 0.8, 0, { map: waferTex() }), mallowMat = std(0xefe6ea, 0.95), stickMat = std(0xffffff, 0.5), rimMat = std(0xffffff, 0.25);
const swirlMats = [[0xff3b6b, 0xffffff], [0x35d6c8, 0xfff39a], [0x9b4dff, 0xffb3e0], [0xff8a1f, 0xfff39a]].map(([a, b]) => { const t = swirlTex(a, b), m = std(0xffffff, 0.2, 0, { map: t, emissive: 0xffffff, emissiveMap: t, emissiveIntensity: 0 }); glowMats.push(m); return m; });
const shadowed = m => { m.castShadow = m.receiveShadow = true; return m; };
const own = g => { tmp.push(g); return g; };

export function buildArena(seed = 1, L = {}) {
  for (const o of tmp) o.dispose();
  tmp = []; arena.clear(); colliders.length = 0; rng = mulberry32(seed * 7919 + 13);
  for (const [r, h] of CAKE) addC(0, 0, r, h, true);
  addC(0, 0, 0.95, 5.6);
  const n = { statues: 2, walls: 7, crates: 6, mallows: 5, drops: 7, pops: 10, ...L };

  for (let i = 0; i < n.statues; i++) {
    const p = place(2.7, 18); if (!p) continue;
    const b = makeBear(spick([C.red, C.green, C.yellow, C.orange]), ['crown']); tmp.push(b.mat);
    b.root.scale.setScalar(5); b.root.position.set(p.x, 0, p.z); b.root.rotation.y = Math.atan2(-p.x, -p.z); arena.add(b.root); addC(p.x, p.z, 2.7, 11);
  }
  for (let i = 0; i < n.walls; i++) {
    const hx = sr(2.2, 3.4), p = place(hx); if (!p) continue;
    const rot = Math.atan2(p.x, p.z) + Math.PI / 2 + sr(-0.5, 0.5), w = shadowed(new THREE.Mesh(G.box, chocMat));
    w.scale.set(hx * 2, 2.3, 0.9); w.position.set(p.x, 1.15, p.z); w.rotation.y = rot; arena.add(w); addB(p.x, p.z, hx, 0.45, rot, 2.3, true);
  }
  for (let i = 0; i < n.crates; i++) {
    const p = place(1.4); if (!p) continue;
    const rot = sr(0, TAU), c = shadowed(new THREE.Mesh(G.box, waferMat)); c.scale.set(2.3, 1.25, 2.3); c.position.set(p.x, 0.625, p.z); c.rotation.y = rot; arena.add(c); addB(p.x, p.z, 1.15, 1.15, rot, 1.25, true);
    if (rng() < 0.4) { const c2 = shadowed(new THREE.Mesh(G.box, waferMat)); c2.scale.set(1.5, 1.1, 1.5); c2.position.set(p.x, 1.8, p.z); c2.rotation.y = rot + 0.5; arena.add(c2); addC(p.x, p.z, 0.75, 2.35, true); }
  }
  for (let i = 0; i < n.mallows; i++) {
    const r = sr(1.1, 1.6), h = sr(1.1, 1.5), p = place(r); if (!p) continue;
    const c = shadowed(new THREE.Mesh(own(new THREE.CylinderGeometry(r, r, h, 28)), mallowMat)); c.position.set(p.x, h / 2, p.z); arena.add(c); addC(p.x, p.z, r, h, true);
  }
  for (let i = 0; i < n.drops; i++) {
    const r = sr(1.5, 3.2), p = place(r); if (!p) continue;
    const d = shadowed(new THREE.Mesh(G.dome, cmat(spick(CANDY), 0.7))); d.scale.set(r, r * 1.15, r); d.position.set(p.x, 0, p.z); arena.add(d); addC(p.x, p.z, r * 0.93, r * 1.05);
  }
  for (let i = 0; i < n.pops; i++) {
    const p = place(0.4); if (!p) continue;
    const h = sr(5, 8.5), R = sr(1.4, 2.1), face = spick(swirlMats), pop = new THREE.Group();
    const stick = new THREE.Mesh(own(new THREE.CylinderGeometry(0.13, 0.13, h, 8)), stickMat); stick.position.y = h / 2; stick.castShadow = true;
    const head = new THREE.Mesh(own(new THREE.CylinderGeometry(R, R, 0.4, 36)), [rimMat, face, face]); head.rotation.x = Math.PI / 2; head.position.y = h + R * 0.6; head.castShadow = true;
    pop.add(stick, head); pop.position.set(p.x, 0, p.z); pop.rotation.y = sr(0, TAU); arena.add(pop); addC(p.x, p.z, 0.35, h);
  }
}

// ───────────────────────── post-processing ─────────────────────────
export const composer = new EffectComposer(renderer, new THREE.WebGLRenderTarget(1, 1, { type: THREE.HalfFloatType, samples: HI ? 4 : 0 }));
composer.addPass(new RenderPass(scene, camera));
export const gunPass = new RenderPass(gunScene, gunCam); gunPass.clear = false; gunPass.clearDepth = true; composer.addPass(gunPass);
const bloom = new UnrealBloomPass(new THREE.Vector2(1, 1), 0.4, 0.6, 1.15); composer.addPass(bloom);
const grade = new ShaderPass({
  uniforms: { tDiffuse: { value: null }, uSat: { value: 1.15 }, uCA: { value: 0.0015 }, uHurt: { value: 0 }, uRush: { value: 0 }, uVig: { value: 0.32 } },
  vertexShader: `varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
  fragmentShader: `uniform sampler2D tDiffuse; uniform float uSat, uCA, uHurt, uRush, uVig; varying vec2 vUv;
    void main(){
      vec2 d = vUv - 0.5; float r2 = dot(d, d); vec2 off = d * uCA * (0.4 + r2 * 6.0);
      vec3 c = vec3(texture2D(tDiffuse, vUv + off).r, texture2D(tDiffuse, vUv).g, texture2D(tDiffuse, vUv - off).b);
      float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
      c = mix(vec3(l), c, uSat);
      c = max(vec3(0.0), (c - 0.2) * 1.14 + 0.2);
      c = mix(c, c * vec3(1.18, 0.86, 1.3) + vec3(0.03, 0.0, 0.05), uRush * (0.35 + r2 * 1.6));
      c *= 1.0 - uVig * smoothstep(0.12, 0.62, r2);
      c = mix(c, vec3(1.1, 0.05, 0.16) * (0.35 + l), uHurt * smoothstep(0.06, 0.5, r2));
      gl_FragColor = vec4(c, 1.0);
    }`,
});
composer.addPass(grade);
composer.addPass(new OutputPass());
export const fx = grade.uniforms;

let pixelRatio = Math.min(window.devicePixelRatio || 1, HI ? 2 : 1.25);
export const getPixelRatio = () => pixelRatio;
export function setPixelRatio(pr) { pixelRatio = pr; resize(); }
export function resize() {
  const w = innerWidth, h = innerHeight;
  if (!w || !h) return; // hidden tab: keep the last good size
  renderer.setPixelRatio(pixelRatio); renderer.setSize(w, h, false); composer.setPixelRatio(pixelRatio); composer.setSize(w, h);
  camera.aspect = gunCam.aspect = w / h; camera.updateProjectionMatrix(); gunCam.updateProjectionMatrix();
}
export function updateWorld(dt) {
  clouds.rotation.y += dt * 0.006; dust.rotation.y += dt * 0.012;
  sky.position.copy(camera.position); sunDisc.position.copy(camera.position).addScaledVector(sunDir, 430);
}

setPalette('morning');
buildArena(1);
resize();
