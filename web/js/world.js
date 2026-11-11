// Renderer, post-processing, lighting, the favela (with its waterfront) and the collision helpers.
import { THREE, G, TAU, rand, pick, clamp, C, settings, glow, std } from './core.js';
import { makeBear } from './models.js';
import { Kit } from './kit.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { ShaderPass } from 'three/addons/postprocessing/ShaderPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

export const ARENA_R = 90;
export const CAKE = [[6.5, 1.3], [4.2, 2.6], [2.2, 3.9]]; // the stepped rooftops in the middle: [half-size, height]
export const WATER_Y = -0.4;
export const SPAWN = { x: 0, z: ARENA_R - 9 };
const BAY = 0.72; // half-angle of the waterfront arc, centred on +Z
const inBay = (x, z) => Math.abs(Math.atan2(x, z)) < BAY + 0.06;
const HI = settings.quality === 'high';

const canvas = document.getElementById('c');
export const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, powerPreference: 'high-performance' });
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping;

export const scene = new THREE.Scene();
scene.fog = new THREE.Fog(0xc4d2dc, 60, 260);
export const camera = new THREE.PerspectiveCamera(75, 1, 0.08, 1200);
camera.rotation.order = 'YXZ';
export const gunScene = new THREE.Scene();
export const gunCam = new THREE.PerspectiveCamera(50, 1, 0.01, 10);

// ───────────────────────── sky, lights, environment ─────────────────────────
const skyU = { top: { value: new THREE.Color() }, mid: { value: new THREE.Color() }, bot: { value: new THREE.Color() } };
function makeSky() {
  return new THREE.Mesh(new THREE.SphereGeometry(700, 32, 16), new THREE.ShaderMaterial({
    side: THREE.BackSide, depthWrite: false, fog: false, uniforms: skyU,
    vertexShader: `varying vec3 vP; void main(){ vP = position; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
    fragmentShader: `uniform vec3 top; uniform vec3 mid; uniform vec3 bot; varying vec3 vP;
      void main(){ float h = normalize(vP).y;
        vec3 c = h > 0.0 ? mix(mid, top, pow(h, 0.5)) : mix(mid, bot, min(1.0, -h * 4.0));
        gl_FragColor = vec4(c, 1.0);
        #include <colorspace_fragment>
      }`,
  }));
}
const sky = makeSky(); scene.add(sky);
const sunDir = new THREE.Vector3(40, 70, 28).normalize();
const sunDisc = new THREE.Mesh(G.sphere, glow(6, 5, 3, { fog: false })); sunDisc.scale.setScalar(20); scene.add(sunDisc);

const hemi = new THREE.HemisphereLight(0xffffff, 0x8a7a6a, 0.45); scene.add(hemi);
const sun = new THREE.DirectionalLight(0xfff1d0, 3);
sun.castShadow = true;
sun.shadow.mapSize.setScalar(HI ? 4096 : 2048);
Object.assign(sun.shadow.camera, { left: -62, right: 62, top: 62, bottom: -62, near: 1, far: 320 });
sun.shadow.bias = -0.0005; sun.shadow.normalBias = 0.05;
scene.add(sun, sun.target);
const lamps = [[0, 7, 0], [-40, 6.5, 22], [38, 6.5, -32]].map(p => { const l = new THREE.PointLight(0xffb060, 0, 48, 1.8); l.position.set(...p); scene.add(l); return l; });
gunScene.add(new THREE.HemisphereLight(0xffffff, 0x9a8a7a, 1.5));
const gunSun = new THREE.DirectionalLight(0xffffff, 3.2); gunSun.position.set(1, 2, 1.5); gunScene.add(gunSun);
const gunRim = new THREE.DirectionalLight(0xbfd8ff, 1.6); gunRim.position.set(-2, 1, -1.5); gunScene.add(gunRim);

const pmrem = new THREE.PMREMGenerator(renderer);
const envScene = new THREE.Scene(), envSun = new THREE.Mesh(G.sphere, glow(30, 26, 18)), envFloor = new THREE.Mesh(new THREE.CircleGeometry(680, 24), new THREE.MeshBasicMaterial({ color: 0x8d877f }));
envSun.scale.setScalar(34); envFloor.rotation.x = -Math.PI / 2; envFloor.position.y = -6; envScene.add(makeSky(), envSun, envFloor);
let envRT = null, pal = null, under = false;

const glowMats = []; // anything that lights up after dark
const PALETTES = {
  morning: { top: 0x3a7bd5, mid: 0xcfe0ee, bot: 0x8f9aa0, fog: 0xc4d2dc, near: 70, far: 330, sun: 0xfff2dc, sunI: 3.0, dir: [40, 70, 28], sky: 0xcfe2ff, ground: 0x8a7a6a, hemiI: 0.5, exp: 0.8, disc: [7, 6.4, 5], glow: 0, bloom: 0.22, dust: [1.5, 1.4, 1.2], floor: 0x8d877f, lamp: 0, water: 0x2f6f78, uw: 0x1c5a5e },
  sunset: { top: 0x2b3f7c, mid: 0xffa25e, bot: 0x6b5a50, fog: 0xdc9670, near: 45, far: 260, sun: 0xff9a4d, sunI: 3.2, dir: [75, 22, -30], sky: 0xffc9a0, ground: 0x7a5a52, hemiI: 0.7, exp: 1.0, disc: [10, 4.5, 1.4], glow: 0.55, bloom: 0.4, dust: [3, 1.9, 1.1], floor: 0x7a6658, lamp: 10, water: 0x8a5a4a, uw: 0x3a4a4a },
  overcast: { top: 0x6f7c8c, mid: 0xb9c0c6, bot: 0x8a8f92, fog: 0xa9b0b6, near: 24, far: 150, sun: 0xe6ecf5, sunI: 1.3, dir: [-30, 80, 40], sky: 0xe3e9f2, ground: 0x7d7872, hemiI: 1.0, exp: 0.88, disc: [1.3, 1.3, 1.4], glow: 0.4, bloom: 0.28, dust: [1.2, 1.25, 1.35], floor: 0x7f7a74, lamp: 5, water: 0x4c5e62, uw: 0x2c4444 },
  night: { top: 0x04060e, mid: 0x1b2540, bot: 0x12141c, fog: 0x121828, near: 28, far: 170, sun: 0x8fa8ff, sunI: 0.9, dir: [-45, 60, -35], sky: 0x4a5a8a, ground: 0x2a2630, hemiI: 0.5, exp: 1.25, disc: [3, 3.6, 5.5], glow: 1.7, bloom: 0.7, dust: [1.4, 1.2, 0.9], floor: 0x2a2a30, lamp: 80, water: 0x0a1a26, uw: 0x061418 },
};
export function setPalette(name) {
  const p = pal = PALETTES[name] || PALETTES.morning;
  skyU.top.value.setHex(p.top); skyU.mid.value.setHex(p.mid); skyU.bot.value.setHex(p.bot);
  sunDir.set(...p.dir).normalize(); sun.color.setHex(p.sun); sun.intensity = p.sunI;
  hemi.color.setHex(p.sky); hemi.groundColor.setHex(p.ground); hemi.intensity = p.hemiI;
  renderer.toneMappingExposure = p.exp; sunDisc.material.color.setRGB(...p.disc); bloom.strength = p.bloom;
  dust.material.color.setRGB(...p.dust); sea.material.color.setHex(p.water);
  for (const l of lamps) l.intensity = p.lamp;
  for (const m of glowMats) m.emissiveIntensity = p.glow * (m.userData.glowK || 1);
  envSun.position.copy(sunDir).multiplyScalar(300); envSun.material.color.setRGB(p.disc[0] * 3, p.disc[1] * 3, p.disc[2] * 3); envFloor.material.color.setHex(p.floor);
  if (envRT) envRT.dispose();
  envRT = pmrem.fromScene(envScene, 0.02, 0.1, 1500);
  scene.environment = gunScene.environment = envRT.texture;
  under = !under; setUnderwater(!under);
}
// murky fog and a wobble while the camera is below the surface
export function setUnderwater(on) {
  if (on === under || !pal) return; under = on;
  scene.fog.color.setHex(on ? pal.uw : pal.fog); scene.fog.near = on ? 0.5 : pal.near; scene.fog.far = on ? 30 : pal.far; fx.uWater.value = on ? 1 : 0;
}

// ───────────────────────── painted-on detail ─────────────────────────
const hex = c => '#' + c.toString(16).padStart(6, '0');
function tex(w, h, draw, repeat) {
  const cv = document.createElement('canvas'); cv.width = w; cv.height = h; draw(cv.getContext('2d'), w, h);
  const t = new THREE.CanvasTexture(cv); t.colorSpace = THREE.SRGBColorSpace; t.anisotropy = 8;
  if (repeat) { t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(repeat[0], repeat[1]); }
  return t;
}
function speckle(x, w, h, n, a) { for (let i = 0; i < n; i++) { const v = (Math.random() * 255) | 0; x.fillStyle = `rgba(${v},${v},${v},${a})`; x.fillRect(Math.random() * w, Math.random() * h, 1 + Math.random() * 3, 1 + Math.random() * 3); } }
function blotch(x, w, h, n, col, a, r0, r1) {
  for (let i = 0; i < n; i++) { const px = rand(0, w), py = rand(0, h), r = rand(r0, r1), g = x.createRadialGradient(px, py, 0, px, py, r); g.addColorStop(0, `rgba(${col},${a})`); g.addColorStop(1, `rgba(${col},0)`); x.fillStyle = g; x.fillRect(px - r, py - r, r * 2, r * 2); }
}
function streaks(x, w, h, n) {
  for (let i = 0; i < n; i++) { const sx = rand(0, w), sy = rand(0, h * 0.5), len = rand(30, h), g = x.createLinearGradient(0, sy, 0, sy + len); g.addColorStop(0, 'rgba(30,22,16,0.24)'); g.addColorStop(1, 'rgba(30,22,16,0)'); x.fillStyle = g; x.fillRect(sx, sy, rand(3, 10), len); }
}
function cracks(x, w, h, n) {
  x.strokeStyle = 'rgba(25,22,20,0.4)'; x.lineWidth = 1;
  for (let c = 0; c < n; c++) { let px = rand(0, w), py = rand(0, h); x.beginPath(); x.moveTo(px, py); for (let i = 0; i < 14; i++) { px += rand(-30, 30); py += rand(-30, 30); x.lineTo(px, py); } x.stroke(); }
}
function bricks(x, w, h) { // hollow clay block, the colour of every unfinished wall
  x.fillStyle = '#8c8279'; x.fillRect(0, 0, w, h);
  for (let r = 0, y = 0; y < h; r++, y += 16) for (let bx = r % 2 ? -20 : 0; bx < w; bx += 40) {
    const t = rand(-18, 18); x.fillStyle = `rgb(${178 + t},${84 + t * 0.6},${48 + t * 0.4})`; x.fillRect(bx + 1.5, y + 1.5, 37, 13);
    x.fillStyle = 'rgba(60,20,8,0.25)'; x.fillRect(bx + 4, y + 5, 32, 1.5); x.fillRect(bx + 4, y + 10, 32, 1.5);
  }
}
function bearStencil(x, cx, cy, s, col, crown) {
  const parts = [[-0.33, 0.75, 0.22, 0.3], [0.33, 0.75, 0.22, 0.3], [-0.62, 0.05, 0.2, 0.3], [0.62, 0.05, 0.2, 0.3], [0, 0.3, 0.55, 0.62], [-0.3, -0.82, 0.16, 0.16], [0.3, -0.82, 0.16, 0.16], [0, -0.5, 0.42, 0.4]];
  x.save(); x.translate(cx, cy); x.scale(s, s); x.lineWidth = 0.16; x.strokeStyle = '#17110f'; x.fillStyle = col;
  for (const [ex, ey, rx, ry] of parts) { x.beginPath(); x.ellipse(ex, ey, rx, ry, 0, 0, TAU); x.stroke(); }
  for (const [ex, ey, rx, ry] of parts) { x.beginPath(); x.ellipse(ex, ey, rx, ry, 0, 0, TAU); x.fill(); }
  x.fillStyle = '#17110f'; x.fillRect(-0.24, -0.6, 0.14, 0.1); x.fillRect(0.1, -0.6, 0.14, 0.1); x.fillStyle = 'rgba(255,255,255,0.55)'; x.beginPath(); x.ellipse(-0.2, 0.05, 0.1, 0.22, 0.4, 0, TAU); x.fill();
  if (crown) { x.fillStyle = '#ffd23f'; x.beginPath(); x.moveTo(-0.3, -0.92); x.lineTo(-0.3, -1.3); x.lineTo(-0.14, -1.08); x.lineTo(0, -1.36); x.lineTo(0.14, -1.08); x.lineTo(0.3, -1.3); x.lineTo(0.3, -0.92); x.closePath(); x.fill(); x.lineWidth = 0.05; x.stroke(); }
  x.fillStyle = col; for (let i = 0; i < 3; i++) x.fillRect(rand(-0.5, 0.5), 0.9, 0.05, rand(0.2, 0.7)); // drips
  x.restore();
}
function graffiti(x, text, cx, cy, size, col, rot) {
  x.save(); x.translate(cx, cy); x.rotate(rot); x.font = `900 italic ${size}px "Marker Felt", "Chalkboard SE", "Comic Sans MS", Impact, sans-serif`; x.textAlign = 'center'; x.lineJoin = 'round';
  x.strokeStyle = '#16110f'; x.lineWidth = size * 0.22; x.strokeText(text, 0, 0); x.fillStyle = col; x.fillText(text, 0, 0);
  const w = x.measureText(text).width; for (let i = 0; i < 4; i++) x.fillRect(rand(-w / 2, w / 2), 2, 2.5, rand(8, 26));
  x.restore();
}
function sign(x, sx, y, text) {
  x.fillStyle = hex(pick(SPRAY)); x.fillRect(sx, y, 224, 40); x.strokeStyle = 'rgba(0,0,0,0.4)'; x.lineWidth = 3; x.strokeRect(sx, y, 224, 40);
  x.fillStyle = '#fff'; x.font = '900 21px Impact, "Arial Black", sans-serif'; x.textAlign = 'center'; x.lineWidth = 4; x.strokeStyle = '#1c1512'; x.strokeText(text, sx + 112, y + 28, 208); x.fillText(text, sx + 112, y + 28, 208);
}
const PL = [0x2f9c95, 0xd9a441, 0xd4705f, 0x4f8fca, 0x86c5a4, 0xc45f42, 0xe6dfd0, 0x9a82bd, 0xdf82a6, 0x7aa957, 0xf0c24a, 0x5bb0c0];
const SPRAY = [0xff3b6b, 0x35d6c8, 0xffd23f, 0x9b4dff, 0xffffff, 0x3fd45c, 0xff8a1f];
const SIGNS = ['BAR DO GUMMY', 'MERCADINHO JUJUBA', 'DOCERIA DO REI', 'AÇAÍ & BALAS', 'BARBEARIA DO URSO', 'LAN HOUSE GELATINA', 'PADARIA DOCE VIDA', 'BORRACHARIA URSÃO'];
const TAGS = ['VIVA O REI GUMMY', 'GUMMY LIVRE', 'DOCE OU MORTE', 'PAZ, AMOR E JUJUBA', 'URSO NÃO DORME', 'JUJUBA 4EVER', 'DOCE DEMAIS!', 'CUIDADO: URSO BRAVO', 'O AÇÚCAR É NOSSO'];
const litMat = (m, k) => { m.userData.glowK = k; glowMats.push(m); return m; };

// one storey of a house front: plaster or bare block, barred windows, maybe a door, a shop sign or a tag
function facade(i) {
  const W = 512, H = 256, em = document.createElement('canvas'); em.width = W; em.height = H; const ex = em.getContext('2d'); ex.fillStyle = '#000'; ex.fillRect(0, 0, W, H);
  const map = tex(W, H, x => {
    if (i % 4 === 3) bricks(x, W, H);
    else {
      x.fillStyle = hex(PL[i % PL.length]); x.fillRect(0, 0, W, H); blotch(x, W, H, 26, '255,255,255', 0.1, 20, 90); blotch(x, W, H, 26, '40,30,20', 0.13, 20, 90);
      if (Math.random() < 0.5) { x.save(); x.beginPath(); x.rect(rand(0, W - 160), rand(120, 190), rand(80, 160), 80); x.clip(); bricks(x, W, H); x.restore(); }
    }
    x.fillStyle = '#8e8a84'; x.fillRect(0, 0, W, 14); x.fillRect(0, 0, 12, H); x.fillRect(W - 12, 0, 12, H);
    const door = Math.random() < 0.6, slots = door ? [[70, true], [300, false]] : [[90, false], [320, false]];
    for (const [sx, isDoor] of slots) {
      if (isDoor) {
        x.fillStyle = pick(['#3d6f9c', '#4d7a52', '#8a4a32', '#6b6f76']); x.fillRect(sx, 86, 84, 170); x.strokeStyle = 'rgba(0,0,0,0.35)'; x.lineWidth = 3; x.strokeRect(sx + 8, 96, 68, 66); x.strokeRect(sx + 8, 172, 68, 74);
        x.fillStyle = '#c9c4b8'; x.fillRect(sx + 66, 168, 9, 5);
      } else {
        x.fillStyle = '#c9c5bb'; x.fillRect(sx - 6, 72, 122, 96); x.fillStyle = '#20262e'; x.fillRect(sx, 78, 110, 84);
        const g = x.createLinearGradient(sx, 78, sx + 110, 162); g.addColorStop(0, 'rgba(140,170,200,0.35)'); g.addColorStop(0.5, 'rgba(140,170,200,0)'); x.fillStyle = g; x.fillRect(sx, 78, 110, 84);
        if (Math.random() < 0.5) { x.fillStyle = hex(pick(PL)); x.globalAlpha = 0.55; x.fillRect(sx, 78, 50, 84); x.globalAlpha = 1; }
        x.strokeStyle = '#8d8a84'; x.lineWidth = 3; for (let b = 1; b < 5; b++) { x.beginPath(); x.moveTo(sx + b * 22, 78); x.lineTo(sx + b * 22, 162); x.stroke(); }
        x.fillStyle = '#9b978f'; x.fillRect(sx - 10, 166, 130, 7);
        const s = x.createLinearGradient(0, 173, 0, 250); s.addColorStop(0, 'rgba(40,30,20,0.3)'); s.addColorStop(1, 'rgba(40,30,20,0)'); x.fillStyle = s; x.fillRect(sx, 173, 110, 77);
        if (Math.random() < 0.55) { ex.fillStyle = pick(['#ffc56e', '#ffd9a0', '#9fd0ff']); ex.fillRect(sx, 78, 110, 84); }
      }
    }
    const roll = Math.random();
    if (roll < 0.3) sign(x, slots[0][0] - 20, 28, pick(SIGNS)); else if (roll < 0.72) graffiti(x, pick(TAGS), rand(170, 340), rand(205, 236), rand(22, 32), hex(pick(SPRAY)), rand(-0.07, 0.07));
    if (Math.random() < 0.35) bearStencil(x, rand(215, 285), rand(195, 212), rand(24, 34), hex(pick(SPRAY)), Math.random() < 0.3);
    streaks(x, W, H, 12);
    const d = x.createLinearGradient(0, H - 70, 0, H); d.addColorStop(0, 'rgba(60,45,30,0)'); d.addColorStop(1, 'rgba(60,45,30,0.5)'); x.fillStyle = d; x.fillRect(0, H - 70, W, 70);
    speckle(x, W, H, 900, 0.06);
  });
  const emt = new THREE.CanvasTexture(em); emt.colorSpace = THREE.SRGBColorSpace;
  return litMat(std(0xffffff, 0.9, 0, { map, emissive: 0xffffff, emissiveMap: emt, emissiveIntensity: 0 }), 0.9);
}
function wallTex(i) {
  return std(0xffffff, 0.92, 0, { map: tex(512, 192, (x, W, H) => {
    if (i % 2) bricks(x, W, H); else { x.fillStyle = hex(pick([0xb9b4aa, 0xc9bfa6, 0x9fb3b0])); x.fillRect(0, 0, W, H); blotch(x, W, H, 30, '40,30,20', 0.14, 20, 80); x.strokeStyle = 'rgba(0,0,0,0.16)'; x.lineWidth = 2; for (let y = 32; y < H; y += 32) { x.beginPath(); x.moveTo(0, y); x.lineTo(W, y); x.stroke(); } }
    graffiti(x, pick(TAGS), W / 2 + rand(-40, 40), rand(90, 130), rand(34, 46), hex(pick(SPRAY)), rand(-0.06, 0.06));
    bearStencil(x, rand(50, 90), 110, 44, hex(pick(SPRAY)), i === 0); if (Math.random() < 0.6) bearStencil(x, rand(420, 460), 112, 40, hex(pick(SPRAY)));
    streaks(x, W, H, 10); speckle(x, W, H, 700, 0.06);
  }) });
}
const concTex = (base, nCracks, repeat) => tex(512, 512, (x, s) => {
  x.fillStyle = hex(base); x.fillRect(0, 0, s, s); blotch(x, s, s, 60, '255,255,255', 0.07, 30, 120); blotch(x, s, s, 70, '20,16,12', 0.13, 20, 110);
  blotch(x, s, s, 8, '10,8,6', 0.3, 14, 40); cracks(x, s, s, nCracks); speckle(x, s, s, 5000, 0.07);
}, repeat);
const corrTex = () => tex(128, 128, (x, s) => {
  for (let px = 0; px < s; px++) { const v = 128 + Math.sin((px / s) * TAU * 8) * 46; x.fillStyle = `rgb(${v},${v + 2},${v + 5})`; x.fillRect(px, 0, 1, s); }
  blotch(x, s, s, 14, '120,58,24', 0.5, 6, 30); speckle(x, s, s, 500, 0.08);
}, [2, 1]);
const crateTex = () => tex(256, 256, (x, s) => {
  x.fillStyle = '#a5804d'; x.fillRect(0, 0, s, s);
  for (let i = 0; i < 6; i++) { x.fillStyle = `rgba(${rand(60, 110)},${rand(40, 70)},20,0.25)`; x.fillRect(0, i * 43, s, 40); x.fillStyle = 'rgba(30,18,8,0.7)'; x.fillRect(0, i * 43 + 40, s, 3); }
  x.fillStyle = '#7a5a30'; x.fillRect(0, 0, 26, s); x.fillRect(s - 26, 0, 26, s); x.fillRect(0, 0, s, 26); x.fillRect(0, s - 26, s, 26);
  x.fillStyle = 'rgba(20,12,6,0.75)'; x.font = '900 46px Impact, sans-serif'; x.textAlign = 'center'; x.fillText('DOCES', s / 2, s / 2 + 8); x.font = '900 20px Impact, sans-serif'; x.fillText('FRÁGIL · 20 KG', s / 2, s / 2 + 40);
  speckle(x, s, s, 1200, 0.07);
});
const sandTex = () => tex(256, 256, (x, s) => { x.fillStyle = '#b9a47c'; x.fillRect(0, 0, s, s); blotch(x, s, s, 30, '90,70,40', 0.15, 10, 50); speckle(x, s, s, 6000, 0.1); }, [3, 2]);
const winTex = lit => tex(64, 64, (x, s) => {
  x.fillStyle = lit ? '#000' : '#fff'; x.fillRect(0, 0, s, s);
  if (!lit) { x.fillStyle = 'rgba(60,45,30,0.25)'; x.fillRect(0, 44, s, 20); x.fillStyle = '#8e8a84'; x.fillRect(0, 0, s, 5); }
  for (const [wx, wy] of [[8, 14], [26, 14], [46, 14], [8, 38], [46, 38]]) { if (lit && Math.random() < 0.45) continue; x.fillStyle = lit ? pick(['#ffc56e', '#ffd9a0', '#9fd0ff']) : '#20262e'; x.fillRect(wx, wy, 10, 12); }
  if (!lit) { x.fillStyle = '#4a4038'; x.fillRect(27, 40, 10, 24); }
});
const softDot = () => tex(32, 32, x => { const g = x.createRadialGradient(16, 16, 0, 16, 16, 16); g.addColorStop(0, '#fff'); g.addColorStop(1, 'rgba(255,255,255,0)'); x.fillStyle = g; x.fillRect(0, 0, 32, 32); });

// ───────────────────────── materials ─────────────────────────
const FACADES = Array.from({ length: 16 }, (_, i) => facade(i));
const WALLS = Array.from({ length: 5 }, (_, i) => wallTex(i));
const BRICK = std(0xffffff, 0.92, 0, { map: tex(512, 256, (x, w, h) => { bricks(x, w, h); streaks(x, w, h, 10); speckle(x, w, h, 800, 0.06); }) });
const CONC = std(0xffffff, 0.9, 0, { map: concTex(0x9a968f, 5) }), CORR = std(0xffffff, 0.5, 0.55, { map: corrTex() }), CRATE = std(0xffffff, 0.85, 0, { map: crateTex() }), SAND = std(0xffffff, 1, 0, { map: sandTex() });
const RUST = std(0x7a3b1e, 0.8, 0.4), GREY = std(0x8a8d92, 0.5, 0.6), BLUE = std(0x1f5fbf, 0.42), POLE = std(0x77736c, 0.9), WIRE = std(0x0e0e0e, 0.7), TIRE = std(0x141414, 0.9), GLASS = std(0x1a2028, 0.08, 0.7), WOODP = std(0x6b4a2c, 0.85);
const LAMP = litMat(std(0xfff0d0, 0.4, 0, { emissive: 0xffc070, emissiveIntensity: 0 }), 4.5);
const BULBS = SPRAY.map(c => litMat(std(c, 0.3, 0, { emissive: c, emissiveIntensity: 0 }), 3.2));
const CLOTH = PL.slice(0, 6).map(c => std(c, 0.95, 0, { side: THREE.DoubleSide }));
const MURAL = std(0xffffff, 0.9, 0, { map: tex(512, 256, (x, W, H) => {
  x.fillStyle = '#2f9c95'; x.fillRect(0, 0, W, H); blotch(x, W, H, 30, '20,30,30', 0.14, 20, 90);
  bearStencil(x, 130, 138, 92, '#ff3b4e', true); graffiti(x, 'REI GUMMY', 350, 110, 54, '#ffd23f', -0.05); graffiti(x, 'TE VIGIA', 356, 176, 40, '#ffffff', 0.03);
  streaks(x, W, H, 10); speckle(x, W, H, 900, 0.06);
}) });
const matCache = new Map();
const cmat = (color, rough, metal = 0) => { const k = color + ':' + rough + ':' + metal; if (!matCache.has(k)) matCache.set(k, std(color, rough, metal)); return matCache.get(k); };

// ───────────────────────── building blocks ─────────────────────────
function house(k, w, d, h, y, wall, roof, xo = 0, zo = 0) { // four painted walls and a concrete slab, in the kit's current frame
  k.plane(wall, w, h, xo, y + h / 2, zo + d / 2).plane(wall, w, h, xo, y + h / 2, zo - d / 2, 0, Math.PI).plane(wall, d, h, xo + w / 2, y + h / 2, zo, 0, Math.PI / 2).plane(wall, d, h, xo - w / 2, y + h / 2, zo, 0, -Math.PI / 2);
  k.box(roof, w + 0.3, 0.16, d + 0.3, xo, y + h + 0.08, zo);
}
function tank(k, x, y, z, r, h) { // the blue rooftop water tank
  k.rod(BLUE, r, x, y, z, x, y + h, z, 18); for (const t of [0.3, 0.62]) k.rod(BLUE, r * 1.04, x, y + h * t, z, x, y + h * t + 0.06, z, 18);
  k.ball(BLUE, r * 0.98, x, y + h, z, 1, 0.22, 1).rod(BLUE, r * 0.25, x, y + h, z, x, y + h + r * 0.32, z, 10);
}
function roofJunk(k, w, d, y, xo, zo) {
  const cx = xo + (w / 2 - 0.9) * (Math.random() < 0.5 ? -1 : 1), cz = zo + (d / 2 - 0.9) * (Math.random() < 0.5 ? -1 : 1);
  if (Math.random() < 0.6) { k.box(CONC, 1.3, 0.5, 1.3, cx, y + 0.25, cz); tank(k, cx, y + 0.5, cz, 0.55, 0.9); }
  if (Math.random() < 0.55) for (const sx of [-1, 1]) for (const sz of [-1, 1]) k.rod(RUST, 0.03, xo + sx * (w / 2 - 0.2), y, zo + sz * (d / 2 - 0.2), xo + sx * (w / 2 - 0.2) + rand(-0.1, 0.1), y + rand(0.5, 1.2), zo + sz * (d / 2 - 0.2), 5);
  if (Math.random() < 0.4) { k.rod(GREY, 0.03, -cx + 2 * xo, y, -cz + 2 * zo, -cx + 2 * xo, y + 0.7, -cz + 2 * zo, 6).ball(GREY, 0.42, -cx + 2 * xo, y + 0.85, -cz + 2 * zo + 0.1, 1, 1, 0.25); }
  if (Math.random() < 0.35) { // washing line
    const x0 = xo - w / 2 + 0.4, x1 = xo + w / 2 - 0.4, lz = zo + rand(-d / 4, d / 4);
    k.rod(GREY, 0.025, x0, y, lz, x0, y + 1.7, lz, 5).rod(GREY, 0.025, x1, y, lz, x1, y + 1.7, lz, 5).rod(WIRE, 0.008, x0, y + 1.6, lz, x1, y + 1.6, lz, 4);
    for (let px = x0 + 0.5; px < x1 - 0.3; px += rand(0.6, 1)) k.plane(pick(CLOTH), 0.5, rand(0.5, 0.8), px, y + 1.25, lz);
  }
}

// ───────────────────────── collision ─────────────────────────
// Circles or rotated boxes with a height. `top` means you can stand (and climb) on it.
export const colliders = [];
const addC = (x, z, r, h, top = false) => colliders.push({ kind: 'c', x, z, r, br: r, h, top });
const addB = (x, z, hx, hz, rot, h, top = false) => colliders.push({ kind: 'b', x, z, hx, hz, cos: Math.cos(rot), sin: Math.sin(rot), br: Math.hypot(hx, hz), h, top });
let lastHit = null;
export const getLastHit = () => lastHit;

export function pushOut(p, rad, feetY) {
  lastHit = null;
  for (const c of colliders) {
    if (feetY >= c.h - 0.4) continue;
    const wx = p.x - c.x, wz = p.z - c.z;
    if (c.kind === 'c') {
      const min = c.r + rad, d2 = wx * wx + wz * wz;
      if (d2 < min * min) { const d = Math.sqrt(d2) || 1e-4; p.x = c.x + (wx / d) * min; p.z = c.z + (wz / d) * min; if (!lastHit || c.top) lastHit = c; }
    } else {
      if (wx * wx + wz * wz > (c.br + rad) * (c.br + rad)) continue;
      const lx = wx * c.cos - wz * c.sin, lz = wx * c.sin + wz * c.cos;
      const qx = clamp(lx, -c.hx, c.hx), qz = clamp(lz, -c.hz, c.hz), ox = lx - qx, oz = lz - qz, d2 = ox * ox + oz * oz;
      let nx, nz;
      if (d2 > 1e-10) { if (d2 >= rad * rad) continue; const d = Math.sqrt(d2); nx = qx + (ox / d) * rad; nz = qz + (oz / d) * rad; }
      else if (c.hx - Math.abs(lx) < c.hz - Math.abs(lz)) { nx = (lx < 0 ? -1 : 1) * (c.hx + rad); nz = lz; }
      else { nx = lx; nz = (lz < 0 ? -1 : 1) * (c.hz + rad); }
      p.x = c.x + nx * c.cos + nz * c.sin; p.z = c.z - nx * c.sin + nz * c.cos; if (!lastHit || c.top) lastHit = c;
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

// ───────────────────────── the neighbourhood (static) ─────────────────────────
const STEPS = []; // walkable stairs up the central rooftops: {x, z, hx, hz, h, base}
for (let i = 0; i < 3; i++) {
  STEPS.push({ x: 0, z: 6.8 + (2 - i) * 0.6, hx: 1.6, hz: 0.3, h: 0.325 * (i + 1), base: 0 });
  STEPS.push({ x: -(4.5 + (2 - i) * 0.6), z: 0, hx: 0.3, hz: 1.4, h: 1.3 + 0.325 * (i + 1), base: 1.3 });
  STEPS.push({ x: 2.5 + (2 - i) * 0.6, z: 0, hx: 0.3, hz: 1.1, h: 2.6 + 0.325 * (i + 1), base: 2.6 });
}
let sea;
const bobbers = [], kites = [];
{
  const k = new Kit(), R = ARENA_R;
  // ground: the square, the land around it, and the bay to the south
  const gmat = d => std(0xb9b0a4, 0.9, 0, { map: concTex(0x77736b, 9, [d / 11, d / 11]) });
  const disc = new THREE.Mesh(new THREE.CircleGeometry(R + 0.5, 96), gmat(2 * R)); disc.rotation.x = -Math.PI / 2; disc.receiveShadow = true;
  const land = new THREE.Mesh(new THREE.RingGeometry(R + 0.5, 520, 96, 1, Math.PI * 1.5 + BAY, TAU - BAY * 2), gmat(1040)); land.rotation.x = -Math.PI / 2; land.receiveShadow = true;
  sea = new THREE.Mesh(new THREE.PlaneGeometry(2400, 2400), new THREE.MeshStandardMaterial({ color: 0x2f6f78, roughness: 0.06, metalness: 0.25, transparent: true, opacity: 0.9, side: THREE.DoubleSide })); sea.rotation.x = -Math.PI / 2; sea.position.y = WATER_Y;
  const bed = new THREE.Mesh(new THREE.PlaneGeometry(900, 900), SAND); bed.rotation.x = -Math.PI / 2; bed.position.set(0, -5.2, R + 200);
  const quay = new THREE.Mesh(new THREE.CylinderGeometry(R + 0.5, R + 0.5, 5.4, 48, 1, true, -BAY, BAY * 2), CONC); quay.position.y = -2.7;
  scene.add(disc, land, sea, bed, quay);
  for (const [a0, a1] of [[-BAY, -0.05], [0.05, BAY]]) { const w = new THREE.Mesh(new THREE.CylinderGeometry(R, R, 0.9, 32, 1, true, a0, a1 - a0), std(0x9a968f, 0.9, 0, { side: THREE.DoubleSide })); w.position.y = 0.45; w.castShadow = true; scene.add(w); }
  k.box(CONC, 8.4, 0.4, 17, 0, -1.72, R + 8.3, 0.2);                                            // boat ramp
  k.box(WOODP, 3, 0.25, 24, -20, 0.15, R + 11); for (let z = R + 1; z < R + 23; z += 4) k.rod(WOODP, 0.16, -21.3, -5, z, -21.3, 0.6, z, 6).rod(WOODP, 0.16, -18.7, -5, z, -18.7, 0.6, z, 6); // pier
  for (let i = 0; i < 900; i++) { const a = rand(0, TAU), d = Math.sqrt(Math.random()) * (R - 2); k.box(cmat(pick([...SPRAY, 0xd8d2c4, 0xd8d2c4, 0x6a665e]), 0.8), rand(0.12, 0.3), 0.012, rand(0.08, 0.2), Math.cos(a) * d, 0.012, Math.sin(a) * d, 0, rand(0, TAU)); } // litter and candy wrappers
  for (let i = 0; i < 46; i++) { const a = rand(0, TAU), d = rand(10, R - 4), p = new THREE.Mesh(new THREE.CircleGeometry(rand(3, 11), 18), std(pick([0x4f4034, 0x3b3530, 0x5d4f3f]), 1, 0, { transparent: true, opacity: rand(0.25, 0.5), depthWrite: false })); p.rotation.x = -Math.PI / 2; p.scale.y = rand(0.4, 1); p.rotation.z = rand(0, TAU); p.position.set(Math.cos(a) * d, 0.006 + i * 0.0001, Math.sin(a) * d); p.receiveShadow = true; scene.add(p); } // dirt and old asphalt patches
  for (let i = 0; i < 22; i++) { const a = rand(0, TAU), d = rand(12, R - 6), p = new THREE.Mesh(new THREE.CircleGeometry(rand(1, 3.4), 20), std(0x1a1f24, 0.03, 0.4, { transparent: true, opacity: 0.6, depthWrite: false })); p.rotation.x = -Math.PI / 2; p.scale.y = rand(0.5, 1); p.position.set(Math.cos(a) * d, 0.012, Math.sin(a) * d); scene.add(p); } // puddles

  // the ring of stacked houses that walls the square in
  const N = 54, wHouse = (TAU * (R + 5)) / N;
  for (let i = 0; i < N; i++) {
    const a = (i / N) * TAU, cx = Math.cos(a), cz = Math.sin(a);
    if (inBay(cx, cz)) continue;
    const alley = i % 9 === 4, d = rand(7, 10), rc = R + 0.8 + d / 2 + (alley ? 6 : 0), rot = Math.atan2(-cx, -cz);
    k.at(cx * rc, 0, cz * rc, rot);
    if (alley) k.box(CORR, wHouse, 2.4, 0.08, 0, 1.2, d / 2 - 5.4);                             // sheet-metal fence across the alley mouth
    const floors = alley ? 2 : 2 + ((Math.random() * 3) | 0); let w = wHouse * rand(0.96, 1.04), dd = d, xo = 0, zo = 0, y = 0;
    for (let f = 0; f < floors; f++) {
      const h = rand(2.9, 3.3); house(k, w, dd, h, y, pick(FACADES), f === floors - 1 && Math.random() < 0.4 ? CORR : CONC, xo, zo);
      if (f === 0 && !alley && Math.random() < 0.45) k.box(pick(CLOTH), w * 0.5, 0.06, 1.5, xo + rand(-1, 1), 2.55, zo + dd / 2 + 0.65, 0.3); // awning
      if (Math.random() < 0.5) k.box(GREY, 0.8, 0.5, 0.45, xo + rand(-w / 3, w / 3), y + rand(1, 2), zo + dd / 2 + 0.2);              // air conditioner
      y += h + 0.16; if (f === floors - 1) roofJunk(k, w, dd, y, xo, zo);
      const nw = w * rand(0.55, 1), nd = dd * rand(0.6, 1); xo += rand(-(w - nw) / 2, (w - nw) / 2); zo += ((dd - nd) / 2) * rand(-1, 1); w = nw; dd = nd;
    }
  }
  for (let i = 0; i < 40; i++) { // a taller second row behind, for the skyline
    const a = (i / 40) * TAU + 0.05, cx = Math.cos(a), cz = Math.sin(a); if (inBay(cx, cz)) continue;
    const rc = R + rand(17, 27); k.at(cx * rc, 0, cz * rc, Math.atan2(-cx, -cz) + rand(-0.3, 0.3));
    let y = 0, w = rand(8, 13), dd = rand(7, 10); for (let f = 0, n = 3 + ((Math.random() * 4) | 0); f < n; f++) { const h = rand(2.9, 3.3); house(k, w, dd, h, y, pick(FACADES), CONC); y += h + 0.16; w *= rand(0.7, 1); dd *= rand(0.7, 1); if (f === n - 1) roofJunk(k, w, dd, y, 0, 0); }
  }
  // street poles round the edge, sagging wires, party lights strung in to the middle
  const poles = [];
  for (let i = 0; i < 14; i++) { const a = (i / 14) * TAU + 0.2, cx = Math.cos(a), cz = Math.sin(a); if (inBay(cx, cz)) continue; poles.push([cx * (R + 0.3), cz * (R + 0.3)]); }
  k.at();
  poles.forEach(([x, z], i) => {
    k.rod(POLE, 0.17, x, 0, z, x, 9, z, 8).box(POLE, 0.14, 0.14, 2.2, x, 8.4, z, 0, Math.atan2(x, z)).rod(GREY, 0.3, x, 6.6, z + 0.45, x, 7.4, z + 0.45, 10);
    const lx = x * 0.975, lz = z * 0.975; k.rod(POLE, 0.05, x, 7.8, z, lx, 8, lz, 5).box(LAMP, 0.5, 0.12, 0.3, lx, 7.95, lz, 0, Math.atan2(x, z));
    const [nx, nz] = poles[(i + 1) % poles.length];
    if (Math.hypot(nx - x, nz - z) < 60) for (const hgt of [8.3, 8.6]) { const mx = (x + nx) / 2, mz = (z + nz) / 2; k.rod(WIRE, 0.018, x, hgt, z, mx, hgt - 0.9, mz, 4).rod(WIRE, 0.018, mx, hgt - 0.9, mz, nx, hgt, nz, 4); }
    if (i % 2 === 0) for (let j = 1; j < 26; j++) { // festa lights
      const t = j / 26, px = x * (1 - t), pz = z * (1 - t), py = 8.6 * (1 - t) + 5.6 * t - Math.sin(t * Math.PI) * 2.2;
      k.ball(BULBS[j % BULBS.length], 0.11, px, py, pz); if (j < 25) { const t2 = (j + 1) / 26; k.rod(WIRE, 0.012, px, py + 0.1, pz, x * (1 - t2), 8.6 * (1 - t2) + 5.6 * t2 - Math.sin(t2 * Math.PI) * 2.2 + 0.1, z * (1 - t2), 3); }
    }
    for (let j = 0; j < 2; j++) { const t = rand(0.2, 0.8), mx = x + (nx - x) * t, mz = z + (nz - z) * t, c = cmat(pick(SPRAY), 0.25); k.rod(WIRE, 0.008, mx, 7.5, mz, mx, 6.9, mz, 3).ball(c, 0.16, mx - 0.12, 6.8, mz, 0.8, 1.2, 0.8).ball(c, 0.16, mx + 0.14, 6.7, mz, 0.8, 1.2, 0.8); } // gummies slung over the wire
  });

  // the middle of the square: a half-built house you can climb, floor by floor
  const walls = [BRICK, FACADES[0], BRICK]; let y0 = 0;
  CAKE.forEach(([half, h], i) => {
    const s = half * 2; k.plane(i === 1 ? MURAL : walls[i], s, h - y0, 0, (h + y0) / 2, half).plane(walls[i], s, h - y0, 0, (h + y0) / 2, -half, 0, Math.PI).plane(walls[i], s, h - y0, half, (h + y0) / 2, 0, 0, Math.PI / 2).plane(walls[i], s, h - y0, -half, (h + y0) / 2, 0, 0, -Math.PI / 2);
    k.box(CONC, s + 0.3, 0.16, s + 0.3, 0, h - 0.08, 0); y0 = h;
  });
  for (const s of STEPS) k.box(CONC, s.hx * 2, s.h - s.base, s.hz * 2, s.x, (s.h + s.base) / 2, s.z);
  k.box(CONC, 1.6, 0.4, 1.6, 0, 4.1, 0); tank(k, 0, 4.3, 0, 0.8, 1.2);
  for (const sx of [-1, 1]) for (const sz of [-1, 1]) for (const [half, h] of CAKE) k.rod(RUST, 0.035, sx * (half - 0.25), h, sz * (half - 0.25), sx * (half - 0.25) + rand(-0.1, 0.1), h + rand(0.7, 1.4), sz * (half - 0.25), 5);
  const built = k.build(); built.traverse(o => { o.receiveShadow = true; }); scene.add(built);

  // hills of houses on three sides, and the statue everyone can see
  const hills = [], NH = 2200, hm = new THREE.InstancedMesh(G.box, litMat(std(0xffffff, 0.95, 0, { map: winTex(false), emissive: 0xffffff, emissiveMap: winTex(true), emissiveIntensity: 0 }), 1), NH);
  for (let i = 0; i < 7; i++) { const a = BAY + 0.35 + (i / 6) * (TAU - 2 * BAY - 0.7), d = rand(190, 270); hills.push({ x: Math.sin(a) * d, z: Math.cos(a) * d, R: rand(70, 110), H: rand(38, 70) }); }
  const m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), col = new THREE.Color(), v = new THREE.Vector3(), sc = new THREE.Vector3();
  for (const h of hills) { const mound = new THREE.Mesh(G.dome, std(0x4d5a3a, 1)); mound.scale.set(h.R, h.H, h.R); mound.position.set(h.x, -1, h.z); scene.add(mound); }
  for (let i = 0; i < NH; i++) {
    const h = hills[i % hills.length], a = rand(0, TAU), rr = Math.sqrt(Math.random()) * h.R * 0.93, sy = rand(2.6, 3.3) * (Math.random() < 0.35 ? 2 : 1);
    sc.set(rand(3.5, 7), sy, rand(3.5, 7)); e.set(0, rand(0, TAU), 0); q.setFromEuler(e);
    m4.compose(v.set(h.x + Math.cos(a) * rr, h.H * Math.sqrt(1 - (rr / h.R) ** 2) - 1 + sy / 2 - 0.6, h.z + Math.sin(a) * rr), q, sc);
    hm.setMatrixAt(i, m4); hm.setColorAt(i, col.setHex(Math.random() < 0.3 ? 0xb5603a : pick(PL)));
  }
  scene.add(hm);
  const top = hills.reduce((b, h) => (h.z < b.z ? h : b)), landmark = makeBear(C.red);
  landmark.armR.rotation.set(-Math.PI / 2, -0.6435, 0); landmark.armL.rotation.set(-Math.PI / 2, 0.6435, 0); landmark.root.scale.setScalar(13); landmark.root.position.set(top.x, top.H + 3, top.z); landmark.root.rotation.y = Math.atan2(-top.x, -top.z);
  const plinth = new THREE.Mesh(G.box, CONC); plinth.scale.set(12, 8, 12); plinth.position.set(top.x, top.H - 1, top.z); scene.add(landmark.root, plinth);

  // on the water: fishing boats and one very large inflatable
  for (const [bx, bz, c] of [[-26, R + 14, 0x2f6fb0], [22, R + 26, 0xc8553a], [-9, R + 34, 0xe6dfd0]]) {
    const b = new THREE.Group(), hull = new THREE.Mesh(G.sphere, std(c, 0.6)); hull.scale.set(1.3, 0.7, 3.4); hull.position.y = 0.05; const cab = new THREE.Mesh(G.box, std(0xe6dfd0, 0.8)); cab.scale.set(1.4, 1.1, 1.6); cab.position.set(0, 0.9, -0.8);
    b.add(hull, cab); b.position.set(bx, WATER_Y, bz); b.rotation.y = rand(0, TAU); scene.add(b); bobbers.push(b);
  }
  const float = makeBear(C.pink); float.root.scale.setScalar(3.2); float.root.rotation.x = -Math.PI / 2; float.root.position.set(17, WATER_Y + 1.1, R + 15); scene.add(float.root); bobbers.push(float.root);
  for (let i = 0; i < 7; i++) { const kt = new THREE.Mesh(new THREE.PlaneGeometry(1.6, 1.6), new THREE.MeshBasicMaterial({ color: pick(SPRAY), side: THREE.DoubleSide, fog: false })); kt.rotation.z = Math.PI / 4; const a = rand(0, TAU), d = rand(30, 110); kt.position.set(Math.cos(a) * d, rand(34, 70), Math.sin(a) * d); kt.userData.a = rand(0, TAU); scene.add(kt); kites.push(kt); }
}
const clouds = new THREE.Group(); scene.add(clouds);
{
  const mat = std(0xf2f4f6, 1, 0, { emissive: 0xffffff, emissiveIntensity: 0.06, fog: false });
  for (let i = 0; i < 14; i++) {
    const cl = new THREE.Group(), a = rand(0, TAU), d = rand(120, 420);
    for (let j = 0; j < 6; j++) { const puff = new THREE.Mesh(G.lowSphere, mat), s = rand(10, 22); puff.scale.set(s, s * 0.45, s); puff.position.set(rand(-24, 24), rand(-2, 2), rand(-10, 10)); cl.add(puff); }
    cl.position.set(Math.cos(a) * d, rand(110, 190), Math.sin(a) * d); clouds.add(cl);
  }
}
const dust = (() => {
  const n = 1100, pos = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) { const a = rand(0, TAU), d = Math.sqrt(Math.random()) * (ARENA_R + 6); pos[i * 3] = Math.cos(a) * d; pos[i * 3 + 1] = rand(0.4, 16); pos[i * 3 + 2] = Math.sin(a) * d; }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  const p = new THREE.Points(g, new THREE.PointsMaterial({ size: 0.11, map: softDot(), transparent: true, opacity: 0.5, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false, fog: false }));
  scene.add(p); return p;
})();

// ───────────────────────── per-mission street furniture (seeded) ─────────────────────────
const arena = new THREE.Group(); scene.add(arena);
let tmp = [], rng = Math.random;
const sr = (a, b) => a + rng() * (b - a), spick = a => a[(rng() * a.length) | 0];
function mulberry32(a) { return () => { a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
function place(r, minR = 16, maxR = ARENA_R - 8) {
  for (let i = 0; i < 90; i++) {
    const a = sr(0, TAU), d = Math.sqrt(sr(minR * minR, maxR * maxR)), x = Math.cos(a) * d, z = Math.sin(a) * d;
    if (Math.hypot(x - SPAWN.x, z - SPAWN.z) < r + 8) continue;                  // keep the insertion point clear
    if (colliders.every(c => Math.hypot(c.x - x, c.z - z) > c.br + r + 4.2)) return { x, z };
  }
  return null;
}
const CARS = [0xd8d2c4, 0x9c2b22, 0x2d5c8a, 0xc9b06a, 0x3a3d40, 0x5a2a16];

export function buildArena(seed = 1, L = {}) {
  for (const o of tmp) o.dispose();
  tmp = []; arena.clear(); colliders.length = 0; rng = mulberry32(seed * 7919 + 13);
  CAKE.forEach(([half, h]) => addB(0, 0, half, half, 0, h, true));
  for (const s of STEPS) addB(s.x, s.z, s.hx, s.hz, 0, s.h, true);
  addC(0, 0, 0.9, 5.8);
  const n = { statues: 2, walls: 22, crates: 20, mallows: 12, drops: 10, pops: 14, shacks: 22, cars: 10, stalls: 8 };
  for (const key in L) n[key] = Math.round(L[key] * 2.2);
  const k = new Kit();

  for (let i = 0; i < n.shacks; i++) { // one-room houses: hard cover with a roof to fight from
    const w = sr(4.6, 6.4), d = sr(3.8, 5), p = place(Math.hypot(w, d) / 2, 20); if (!p) continue;
    const rot = sr(0, TAU); k.at(p.x, 0, p.z, rot); house(k, w, d, 2.55, 0, spick(FACADES), rng() < 0.5 ? CORR : CONC); roofJunk(k, w, d, 2.71, 0, 0);
    addB(p.x, p.z, w / 2 + 0.1, d / 2 + 0.1, rot, 2.71, true);
  }
  for (let i = 0; i < n.statues; i++) {
    const p = place(2.4, 22); if (!p) continue;
    const b = makeBear(spick([C.red, C.green, C.yellow, C.orange]), ['crown']); tmp.push(b.mat);
    b.root.scale.setScalar(3.6); b.root.position.set(p.x, 1.2, p.z); b.root.rotation.y = Math.atan2(-p.x, -p.z); arena.add(b.root);
    k.at(p.x, 0, p.z, sr(0, TAU)).box(CONC, 3.6, 1.2, 3.6, 0, 0.6, 0); addC(p.x, p.z, 2.3, 8.4);
  }
  for (let i = 0; i < n.walls; i++) {
    const hx = sr(2.2, 3.6), p = place(hx); if (!p) continue;
    const rot = sr(0, TAU); k.at(p.x, 0, p.z, rot).box(spick(WALLS), hx * 2, 2.3, 0.9, 0, 1.15, 0).box(CONC, hx * 2 + 0.1, 0.1, 1, 0, 2.3, 0);
    addB(p.x, p.z, hx, 0.45, rot, 2.35, true);
  }
  for (let i = 0; i < n.cars; i++) {
    const p = place(2.4); if (!p) continue;
    const rot = sr(0, TAU), paint = cmat(spick(CARS), 0.45, 0.3); k.at(p.x, 0, p.z, rot);
    k.box(paint, 4.2, 0.6, 1.75, 0, 0.62, 0).box(paint, 2.2, 0.52, 1.6, -0.25, 1.16, 0).box(GLASS, 2.24, 0.36, 1.62, -0.25, 1.17, 0).box(GLASS, 1.9, 0.38, 1.64, -0.25, 1.17, 0).box(GREY, 0.14, 0.2, 1.8, 2.14, 0.5, 0).box(GREY, 0.14, 0.2, 1.8, -2.14, 0.5, 0);
    for (const wx of [-1.35, 1.35]) k.tube(TIRE, 0.34, -0.92, 0.92, wx, 0.34, 12);
    addB(p.x, p.z, 2.15, 0.9, rot, 1.42, true);
  }
  for (let i = 0; i < n.stalls; i++) { // market stalls, selling exactly what you think
    const p = place(1.8); if (!p) continue;
    const rot = sr(0, TAU), cloth = spick(CLOTH); k.at(p.x, 0, p.z, rot).box(WOODP, 2.8, 0.1, 1.3, 0, 0.95, 0);
    for (const sx of [-1.3, 1.3]) for (const sz of [-0.55, 0.55]) k.rod(WOODP, 0.05, sx, 0, sz, sx, 2.3, sz, 5);
    k.box(cloth, 3.1, 0.05, 1.7, 0, 2.34, 0, 0.12);
    for (let j = 0; j < 5; j++) { const c = cmat(SPRAY[j % SPRAY.length], 0.2); for (let b = 0; b < 7; b++) k.ball(c, 0.09, -1.1 + j * 0.55 + sr(-0.12, 0.12), 1.06 + sr(0, 0.12), sr(-0.3, 0.3), 0.8, 1.2, 0.8); }
    addB(p.x, p.z, 1.4, 0.65, rot, 1.0, true);
  }
  for (let i = 0; i < n.crates; i++) {
    const p = place(1.4); if (!p) continue;
    const rot = sr(0, TAU); k.at(p.x, 0, p.z, rot).box(CRATE, 2.3, 1.25, 2.3, 0, 0.625, 0); addB(p.x, p.z, 1.15, 1.15, rot, 1.25, true);
    if (rng() < 0.4) { k.box(CRATE, 1.5, 1.1, 1.5, 0, 1.8, 0, 0, 0.5); addC(p.x, p.z, 0.75, 2.35, true); }
  }
  k.at();
  for (let i = 0; i < n.mallows; i++) { const r = sr(1.0, 1.5), h = sr(1.1, 1.5), p = place(r); if (!p) continue; tank(k, p.x, 0, p.z, r, h - r * 0.22); addC(p.x, p.z, r, h, true); }
  for (let i = 0; i < n.drops; i++) { // sand and rubble waiting for the next floor to be built
    const r = sr(1.6, 3.2), p = place(r); if (!p) continue;
    const g = G.dome.clone(); g.scale(r, r * 0.55, r); k.push(SAND, g, p.x, 0, p.z); addC(p.x, p.z, r * 0.9, r * 0.5);
    for (let j = 0; j < 5; j++) k.box(BRICK, 0.5, 0.25, 0.28, p.x + sr(-r, r), 0.13, p.z + sr(-r, r), 0, sr(0, TAU));
  }
  const poles = [];
  for (let i = 0; i < n.pops; i++) {
    const p = place(0.4); if (!p) continue;
    k.rod(POLE, 0.17, p.x, 0, p.z, p.x, 8.6, p.z, 8).box(POLE, 0.14, 0.14, 2, p.x, 8.1, p.z, 0, sr(0, TAU)).rod(POLE, 0.05, p.x, 7.6, p.z, p.x + 0.9, 7.8, p.z, 5).box(LAMP, 0.5, 0.12, 0.3, p.x + 1, 7.74, p.z);
    addC(p.x, p.z, 0.3, 8.6); poles.push(p);
  }
  for (let i = 1; i < poles.length; i++) { const a = poles[i - 1], b = poles[i]; if (Math.hypot(a.x - b.x, a.z - b.z) < 55) { const mx = (a.x + b.x) / 2, mz = (a.z + b.z) / 2; k.rod(WIRE, 0.018, a.x, 8.2, a.z, mx, 7.2, mz, 4).rod(WIRE, 0.018, mx, 7.2, mz, b.x, 8.2, b.z, 4); } }
  for (const [geo, mat] of k.parts()) { const m = new THREE.Mesh(geo, mat); m.castShadow = m.receiveShadow = true; arena.add(m); tmp.push(geo); }
}

// ───────────────────────── post-processing ─────────────────────────
export const composer = new EffectComposer(renderer, new THREE.WebGLRenderTarget(1, 1, { type: THREE.HalfFloatType, samples: HI ? 4 : 0 }));
composer.addPass(new RenderPass(scene, camera));
export const gunPass = new RenderPass(gunScene, gunCam); gunPass.clear = false; gunPass.clearDepth = true; composer.addPass(gunPass);
const bloom = new UnrealBloomPass(new THREE.Vector2(1, 1), 0.3, 0.6, 1.15); composer.addPass(bloom);
const grade = new ShaderPass({
  uniforms: { tDiffuse: { value: null }, uSat: { value: 1.03 }, uCA: { value: 0.0015 }, uHurt: { value: 0 }, uRush: { value: 0 }, uVig: { value: 0.36 }, uWater: { value: 0 }, uTime: { value: 0 } },
  vertexShader: `varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
  fragmentShader: `uniform sampler2D tDiffuse; uniform float uSat, uCA, uHurt, uRush, uVig, uWater, uTime; varying vec2 vUv;
    void main(){
      vec2 uv = vUv + uWater * 0.005 * vec2(sin(vUv.y * 38.0 + uTime * 3.0), cos(vUv.x * 34.0 + uTime * 2.6));
      vec2 d = uv - 0.5; float r2 = dot(d, d); vec2 off = d * (uCA + uWater * 0.004) * (0.4 + r2 * 6.0);
      vec3 c = vec3(texture2D(tDiffuse, uv + off).r, texture2D(tDiffuse, uv).g, texture2D(tDiffuse, uv - off).b);
      float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
      c = mix(vec3(l), c, uSat);
      c = max(vec3(0.0), (c - 0.2) * 1.16 + 0.2);
      c = mix(c, c * vec3(1.18, 0.86, 1.3) + vec3(0.03, 0.0, 0.05), uRush * (0.35 + r2 * 1.6));
      c = mix(c, c * vec3(0.55, 0.95, 1.0), uWater * 0.7);
      c *= 1.0 - (uVig + uWater * 0.3) * smoothstep(0.12, 0.62, r2);
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
let wt = 0;
export function updateWorld(dt) {
  wt += dt; fx.uTime.value = wt;
  clouds.rotation.y += dt * 0.004; dust.rotation.y += dt * 0.01;
  sky.position.copy(camera.position); sunDisc.position.copy(camera.position).addScaledVector(sunDir, 600);
  // the shadow map follows the camera across the bigger map, in 4 m steps so it does not crawl
  sun.target.position.set(Math.round(camera.position.x / 4) * 4, 0, Math.round(camera.position.z / 4) * 4); sun.position.copy(sun.target.position).addScaledVector(sunDir, 130);
  bobbers.forEach((b, i) => { b.position.y += Math.sin(wt * 0.9 + i * 1.7) * dt * 0.08; b.rotation.z = Math.sin(wt * 0.7 + i) * 0.04; });
  for (const kt of kites) { kt.position.y += Math.sin(wt * 0.6 + kt.userData.a) * dt * 0.8; kt.rotation.y = Math.sin(wt * 0.4 + kt.userData.a) * 0.6; }
  setUnderwater(camera.position.y < WATER_Y);
}

setPalette('morning');
buildArena(1);
resize();
