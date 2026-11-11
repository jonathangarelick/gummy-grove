// Firearms, modelled to read as the real thing: extruded side profiles, barrels, rails, optics and furniture,
// merged per material so each gun is only a handful of draw calls. Everything is built pointing +Z, in metres.
import { THREE, G, glow, std } from './core.js';
import { Kit, inst } from './kit.js';

const STEEL = std(0x464a53, 0.44, 0.6), STEEL_L = std(0x7b818c, 0.3, 0.85), POLY = std(0x2a2d33, 0.68, 0.05), TAN = std(0xa58a5e, 0.72, 0.05);
const WOOD = new THREE.MeshPhysicalMaterial({ color: 0x7a3f1c, roughness: 0.5, clearcoat: 0.5, clearcoatRoughness: 0.35 });
const OLIVE = std(0x3d4a2b, 0.62, 0.25), RUBBER = std(0x08080a, 0.95), BRASS = std(0xc9a24a, 0.3, 1), FABRIC = std(0x2c3626, 0.95), GLOVE = std(0x15171a, 0.85);
const GLASS = new THREE.MeshPhysicalMaterial({ color: 0x9fd0ff, transparent: true, opacity: 0.14, roughness: 0.02, depthWrite: false });
const MAG = new THREE.MeshPhysicalMaterial({ color: 0x3a3f47, transparent: true, opacity: 0.5, roughness: 0.15, depthWrite: false });
const DOT = glow(9, 0.35, 0.25), LENS = glow(0.25, 1.3, 2.2);
const FLASH = glow(6, 4, 1.4, { transparent: true, blending: THREE.AdditiveBlending, depthWrite: false });
const EXHAUST = glow(6, 3, 0.8);
const BEANS = [0xff3b4e, 0x3fd45c, 0xffc21f, 0xff8a1f, 0xff5fb8, 0x52e0ff].map(c => std(c, 0.3, 0, { emissive: c, emissiveIntensity: 0.9 }));

function makeFlash(s) {
  const g = new THREE.Group(), a = new THREE.Mesh(G.sphere, FLASH); a.scale.set(0.045 * s, 0.045 * s, 0.2 * s); a.position.z = 0.1 * s; g.add(a);
  for (let i = 0; i < 3; i++) { const b = new THREE.Mesh(G.box, FLASH); b.scale.set(0.24 * s, 0.012 * s, 0.04 * s); b.rotation.z = (i / 3) * Math.PI; g.add(b); }
  return g;
}
// holographic sight; returns the height of its sight line
function optic(k, z, y) {
  k.box(STEEL, 0.03, 0.012, 0.056, 0, y + 0.006, z);
  k.box(STEEL, 0.004, 0.038, 0.04, -0.02, y + 0.03, z).box(STEEL, 0.004, 0.038, 0.04, 0.02, y + 0.03, z).box(STEEL, 0.044, 0.004, 0.04, 0, y + 0.051, z);
  k.box(GLASS, 0.036, 0.036, 0.002, 0, y + 0.03, z + 0.012);
  k.rod(STEEL, 0.004, -0.024, y + 0.012, z - 0.012, -0.031, y + 0.012, z - 0.012, 8);
  return y + 0.03;
}
// gloved hands and sleeves for the first-person view (shooter's right is -X)
function arms(k, grip, fore) {
  k.ball(GLOVE, 0.034, grip[0] - 0.004, grip[1], grip[2], 1, 1.25, 1.1);
  k.rod(FABRIC, 0.037, grip[0] - 0.012, grip[1] - 0.02, grip[2] - 0.03, -0.15, grip[1] - 0.2, grip[2] - 0.36, 10, true);
  if (fore) lead(k, fore);
}
function lead(k, fore) {
  k.ball(GLOVE, 0.034, fore[0], fore[1], fore[2], 1.1, 1, 1.35);
  k.rod(FABRIC, 0.037, fore[0] + 0.015, fore[1] - 0.02, fore[2] - 0.03, 0.24, fore[1] - 0.2, fore[2] - 0.38, 10, true);
}

// ───────────────────────── player weapons ─────────────────────────
function carbine(k) {
  k.tube(STEEL_L, 0.0085, 0.2, 0.47).tube(STEEL, 0.0125, 0.47, 0.525, 0, 0, 10).tube(RUBBER, 0.0062, 0.52, 0.5265, 0, 0, 8);
  for (let i = 0; i < 3; i++) k.box(RUBBER, 0.028, 0.004, 0.008, 0, 0, 0.485 + i * 0.013);                                  // flash hider slots
  k.tube(POLY, 0.026, 0.088, 0.375, 0, 0, 8);                                                                              // free-float handguard
  for (let z = 0.12; z < 0.36; z += 0.046) k.box(RUBBER, 0.054, 0.009, 0.03, 0, 0, z).box(RUBBER, 0.009, 0.054, 0.03, 0, -0.002, z); // slots
  k.prof(STEEL, [[-0.13, -0.012], [0.09, -0.012], [0.09, 0.03], [0.06, 0.036], [-0.11, 0.036], [-0.13, 0.02]], 0.04);       // upper receiver
  k.prof(STEEL, [[-0.13, -0.012], [0.09, -0.012], [0.092, -0.075], [0.044, -0.082], [0.04, -0.05], [-0.048, -0.05], [-0.06, -0.036], [-0.13, -0.036]], 0.036, 0, 0.004, [[-0.04, -0.02], [0.03, -0.02], [0.03, -0.042], [-0.04, -0.042]]); // lower + trigger guard
  k.box(RUBBER, 0.004, 0.016, 0.056, -0.0205, 0.012, 0.03).box(STEEL_L, 0.003, 0.014, 0.05, -0.0215, 0.012, 0.03);          // ejection port + bolt
  k.rod(STEEL, 0.006, -0.02, 0.012, -0.05, -0.03, 0.012, -0.05, 8);                                                        // forward assist
  k.box(STEEL, 0.034, 0.008, 0.022, 0, 0.033, -0.128).box(STEEL_L, 0.005, 0.02, 0.006, 0, -0.03, -0.004, 0.3);              // charging handle, trigger
  k.prof(TAN, [[-0.078, -0.036], [-0.03, -0.036], [-0.05, -0.088], [-0.064, -0.142], [-0.108, -0.136], [-0.092, -0.08]], 0.03, 0, 0.007); // pistol grip
  k.tube(STEEL, 0.014, -0.31, -0.13);
  k.prof(TAN, [[-0.19, 0.022], [-0.33, 0.022], [-0.338, -0.09], [-0.31, -0.096], [-0.27, -0.035], [-0.19, -0.03]], 0.034, 0, 0.006); // stock
  k.box(RUBBER, 0.036, 0.116, 0.01, 0, -0.035, -0.34);
  k.rail(STEEL, -0.11, 0.37, 0.039);
  k.prof(MAG, [[0.047, -0.07], [0.09, -0.07], [0.105, -0.16], [0.112, -0.212], [0.068, -0.218], [0.058, -0.16]], 0.024, 0, 0.003); // see-through magazine…
  for (let i = 0; i < 9; i++) k.push(BEANS[i % BEANS.length], new THREE.CapsuleGeometry(0.0058, 0.008, 3, 6), (i % 2 ? 0.004 : -0.004), -0.085 - i * 0.013, 0.07 + i * 0.0023, 1.2, i, 0.4); // …full of jellybeans
  const sightY = optic(k, 0.0, 0.045);
  k.box(TAN, 0.03, 0.022, 0.07, 0, 0.056, 0.29).tube(LENS, 0.006, 0.324, 0.3265, 0.006, 0.058, 8);                         // laser/light unit
  k.box(STEEL, 0.006, 0.02, 0.004, 0, 0.055, 0.355);                                                                       // front backup sight
  k.rod(TAN, 0.0135, 0, -0.03, 0.2, 0, -0.108, 0.192, 10, true);                                                           // vertical foregrip
  arms(k, [0, -0.095, -0.07], [0.004, -0.085, 0.196]);
  return { sightY, mz: 0.53, dotZ: 0.012 };
}
function shotgun(k) {
  k.tube(STEEL_L, 0.0115, 0.1, 0.66).tube(RUBBER, 0.0088, 0.655, 0.662, 0, 0, 10);
  k.tube(STEEL, 0.0115, 0.1, 0.56, 0, -0.027).tube(STEEL_L, 0.013, 0.56, 0.578, 0, -0.027);                                 // magazine tube
  k.box(STEEL, 0.016, 0.052, 0.014, 0, -0.013, 0.5);
  k.prof(STEEL, [[-0.11, 0.024], [0.1, 0.024], [0.1, -0.046], [0.0, -0.052], [-0.11, -0.04]], 0.04);                        // receiver
  k.box(RUBBER, 0.004, 0.02, 0.062, -0.0205, 0.004, 0.02).box(STEEL_L, 0.003, 0.016, 0.03, -0.0215, 0.004, 0.005);
  k.box(STEEL, 0.008, 0.004, 0.074, 0, -0.082, -0.06).box(STEEL, 0.008, 0.034, 0.004, 0, -0.066, -0.097).box(STEEL, 0.008, 0.034, 0.004, 0, -0.066, -0.023).box(STEEL_L, 0.005, 0.02, 0.006, 0, -0.062, -0.055, 0.3);
  k.prof(POLY, [[-0.11, 0.02], [-0.2, 0.014], [-0.4, -0.012], [-0.405, -0.115], [-0.37, -0.12], [-0.2, -0.055], [-0.165, -0.055], [-0.185, -0.132], [-0.14, -0.137], [-0.11, -0.045]], 0.036, 0, 0.008); // stock + grip
  k.box(RUBBER, 0.038, 0.108, 0.012, 0, -0.064, -0.409);
  k.ball(BRASS, 0.0042, 0, 0.0165, 0.645).box(STEEL, 0.02, 0.008, 0.03, 0, 0.028, -0.07).box(STEEL, 0.004, 0.012, 0.004, -0.008, 0.036, -0.07).box(STEEL, 0.004, 0.012, 0.004, 0.008, 0.036, -0.07); // bead + rear sight
  k.box(POLY, 0.008, 0.034, 0.118, 0.024, -0.006, 0.0);                                                                    // shell saddle
  for (let i = 0; i < 5; i++) k.rod(BEANS[i], 0.0098, 0.032, -0.03, -0.044 + i * 0.022, 0.032, 0.012, -0.044 + i * 0.022).rod(BRASS, 0.0102, 0.032, 0.012, -0.044 + i * 0.022, 0.032, 0.02, -0.044 + i * 0.022);
  arms(k, [0, -0.1, -0.165], null);
  return { sightY: 0.036, mz: 0.665 };
}
function pumpKit(k) {
  k.tube(POLY, 0.026, 0.22, 0.4, 0, -0.02, 12);
  for (let z = 0.235; z < 0.39; z += 0.022) k.tube(POLY, 0.0285, z, z + 0.009, 0, -0.02, 12);
  lead(k, [0.004, -0.058, 0.31]);
}
function launcher(k) {
  k.tube(STEEL, 0.03, 0.1, 0.3, 0, 0, 16).tube(STEEL_L, 0.033, 0.285, 0.302, 0, 0, 16).tube(RUBBER, 0.0245, 0.298, 0.3035, 0, 0, 14);
  k.tube(STEEL_L, 0.062, -0.03, 0.094, 0, -0.03, 18);                                                                      // six-shot cylinder
  for (let i = 0; i < 6; i++) { const a = (i / 6) * Math.PI * 2 + 0.52; k.tube(RUBBER, 0.017, 0.092, 0.0955, Math.cos(a) * 0.036, -0.03 + Math.sin(a) * 0.036, 10).box(STEEL, 0.006, 0.006, 0.1, Math.cos(a + 0.52) * 0.06, -0.03 + Math.sin(a + 0.52) * 0.06, 0.032); }
  k.prof(STEEL, [[-0.12, 0.05], [0.12, 0.05], [0.12, 0.034], [-0.04, 0.034], [-0.04, -0.06], [-0.12, -0.06]], 0.04);        // frame
  k.box(STEEL, 0.05, 0.15, 0.012, 0, -0.03, 0.102).box(STEEL, 0.028, 0.016, 0.16, 0, -0.1, 0.03);
  k.prof(TAN, [[-0.112, -0.06], [-0.06, -0.06], [-0.085, -0.168], [-0.132, -0.162]], 0.03, 0, 0.007);                       // grip
  k.box(STEEL, 0.007, 0.004, 0.05, 0, -0.1, -0.03).box(STEEL_L, 0.005, 0.02, 0.006, 0, -0.075, -0.045, 0.3);
  k.box(STEEL, 0.02, 0.012, 0.15, 0, -0.037, 0.2).rod(TAN, 0.014, 0, -0.04, 0.2, 0, -0.122, 0.19, 10, true);                // foregrip
  k.tube(STEEL, 0.012, -0.36, -0.12, 0, 0.02).prof(TAN, [[-0.33, 0.045], [-0.375, 0.045], [-0.38, -0.085], [-0.345, -0.09], [-0.33, -0.02]], 0.034, 0, 0.006);
  k.rail(STEEL, -0.1, 0.11, 0.053);
  const sightY = optic(k, 0.0, 0.059);
  arms(k, [0, -0.12, -0.105], [0.004, -0.1, 0.194]);
  return { sightY, mz: 0.31, dotZ: 0.012 };
}

function ballTex() {
  const cv = document.createElement('canvas'); cv.width = 8; cv.height = 64; const x = cv.getContext('2d');
  ['#ff3b4e', '#ffffff', '#ffc21f', '#3fd45c', '#ffffff', '#52e0ff', '#9b4dff', '#ff5fb8'].forEach((c, i) => { x.fillStyle = c; x.fillRect(0, i * 8, 8, 8); });
  const t = new THREE.CanvasTexture(cv); t.colorSpace = THREE.SRGBColorSpace; return t;
}
export const jawMat = new THREE.MeshStandardMaterial({ map: ballTex(), roughness: 0.2, emissive: 0xffffff, emissiveIntensity: 0.35 });

function vm(build, hip, adsZ, extra) {
  {
    const k = new Kit(), meta = build(k), inner = k.build(), g = new THREE.Group();
    inner.rotation.y = Math.PI; g.add(inner);
    const flash = makeFlash(0.55); flash.position.set(0, 0, meta.mz + 0.02); flash.visible = false; inner.add(flash);
    Object.assign(g, { flash, inner, sightY: meta.sightY, hip: new THREE.Vector3(...hip), adsZ });
    if (meta.dotZ !== undefined) { // holographic reticle: dot + ring, free to drift a little inside the window
      const dot = new THREE.Mesh(G.sphere, DOT), ring = new THREE.Mesh(new THREE.TorusGeometry(0.0052, 0.00032, 6, 28), DOT);
      dot.scale.setScalar(0.0011); g.dot = new THREE.Group(); g.dot.add(dot, ring); g.dot.position.set(0, meta.sightY, meta.dotZ); inner.add(g.dot);
    }
    if (extra) extra(g, inner);
    g.traverse(o => { o.castShadow = false; o.frustumCulled = false; });
    return g;
  }
}
export function makeViewmodels() {
  return [
    vm(carbine, [0.135, -0.118, -0.27], -0.17),
    vm(shotgun, [0.14, -0.105, -0.3], -0.3, (g, inner) => { const p = new Kit(); pumpKit(p); g.pump = p.build(); inner.add(g.pump); }),
    vm(launcher, [0.15, -0.13, -0.3], -0.2, (g, inner) => { g.ball = new THREE.Mesh(G.sphere, jawMat); g.ball.scale.setScalar(0.0255); g.ball.position.set(0, 0, 0.292); inner.add(g.ball); }),
  ];
}

// ───────────────────────── bear weapons ─────────────────────────
function pistol(k) {
  k.prof(STEEL, [[-0.075, 0.03], [0.11, 0.03], [0.115, 0.022], [0.115, 0.002], [-0.075, 0.002]], 0.027, 0, 0.003);          // slide
  for (let i = 0; i < 5; i++) k.box(RUBBER, 0.028, 0.02, 0.0025, 0, 0.016, -0.066 + i * 0.006);
  k.box(RUBBER, 0.003, 0.012, 0.036, -0.0135, 0.018, 0.03).tube(STEEL_L, 0.0065, 0.1, 0.119, 0, 0.016, 8);
  k.box(STEEL, 0.004, 0.006, 0.006, 0, 0.034, 0.1).box(STEEL, 0.014, 0.006, 0.006, 0, 0.034, -0.066);
  k.prof(POLY, [[-0.075, 0.002], [0.11, 0.002], [0.11, -0.016], [0.046, -0.02], [0.044, -0.05], [-0.01, -0.052], [-0.03, -0.03], [-0.048, -0.118], [-0.098, -0.11], [-0.08, -0.03]], 0.028, 0, 0.004, [[-0.002, -0.022], [0.038, -0.022], [0.037, -0.044], [-0.004, -0.045]]);
  k.box(STEEL_L, 0.004, 0.016, 0.005, 0, -0.03, 0.014, 0.3);
}
function ak(k) {
  k.tube(STEEL_L, 0.008, 0.1, 0.6).tube(STEEL, 0.0115, 0.575, 0.618, 0, 0, 10).tube(STEEL, 0.0085, 0.1, 0.34, 0, 0.024);    // barrel, brake, gas tube
  k.box(STEEL, 0.02, 0.04, 0.02, 0, 0.012, 0.345).box(STEEL, 0.016, 0.05, 0.016, 0, 0.022, 0.535).box(STEEL, 0.004, 0.022, 0.004, 0, 0.055, 0.535);
  k.prof(STEEL, [[-0.15, 0.028], [-0.13, 0.036], [0.08, 0.036], [0.1, 0.028], [0.1, -0.03], [-0.15, -0.03]], 0.036);        // receiver + dust cover
  k.box(STEEL, 0.018, 0.012, 0.05, 0, 0.042, 0.075).box(STEEL_L, 0.014, 0.008, 0.02, -0.024, 0.012, 0.03).box(RUBBER, 0.003, 0.014, 0.07, -0.0185, 0.012, 0.0);
  k.prof(WOOD, [[0.1, 0.012], [0.32, 0.01], [0.32, -0.028], [0.1, -0.032]], 0.04, 0, 0.007);                                 // lower handguard
  k.prof(WOOD, [[0.11, 0.04], [0.3, 0.038], [0.3, 0.014], [0.11, 0.014]], 0.032, 0, 0.007);                                  // upper handguard
  k.prof(STEEL, [[0.015, -0.03], [0.07, -0.03], [0.1, -0.11, 0.135, -0.19], [0.09, -0.205], [0.058, -0.12, 0.015, -0.03]], 0.024, 0, 0.003); // banana magazine
  k.prof(WOOD, [[-0.085, -0.03], [-0.04, -0.03], [-0.065, -0.125], [-0.11, -0.12]], 0.028, 0, 0.007);                        // grip
  k.box(STEEL, 0.006, 0.004, 0.075, 0, -0.075, -0.005).box(STEEL, 0.006, 0.045, 0.004, 0, -0.053, 0.03).box(STEEL_L, 0.004, 0.02, 0.005, 0, -0.045, -0.01, 0.3);
  k.prof(WOOD, [[-0.15, 0.022], [-0.4, -0.002], [-0.405, -0.1], [-0.375, -0.104], [-0.15, -0.03]], 0.034, 0, 0.008);         // stock
  k.box(STEEL, 0.036, 0.1, 0.006, 0, -0.05, -0.407);
}
function sniper(k) {
  k.tube(STEEL_L, 0.011, 0.12, 0.74, 0, 0.012, 12, 0.008).box(STEEL, 0.026, 0.02, 0.05, 0, 0.012, 0.762).box(RUBBER, 0.03, 0.007, 0.01, 0, 0.012, 0.752).box(RUBBER, 0.03, 0.007, 0.01, 0, 0.012, 0.772);
  k.tube(STEEL, 0.016, -0.13, 0.13, 0, 0.012).rod(STEEL_L, 0.004, -0.018, 0.012, -0.07, -0.05, -0.006, -0.075).ball(STEEL_L, 0.008, -0.05, -0.006, -0.075); // action + bolt
  k.prof(OLIVE, [[0.34, 0.0], [0.34, -0.034], [0.02, -0.046], [-0.02, -0.05], [-0.045, -0.13], [-0.09, -0.125], [-0.085, -0.05], [-0.16, -0.045], [-0.36, -0.07], [-0.42, -0.07], [-0.425, 0.035], [-0.36, 0.04], [-0.3, 0.012], [-0.13, 0.0]], 0.044, 0, 0.008);
  k.box(RUBBER, 0.046, 0.11, 0.012, 0, -0.018, -0.431).box(POLY, 0.03, 0.02, 0.1, 0, 0.05, -0.33).box(STEEL, 0.024, 0.06, 0.05, 0, -0.07, 0.05);
  k.tube(STEEL, 0.015, -0.1, 0.16, 0, 0.068).tube(STEEL, 0.015, 0.16, 0.2, 0, 0.068, 14, 0.025).tube(STEEL, 0.025, 0.2, 0.262, 0, 0.068).tube(STEEL, 0.02, -0.17, -0.1, 0, 0.068); // scope
  k.rod(STEEL, 0.009, 0, 0.08, 0.03, 0, 0.1, 0.03).rod(STEEL, 0.009, -0.014, 0.068, 0.03, -0.036, 0.068, 0.03).box(STEEL, 0.016, 0.03, 0.014, 0, 0.045, -0.04).box(STEEL, 0.016, 0.03, 0.014, 0, 0.045, 0.11);
  k.ball(LENS, 0.022, 0, 0.068, 0.26, 1, 1, 0.12);
  k.rod(STEEL, 0.005, 0, -0.034, 0.3, 0.06, -0.2, 0.36).rod(STEEL, 0.005, 0, -0.034, 0.3, -0.06, -0.2, 0.36);               // bipod
}
function rpg(k) {
  k.tube(STEEL, 0.021, -0.42, 0.42).tube(STEEL_L, 0.026, 0.4, 0.43).tube(STEEL, 0.05, -0.52, -0.42, 0, 0, 14, 0.021);      // tube, collar, rear flare
  k.tube(WOOD, 0.029, -0.22, 0.02, 0, 0, 14);                                                                              // heat shield
  k.prof(WOOD, [[0.1, -0.02], [0.14, -0.02], [0.125, -0.11], [0.085, -0.105]], 0.026, 0, 0.006).prof(WOOD, [[0.0, -0.02], [0.04, -0.02], [0.02, -0.1], [-0.02, -0.095]], 0.026, 0, 0.006);
  k.box(STEEL, 0.008, 0.004, 0.07, 0, -0.06, 0.09).box(STEEL, 0.03, 0.05, 0.09, 0.036, 0.045, 0.12).tube(LENS, 0.011, 0.164, 0.167, 0.036, 0.048, 10);
  k.box(STEEL, 0.004, 0.03, 0.004, 0, 0.035, 0.36).box(STEEL, 0.012, 0.025, 0.004, 0, 0.033, 0.2);
}
function warhead(k) {
  k.tube(OLIVE, 0.012, 0.3, 0.46).tube(OLIVE, 0.022, 0.46, 0.52, 0, 0, 14, 0.043).tube(OLIVE, 0.043, 0.52, 0.6).tube(OLIVE, 0.043, 0.6, 0.72, 0, 0, 14, 0.012).tube(STEEL_L, 0.012, 0.72, 0.765, 0, 0, 10, 0.007);
}
function minigun(k) {
  k.tube(STEEL, 0.055, -0.1, 0.12, 0, 0, 16).box(STEEL, 0.1, 0.1, 0.16, 0, 0, -0.17).box(OLIVE, 0.09, 0.13, 0.12, 0.0, -0.115, -0.15);
  k.rod(STEEL, 0.008, 0, 0.05, -0.2, 0, 0.1, -0.2).rod(STEEL, 0.008, 0, 0.1, -0.2, 0, 0.1, 0.04).rod(STEEL, 0.008, 0, 0.1, 0.04, 0, 0.05, 0.04); // carry handle
  k.rod(POLY, 0.012, -0.04, -0.02, -0.25, -0.04, -0.02, -0.34).rod(POLY, 0.012, 0.04, -0.02, -0.25, 0.04, -0.02, -0.34);
  for (let i = 0; i < 7; i++) k.box(BRASS, 0.05, 0.012, 0.012, 0.055, -0.07 + i * 0.012, -0.15 + Math.sin(i * 0.5) * 0.03);  // ammo belt
}
function barrels(k) {
  for (let i = 0; i < 6; i++) { const a = (i / 6) * Math.PI * 2; k.tube(STEEL_L, 0.008, 0.12, 0.62, Math.cos(a) * 0.028, Math.sin(a) * 0.028, 8); }
  k.tube(STEEL, 0.04, 0.3, 0.315, 0, 0, 14).tube(STEEL, 0.04, 0.5, 0.515, 0, 0, 14).tube(STEEL, 0.04, 0.6, 0.612, 0, 0, 14).tube(STEEL, 0.012, 0.12, 0.6);
}
function knife(k) {
  k.prof(STEEL_L, [[0.01, -0.014], [0.01, 0.014], [0.15, 0.014], [0.2, -0.004], [0.185, -0.014]], 0.004, 0, 0.001);
  k.box(STEEL, 0.012, 0.05, 0.008, 0, 0, 0.008).prof(POLY, [[-0.11, -0.014], [-0.11, 0.012], [0, 0.014], [0, -0.016]], 0.022, 0, 0.006);
}

const DEF = {
  pistol: { s: 2.0, build: pistol, muzzle: [0, 0.016, 0.12] }, rifle: { s: 1.4, build: ak, muzzle: [0, 0, 0.62] }, sniper: { s: 1.35, build: sniper, muzzle: [0, 0.012, 0.79] },
  rpg: { s: 1.35, build: rpg, muzzle: [0, 0, 0.78] }, minigun: { s: 1.5, build: minigun, muzzle: [0, 0, 0.64] }, knife: { s: 2.2, build: knife, muzzle: [0, 0, 0.2] },
};
const cache = {};
const bake = (fn, s) => { const k = new Kit(); fn(k); return k.parts(s); };
export function makeBearWeapon(kind) {
  const d = DEF[kind] || DEF.knife;
  const c = cache[kind] || (cache[kind] = { body: bake(d.build, d.s), warhead: kind === 'rpg' ? bake(warhead, d.s) : null, barrels: kind === 'minigun' ? bake(barrels, d.s) : null });
  const w = inst(c.body);
  if (c.warhead) { w.warhead = inst(c.warhead); w.add(w.warhead); }
  if (c.barrels) { w.barrels = inst(c.barrels); w.add(w.barrels); }
  const [mx, my, mz] = d.muzzle.map(v => v * d.s);
  const muzzle = new THREE.Object3D(); muzzle.position.set(mx, my, mz); w.add(muzzle); w.muzzle = muzzle;
  const flash = makeFlash(kind === 'minigun' ? 1.6 : kind === 'pistol' ? 1.0 : 1.3); flash.position.set(mx, my, mz); flash.visible = false; w.add(flash); w.flash = flash;
  w.position.set(0, 0.1, -0.04);
  return w;
}
let rocketParts;
export function makeRocket() {
  if (!rocketParts) { const k = new Kit(); warhead(k); k.box(OLIVE, 0.12, 0.004, 0.06, 0, 0, 0.32).box(OLIVE, 0.004, 0.12, 0.06, 0, 0, 0.32).ball(EXHAUST, 0.03, 0, 0, 0.27, 1, 1, 2.6); rocketParts = k.parts(1.35); for (const [g] of rocketParts) g.translate(0, 0, -0.7); }
  return inst(rocketParts);
}

// first-person versions of the weapons you take off the bears
export function makeLootViewmodel(kind) {
  if (kind === 'pistol') return vm(k => { pistol(k); arms(k, [0, -0.075, -0.06], null); return { sightY: 0.042, mz: 0.12 }; }, [0.11, -0.085, -0.36], -0.32);
  if (kind === 'sniper') return vm(k => { sniper(k); arms(k, [0, -0.1, -0.07], [0.004, -0.075, 0.2]); return { sightY: 0.068, mz: 0.79 }; }, [0.135, -0.12, -0.27], -0.15);
  if (kind === 'rpg') return vm(k => { rpg(k); warhead(k); arms(k, [0, -0.08, 0.11], [0.004, -0.07, 0.02]); return { sightY: 0.06, mz: 0.78 }; }, [0.16, -0.09, -0.2], -0.2);
  if (kind === 'minigun') return vm(k => { minigun(k); arms(k, [-0.04, -0.03, -0.3], [0.004, 0.09, -0.08]); return { sightY: 0.12, mz: 0.64 }; }, [0.14, -0.25, -0.5], -0.5, (g, inner) => { const b = new Kit(); barrels(b); g.barrels = b.build(); inner.add(g.barrels); });
  return vm(k => { ak(k); arms(k, [0, -0.085, -0.065], [0.004, -0.065, 0.2]); return { sightY: 0.07, mz: 0.62 }; }, [0.135, -0.118, -0.27], -0.2);
}
