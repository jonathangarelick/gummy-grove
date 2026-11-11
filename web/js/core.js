// Shared helpers, settings, geometry and materials.
import * as THREE from 'three';
export { THREE };

export const TAU = Math.PI * 2;
export const rand = (a, b) => a + Math.random() * (b - a);
export const pick = a => a[(Math.random() * a.length) | 0];
export const clamp = (v, a, b) => (v < a ? a : v > b ? b : v);
export const lerp = (a, b, t) => a + (b - a) * t;
export const damp = (a, b, k, dt) => b + (a - b) * Math.exp(-k * dt);

export const C = { red: 0xff3b4e, green: 0x3fd45c, yellow: 0xffc21f, orange: 0xff8a1f, pink: 0xff5fb8, cyan: 0x52e0ff, purple: 0x9b4dff, blue: 0x2f7bff, white: 0xf2f6ff };
export const CANDY = [C.red, C.green, C.yellow, C.orange, C.pink, C.cyan, C.purple];

// persisted settings
export const settings = { quality: 'high', sens: 1, volume: 0.8, best: 0 };
try { Object.assign(settings, JSON.parse(localStorage.getItem('gummy-grove') || '{}')); } catch {}
export function saveSettings() { try { localStorage.setItem('gummy-grove', JSON.stringify(settings)); } catch {} }

export const G = {
  sphere: new THREE.SphereGeometry(1, 28, 18),
  lowSphere: new THREE.SphereGeometry(1, 10, 7),
  dome: new THREE.SphereGeometry(1, 32, 14, 0, TAU, 0, Math.PI / 2),
  bean: new THREE.CapsuleGeometry(0.075, 0.13, 4, 10),
  cone: new THREE.ConeGeometry(1, 1, 12),
  box: new THREE.BoxGeometry(1, 1, 1),
  cyl: new THREE.CylinderGeometry(1, 1, 1, 16),
  beam: new THREE.CylinderGeometry(1, 1, 1, 6).rotateX(Math.PI / 2), // unit length along Z
};

export const std = (color, roughness, metalness = 0, extra = {}) => new THREE.MeshStandardMaterial({ color, roughness, metalness, ...extra });
export const candy = (color, extra = {}) => new THREE.MeshPhysicalMaterial({ color, roughness: 0.28, clearcoat: 1, clearcoatRoughness: 0.15, ...extra });
// unlit HDR colour: values above 1 get picked up by bloom
export const glow = (r, g, b, extra = {}) => new THREE.MeshBasicMaterial({ color: new THREE.Color(r, g, b), toneMapped: false, ...extra });

export const M = {
  dark: std(0x1c0612, 0.25), gold: std(0xffc21f, 0.25, 0.9), gunmetal: std(0x25282e, 0.42, 0.85), steel: std(0xc2c9d2, 0.22, 1),
  wood: std(0x7a4520, 0.65), olive: std(0x4d5a34, 0.6, 0.1), black: std(0x101114, 0.5, 0.3), red: std(0xd9263c, 0.5),
  shade: std(0x050508, 0.08, 1), navy: std(0x1b2a4a, 0.3, 0.5),
};

const WHITE = new THREE.Color(0xffffff);
// Translucent gummy: light transmits through and picks up colour by thickness, plus a rim glow where the candy is thin.
export function gummyMat(color) {
  const c = new THREE.Color(color), hi = settings.quality === 'high';
  const m = new THREE.MeshPhysicalMaterial({
    color: hi ? c.clone().lerp(WHITE, 0.45) : c, roughness: 0.17, clearcoat: 1, clearcoatRoughness: 0.1,
    emissive: c, emissiveIntensity: hi ? 0.08 : 0.26,
    transmission: hi ? 1 : 0, thickness: 0.85, ior: 1.38, attenuationColor: c, attenuationDistance: 0.5,
    transparent: !hi, opacity: hi ? 1 : 0.93,
  });
  m.userData.base = c;
  m.userData.glow = m.emissiveIntensity;
  m.onBeforeCompile = sh => {
    sh.fragmentShader = sh.fragmentShader.replace('#include <emissivemap_fragment>', `#include <emissivemap_fragment>
      float gRim = pow(1.0 - saturate(dot(normal, normalize(vViewPosition))), 2.2);
      totalEmissiveRadiance += emissive * gRim * 1.6;`);
  };
  m.customProgramCacheKey = () => 'gummy';
  return m;
}
