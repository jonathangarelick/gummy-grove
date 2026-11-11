// Gummy Grove — combat, bear AI, missions and the main loop.
import { THREE, TAU, rand, pick, clamp, lerp, damp, C, CANDY, G, settings, glow, std } from './core.js';
import { makeBear, makeShield } from './models.js';
import { makeBearWeapon, makeRocket, makeViewmodels, makeLootViewmodel, jawMat } from './guns.js';
import { Kit } from './kit.js';
import { scene, camera, gunScene, gunPass, composer, fx, ARENA_R, CAKE, pushOut, getLastHit, floorAt, segBlocked, WATER_Y, SPAWN, resize, updateWorld, setPalette, buildArena, getPixelRatio, setPixelRatio } from './world.js';
import { audio } from './audio.js';
import { profile, save, mods, MISSIONS, TIPS, addXp, arcadeWave, rankName, xpFor } from './meta.js';
import { ui } from './ui.js';

const $ = id => document.getElementById(id);
const V = (x = 0, y = 0, z = 0) => new THREE.Vector3(x, y, z);
const _a = V(), _b = V(), _c = V(), _d = V(), _e = V(), _m = V(), _fwd = V(), _right = V(), _up = V(), _hit = V(), _s1 = V(), _s2 = V(), _p = V();
const _col = new THREE.Color(), WHITE = new THREE.Color(0xffffff), Y_AXIS = V(0, 1, 0);
const canvas = $('c'), hudEl = $('hud');

// ───────────────────────── data ─────────────────────────
const BEARS = {
  grunt: { name: 'Pistol Bear', s: 1, hp: 4, speed: 3.6, weapon: 'pistol', range: [8, 17], aim: 0.55, burst: 1, burstGap: 0.3, cool: [1.1, 2], bSpeed: 30, dmg: 8, acc: 0.03, track: 5, pts: 100, colors: [C.red, C.orange, C.yellow, C.green], gear: ['shades'] },
  knife: { name: 'Stabby Runt', s: 0.62, hp: 2, speed: 6.8, weapon: 'knife', melee: true, mdmg: 11, pts: 120, colors: [C.pink, C.cyan], gear: ['bandana'], armHp: 1 },
  rifle: { name: 'Rifle Bear', s: 1.05, hp: 6, speed: 3.2, weapon: 'rifle', two: true, range: [11, 24], aim: 0.7, burst: 4, burstGap: 0.11, cool: [1.6, 2.6], bSpeed: 40, dmg: 5, acc: 0.035, track: 4, pts: 200, colors: [C.green, C.yellow, C.orange], gear: ['helmet'], say: 'rifle', armHp: 3 },
  shield: { name: 'Shield Bear', s: 1.25, hp: 9, speed: 2.7, weapon: 'pistol', shield: true, range: [4.5, 10], aim: 0.6, burst: 2, burstGap: 0.28, cool: [1.3, 2.2], bSpeed: 30, dmg: 8, acc: 0.03, track: 5, aimMove: 0.8, pts: 300, colors: [C.blue], gear: ['helmet'], say: 'shield', armHp: 3 },
  rpg: { name: 'Rocket Bear', s: 1.15, hp: 6, speed: 2.7, weapon: 'rpg', two: true, laser: true, range: [17, 34], aim: 1.5, burst: 1, burstGap: 1, cool: [3.2, 4.8], bSpeed: 17, dmg: 40, acc: 0.01, track: 3, aimMove: 0.2, pts: 350, colors: [C.purple], gear: ['beret'], say: 'rpg', armHp: 3 },
  sniper: { name: 'Sniper Bear', s: 1, hp: 4, speed: 2.9, weapon: 'sniper', two: true, laser: true, hitscan: true, range: [24, 60], aim: 2.0, burst: 1, burstGap: 1, cool: [2.4, 3.6], dmg: 32, acc: 0, track: 8, aimMove: 0, pts: 350, colors: [C.cyan, 0x9fd4ff], gear: ['beret'], say: 'sniper', armHp: 2 },
  boss: { name: 'King Gummy', s: 3.2, hp: 150, speed: 2.5, weapon: 'minigun', two: true, range: [7, 28], aim: 1.1, burst: 34, burstGap: 0.065, cool: [2, 3], bSpeed: 36, dmg: 4, acc: 0.05, track: 2.2, aimMove: 0.5, pts: 3000, colors: [C.red], gear: ['crown', 'shades'], say: 'boss', armHp: 999 },
};
const L = {
  spot: ['CONTACT!', 'THERE HE IS!', 'CANDY THIEF, 12 O\'CLOCK!', 'WEAPONS FREE!', 'GET THE BIG ONE!'],
  hit: ['OW! MY GELATIN!', 'I\'M HIT!', 'MEDIC!', 'THAT\'LL LEAVE A DENT!', 'RUDE!'],
  disarm: ['MY ARM!', 'I NEEDED THAT!', 'FINE. I\'LL BITE.', 'WHO NEEDS ARMS?!', 'THAT WAS MY GOOD ARM!'],
  ally: ['TEDDY, NOOO!', 'MAN DOWN!', 'HE WAS SO SWEET!', 'THEY GOT GARY!', 'AVENGE HIM!'],
  rifle: ['SUPPRESSING FIRE!', 'COVER ME!', 'LIGHT HIM UP!'],
  rpg: ['FIRE IN THE HOLE!', 'ROCKET OUT!', 'SAY CHEESE!'],
  sniper: ['HOLD STILL…', 'STEADY…', 'ONE SHOT…'],
  shield: ['ADVANCE!', 'PUSH! PUSH!', 'S.W.E.E.T. TEAM, GO!'],
  melee: ['NOM NOM!', 'CHOMP!', 'COME HERE!'],
  boss: ['KNEEL BEFORE YOUR KING!', 'YOU CALL THOSE BULLETS?', 'I AM THE ONE TRUE GUMMY!'],
  ff: ['WATCH YOUR FIRE!', 'WRONG TEAM, STEVE!', 'FRIENDLY! FRIENDLY!'],
  rush: ['WHY IS HE SO FAST?!', 'HE\'S ON A SUGAR HIGH!', 'RETREAT?!'],
  die: ['TELL MY WIFE… I\'M DELICIOUS', 'SO… CHEWY…', 'I REGRET NOTHING', 'AVENGE… ME…'],
};
const GUNS = [
  { name: 'JB-4 Jellybean Carbine', short: 'Carbine', clip: 24, rate: 0.105, reload: 1.1, pellets: 1, spread: 0.008, ads: 0.0015, speed: 85, dmg: 1, kick: 1, grav: 3 },
  { name: 'GB-12 Gumball Shotgun', short: 'Shotgun', clip: 6, rate: 0.6, reload: 1.5, pellets: 9, spread: 0.075, ads: 0.045, speed: 70, dmg: 1, kick: 2.8, grav: 6 },
  { name: 'JW-40 Jawbreaker Launcher', short: 'Launcher', clip: 2, rate: 0.75, reload: 2.0, pellets: 1, spread: 0.002, ads: 0, speed: 42, dmg: 9, radius: 5.5, kick: 3.4, grav: 10 },
];
const STREAKS = { 2: 'DOUBLE SCOOP', 3: 'TRIPLE SCOOP', 5: 'SWEET STREAK', 8: 'CANDY CARNAGE', 12: 'TOTAL MELTDOWN' };

// ───────────────────────── state ─────────────────────────
let state = 'menu', noLock = false, clock = 0, MOD = mods(), run = null;
const player = { pos: V(), vel: V(), yaw: 0, pitch: 0, hp: 100, armor: 0, h: 1.7, onGround: true, crouch: false, iframes: 0, sugar: 0, rush: 0, dash: 2 };
let score = 0, wave = 0, tokens = 0, combo = 0, comboT = 0, hitstop = 0, slowT = 0, tsW = 1, tsP = 1, trauma = 0, hurtFlash = 0, hpGhost = 1;
let kickPitch = 0, kickYaw = 0, landDip = 0, bobT = 0, roll = 0, turnV = 0, stepOff = 0, sprintK = 0, slideK = 0, slideT = 0, meleeT = 0, chuteT = 0, deadT = 0, menuT = 0, heartT = 0, muffled = false;
let jumpReq = false, slideReq = false, meleeReq = false, useReq = false, mantle = null, intro = null, nearPick = null;
let gi = 0, lastGi = 0, swT = 1, swTo = -1, reloadT = 0, fireCool = 0, firing = false, ads = false, adsT = 0, recoil = 0, flashT = 0, pumpT = 0, swayX = 0, swayY = 0, mdx = 0, mdy = 0, crossBloom = 0;
let barkCd = 0, chainN = 0, chainT = 0;
const stats = { kills: 0, heads: 0, disarms: 0, shots: 0, hits: 0, ff: 0 };
const bears = [], dying = [], shots = [], ebullets = [], bits = [], debris = [], props = [], beams = [], pendingBooms = [], keys = {};

// ───────────────────────── particles, decals, lights ─────────────────────────
class Pool {
  constructor(mat, max, grav, bounce) {
    this.mesh = new THREE.InstancedMesh(G.lowSphere, mat, max); this.mesh.frustumCulled = false; this.mesh.count = 0;
    this.max = max; this.n = 0; this.grav = grav; this.bounce = bounce; this.dirty = false;
    this.p = new Float32Array(max * 3); this.v = new Float32Array(max * 3); this.s = new Float32Array(max); this.life = new Float32Array(max); this.max0 = new Float32Array(max);
    this.col = new Float32Array(max * 3).fill(1); this.mesh.instanceColor = new THREE.InstancedBufferAttribute(this.col, 3);
    scene.add(this.mesh);
  }
  add(x, y, z, vx, vy, vz, size, life, hex, boost = 1) {
    const i = this.n < this.max ? this.n++ : (Math.random() * this.max) | 0, j = i * 3;
    this.p[j] = x; this.p[j + 1] = y; this.p[j + 2] = z; this.v[j] = vx; this.v[j + 1] = vy; this.v[j + 2] = vz;
    this.s[i] = size; this.life[i] = this.max0[i] = life;
    _col.setHex(hex); this.col[j] = _col.r * boost; this.col[j + 1] = _col.g * boost; this.col[j + 2] = _col.b * boost; this.dirty = true;
  }
  update(dt) {
    const { p, v, s, life, max0, col } = this;
    for (let i = this.n - 1; i >= 0; i--) {
      life[i] -= dt; const j = i * 3;
      if (life[i] <= 0) {
        const l = --this.n, k = l * 3;
        if (i !== l) { for (let q = 0; q < 3; q++) { p[j + q] = p[k + q]; v[j + q] = v[k + q]; col[j + q] = col[k + q]; } s[i] = s[l]; life[i] = life[l]; max0[i] = max0[l]; this.dirty = true; }
        continue;
      }
      v[j + 1] -= this.grav * dt; p[j] += v[j] * dt; p[j + 1] += v[j + 1] * dt; p[j + 2] += v[j + 2] * dt;
      if (p[j + 1] < s[i] && this.grav > 0) { p[j + 1] = s[i]; v[j + 1] *= -this.bounce; v[j] *= 0.7; v[j + 2] *= 0.7; }
    }
    const arr = this.mesh.instanceMatrix.array;
    for (let i = 0; i < this.n; i++) {
      const j = i * 3, o = i * 16, sc = s[i] * Math.min(1, life[i] / (max0[i] * 0.4));
      arr.fill(0, o, o + 16); arr[o] = arr[o + 5] = arr[o + 10] = sc; arr[o + 12] = p[j]; arr[o + 13] = p[j + 1]; arr[o + 14] = p[j + 2]; arr[o + 15] = 1;
    }
    this.mesh.count = this.n; this.mesh.instanceMatrix.needsUpdate = true;
    if (this.dirty) { this.mesh.instanceColor.needsUpdate = true; this.dirty = false; }
  }
}
const chunks = new Pool(std(0xffffff, 0.2), 520, 24, 0.45);
const sparks = new Pool(new THREE.MeshBasicMaterial({ toneMapped: false }), 420, 7, 0);
const smoke = new Pool(new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.2, depthWrite: false }), 200, -1.2, 0);
chunks.mesh.castShadow = true;
function burst(pos, hex, n, size, power) {
  for (let i = 0; i < n; i++) chunks.add(pos.x, pos.y, pos.z, rand(-1, 1) * power, rand(0.2, 1.3) * power, rand(-1, 1) * power, size * rand(0.6, 1.4), rand(0.6, 1.3), hex);
}
function spark(pos, hex, n, power, size = 0.05, life = 0.3, boost = 3) {
  for (let i = 0; i < n; i++) sparks.add(pos.x, pos.y, pos.z, rand(-1, 1) * power, rand(-0.3, 1) * power, rand(-1, 1) * power, size * rand(0.6, 1.4), life * rand(0.6, 1.3), hex, boost);
}

const splatTex = (() => {
  const cv = document.createElement('canvas'); cv.width = cv.height = 128; const x = cv.getContext('2d');
  x.fillStyle = '#000'; x.fillRect(0, 0, 128, 128); x.fillStyle = '#fff'; x.beginPath(); x.arc(64, 64, 34, 0, TAU); x.fill();
  for (let i = 0; i < 12; i++) { const a = rand(0, TAU), d = rand(26, 46); x.beginPath(); x.arc(64 + Math.cos(a) * d, 64 + Math.sin(a) * d, rand(6, 16), 0, TAU); x.fill(); }
  return new THREE.CanvasTexture(cv);
})();
const decals = []; let decalI = 0;
{
  const geo = new THREE.CircleGeometry(1, 24);
  for (let i = 0; i < 40; i++) {
    const m = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ alphaMap: splatTex, transparent: true, depthWrite: false, roughness: 0.1, polygonOffset: true, polygonOffsetFactor: -3, polygonOffsetUnits: -3 }));
    m.rotation.x = -Math.PI / 2; m.visible = false; m.receiveShadow = true; scene.add(m); decals.push({ m, life: 0 });
  }
}
function splat(x, z, hex, size) {
  if (Math.hypot(x, z) < CAKE[0][0] + size) return;
  const d = decals[decalI++ % decals.length];
  d.m.position.set(x, 0.02 + (decalI % decals.length) * 0.0008, z); d.m.rotation.z = rand(0, TAU); d.m.scale.setScalar(size); d.m.material.color.setHex(hex); d.life = 26; d.m.visible = true;
}
function updateDecals(dt) { for (const d of decals) if (d.life > 0) { d.life -= dt; d.m.material.opacity = Math.min(0.85, (d.life / 4) * 0.85); if (d.life <= 0) d.m.visible = false; } }

const lightA = new THREE.PointLight(0xffd9a0, 0, 16, 2), lightB = new THREE.PointLight(0xffc070, 0, 14, 2); scene.add(lightA, lightB);
function flashLight(Lt, pos, intensity, hex) { Lt.position.copy(pos); Lt.intensity = intensity; if (hex) Lt.color.setHex(hex); }

const booms = [];
for (let i = 0; i < 5; i++) {
  const s = new THREE.Mesh(G.sphere, glow(4, 2.4, 1, { transparent: true, depthWrite: false })), r = new THREE.Mesh(new THREE.TorusGeometry(1, 0.06, 6, 40), glow(4, 3, 2, { transparent: true, depthWrite: false }));
  r.rotation.x = Math.PI / 2; s.visible = r.visible = false; scene.add(s, r); booms.push({ s, r, t: 1, rad: 1 });
}
let boomI = 0;
function boomFx(pos, radius, pinkish) {
  const b = booms[boomI++ % booms.length]; b.t = 0; b.rad = radius; b.s.position.copy(pos); b.r.position.set(pos.x, Math.max(0.15, pos.y), pos.z);
  b.s.material.color.setRGB(...(pinkish ? [4, 1.6, 3.2] : [4.5, 2.4, 0.8])); b.s.visible = b.r.visible = true;
  for (let i = 0; i < 46; i++) sparks.add(pos.x, pos.y, pos.z, rand(-1, 1) * 14, rand(-0.2, 1) * 14, rand(-1, 1) * 14, rand(0.05, 0.14), rand(0.3, 0.9), pinkish ? pick(CANDY) : 0xffb347, 3.5);
  for (let i = 0; i < 10; i++) smoke.add(pos.x + rand(-1, 1), pos.y + rand(0, 1), pos.z + rand(-1, 1), rand(-1, 1), rand(1, 3), rand(-1, 1), rand(0.5, 1.1), rand(0.8, 1.6), pinkish ? 0xffc4e4 : 0x5a4a52);
}
function updateBooms(dt) {
  for (const b of booms) {
    if (b.t >= 1) continue; b.t = Math.min(1, b.t + dt / 0.38); const e = 1 - Math.pow(1 - b.t, 3);
    b.s.scale.setScalar(b.rad * (0.25 + 0.6 * e)); b.s.material.opacity = 1 - b.t; b.r.scale.setScalar(b.rad * 1.25 * e); b.r.material.opacity = (1 - b.t) * 0.9;
    if (b.t >= 1) b.s.visible = b.r.visible = false;
  }
}
const laserMat = glow(3.2, 0.15, 0.2, { transparent: true, opacity: 0.6, depthWrite: false }), lockMat = glow(6, 5.4, 5.4), tracerMat = glow(5, 3.6, 1.2), beamMat = glow(7, 6, 5, { transparent: true });
function setBeam(mesh, a, b, r) { mesh.position.copy(a).add(b).multiplyScalar(0.5); mesh.lookAt(b); mesh.scale.set(r, r, Math.max(0.01, a.distanceTo(b))); }
function updateBeams(dt) { for (let i = beams.length - 1; i >= 0; i--) { const b = beams[i]; b.t -= dt; b.mesh.material.opacity = clamp(b.t / 0.16, 0, 1); if (b.t <= 0) { scene.remove(b.mesh); beams.splice(i, 1); } } }

// ───────────────────────── HUD plumbing ─────────────────────────
const scr = { x: 0, y: 0 };
function project(v) { _p.copy(v).project(camera); scr.x = (_p.x * 0.5 + 0.5) * innerWidth; scr.y = (-_p.y * 0.5 + 0.5) * innerHeight; return _p.z < 1 && Math.abs(_p.x) < 1.15 && Math.abs(_p.y) < 1.15; }
const tc = {}, stc = {}, clc = {};
function set(id, v) { if (tc[id] !== v) { tc[id] = v; $(id).textContent = v; } }
function style(id, prop, v) { const k = id + prop; if (stc[k] !== v) { stc[k] = v; $(id).style[prop] = v; } }
function cls(id, name, on) { const k = id + name; if (clc[k] !== on) { clc[k] = on; $(id).classList.toggle(name, on); } }
function replay(el, name) { el.classList.remove(name); void el.offsetWidth; el.classList.add(name); }

const floats = []; let floatI = 0;
for (let i = 0; i < 22; i++) { const el = document.createElement('div'); el.className = 'fl'; $('float').appendChild(el); floats.push({ el, pos: V(), t: 9, life: 1 }); }
function floatText(pos, text, c = '', life = 0.75) { const f = floats[floatI++ % floats.length]; f.pos.copy(pos); f.t = 0; f.life = life; f.el.textContent = text; f.el.className = 'fl ' + c; }
function updateFloats(dt) {
  for (const f of floats) {
    if (f.t >= f.life) continue; f.t += dt; const k = f.t / f.life; _a.copy(f.pos); _a.y += k * 1.1;
    if (k >= 1 || !project(_a)) { f.el.style.opacity = 0; continue; }
    f.el.style.opacity = k < 0.7 ? 1 : (1 - k) / 0.3; f.el.style.transform = `translate3d(${scr.x.toFixed(1)}px,${scr.y.toFixed(1)}px,0) translate(-50%,-50%) scale(${(1 + Math.max(0, 0.25 - k) * 2).toFixed(2)})`;
  }
}
const bubbles = [];
for (let i = 0; i < 3; i++) { const el = document.createElement('div'); el.className = 'bub'; el.style.opacity = 0; $('float').appendChild(el); bubbles.push({ el, b: null, pos: V(), s: 1, t: 9 }); }
function say(b, text, force) {
  if (!force && barkCd > 0) return; barkCd = 1.4;
  const u = bubbles.reduce((m, x) => (x.t > m.t ? x : m)); u.b = b; u.t = 0; u.s = b.s; u.pos.copy(b.root.position); u.el.textContent = text;
  audio.babble(audio.sp(u.pos.x, u.pos.z), 1.25 / Math.sqrt(b.s));
}
function updateBubbles(dt) {
  for (const u of bubbles) {
    if (u.t > 2) { if (u.el.style.opacity !== '0') u.el.style.opacity = 0; continue; }
    u.t += dt; if (u.b && bears.includes(u.b)) u.pos.copy(u.b.root.position);
    _a.copy(u.pos); _a.y += 2.5 * u.s;
    if (!project(_a)) { u.el.style.opacity = 0; continue; }
    u.el.style.opacity = u.t < 1.6 ? 1 : Math.max(0, (2 - u.t) / 0.4); u.el.style.transform = `translate3d(${scr.x.toFixed(1)}px,${scr.y.toFixed(1)}px,0) translate(-50%,-100%)`;
  }
}
function announce(text, c = '') { const el = $('announce'); el.textContent = text; el.className = c; replay(el, 'pop'); }
function feed(html) { const el = document.createElement('div'); el.innerHTML = html; const f = $('feed'); f.prepend(el); while (f.children.length > 5) f.lastChild.remove(); setTimeout(() => el.remove(), 4200); }
function hitmark(crit, kill) { const el = $('hit'); el.className = (crit ? 'crit ' : '') + (kill ? 'kill' : ''); replay(el, 'on'); }
let cardT = 0;
function showCard(kicker, title, sub) { set('cardk', kicker); set('cardt', title); set('cards', sub || ''); $('card').classList.add('show'); $('bars').classList.add('show'); cardT = 3.4; }
const dds = []; let ddI = 0;
for (let i = 0; i < 4; i++) { const el = document.createElement('i'); el.className = 'dd'; $('dmgdir').appendChild(el); dds.push(el); }
function dmgDir(sx, sz) {
  const dx = sx - player.pos.x, dz = sz - player.pos.z, sy = Math.sin(player.yaw), cy = Math.cos(player.yaw);
  const el = dds[ddI++ % dds.length]; el.style.transform = `rotate(${Math.atan2(dx * cy - dz * sy, -dx * sy - dz * cy).toFixed(3)}rad)`; replay(el, 'on');
}
const radioQ = []; let radioT = 0;
function radio(text) { radioQ.push(text); }
function updateRadio(dt) {
  if (radioT > 0) { radioT -= dt; if (radioT <= 0) $('radio').classList.remove('show'); }
  else if (radioQ.length && cardT <= 0.6) { set('radiotext', radioQ.shift()); $('radio').classList.add('show'); radioT = 5; audio.sfx.radio(); }
}

// ───────────────────────── player weapons ─────────────────────────
const VM = makeViewmodels(), gun = new THREE.Group();
gunScene.add(gun); VM.forEach(v => gun.add(v));
const beanMats = CANDY.map(c => std(c, 0.3, 0, { emissive: c, emissiveIntensity: 1.7 }));
// Weapons taken off the bears fire real lead. One magazine, then you throw them away.
const LOOT = {
  pistol: { name: 'Captured Pistol', short: 'Pistol', clip: 12, rate: 0.17, pellets: 1, spread: 0.01, ads: 0.003, speed: 150, dmg: 2, kick: 1.5, grav: 0, lead: true },
  rifle: { name: 'Captured AK', short: 'AK', clip: 30, rate: 0.095, pellets: 1, spread: 0.014, ads: 0.004, speed: 160, dmg: 2, kick: 1.5, grav: 0, lead: true },
  sniper: { name: 'Captured Sniper Rifle', short: 'Sniper', clip: 5, rate: 1.1, pellets: 1, spread: 0.03, ads: 0, speed: 260, dmg: 14, kick: 4.5, grav: 0, lead: true, scope: true },
  rpg: { name: 'Captured RPG', short: 'RPG', clip: 1, rate: 1, pellets: 1, spread: 0.004, ads: 0.002, speed: 34, dmg: 14, radius: 6.5, kick: 4, grav: 2, rocket: true },
  minigun: { name: 'The King\'s Minigun', short: 'Minigun', clip: 200, rate: 0.045, pellets: 1, spread: 0.03, ads: 0.03, speed: 150, dmg: 1.5, kick: 0.5, grav: 0, lead: true, noAds: true },
};
const LOOT_VM = {};
function configGuns() {
  GUNS[0].dmg = MOD.blasterDmg; GUNS[0].clip = MOD.blasterClip; GUNS[1].pellets = MOD.pellets; GUNS[2].radius = MOD.boomR; GUNS[2].dmg = MOD.boomDmg;
  GUNS.length = 3; if (VM[3]) { VM[3].visible = false; VM.length = 3; }
  GUNS.forEach((g, i) => { g.ammo = g.clip; g.owned = MOD.guns[i]; });
}
function showGun() { VM.forEach((v, i) => { v.visible = i === gi; }); }
function hudWeapon() {
  const g = GUNS[gi]; set('wname', g.name); set('ammomax', g.loot ? 'no reloads' : '/ ' + g.clip);
  $('slots').innerHTML = GUNS.map((w, i) => (i === 3 && !w.owned ? '' : `<i class="${i === gi ? 'on' : ''} ${w.owned ? '' : 'locked'}"><b>${i + 1}</b>${w.owned ? w.short : 'Locked'}</i>`)).join('');
}
function switchTo(i) {
  if (state !== 'playing' || intro || i === gi || i === swTo || !GUNS[i] || !GUNS[i].owned) return;
  swTo = i; swT = 0; reloadT = 0; audio.sfx.swap();
}
function startReload() {
  const g = GUNS[gi]; if (g.loot || reloadT > 0 || g.ammo === g.clip || state !== 'playing' || swT < 1) return;
  reloadT = g.reload * MOD.reload; audio.sfx.reload(reloadT);
}
function equipLoot(kind) {
  const L0 = LOOT[kind], cur = GUNS[3];
  if (cur && cur.owned && cur.kind === kind) { cur.ammo = Math.min(cur.clip * 2, cur.ammo + L0.clip); audio.sfx.pickup(); announce('+ AMMO', 'mint'); return; }
  if (VM[3]) VM[3].visible = false;
  if (!LOOT_VM[kind]) { LOOT_VM[kind] = makeLootViewmodel(kind); gun.add(LOOT_VM[kind]); }
  VM[3] = LOOT_VM[kind]; GUNS[3] = { ...L0, kind, ammo: L0.clip, owned: true, reload: 0, loot: true };
  audio.sfx.pickup(); announce(L0.name.toUpperCase(), 'mint');
  if (gi === 3) { swT = 0.45; swTo = -1; showGun(); hudWeapon(); } else { swTo = -1; switchTo(3); hudWeapon(); }
}
function dropLoot() {
  const g = GUNS[3]; if (!g || !g.owned) return;
  g.owned = false; announce(`${g.short.toUpperCase()} EMPTY`); switchTo(lastGi < 3 && GUNS[lastGi].owned ? lastGi : 0);
}
function fire() {
  const g = GUNS[gi];
  if (swT < 1 || reloadT > 0 || fireCool > 0 || meleeT > 0 || sprintK > 0.5 || mantle || !g.owned) return;
  if (g.ammo <= 0) { g.loot ? dropLoot() : startReload(); return; }
  const rushing = player.rush > 0;
  if (!rushing || g.loot) g.ammo--;
  fireCool = g.rate / (rushing ? 1.5 : 1); recoil = Math.min(1.6, recoil + g.kick * 0.55); kickPitch += g.kick * 0.011; kickYaw += rand(-1, 1) * g.kick * 0.004; crossBloom = Math.min(1, crossBloom + g.kick * 0.3);
  trauma = Math.min(1, trauma + g.kick * 0.035); VM[gi].flash.visible = true; VM[gi].flash.rotation.z = rand(0, TAU); flashT = 0.05;
  if (g.rocket) audio.sfx.egun('rpg'); else if (g.lead) audio.sfx.egun(g.kind === 'pistol' ? 'pistol' : g.kind === 'sniper' ? 'sniper' : 'rifle');
  else if (gi === 0) audio.sfx.blaster(); else if (gi === 1) { audio.sfx.scatter(); pumpT = 0.5; } else audio.sfx.cannon();
  camera.getWorldDirection(_fwd); _right.set(1, 0, 0).applyQuaternion(camera.quaternion); _up.set(0, 1, 0).applyQuaternion(camera.quaternion);
  const origin = camera.position.clone().addScaledVector(_right, 0.2 * (1 - adsT)).addScaledVector(_up, -0.15).addScaledVector(_fwd, 0.75);
  flashLight(lightA, origin, 22, 0xffd9a0);
  if (gi === 0 || g.lead) chunks.add(origin.x + _right.x * 0.12 - _fwd.x * 0.45, origin.y + 0.06, origin.z + _right.z * 0.12 - _fwd.z * 0.45, _right.x * 3.5 + player.vel.x, rand(2, 3.4), _right.z * 3.5 + player.vel.z, 0.022, 0.9, 0xd4a84a); // spent brass
  const moveK = Math.min(1, Math.hypot(player.vel.x, player.vel.z) / 9), spread = lerp(g.spread, g.ads, adsT) * (player.crouch ? 0.7 : 1) * (1 + moveK * 0.7 + (slideT > 0 ? 0.6 : 0));
  for (let k = 0; k < g.pellets; k++) {
    const a = rand(0, TAU), r = Math.sqrt(Math.random()) * spread;
    const dir = _d.copy(_fwd).addScaledVector(_right, Math.cos(a) * r).addScaledVector(_up, Math.sin(a) * r).normalize();
    const aim = camera.position.clone().addScaledVector(dir, 60).sub(origin).normalize(), ci = (Math.random() * beanMats.length) | 0;
    let mesh, boom = null, spin = false;
    if (g.rocket) { mesh = makeRocket(); boom = { r: g.radius, d: g.dmg, self: MOD.noSelf ? 0 : 30 }; }
    else if (g.lead) { mesh = new THREE.Mesh(G.beam, tracerMat); mesh.scale.set(0.03, 0.03, 1.5); }
    else if (gi === 0) { mesh = new THREE.Mesh(G.bean, beanMats[ci]); mesh.quaternion.setFromUnitVectors(Y_AXIS, aim); mesh.scale.setScalar(1.3); }
    else if (gi === 1) { mesh = new THREE.Mesh(G.lowSphere, beanMats[ci]); mesh.scale.setScalar(0.085); }
    else { mesh = new THREE.Mesh(G.sphere, jawMat); mesh.scale.setScalar(0.2); boom = { r: g.radius, d: g.dmg, self: MOD.noSelf ? 0 : 18 }; spin = true; }
    mesh.position.copy(origin); if (g.rocket || g.lead) mesh.lookAt(_a.copy(origin).add(aim)); scene.add(mesh); stats.shots++;
    shots.push({ mesh, vel: aim.multiplyScalar(g.speed * rand(0.96, 1.04)), life: 1.8, prev: V(), from: camera.position.clone(), fresh: true, color: g.lead ? 0xffd27a : CANDY[ci], dmg: g.dmg, grav: g.grav, boom, spin, rocket: !!g.rocket });
  }
  if (g.ammo === 0) { if (g.loot) dropLoot(); else if (!rushing) startReload(); }
}
// rifle butt / knife hand: short reach, hits everything in front, built for the small ones
function meleeHit() {
  camera.getWorldDirection(_fwd); let any = false;
  for (const b of bears.slice()) {
    zone(b, 0, 0.9, 0, _c); _d.subVectors(_c, camera.position); const d = _d.length() - 0.6 * b.s;
    if (d < 2.5 && _d.normalize().dot(_fwd) > 0.4) { any = true; b.kx += (_fwd.x * 14) / b.s; b.kz += (_fwd.z * 14) / b.s; hitBear(b, 'body', 3, _fwd.clone(), _c.clone(), 'melee'); }
  }
  if (any) { hitstop = Math.max(hitstop, 0.07); trauma = Math.min(1, trauma + 0.25); audio.sfx.thwack(); }
}
function segSphere(a, b, c, r) {
  _s1.subVectors(b, a); const t = clamp(_s2.subVectors(c, a).dot(_s1) / (_s1.lengthSq() || 1e-9), 0, 1);
  _hit.copy(a).addScaledVector(_s1, t); return _hit.distanceToSquared(c) < r * r;
}
function zone(b, lx, ly, lz, out) {
  const p = b.root.position, s = b.s, c = Math.cos(b.root.rotation.y), sn = Math.sin(b.root.rotation.y);
  return out.set(p.x + (lx * c + lz * sn) * s, (b.g.position.y + ly) * s, p.z + (-lx * sn + lz * c) * s);
}
function updateShots(dt) {
  for (let i = shots.length - 1; i >= 0; i--) {
    const s = shots[i], p = s.mesh.position;
    if (s.fresh) { s.prev.copy(s.from); s.fresh = false; } else s.prev.copy(p);
    s.vel.y -= s.grav * dt; p.addScaledVector(s.vel, dt); s.life -= dt;
    if (s.spin) { s.mesh.rotation.x += dt * 9; if (Math.random() < 0.6) sparks.add(p.x, p.y, p.z, rand(-1, 1), rand(0, 2), rand(-1, 1), 0.05, 0.35, pick(CANDY), 2.5); }
    if (s.rocket && Math.random() < 0.8) smoke.add(s.prev.x, s.prev.y, s.prev.z, rand(-0.4, 0.4), rand(0, 0.8), rand(-0.4, 0.4), rand(0.14, 0.26), rand(0.5, 0.9), 0xd8d0d8);
    let dead = s.life <= 0; const tw = segBlocked(s.prev, p); let end = p;
    if (tw >= 0) { _e.lerpVectors(s.prev, p, tw); end = _e; }
    if (!dead) for (const r of ebullets) if (r.rocket && segSphere(s.prev, end, r.mesh.position, 0.6)) { r.life = 0; r.shotDown = true; dead = true; break; }
    if (!dead) for (const b of bears) {
      const th = b.root.rotation.y, fxx = Math.sin(th), fzz = Math.cos(th), boom = !!s.boom;
      zone(b, 0, 1.6, 0.02, _c);
      if (segSphere(s.prev, end, _c, 0.44 * b.s + 0.1)) { p.copy(_hit); if (!boom) hitBear(b, 'head', s.dmg, s.vel, p); dead = true; break; }
      if (b.shield) {
        const vl = Math.hypot(s.vel.x, s.vel.z) || 1;
        if ((s.vel.x * fxx + s.vel.z * fzz) / vl < -0.3) {
          zone(b, 0.12, 0.95, 0.5, _c);
          if (segSphere(s.prev, end, _c, 0.7 * b.s)) { p.copy(_hit); if (!boom) { spark(p, 0xffffff, 6, 5, 0.04, 0.2, 4); audio.sfx.clang(audio.sp(p.x, p.z)); b.jv += 2; } dead = true; break; }
        }
      }
      if (b.armed && b.armHp < 99) { zone(b, -0.5, 1.18, 0.5, _c); if (segSphere(s.prev, end, _c, 0.24 * b.s + 0.08)) { p.copy(_hit); if (!boom) hitBear(b, 'arm', s.dmg, s.vel, p); dead = true; break; } }
      zone(b, 0, 0.82, 0, _c);
      if (segSphere(s.prev, end, _c, 0.68 * b.s + 0.1)) { p.copy(_hit); if (!boom) hitBear(b, 'body', s.dmg, s.vel, p); dead = true; break; }
    }
    if (!dead && (tw >= 0 || p.y < 0.04)) { dead = true; if (tw >= 0) p.copy(_e); else p.y = 0.04; if (!s.boom) spark(p, s.color, 4, 3, 0.05, 0.28, 2.2); }
    if (dead) { scene.remove(s.mesh); shots.splice(i, 1); if (s.boom) explode(p, s.boom.r, s.boom.d, s.boom.self, true); }
    if (state !== 'playing') return;
  }
}

// ───────────────────────── bears ─────────────────────────
function makeEnemy(type) {
  const T = BEARS[type], color = pick(T.colors), model = makeBear(color, T.gear), weapon = makeBearWeapon(T.weapon);
  model.armR.hand.add(weapon);
  let shield = null;
  if (T.shield) { shield = makeShield(); shield.position.set(-0.36, -0.22, 0.14); model.armL.hand.add(shield); }
  if (T.two) model.armL.rotation.y = -0.5;
  return { ...model, T, type, color, base: new THREE.Color(color), weapon, shield, s: T.s, armed: true, melee: !!T.melee, phase: rand(0, TAU), hop: rand(7, 9) / Math.sqrt(T.s), j: 0, jv: 0, flash: 0, kick: 0, lunge: 0, flashT: 0 };
}
function animateBear(b, dt, moving, elev, aiming) {
  b.phase += dt * b.hop * (moving ? 1 : 0.45);
  const h = Math.abs(Math.sin(b.phase));
  b.g.position.y = h * (moving ? (aiming ? 0.12 : 0.28) : 0.05);
  b.jv += (-200 * b.j - 9 * b.jv) * dt; b.j += b.jv * dt;
  const sq = (h - 0.5) * (moving ? 0.14 : 0.04) + clamp(b.j, -0.6, 0.6) * 0.5;
  b.g.scale.set(1 - sq * 0.5, 1 + sq, 1 - sq * 0.5); b.g.rotation.z = Math.sin(b.phase) * (moving ? 0.09 : 0.03);
  b.kick = damp(b.kick, 0, 12, dt); b.lunge = damp(b.lunge, 0, 7, dt);
  if (b.armed) {
    b.armR.rotation.x = -elev - b.kick * 0.35 + (b.melee ? Math.sin(b.phase * 2) * 0.35 - b.lunge * 1.1 : 0);
    if (b.T.two) b.armL.rotation.x = b.armR.rotation.x;
  } else b.armL.rotation.x = Math.sin(b.phase * 2.2) * 0.6 - 0.4;
  b.head.rotation.x = -elev * 0.5;
  if (b.weapon.barrels && b.fsm !== 'idle') b.weapon.barrels.rotation.z += dt * 26;
  if (b.marker) { b.marker.rotation.y += dt * 3; b.marker.position.y = 2.75 + Math.sin(clock * 3) * 0.1; }
  if (b.flash > 0) b.flash = Math.max(0, b.flash - dt);
  const fl = b.flash / 0.12; b.mat.emissive.copy(b.base).lerp(WHITE, fl); b.mat.emissiveIntensity = b.mat.userData.glow + fl * 1.6;
  if (b.flashT > 0) { b.flashT -= dt; if (b.flashT <= 0) b.weapon.flash.visible = false; }
}
function spawnBear(type, at, elite) {
  const b = makeEnemy(type), T = b.T; let x, z;
  if (at) { x = at.x; z = at.z; }
  else for (let i = 0; i < 16; i++) { // somewhere 28–48 m off, preferably round a corner
    const a = rand(0, TAU), d = rand(28, 48); x = player.pos.x + Math.cos(a) * d; z = player.pos.z + Math.sin(a) * d;
    const r = Math.hypot(x, z); if (r > ARENA_R - 4) { x *= (ARENA_R - 4) / r; z *= (ARENA_R - 4) / r; }
    if (Math.hypot(x - player.pos.x, z - player.pos.z) < 22 || floorAt(x, z, 99) > 0) continue;
    _a.set(x, 1.5, z); _b.set(player.pos.x, player.pos.y + 1.4, player.pos.z);
    if (i > 10 || segBlocked(_a, _b) >= 0) break;
  }
  b.root.position.set(x, 0, z); b.root.scale.setScalar(0.001); scene.add(b.root);
  const hpMul = (type === 'boss' ? 1 + Math.max(0, wave / 5 - 1) * 0.5 : 1 + Math.max(0, wave - 7) * 0.07) * (elite ? 2.2 : 1);
  Object.assign(b, {
    hp: Math.round(T.hp * hpMul), speed: T.speed * rand(0.92, 1.08) * (1 + Math.min(0.3, wave * 0.015)), mdmg: T.mdmg ?? 9, reach: 0.75 * T.s + 0.6, armHp: T.armHp ?? 2,
    spawn: 0, thinkT: rand(0, 0.4), los: false, seen: false, strafe: Math.random() < 0.5 ? -1 : 1, side: Math.random() < 0.5 ? -1 : 1, stuck: 0,
    fsm: 'idle', coolT: rand(0.8, 1.8), aimT: 0, burst: 0, burstT: 0, lostT: 0, hasToken: false, locked: false, lock: V(), track: player.pos.clone(),
    kx: 0, kz: 0, cool: 0.6, seed: rand(0, 10), laser: null, minionT: 7, elite: !!elite,
  });
  b.maxHp = b.hp;
  if (T.laser) { b.laser = new THREE.Mesh(G.beam, laserMat); b.laser.visible = false; b.laser.frustumCulled = false; scene.add(b.laser); }
  if (elite) { b.marker = new THREE.Mesh(new THREE.OctahedronGeometry(0.22), glow(5, 3.6, 0.6)); b.marker.position.y = 2.75; b.root.add(b.marker); b.armHp = 999; }
  bears.push(b);
  _c.set(x, 0.3, z); spark(_c, b.color, 18, 5, 0.07, 0.5, 2);
  if (type === 'boss') { say(b, pick(L.boss), true); audio.sfx.boss(); trauma = Math.min(1, trauma + 0.4); }
  if (run && !run.seen[type] && TIPS[type]) { run.seen[type] = true; radio(TIPS[type]); }
  return b;
}
const maxTokens = () => Math.min(5, 2 + Math.floor(wave / 3));
function endAttack(b, cool) { b.fsm = 'idle'; b.coolT = cool; b.locked = false; if (b.hasToken) { tokens = Math.max(0, tokens - 1); b.hasToken = false; } if (b.laser) b.laser.visible = false; if (b.weapon.warhead) b.weapon.warhead.visible = true; }
function removeLaser(b) { if (b.laser) { scene.remove(b.laser); b.laser = null; } }

function enemyShoot(b) {
  const T = b.T; b.weapon.muzzle.getWorldPosition(_m); b.kick = 1; b.weapon.flash.visible = true; b.flashT = 0.06;
  flashLight(lightB, _m, 26, 0xffc070); audio.sfx.egun(T.weapon, audio.sp(_m.x, _m.z));
  for (let k = 0; k < 4; k++) sparks.add(_m.x, _m.y, _m.z, rand(-2, 2), rand(0, 3), rand(-2, 2), 0.04, 0.15, 0xffd27a, 3);
  if (T.hitscan) return sniperFire(b);
  const dist = _m.distanceTo(b.track), rpg = T.weapon === 'rpg';
  _d.set(b.track.x + rand(-1, 1) * T.acc * dist, b.track.y + (rpg ? 0.15 : player.h * 0.65) + rand(-1, 1) * T.acc * dist * 0.6, b.track.z + rand(-1, 1) * T.acc * dist).sub(_m).normalize();
  let mesh;
  if (rpg) { mesh = makeRocket(); mesh.scale.setScalar(b.s); b.weapon.warhead.visible = false; }
  else { mesh = new THREE.Mesh(G.beam, tracerMat); mesh.scale.set(0.045, 0.045, 1.2); }
  mesh.position.copy(_m); mesh.lookAt(_a.copy(_m).add(_d)); scene.add(mesh);
  ebullets.push({ mesh, vel: _d.clone().multiplyScalar(T.bSpeed), prev: _m.clone(), life: rpg ? 4.5 : 3, dmg: T.dmg, rocket: rpg, whiz: false, shotDown: false });
}
function sniperFire(b) {
  _d.copy(b.lock).sub(_m).normalize(); _a.copy(_m).addScaledVector(_d, 95);
  const t = segBlocked(_m, _a); if (t >= 0) _a.lerpVectors(_m, _a, t);
  _c.set(player.pos.x, player.pos.y + player.h * 0.6, player.pos.z);
  const hit = segSphere(_m, _a, _c, 0.62) && player.iframes <= 0;
  const mesh = new THREE.Mesh(G.beam, beamMat.clone()); setBeam(mesh, _m, _a, 0.028); scene.add(mesh); beams.push({ mesh, t: 0.16 });
  if (hit) hurt(b.T.dmg, _m.x, _m.z); else audio.sfx.whiz();
}

function updateBears(dt) {
  const pp = player.pos;
  for (let i = 0; i < bears.length; i++) {
    const b = bears[i], p = b.root.position, T = b.T;
    if (b.spawn < 1) { b.spawn = Math.min(1, b.spawn + dt * 2.2); b.root.scale.setScalar(b.s * (1 - Math.pow(1 - b.spawn, 3)) * (1 + 0.25 * Math.sin(b.spawn * Math.PI))); }
    let dx = pp.x - p.x, dz = pp.z - p.z; const dist = Math.hypot(dx, dz) || 1e-4; dx /= dist; dz /= dist;
    b.track.x = damp(b.track.x, pp.x, T.track || 5, dt); b.track.y = damp(b.track.y, pp.y, 6, dt); b.track.z = damp(b.track.z, pp.z, T.track || 5, dt);
    b.thinkT -= dt;
    if (b.thinkT <= 0) {
      b.thinkT = rand(0.3, 0.55);
      _a.set(p.x, 1.5 * b.s, p.z); _b.set(pp.x, pp.y + player.h * 0.82, pp.z); b.los = segBlocked(_a, _b) < 0;
      if (Math.random() < 0.22) b.strafe = -b.strafe;
      if (b.los && !b.seen) { b.seen = true; if (Math.random() < 0.5) say(b, pick(L.spot)); }
    }
    // movement intent
    let mx = 0, mz = 0, sp = b.speed;
    if (b.melee) {
      if (dist > b.reach) { const zig = dist > 5 ? Math.sin(clock * 3 + b.seed) * 0.5 : 0; mx = dx - dz * zig; mz = dz + dx * zig; }
    } else {
      const [rmin, rmax] = T.range;
      if (!b.los || dist > rmax) { mx = dx; mz = dz; }
      else if (dist < rmin) { mx = -dx - dz * b.strafe * 0.5; mz = -dz + dx * b.strafe * 0.5; }
      else { mx = -dz * b.strafe; mz = dx * b.strafe; sp *= 0.7; }
      if (b.fsm !== 'idle') sp *= T.aimMove ?? 0.45;
    }
    const ml = Math.hypot(mx, mz), moving = ml > 0.01 && sp > 0.05;
    if (moving) { p.x += (mx / ml) * sp * dt; p.z += (mz / ml) * sp * dt; }
    p.x += b.kx * dt; p.z += b.kz * dt; const kd = Math.exp(-8 * dt); b.kx *= kd; b.kz *= kd;
    for (let j = i + 1; j < bears.length; j++) {
      const o = bears[j], q = o.root.position; let ex = q.x - p.x, ez = q.z - p.z; const d = Math.hypot(ex, ez) || 1e-4, min = (b.s + o.s) * 0.55;
      if (d < min) { ex /= d; ez /= d; const push = min - d, wb = o.s / (b.s + o.s); p.x -= ex * push * wb; p.z -= ez * push * wb; q.x += ex * push * (1 - wb); q.z += ez * push * (1 - wb); }
    }
    const bx = p.x, bz = p.z; pushOut(p, 0.5 * b.s, 0);
    const nx = p.x - bx, nz = p.z - bz, nl = Math.hypot(nx, nz);
    if (nl > 1e-5) { // slide around whatever we bumped into
      p.x += (-nz / nl) * b.side * sp * dt * 0.8; p.z += (nx / nl) * b.side * sp * dt * 0.8; b.stuck += dt;
      if (b.stuck > 1.6) { b.side = -b.side; b.strafe = -b.strafe; b.stuck = 0; }
    } else b.stuck = Math.max(0, b.stuck - dt * 0.5);
    const r = Math.hypot(p.x, p.z), lim = ARENA_R - 1.5; if (r > lim) { p.x *= lim / r; p.z *= lim / r; b.strafe = -b.strafe; }
    let da = Math.atan2(dx, dz) - b.root.rotation.y; da = Math.atan2(Math.sin(da), Math.cos(da)); b.root.rotation.y += da * Math.min(1, dt * 8);

    // weapon
    if (b.melee) {
      b.cool -= dt;
      if (dist < b.reach + 0.45 && b.cool <= 0 && b.spawn >= 1 && pp.y < 1.8 * b.s) { b.cool = 0.9; b.lunge = 1; b.jv += 4; if (Math.random() < 0.3) say(b, pick(L.melee)); hurt(b.mdmg, p.x, p.z); if (state !== 'playing') return; }
    } else if (b.fsm === 'idle') {
      b.coolT -= dt;
      if (b.coolT <= 0 && b.los && b.spawn >= 1 && dist < T.range[1] * 1.3 && (b.type === 'boss' || tokens < maxTokens())) {
        b.fsm = 'aim'; b.aimT = T.aim; b.lostT = 0; b.locked = false;
        if (b.type !== 'boss') { tokens++; b.hasToken = true; }
        if (T.say && Math.random() < 0.4) say(b, pick(L[T.say]));
        if (T.laser) audio.sfx.charge(audio.sp(p.x, p.z));
      }
    } else if (b.fsm === 'aim') {
      b.aimT -= dt;
      if (!b.los) { b.lostT += dt; if (b.lostT > 0.5) endAttack(b, 0.6); }
      if (b.fsm === 'aim' && T.hitscan && !b.locked && b.aimT <= 0.34) { b.locked = true; b.lock.set(pp.x, pp.y + player.h * 0.62, pp.z); audio.sfx.lock(audio.sp(p.x, p.z)); }
      if (b.fsm === 'aim' && b.aimT <= 0) { b.fsm = 'fire'; b.burst = T.burst; b.burstT = 0; }
    } else {
      b.burstT -= dt;
      while (b.burstT <= 0 && b.burst > 0) { enemyShoot(b); b.burst--; b.burstT += T.burstGap; if (state !== 'playing') return; }
      if (b.burst <= 0) endAttack(b, rand(T.cool[0], T.cool[1]));
    }
    if (b.laser) {
      if (b.fsm === 'aim') {
        b.weapon.muzzle.getWorldPosition(_m);
        if (b.locked) _a.copy(b.lock); else _a.set(pp.x, pp.y + (T.hitscan ? player.h * 0.62 : 0.2), pp.z);
        setBeam(b.laser, _m, _a, b.locked ? 0.035 : 0.016); b.laser.material = b.locked ? lockMat : laserMat; b.laser.visible = true;
      } else b.laser.visible = false;
    }
    if (b.type === 'boss' && b.spawn >= 1) {
      b.minionT -= dt;
      if (b.minionT <= 0 && bears.length < 14) { b.minionT = 8; for (let k = 0; k < 2; k++) spawnBear('knife', { x: p.x + rand(-2, 2), z: p.z + rand(-2, 2) }); }
    }
    animateBear(b, dt, moving, Math.atan2(pp.y + player.h * 0.65 - 1.18 * b.s, dist), b.fsm !== 'idle');
  }
}

function hitBear(b, zoneName, dmg, dir, pos, src = 'shot') {
  if (src === 'shot') stats.hits++;
  b.jv += 7; b.seen = true;
  const dl = Math.hypot(dir.x, dir.z) || 1, k = (src === 'shot' ? 3.5 : src === 'melee' ? 2 : 9) / b.s; b.kx += (dir.x / dl) * k; b.kz += (dir.z / dl) * k;
  if (zoneName === 'arm') {
    b.armHp -= dmg; burst(pos, b.color, 3, 0.07, 4); hitmark(false); audio.sfx.hit();
    if (b.armHp <= 0) disarm(b, dir);
    return;
  }
  const crit = zoneName === 'head', d = dmg * (crit ? MOD.headMul : 1);
  b.hp -= d; b.flash = 0.12;
  burst(pos, b.color, crit ? 5 : 3, 0.09 * Math.sqrt(b.s), 4.5);
  if (src !== 'ff') { floatText(pos, Math.round(d * 10), crit ? 'crit' : ''); if (src === 'shot' || src === 'melee') { hitmark(crit, b.hp <= 0); crit ? audio.sfx.head() : audio.sfx.hit(); } }
  if (b.hp <= 0) killBear(b, crit && src === 'shot', dir, src);
  else if (Math.random() < 0.12) say(b, pick(src === 'ff' ? L.ff : L.hit));
}
function disarm(b, dir) {
  const dl = Math.hypot(dir.x, dir.z) || 1;
  b.armed = false; b.melee = true; b.speed *= 1.55; endAttack(b, 0); removeLaser(b);
  scene.attach(b.armR); addDebris(b.armR, V((dir.x / dl) * 3 + rand(-2, 2), rand(6, 9), (dir.z / dl) * 3 + rand(-2, 2)), 4.5, 0.2 * b.s, LOOT[b.T.weapon] ? { kind: b.T.weapon } : null);
  stats.disarms++; const pts = MOD.smile ? 150 : 50; score += pts; announce('DISARMED', 'mint'); audio.sfx.disarm(); say(b, pick(L.disarm), true); hitstop = Math.max(hitstop, 0.05);
  zone(b, -0.5, 1.18, 0.2, _c); burst(_c, b.color, 6, 0.1, 5); floatText(_c, '+' + pts, 'pts', 1);
  if (MOD.smile) dropBits(b.root.position, b.color, 3);
}
function killBear(b, crit, dir, src) {
  const i = bears.indexOf(b); if (i < 0) return;
  bears.splice(i, 1); endAttack(b, 0); removeLaser(b);
  const p = b.root.position, dl = Math.hypot(dir.x, dir.z) || 1, T = b.T;
  stats.kills++; if (crit) stats.heads++; if (src === 'ff') stats.ff++;
  combo++; comboT = 4;
  const pts = Math.round(T.pts * Math.min(combo, 8) * (crit ? 1.5 : 1) * (b.elite ? 2 : 1)); score += pts;
  if (player.rush <= 0) player.sugar = Math.min(100, player.sugar + (crit ? 14 : 9) * MOD.rushGain);
  const xp = Math.round(T.pts / 8) * (b.elite ? 3 : 1); run.xp += xp; run.candy += Math.round(T.pts / 20) * (b.elite ? 3 : 1);
  const ups = addXp(xp);
  if (ups) { announce(`LEVEL UP · ${rankName(profile.level).toUpperCase()}`, 'gold'); audio.sfx.levelup(); player.hp = Math.min(MOD.maxHp, player.hp + 40); run.levelUps += ups; }
  else if (STREAKS[combo]) announce(STREAKS[combo], combo >= 5 ? 'gold' : '');
  feed(`${src === 'ff' ? '<b class="ff">FRIENDLY FIRE</b> ' : src === 'melee' ? '<b>MELEE</b> ' : crit ? '<b>HEADSHOT</b> ' : ''}${b.elite ? 'Target ' : ''}${T.name} <i>+${pts}</i>`);
  _c.set(p.x, 1.5 * b.s, p.z); floatText(_c, '+' + pts, 'pts', 1.1);
  hitstop = Math.max(hitstop, b.type === 'boss' ? 0.25 : crit ? 0.07 : 0.045); trauma = Math.min(1, trauma + (b.type === 'boss' ? 0.7 : 0.12));
  if (crit && b.type !== 'boss') { // head pops clean off; the body staggers on for a moment
    scene.attach(b.head); addDebris(b.head, V((dir.x / dl) * 4 + rand(-1, 1), rand(7, 10), (dir.z / dl) * 4 + rand(-1, 1)), 3.5, 0.4 * b.s);
    audio.sfx.pop(audio.sp(p.x, p.z)); _c.set(p.x, 1.5 * b.s, p.z); burst(_c, b.color, 8, 0.08, 6); b.dieT = 0.55; dying.push(b);
  } else burstBear(b, src !== 'shot');
  if (MOD.chain && player.rush > 0) pendingBooms.push(V(p.x, 0.9 * b.s, p.z));
  if (Math.random() < 0.14) { barkCd = 0; say(b, pick(L.die), true); }
  else if (bears.length && Math.random() < 0.3) say(pick(bears), pick(L.ally));
  if (b.elite && run.o) { run.killed++; announce(`TARGET DOWN  ${run.killed}/${run.o.count}`, 'gold'); }
  if (b.type === 'boss') { announce('THE KING IS JELLY', 'gold'); slowT = Math.max(slowT, 1.2); }
  if (!bears.length && !run.queue.length && run.type !== 'survive' && run.type !== 'hunt') slowT = Math.max(slowT, 0.8);
}
function burstBear(b, big) {
  const p = b.root.position, s = b.s;
  if (b.armed && b.weapon.parent) { scene.attach(b.weapon); addDebris(b.weapon, V(rand(-2, 2), rand(4, 7), rand(-2, 2)), 5, 0.12 * s, LOOT[b.T.weapon] ? { kind: b.T.weapon } : null); }
  if (b.shield && b.shield.parent) { scene.attach(b.shield); addDebris(b.shield, V(rand(-2, 2), rand(3, 6), rand(-2, 2)), 5, 0.3 * s, { armor: 50 }); }
  scene.remove(b.root);
  _c.set(p.x, 0.9 * s, p.z); burst(_c, b.color, Math.round(14 * s) + (big ? 10 : 0), 0.17 * Math.sqrt(s), (big ? 11 : 7) + s);
  splat(p.x, p.z, b.color, rand(1.1, 1.7) * s);
  dropBits(p, b.color, (b.type === 'boss' ? 16 : 2 + ((Math.random() * 3) | 0)) * (player.rush > 0 ? 2 : 1));
  audio.sfx.kill(audio.sp(p.x, p.z));
}
function updateDying(dt) {
  for (let i = dying.length - 1; i >= 0; i--) {
    const b = dying[i]; b.dieT -= dt; b.g.rotation.z = Math.sin(clock * 34) * 0.22; b.armL.rotation.x = Math.sin(clock * 40) * 0.9; if (b.armed) b.armR.rotation.x = Math.cos(clock * 40) * 0.9;
    b.root.position.x += Math.sin(b.root.rotation.y) * dt * 1.5; b.root.position.z += Math.cos(b.root.rotation.y) * dt * 1.5;
    if (b.dieT <= 0) { dying.splice(i, 1); burstBear(b, false); }
  }
}
const lootMat = glow(4, 3.2, 0.8, { transparent: true, opacity: 0.4, depthWrite: false });
function addDebris(o, vel, life, r, loot = null) {
  const d = { o, v: vel, w: V(rand(-9, 9), rand(-9, 9), rand(-9, 9)), life: loot ? 26 : life, r, loot, beam: null };
  if (loot) { d.beam = new THREE.Mesh(G.beam, lootMat); d.beam.rotation.x = -Math.PI / 2; d.beam.scale.set(0.045, 0.045, 2.6); scene.add(d.beam); }
  debris.push(d);
}
function removeDebris(i) { const d = debris[i]; scene.remove(d.o); if (d.beam) scene.remove(d.beam); debris.splice(i, 1); }
function updateDebris(dt) {
  nearPick = null; let best = 2.7;
  for (let i = debris.length - 1; i >= 0; i--) {
    const d = debris[i], p = d.o.position; d.v.y -= 26 * dt; p.addScaledVector(d.v, dt);
    d.o.rotation.x += d.w.x * dt; d.o.rotation.y += d.w.y * dt; d.o.rotation.z += d.w.z * dt;
    if (p.y < d.r) { p.y = d.r; d.v.y *= -0.42; d.v.x *= 0.6; d.v.z *= 0.6; d.w.multiplyScalar(0.55); }
    d.life -= dt; if (d.life < 0.4) d.o.scale.multiplyScalar(Math.max(0, 1 - dt * 6));
    if (d.beam) { // a little column of light so dropped gear is easy to spot
      d.beam.position.set(p.x, 1.4, p.z); d.beam.visible = d.life > 4 || Math.sin(d.life * 12) > 0;
      const dd = Math.hypot(p.x - player.pos.x, p.z - player.pos.z); if (dd < best && Math.abs(player.pos.y - p.y) < 2.6) { best = dd; nearPick = d; }
    }
    if (d.life <= 0) removeDebris(i);
  }
}
function takeLoot() {
  const d = nearPick, i = d ? debris.indexOf(d) : -1; if (i < 0) return;
  if (d.loot.armor) { player.armor = Math.min(100, player.armor + d.loot.armor); announce('+50 ARMOR', 'mint'); audio.sfx.pickup(); } else equipLoot(d.loot.kind);
  removeDebris(i); nearPick = null;
}

// ───────────────────────── enemy fire, explosions, damage ─────────────────────────
function updateEbullets(dt) {
  const pp = player.pos;
  for (let i = ebullets.length - 1; i >= 0; i--) {
    const e = ebullets[i], p = e.mesh.position;
    e.prev.copy(p); p.addScaledVector(e.vel, dt); e.life -= dt;
    let dead = e.life <= 0;
    if (e.rocket && Math.random() < 0.8) smoke.add(e.prev.x, e.prev.y, e.prev.z, rand(-0.4, 0.4), rand(0, 0.8), rand(-0.4, 0.4), rand(0.14, 0.26), rand(0.5, 0.9), 0xd8d0d8);
    if (!dead) {
      const t = segBlocked(e.prev, p);
      if (t >= 0) { p.lerpVectors(e.prev, p, t); dead = true; } else if (p.y < 0.05) { p.y = 0.05; dead = true; }
      if (dead && !e.rocket) spark(p, 0xffd27a, 4, 4, 0.04, 0.2, 3);
    }
    if (!dead) {
      const half = player.h * 0.56; _c.set(pp.x, pp.y + half, pp.z); segSphere(e.prev, p, _c, 1);
      const hd = Math.hypot(_hit.x - _c.x, _hit.z - _c.z), vd = Math.abs(_hit.y - _c.y);
      if (hd < (e.rocket ? 1.2 : 0.45) && vd < (e.rocket ? 1.6 : half)) {
        if (e.rocket) dead = true; else if (player.iframes <= 0) { dead = true; hurt(e.dmg, e.prev.x - e.vel.x, e.prev.z - e.vel.z); }
      } else if (!e.whiz && !e.rocket && hd < 2.2 && vd < 2) { e.whiz = true; audio.sfx.whiz(); }
    }
    if (dead) { scene.remove(e.mesh); ebullets.splice(i, 1); if (e.rocket) explode(p, 5, 5, 40, e.shotDown, e.shotDown); }
    if (state !== 'playing') return;
  }
}
function explode(pos, radius, bearDmg, playerDmg, byPlayer, shotDown) {
  audio.sfx.boom(audio.sp(pos.x, pos.z)); boomFx(pos, radius, byPlayer && !shotDown); flashLight(lightA, pos, 380, byPlayer ? 0xff9de0 : 0xffb060);
  trauma = Math.min(1, trauma + clamp(1.2 - pos.distanceTo(camera.position) / 22, 0, 0.8));
  if (pos.y < 2) splat(pos.x, pos.z, byPlayer && !shotDown ? pick(CANDY) : 0x3a2030, radius * 0.45);
  if (shotDown) { score += 75; announce('INTERCEPTED', 'mint'); }
  _c.set(player.pos.x, player.pos.y + 0.9, player.pos.z); const d = _c.distanceTo(pos);
  if (d < radius) {
    const f = 1 - d / radius; _d.subVectors(_c, pos).normalize();
    player.vel.x += _d.x * 16 * f; player.vel.z += _d.z * 16 * f; player.vel.y += (5 + 7 * f) * f + 2; player.onGround = false;
    if (playerDmg > 0) hurt(playerDmg * f, pos.x, pos.z);
  }
  for (const b of bears.slice()) {
    zone(b, 0, 0.85, 0, _c); const bd = _c.distanceTo(pos) - 0.5 * b.s;
    if (bd < radius) hitBear(b, 'body', Math.max(1, Math.round(bearDmg * clamp(1 - bd / radius, 0, 1))), _c.clone().sub(pos), _c.clone(), byPlayer ? 'boom' : 'ff');
  }
}
function hurt(dmg, sx, sz) {
  if (state !== 'playing' || player.iframes > 0 || run.done || intro) return;
  let d = dmg * (player.rush > 0 ? 0.5 : 1);
  if (player.armor > 0) { const a = Math.min(player.armor, d * 0.7); player.armor -= a; d -= a; }
  player.hp -= d; player.iframes = 0.12;
  hurtFlash = Math.min(1, hurtFlash + 0.35 + dmg / 40); trauma = Math.min(1, trauma + 0.25 + dmg / 60); kickPitch += rand(-0.02, 0.02);
  dmgDir(sx, sz); audio.sfx.hurt();
  if (player.hp <= 0) {
    if (MOD.wind && !run.wind) { run.wind = true; player.hp = 30; player.sugar = 100; player.iframes = 1; startRush(); announce('SECOND WIND', 'gold'); }
    else { player.hp = 0; state = 'dead'; deadT = 2.2; firing = false; audio.sfx.over(); audio.musicIntensity(0.15); announce(run.m ? 'MISSION FAILED' : 'CHEWED UP'); }
  }
}

// ───────────────────────── pickups ─────────────────────────
const bitMats = new Map();
function dropBits(p, hex, n) {
  if (!bitMats.has(hex)) bitMats.set(hex, std(hex, 0.2, 0, { emissive: hex, emissiveIntensity: 1.5 }));
  for (let i = 0; i < n; i++) { const m = new THREE.Mesh(G.lowSphere, bitMats.get(hex)); m.scale.setScalar(0.14); m.position.set(p.x, 0.8, p.z); scene.add(m); bits.push({ m, v: V(rand(-4, 4), rand(5, 9), rand(-4, 4)), t: 0 }); }
}
function updateBits(dt) {
  _c.set(player.pos.x, player.pos.y + 1, player.pos.z);
  const range = player.rush > 0 ? MOD.magnet * 2 : MOD.magnet;
  for (let i = bits.length - 1; i >= 0; i--) {
    const b = bits[i], p = b.m.position; b.t += dt; const d = p.distanceTo(_c);
    if (b.t > 0.5 && d < range) { _d.subVectors(_c, p).multiplyScalar(20 / (d || 1)); b.v.lerp(_d, Math.min(1, dt * 7)); }
    else { b.v.y -= 24 * dt; if (p.y < 0.15) { p.y = 0.15; b.v.y *= -0.4; b.v.x *= 0.7; b.v.z *= 0.7; } }
    p.addScaledVector(b.v, dt);
    if (d < 1.1 && b.t > 0.3) {
      player.hp = Math.min(MOD.maxHp, player.hp + MOD.bitHeal); if (player.rush <= 0) player.sugar = Math.min(100, player.sugar + 2.5 * MOD.rushGain);
      run.candy++; audio.sfx.bit(chainN++); chainT = 0.7; scene.remove(b.m); bits.splice(i, 1);
    } else if (b.t > 16) { scene.remove(b.m); bits.splice(i, 1); }
  }
}

// ───────────────────────── run / objectives ─────────────────────────
const shuffle = a => { for (let i = a.length - 1; i > 0; i--) { const j = (Math.random() * (i + 1)) | 0; [a[i], a[j]] = [a[j], a[i]]; } return a; };
function weighted(mix) { let n = 0; for (const k in mix) n += mix[k]; let r = Math.random() * n; for (const k in mix) { r -= mix[k]; if (r < 0) return k; } return Object.keys(mix)[0]; }
function clearAll() {
  for (const b of [...bears, ...dying, ...props]) { scene.remove(b.root); removeLaser(b); }
  for (const s of shots) scene.remove(s.mesh); for (const e of ebullets) scene.remove(e.mesh); for (const b of bits) scene.remove(b.m); for (const d of debris) { scene.remove(d.o); if (d.beam) scene.remove(d.beam); } for (const b of beams) scene.remove(b.mesh);
  bears.length = dying.length = props.length = shots.length = ebullets.length = bits.length = debris.length = beams.length = pendingBooms.length = 0;
  chunks.n = sparks.n = smoke.n = 0; chunks.update(0); sparks.update(0); smoke.update(0);
  for (const d of decals) { d.life = 0; d.m.visible = false; }
  for (const b of booms) { b.t = 1; b.s.visible = b.r.visible = false; }
  for (const f of floats) { f.t = 9; f.el.style.opacity = 0; } for (const u of bubbles) u.t = 9;
  radioQ.length = 0; radioT = 0; $('radio').classList.remove('show'); $('feed').innerHTML = ''; tokens = 0; lightA.intensity = lightB.intensity = 0;
}
function objLine(m) {
  const o = m.obj;
  return o.type === 'survive' ? `Survive for ${o.time} seconds` : o.type === 'hunt' ? `Eliminate ${o.count} ${o.label}` : o.type === 'boss' ? 'Defeat the King' : `Eliminate all hostiles · ${o.waves.length} waves`;
}
function objText() {
  if (!run) return '';
  const left = bears.length + run.queue.length, o = run.o;
  if (run.done) return 'MISSION COMPLETE';
  if (run.type === 'arcade') return `WAVE ${Math.max(1, wave)} · ${left} HOSTILES`;
  if (run.type === 'survive') { const t = Math.max(0, Math.ceil(run.timer)); return `SURVIVE · ${Math.floor(t / 60)}:${String(t % 60).padStart(2, '0')}`; }
  if (run.type === 'hunt') return `${o.label.toUpperCase()} · ${run.killed}/${o.count}`;
  return `WAVE ${Math.max(1, wave)}/${o.waves.length} · ${left} HOSTILES`;
}
function startRun(id) {
  const m = MISSIONS.find(x => x.id === id) || null;
  ui.hide(); clearAll(); setPalette(m ? m.palette : pick(['morning', 'sunset', 'night', 'overcast'])); buildArena(m ? m.seed : 1 + ((Math.random() * 999) | 0), m ? m.layout : {});
  MOD = mods(); configGuns();
  Object.assign(player, { yaw: 0, pitch: 0, hp: MOD.maxHp, armor: 0, h: 1.7, onGround: true, crouch: false, iframes: 0, sugar: 0, rush: 0, dash: MOD.dashMax });
  player.pos.set(SPAWN.x, 0, SPAWN.z); player.vel.set(0, 0, 0); hpGhost = 1;
  score = wave = combo = comboT = hitstop = slowT = trauma = hurtFlash = kickPitch = kickYaw = landDip = roll = turnV = stepOff = sprintK = slideK = slideT = meleeT = chuteT = 0; tsW = tsP = 1; clock = 0;
  mantle = nearPick = null; jumpReq = slideReq = meleeReq = useReq = false;
  for (const k in stats) stats[k] = 0;
  gi = lastGi = 0; swT = 1; swTo = -1; reloadT = fireCool = recoil = adsT = crossBloom = 0; firing = ads = false; showGun();
  run = { m, id: m ? m.id : 0, type: m ? m.obj.type : 'arcade', o: m ? m.obj : null, waveI: -1, queue: [], spawnT: 2.5, breakT: 2.4, timer: (m && m.obj.time) || 0, killed: 0, spawned: 0, targetT: 3.5, candy: 0, xp: 0, levelUps: 0, done: false, endT: 0, wind: false, seen: {}, t: 0 };
  intro = { type: m ? m.insert || 'para' : pick(['para', 'sdv']), t: 0, dur: 8, wet: false };
  if (intro.type === 'sdv') { intro.dur = 9.5; player.pitch = 0.04; audio.sfx.dive(); } else { player.pitch = -0.5; audio.sfx.wind(); }
  chute.visible = sdv.visible = false;
  state = 'playing'; hudEl.hidden = false; gunPass.enabled = false; $('boss').hidden = true; hudWeapon();
  $('dash').innerHTML = '<i></i>'.repeat(MOD.dashMax);
  if (m) { showCard(`MISSION ${m.id}`, m.title, objLine(m)); m.radio.forEach(t => radio(t)); } else showCard('ARCADE', 'Endless', 'How long can you last?');
  cardT = Math.max(cardT, intro.dur - 2.5);
  audio.sfx.wave(); audio.musicIntensity(0.6); audio.muffle(false); muffled = false; lock();
}
function nextWave() {
  run.waveI++; wave = run.waveI + 1;
  const arcade = run.type === 'arcade', spec = arcade ? arcadeWave(wave) : { mix: run.o.waves[run.waveI] };
  run.queue = []; for (const [t, c] of Object.entries(spec.mix)) if (t !== 'boss') for (let i = 0; i < c; i++) run.queue.push(t);
  shuffle(run.queue); for (let i = 0; i < (spec.mix.boss || 0); i++) run.queue.splice(Math.max(0, run.queue.length - 2), 0, 'boss');
  run.spawnT = 1.2;
  if (arcade) { showCard(`WAVE ${wave}`, spec.title, ''); audio.sfx.wave(); }
  else if (run.waveI > 0) { showCard(`WAVE ${wave} / ${run.o.waves.length}`, run.waveI === run.o.waves.length - 1 ? 'Final Wave' : 'Reinforcements', ''); audio.sfx.wave(); }
}
function maintain(dt, o) {
  run.spawnT -= dt;
  if (run.spawnT <= 0 && bears.reduce((n, b) => n + (b.elite ? 0 : 1), 0) < o.cap) { spawnBear(weighted(o.mix)); run.spawnT = rand(1.1, 2.1); }
}
function updateRun(dt) {
  const o = run.o;
  if (run.type === 'survive') { run.timer -= dt; maintain(dt, o); if (run.timer <= 0) complete(); }
  else if (run.type === 'hunt') {
    maintain(dt, o);
    if (run.killed >= o.count) return complete();
    if (!bears.some(b => b.elite) && run.spawned < o.count) { run.targetT -= dt; if (run.targetT <= 0) { spawnBear(o.target, null, true); run.spawned++; run.targetT = 3.5; announce('TARGET SPOTTED', 'gold'); } }
  } else if (run.queue.length) {
    run.spawnT -= dt;
    if (run.spawnT <= 0 && bears.length < (run.type === 'arcade' ? Math.min(7 + wave, 14) : 10)) { spawnBear(run.queue.pop()); run.spawnT = Math.max(0.35, 1 - wave * 0.05); }
  } else if (!bears.length && !dying.length) {
    if (run.waveI >= 0 && run.type !== 'arcade' && run.waveI >= o.waves.length - 1) return complete();
    if (run.breakT <= 0) { run.breakT = 4; const bonus = wave * 100; score += bonus; announce(`WAVE CLEARED  +${bonus}`); audio.sfx.clear(); player.hp = Math.min(MOD.maxHp, player.hp + 20); }
    else { run.breakT -= dt; if (run.breakT <= 0) nextWave(); }
  }
}
function complete() {
  if (run.done) return; run.done = true; run.endT = 3.4; slowT = 1.4;
  for (const b of bears.slice()) { endAttack(b, 0); removeLaser(b); burstBear(b, true); } bears.length = 0;
  for (const e of ebullets) scene.remove(e.mesh); ebullets.length = 0; tokens = 0; $('boss').hidden = true;
  announce('MISSION COMPLETE', 'gold'); audio.sfx.win(); audio.musicIntensity(0.3);
}
function finishRun(win) {
  const m = run.m, r = { win, m, id: run.id, score, wave, stats: { ...stats }, candy: run.candy, xp: run.xp, stars: 0, bonus: [], unlock: null, levelUps: run.levelUps, newBest: false };
  if (m && win) {
    const b1 = player.hp >= MOD.maxHp * 0.5, b2 = stats.heads >= m.bonus.heads, first = !profile.done[m.id];
    r.stars = 1 + b1 + b2; r.bonus = [['Mission complete', true], ['Finish with 50%+ health', b1], [`${m.bonus.heads} headshots (${stats.heads})`, b2]];
    r.candy += Math.round(m.reward.candy * (first ? 1 : 0.4) * (1 + 0.25 * (r.stars - 1)));
    const bonusXp = Math.round(m.reward.xp * (first ? 1 : 0.4)); r.xp += bonusXp; r.levelUps += addXp(bonusXp);
    if (first && m.reward.unlock) r.unlock = m.reward.unlock;
    profile.done[m.id] = Math.max(profile.done[m.id] || 0, r.stars);
  } else if (!m) { r.candy += Math.floor(score / 150); if (score > (settings.best || 0)) { settings.best = score; r.newBest = true; } }
  profile.candy += r.candy; save();
  state = 'results'; hudEl.hidden = true; firing = false; audio.musicIntensity(0.15); audio.muffle(false); muffled = false;
  if (document.pointerLockElement) document.exitPointerLock();
  ui.show('results', r, true);
}
function toMenu() {
  clearAll(); run = null; setPalette('morning'); buildArena(1);
  const types = ['rifle', 'grunt', 'shield', 'rpg', 'sniper', 'knife', 'rifle', 'grunt'];
  types.forEach((t, i) => {
    const b = makeEnemy(t), a = (i / types.length) * TAU, up = i % 2, r = up ? 3.4 : 6.1;
    b.root.position.set(Math.sin(a) * r, CAKE[up ? 1 : 0][1], Math.cos(a) * r); b.root.rotation.y = a; b.root.scale.setScalar(b.s); b.fsm = 'idle'; scene.add(b.root); props.push(b);
  });
  state = 'menu'; intro = null; chute.visible = sdv.visible = false; hudEl.hidden = true; gunPass.enabled = false; audio.musicIntensity(0.15); audio.muffle(false);
  if (document.pointerLockElement) document.exitPointerLock();
  ui.show('menu', null, true);
}
function startRush() {
  if (state !== 'playing' || player.sugar < 100 || player.rush > 0) return;
  player.rush = MOD.rushTime; audio.sfx.rushOn(); announce('SUGAR RUSH', 'rush'); GUNS.forEach(g => { if (!g.loot) g.ammo = g.clip; }); reloadT = 0;
  if (bears.length) say(pick(bears), pick(L.rush), true);
}
function pause() {
  if (state !== 'playing') return;
  state = 'paused'; firing = ads = false; audio.muffle(true); ui.show('pause', null, true);
  if (document.pointerLockElement) document.exitPointerLock();
}
function lock() {
  if (document.pointerLockElement === canvas) return;
  try { const p = canvas.requestPointerLock(); if (p && p.catch) p.catch(() => { noLock = true; }); } catch { noLock = true; }
}

// ───────────────────────── player ─────────────────────────
function updatePlayer(dt, rdt) {
  const p = player.pos, v = player.vel;
  const f = (keys.KeyW || keys.ArrowUp ? 1 : 0) - (keys.KeyS || keys.ArrowDown ? 1 : 0), s = (keys.KeyD || keys.ArrowRight ? 1 : 0) - (keys.KeyA || keys.ArrowLeft ? 1 : 0);
  const sy = Math.sin(player.yaw), cy = Math.cos(player.yaw);
  let wx = -sy * f + cy * s, wz = -cy * f - sy * s; const wl = Math.hypot(wx, wz); if (wl > 0) { wx /= wl; wz /= wl; }
  turnV = damp(turnV, rdt > 0 ? (mdx * 0.0022 * settings.sens) / rdt : 0, 12, rdt); // how fast you are turning, for the lean
  let sprinting = false;
  if (mantle) { // hauling up onto a ledge: straight up first, then over
    mantle.t += dt / mantle.dur; const t = Math.min(1, mantle.t), over = clamp((t - 0.35) / 0.65, 0, 1);
    p.set(lerp(mantle.fx, mantle.tx, over), lerp(mantle.fy, mantle.ty, Math.min(1, t * 1.5)), lerp(mantle.fz, mantle.tz, over)); v.set(0, 0, 0);
    if (t >= 1) { p.y = mantle.ty; player.onGround = true; mantle = null; landDip = -0.08; }
  } else {
    if (slideReq) {
      slideReq = false; const sp = Math.hypot(v.x, v.z);
      if (player.onGround && slideT <= 0 && sp > 6 && player.dash >= 1) { player.dash -= 1; slideT = 0.9; const k = Math.max(sp + 3.5, 12.5) / sp; v.x *= k; v.z *= k; player.iframes = Math.max(player.iframes, 0.28); audio.sfx.slide(); }
    }
    player.dash = Math.min(MOD.dashMax, player.dash + dt / 1.6);
    if (slideT > 0) { // committed: friction bleeds the speed, you can only nudge the line
      slideT -= dt; const fr = Math.exp(-1.4 * dt); v.x = v.x * fr + wx * 2.5 * dt; v.z = v.z * fr + wz * 2.5 * dt; player.crouch = false;
      if (Math.hypot(v.x, v.z) < 4.5 || !player.onGround) slideT = 0;
    } else {
      player.crouch = !!(keys.KeyC || keys.ControlLeft) && player.onGround;
      sprinting = !!(keys.ShiftLeft || keys.ShiftRight) && f > 0 && player.onGround && !ads && !firing && !player.crouch && meleeT <= 0;
      const speed = (player.crouch ? 3.4 : sprinting ? 9.8 : 6.6) * (1 - adsT * 0.4), acc = Math.min(1, dt * (player.onGround ? 10 : 2.5));
      v.x += (wx * speed - v.x) * acc; v.z += (wz * speed - v.z) * acc;
    }
    if (jumpReq) { jumpReq = false; if (player.onGround) { v.y = 8.8; player.onGround = false; slideT = 0; audio.sfx.jump(); } }
    v.y -= 30 * dt;
    const wasAir = !player.onGround, vy0 = v.y, yPrev = p.y;
    p.addScaledVector(v, dt); pushOut(p, 0.45, p.y);
    const hit = getLastHit();
    if (hit && hit.top && wl > 0 && (wasAir || keys.Space)) { // pushing into something climbable: grab the edge
      const dh = hit.h - p.y, tx = p.x + wx * 1.05, tz = p.z + wz * 1.05;
      if (dh > 0.42 && dh < 2.5) { const top = floorAt(tx, tz, hit.h + 0.01); if (top >= hit.h - 0.01) { mantle = { t: 0, dur: 0.26 + dh * 0.13, fx: p.x, fy: p.y, fz: p.z, tx, ty: top, tz }; slideT = 0; audio.sfx.mantle(); } }
    }
    const d = Math.hypot(p.x, p.z), lim = ARENA_R - 1.6; if (d > lim) { p.x *= lim / d; p.z *= lim / d; }
    const fl = floorAt(p.x, p.z, p.y);
    if (p.y <= fl) {
      if (!wasAir && fl - yPrev > 0.04) stepOff -= fl - yPrev; // walked up a stair: ease the camera instead of popping
      p.y = fl; if (wasAir && vy0 < -7) { landDip = clamp(vy0 * 0.014, -0.32, 0); trauma = Math.min(1, trauma + 0.08); audio.sfx.land(); }
      v.y = 0; player.onGround = true;
    } else if (p.y > fl + 0.05) player.onGround = false;
  }
  player.iframes -= dt;
  player.h = damp(player.h, slideT > 0 ? 0.85 : player.crouch ? 1.12 : 1.7, 12, rdt);
  sprintK = damp(sprintK, sprinting ? 1 : 0, 10, rdt); slideK = damp(slideK, slideT > 0 ? 1 : 0, 12, rdt);
  const moving = player.onGround && slideT <= 0 && !mantle ? Math.hypot(v.x, v.z) : 0, prevBob = bobT;
  bobT += dt * moving * 1.2;
  if (moving > 1.5 && Math.floor(bobT / Math.PI) !== Math.floor(prevBob / Math.PI)) audio.sfx.step(sprinting);
  landDip = damp(landDip, 0, 9, rdt); stepOff = damp(stepOff, 0, 14, rdt); kickPitch = damp(kickPitch, 0, 9, rdt); kickYaw = damp(kickYaw, 0, 9, rdt);
  roll = damp(roll, -s * 0.02 - clamp(turnV * 0.022, -0.075, 0.075) + (slideT > 0 ? 0.07 : 0), 9, rdt); // lean into strafes and turns
  const amp = Math.min(1, moving / 7) * (1 - adsT * 0.7) * (sprinting ? 1.6 : 1), bob = Math.sin(bobT) * 0.032 * amp, sway = Math.cos(bobT * 0.5) * 0.02 * amp, sh = trauma * trauma, mk = mantle ? Math.sin(Math.min(1, mantle.t) * Math.PI) : 0;
  camera.position.set(p.x + sway * cy, p.y + player.h + bob + landDip + stepOff - mk * 0.25, p.z - sway * sy);
  camera.rotation.set(player.pitch + kickPitch - mk * 0.25 + rand(-1, 1) * sh * 0.05, player.yaw + kickYaw + rand(-1, 1) * sh * 0.05, roll + Math.cos(bobT * 0.5) * 0.004 * amp + rand(-1, 1) * sh * 0.04);
  const fov = lerp(75, GUNS[gi].scope ? 16 : 50, adsT) + (player.rush > 0 ? 7 : 0) + sprintK * 5 + slideK * 7;
  if (Math.abs(camera.fov - fov) > 0.05) { camera.fov = damp(camera.fov, fov, 12, rdt); camera.updateProjectionMatrix(); }
}
const HIP = V(), ADS = V();
function updateGun(dt, rdt) {
  fireCool -= dt;
  if (swT < 1) { swT += rdt / 0.34; if (swTo >= 0 && swT >= 0.45) { lastGi = gi; gi = swTo; swTo = -1; showGun(); hudWeapon(); } }
  const g = GUNS[gi], vm = VM[gi];
  if (reloadT > 0) { reloadT -= dt; if (reloadT <= 0) { reloadT = 0; g.ammo = g.clip; } }
  adsT = damp(adsT, ads && !g.noAds && reloadT <= 0 && swT >= 1 && sprintK < 0.3 && meleeT <= 0 && !mantle ? 1 : 0, 14, rdt);
  if (meleeReq) { meleeReq = false; if (meleeT <= 0 && swT >= 1 && !mantle) { meleeT = 0.42; audio.sfx.swing(); } }
  if (meleeT > 0) { const was = meleeT; meleeT -= dt; if (was > 0.27 && meleeT <= 0.27) meleeHit(); }
  if (firing) fire();
  recoil = damp(recoil, 0, 13, dt); crossBloom = damp(crossBloom, 0, 6, rdt);
  if (flashT > 0) { flashT -= rdt; if (flashT <= 0) VM.forEach(v => { v.flash.visible = false; }); }
  if (pumpT > 0) { const was = pumpT; pumpT -= dt; if (was > 0.3 && pumpT <= 0.3) audio.sfx.pump(); VM[1].pump.position.z = -Math.sin(clamp(1 - pumpT / 0.5, 0, 1) * Math.PI) * 0.075; }
  VM[2].ball.visible = GUNS[2].ammo > 0 && reloadT <= 0;
  if (vm.barrels && fireCool > -0.25) vm.barrels.rotation.z += rdt * 30;
  // weapon inertia: the gun trails behind where you are looking and catches up
  swayX = damp(swayX, clamp(mdx * 0.0009, -0.06, 0.06), 9, rdt); swayY = damp(swayY, clamp(mdy * 0.0009, -0.05, 0.05), 9, rdt); mdx = mdy = 0;
  const moving = player.onGround ? Math.min(1, Math.hypot(player.vel.x, player.vel.z) / 8) : 0, hipK = 1 - adsT * 0.85, aimK = 1 - adsT * 0.65;
  const dip = reloadT > 0 ? Math.sin((1 - reloadT / (g.reload * MOD.reload)) * Math.PI) : 0;
  const drop = swT < 0.45 ? swT / 0.45 : swT < 1 ? 1 - (swT - 0.45) / 0.55 : 0;
  const mel = meleeT > 0 ? Math.sin((1 - meleeT / 0.42) * Math.PI) : 0, mk = mantle ? Math.sin(Math.min(1, mantle.t) * Math.PI) : 0;
  HIP.copy(vm.hip); HIP.x = Math.min(vm.hip.x, 0.62 * -vm.hip.z * Math.tan(THREE.MathUtils.degToRad(25)) * camera.aspect); ADS.set(0, -vm.sightY, vm.adsZ);
  gun.position.lerpVectors(HIP, ADS, adsT);
  gun.position.x += Math.cos(bobT * 0.5) * 0.007 * moving * hipK - swayX * 0.5 + Math.sin(clock * 1.3) * 0.0006 - sprintK * 0.03 - mel * 0.06;
  gun.position.y += Math.abs(Math.sin(bobT * 0.5)) * 0.008 * moving * hipK - dip * 0.13 - drop * 0.2 + swayY * 0.5 - landDip * 0.12 + Math.sin(clock * 2.1 + 1) * 0.0005 - sprintK * 0.035 - mk * 0.22 - slideK * 0.02;
  gun.position.z += recoil * 0.035 - mel * 0.16;
  gun.rotation.set(recoil * 0.1 - dip * 0.9 - drop * 1.1 + swayY * 1.5 * aimK - sprintK * 0.3 - mk * 0.9 + mel * 0.15,
    0.11 * hipK + swayX * 2 * aimK + sprintK * 0.7 + mel * 0.9,
    dip * 0.5 - roll * 2 - 0.05 * hipK - clamp(turnV * 0.04, -0.14, 0.14) + sprintK * 0.22 + slideK * 0.28 - mel * 0.5);
  // the holographic reticle floats: it lags a turn and settles, like the real thing
  if (vm.dot) vm.dot.position.set(swayX * 0.09 + Math.sin(clock * 1.3) * 0.0003, vm.sightY + swayY * 0.09 + Math.sin(bobT) * 0.0006 * moving, vm.dot.position.z);
  const scoped = !!g.scope && adsT > 0.85; cls('scope', 'on', scoped); gunPass.enabled = !scoped;
}

// ───────────────────────── insertion: by parachute or under the water ─────────────────────────
const chute = new THREE.Group(), sdv = new THREE.Group(); chute.visible = sdv.visible = false; scene.add(chute, sdv);
{
  const cv = document.createElement('canvas'); cv.width = 256; cv.height = 8; const x = cv.getContext('2d');
  for (let i = 0; i < 16; i++) { x.fillStyle = i === 5 ? '#ff5fb8' : i % 2 ? '#5d6b3c' : '#48552e'; x.fillRect(i * 16, 0, 16, 8); }
  const t = new THREE.CanvasTexture(cv); t.colorSpace = THREE.SRGBColorSpace;
  const canopy = new THREE.Mesh(G.dome, new THREE.MeshStandardMaterial({ map: t, roughness: 0.9, side: THREE.DoubleSide })); canopy.scale.set(4.8, 2.6, 4.8); chute.add(canopy);
  const pts = []; for (let i = 0; i < 12; i++) { const a = (i / 12) * TAU; pts.push(V(Math.cos(a) * 4.7, 0.1, Math.sin(a) * 4.7), V(i % 2 ? 0.25 : -0.25, -5.4, 0)); }
  chute.add(new THREE.LineSegments(new THREE.BufferGeometry().setFromPoints(pts), new THREE.LineBasicMaterial({ color: 0x22241c })));
  const k = new Kit(), hull = std(0x1b1d21, 0.4, 0.6);
  k.tube(hull, 0.17, -0.55, 0.5, 0, 0, 16).ball(hull, 0.17, 0, 0, 0.5, 1, 1, 1.6).tube(std(0xd9a441, 0.5, 0.3), 0.175, 0.1, 0.16, 0, 0, 16).rod(hull, 0.02, -0.34, 0.02, -0.4, 0.34, 0.02, -0.4, 6).tube(hull, 0.23, -0.72, -0.55, 0, 0, 16).ball(glow(4, 3.6, 2.6), 0.05, 0, 0, 0.77);
  sdv.add(k.build()); sdv.rotation.y = Math.PI;
}
function updateIntro(rdt) {
  const I = intro, p = player.pos; I.t += rdt; const t = Math.min(1, I.t / I.dur);
  if (I.type === 'para') { // under canopy from the bay side: fast at first, then the flare
    const k = Math.pow(1 - t, 1.7), sw = Math.sin(I.t * 1.3);
    camera.position.set(p.x + k * 14 + sw * k * 2.5, 105 * k + player.h, p.z + k * 34);
    if (t > 0.72) player.pitch += (0 - player.pitch) * Math.min(1, rdt * 3); // level out for the landing
    camera.rotation.set(player.pitch, player.yaw, sw * 0.07 * Math.min(1, k * 4));
    chute.visible = true; chute.scale.setScalar(1); chute.rotation.set(0, 0, -sw * 0.06); chute.position.set(camera.position.x, camera.position.y + 5.4, camera.position.z);
  } else { // towed in under the surface, then up the boat ramp
    const u = t < 0.6 ? 0 : (t - 0.6) / 0.4, z = t < 0.6 ? lerp(ARENA_R + 52, ARENA_R + 15, t / 0.6) : lerp(ARENA_R + 15, SPAWN.z, u * u * (3 - 2 * u));
    const gy = z > ARENA_R ? Math.max(-3.2, -(z - ARENA_R) * 0.205) : 0, eye = lerp(0.5, player.h, clamp(u * 1.6, 0, 1));
    camera.position.set(p.x + Math.sin(I.t * 0.8) * 0.25 * (1 - u), gy + eye + Math.sin(I.t * 1.7) * 0.06 * (1 - u), z);
    camera.rotation.set(player.pitch, player.yaw, Math.sin(I.t * 0.9) * 0.04 * (1 - u));
    const wet = camera.position.y < WATER_Y; if (I.wet && !wet) { audio.sfx.splash(); trauma = 0.25; } I.wet = wet;
    sdv.visible = u < 0.3; if (sdv.visible) sdv.position.set(camera.position.x, camera.position.y - 0.42, camera.position.z - 0.95);
    if (wet && Math.random() < 0.5) smoke.add(camera.position.x + rand(-0.6, 0.6), camera.position.y - 0.5, camera.position.z - rand(0.5, 2.5), rand(-0.2, 0.2), rand(0.6, 1.4), rand(-0.2, 0.2), rand(0.03, 0.07), 1.2, 0xcfeff5);
  }
  if (t >= 1) {
    intro = null; sdv.visible = false; swT = 0.45; player.vel.set(0, 0, 0);
    if (I.type === 'para') { chuteT = 1.8; landDip = -0.3; trauma = 0.35; audio.sfx.land(); }
  }
}

// ───────────────────────── HUD ─────────────────────────
function updateHud(rdt) {
  set('obj', objText()); set('score', score.toLocaleString()); cls('hud', 'cine', !!intro);
  style('arfill', 'width', player.armor.toFixed(0) + '%'); cls('armor', 'on', player.armor > 0);
  set('prompt', nearPick && !intro ? (nearPick.loot.armor ? 'Riot shield plating  +50 armor' : LOOT[nearPick.loot.kind].name) : ''); cls('promptbox', 'on', !!nearPick && !intro);
  const hpP = clamp(player.hp / MOD.maxHp, 0, 1); hpGhost = Math.max(hpP, damp(hpGhost, hpP, 2.5, rdt));
  style('hpfill', 'width', (hpP * 100).toFixed(1) + '%'); style('hpghost', 'width', (hpGhost * 100).toFixed(1) + '%'); set('hpnum', Math.max(0, Math.ceil(player.hp))); cls('bl', 'low', hpP < 0.3);
  style('sugarfill', 'width', player.sugar.toFixed(1) + '%'); cls('sugar', 'ready', player.sugar >= 100 && player.rush <= 0); cls('sugar', 'active', player.rush > 0); cls('rushfx', 'on', player.rush > 0);
  const pips = $('dash').children; for (let i = 0; i < pips.length; i++) pips[i].classList.toggle('on', player.dash >= i + 1);
  const g = GUNS[gi]; set('ammonum', reloadT > 0 ? '··' : player.rush > 0 ? '∞' : g.ammo); cls('br', 'low', g.ammo <= g.clip * 0.25 && reloadT <= 0 && player.rush <= 0);
  cls('combo', 'show', combo >= 2); if (combo >= 2) { set('combox', 'x' + Math.min(combo, 8)); style('combofill', 'width', ((comboT / 4) * 100).toFixed(0) + '%'); }
  let bh = 0, bm = 0; for (const b of bears) if (b.type === 'boss') { bh += Math.max(0, b.hp); bm += b.maxHp; }
  $('boss').hidden = bm === 0; if (bm) style('bossfill', 'width', ((bh / bm) * 100).toFixed(1) + '%');
  set('lvl', 'LV ' + profile.level); style('xpfill', 'width', ((profile.xp / xpFor(profile.level)) * 100).toFixed(0) + '%'); set('candy', run.candy);
  const gap = 5 + lerp(g.spread, g.ads, adsT) * 260 + crossBloom * 9 + Math.min(1, Math.hypot(player.vel.x, player.vel.z) / 9) * 3;
  style('cross', 'opacity', adsT > 0.6 ? (gi === 1 ? '0.45' : '0') : '1'); $('cross').style.setProperty('--gap', gap.toFixed(1) + 'px');
  if (cardT > 0) { cardT -= rdt; if (cardT <= 0) { $('card').classList.remove('show'); $('bars').classList.remove('show'); } }
}
function applyFx(rdt) {
  const rushT = player.rush > 0 && state !== 'menu' ? 1 : 0, low = state === 'playing' && player.hp < MOD.maxHp * 0.3 ? 0.22 + Math.sin(clock * 6) * 0.08 : 0;
  fx.uRush.value = damp(fx.uRush.value, rushT, 8, rdt);
  fx.uHurt.value = Math.max(hurtFlash * 0.85, low, state === 'dead' ? 0.6 : 0);
  fx.uSat.value = damp(fx.uSat.value, state === 'dead' ? 0.3 : 1.03 + rushT * 0.3, 5, rdt);
  fx.uCA.value = 0.0012 + hurtFlash * 0.006 + fx.uRush.value * 0.004 + slideK * 0.004 + trauma * 0.003;
}

// ───────────────────────── main loop ─────────────────────────
function tick(rdt) {
  clock += rdt; run.t += rdt;
  if (intro) { updateIntro(rdt); chunks.update(rdt); sparks.update(rdt); smoke.update(rdt); updateFloats(rdt); updateHud(rdt); mdx = mdy = 0; return; }
  const rushing = player.rush > 0;
  let ws = rushing ? 0.35 : 1, ps = rushing ? 0.92 : 1;
  if (slowT > 0) { slowT -= rdt; ws = Math.min(ws, 0.2); ps = Math.min(ps, 0.4); }
  tsW = damp(tsW, ws, 14, rdt); tsP = damp(tsP, ps, 14, rdt);
  let hs = 1; if (hitstop > 0) { hitstop -= rdt; hs = 0.03; }
  const pdt = rdt * tsP * hs, wdt = rdt * tsW * hs;
  if (useReq) { useReq = false; takeLoot(); }
  updatePlayer(pdt, rdt); updateGun(pdt, rdt); updateShots(pdt);
  if (state === 'playing') updateBears(wdt);
  if (state === 'playing') updateEbullets(wdt);
  if (state === 'playing') for (const pb of pendingBooms.splice(0)) explode(pb, 3.6, 4, 0, true);
  updateDying(wdt); updateBits(pdt); updateDebris(wdt); chunks.update(wdt); sparks.update(wdt); smoke.update(wdt); updateDecals(wdt); updateBooms(rdt); updateBeams(rdt);
  if (chuteT > 0) { chuteT -= rdt; chute.position.y -= rdt * 2.2; chute.position.z += rdt * 3; chute.rotation.x += rdt * 0.7; chute.scale.multiplyScalar(Math.max(0, 1 - rdt * 0.9)); if (chuteT <= 0) chute.visible = false; }
  if (state !== 'playing') return;
  if (run.done) { run.endT -= rdt; if (run.endT <= 0) return finishRun(true); } else updateRun(wdt);
  if (combo > 0) { comboT -= rdt; if (comboT <= 0) combo = 0; }
  if (rushing) { player.rush -= rdt; player.sugar = Math.max(0, (player.rush / MOD.rushTime) * 100); if (player.rush <= 0) { player.rush = 0; audio.sfx.rushOff(); } }
  barkCd -= rdt; chainT -= rdt; if (chainT <= 0) chainN = 0;
  hurtFlash = Math.max(0, hurtFlash - rdt * 1.6); trauma = Math.max(0, trauma - rdt * 1.5);
  lightA.intensity = damp(lightA.intensity, 0, 20, rdt); lightB.intensity = damp(lightB.intensity, 0, 20, rdt);
  if (player.hp < MOD.maxHp * 0.3) { heartT -= rdt; if (heartT <= 0) { heartT = 0.85; audio.sfx.heart(); } }
  updateRadio(rdt); updateFloats(rdt); updateBubbles(rdt); updateHud(rdt);
  audio.setListener(player.pos.x, player.pos.z, player.yaw); audio.setRate(lerp(1, 0.62, fx.uRush.value));
  if (rushing !== muffled) { muffled = rushing; audio.muffle(rushing); }
  if (!run.done) audio.musicIntensity(player.rush > 0 ? 1 : bears.length ? Math.min(1, 0.55 + bears.length * 0.07) : 0.4);
}
function idleFx(dt) { updateDying(dt); updateDebris(dt); chunks.update(dt); sparks.update(dt); smoke.update(dt); updateBooms(dt); updateBeams(dt); updateFloats(dt); updateBubbles(dt); }
let last = performance.now(), perfT = 0, perfN = 0;
function frame(now) {
  requestAnimationFrame(frame);
  const raw = Math.min(0.1, (now - last) / 1000), rdt = Math.min(0.05, raw); last = now;
  if (state === 'playing') tick(rdt);
  else if (state === 'menu') {
    menuT += rdt * 0.1; camera.position.set(Math.sin(menuT) * 15 + 3, 5.2, Math.cos(menuT) * 15); camera.lookAt(-2.5, 2.7, 0);
    if (camera.fov !== 55) { camera.fov = 55; camera.updateProjectionMatrix(); }
    for (const b of props) animateBear(b, rdt, false, 0, false);
  } else if (state === 'dead') {
    clock += rdt; deadT -= rdt; camera.position.y = damp(camera.position.y, player.pos.y + 0.45, 4, rdt); camera.rotation.z = damp(camera.rotation.z, 0.55, 3, rdt);
    for (const b of bears) animateBear(b, rdt * 0.3, false, 0, false);
    idleFx(rdt * 0.3); hurtFlash = Math.max(0, hurtFlash - rdt);
    if (deadT <= 0) finishRun(false);
  } else if (state === 'results') idleFx(rdt * 0.3);
  applyFx(rdt); updateWorld(rdt); composer.render(rdt);
  if (state === 'playing' && !document.hidden) {
    perfT += raw; perfN++;
    if (perfN >= 150) { if (perfT / perfN > 1 / 40 && getPixelRatio() > 1) setPixelRatio(Math.max(1, getPixelRatio() - 0.25)); perfT = perfN = 0; }
  }
}

// ───────────────────────── input ─────────────────────────
document.addEventListener('pointerlockchange', () => { if (document.pointerLockElement === canvas) noLock = false; else if (state === 'playing' && !noLock) pause(); });
document.addEventListener('pointerlockerror', () => { noLock = true; });
document.addEventListener('visibilitychange', () => { if (document.hidden) pause(); });
const skipIntro = () => { if (intro) intro.t = Math.max(intro.t, intro.dur * 0.94); };
addEventListener('keydown', e => {
  if (e.target instanceof HTMLInputElement) return;
  const first = !keys[e.code]; keys[e.code] = true;
  if (e.code === 'Space' || e.code === 'Tab' || e.code.startsWith('Arrow')) e.preventDefault();
  if (state !== 'playing') return;
  if (e.code === 'Escape' || e.code === 'KeyP') return pause();
  if (!first) return;
  if (intro) { if (e.code === 'Space' || e.code === 'Enter') skipIntro(); return; }
  if (e.code === 'Space') jumpReq = true;
  else if (e.code === 'KeyC' || e.code === 'ControlLeft') slideReq = true;
  else if (e.code === 'KeyV') meleeReq = true;
  else if (e.code === 'KeyE') useReq = true;
  else if (e.code === 'KeyR') startReload();
  else if (e.code === 'KeyF') startRush();
  else if (e.code === 'KeyQ') switchTo(lastGi);
  else if (e.code === 'KeyM') audio.toggleMute();
  else if (/^Digit[1-4]$/.test(e.code)) switchTo(+e.code[5] - 1);
});
addEventListener('keyup', e => { keys[e.code] = false; });
addEventListener('blur', () => { for (const k in keys) keys[k] = false; firing = ads = false; });
document.addEventListener('mousemove', e => {
  if (state !== 'playing' || (document.pointerLockElement !== canvas && !noLock)) return;
  const k = 0.0022 * settings.sens * (1 - adsT * (GUNS[gi].scope ? 0.8 : 0.35));
  player.yaw -= e.movementX * k; player.pitch = clamp(player.pitch - e.movementY * k, -1.5, 1.5); mdx += e.movementX; mdy += e.movementY;
});
canvas.addEventListener('mousedown', e => {
  if (state !== 'playing') return;
  if (document.pointerLockElement !== canvas) lock();
  if (intro) return;
  if (e.button === 0) firing = true; else if (e.button === 2) ads = true; else if (e.button === 1) { meleeReq = true; e.preventDefault(); }
});
addEventListener('mouseup', e => { if (e.button === 0) firing = false; else if (e.button === 2) ads = false; });
addEventListener('contextmenu', e => e.preventDefault());
addEventListener('wheel', e => {
  if (state !== 'playing') return;
  const owned = GUNS.map((g, i) => (g.owned ? i : -1)).filter(i => i >= 0), at = owned.indexOf(swTo >= 0 ? swTo : gi);
  switchTo(owned[(at + (e.deltaY > 0 ? 1 : owned.length - 1)) % owned.length]);
}, { passive: true });
addEventListener('resize', resize);

ui.init({
  play: id => { audio.init(); startRun(id); },
  resume: () => { if (state !== 'paused') return; state = 'playing'; audio.muffle(player.rush > 0); lock(); },
  restart: () => startRun(run ? run.id : 0),
  quit: toMenu,
});
audio.setVolume(settings.volume);
$('loading').hidden = true;
toMenu();
requestAnimationFrame(frame);

// console hook for poking at the game
window.__gummy = {
  get state() { return state; }, get run() { return run; }, get score() { return score; }, get mod() { return MOD; },
  get intro() { return intro; }, get near() { return nearPick; }, get mantle() { return mantle; }, get gi() { return gi; },
  player, bears, shots, ebullets, debris, stats, profile, GUNS, startRun, spawnBear, fire, switchTo, startRush, complete, toMenu, hurt, explode, equipLoot, takeLoot, skipIntro, meleeHit,
  step(dt = 1 / 60, n = 1) { for (let i = 0; i < n && state === 'playing'; i++) tick(dt); },
};
