//! Firearms and gear, modelled to read as the real thing: extruded side profiles, barrels, rails, optics, furniture.
//! Everything is built pointing +Z, in metres.
use crate::mesh::{mat, Kit, Mat, Mesh};
use crate::tex::*;
use glam::Vec3;
use std::f32::consts::{PI, TAU};

const STEEL: Mat = mat(L_METAL, 0x464a53, 0.44, 0.6);
const STEEL_L: Mat = mat(L_METAL, 0x7b818c, 0.3, 0.85);
const POLY: Mat = mat(L_FLAT, 0x3a3d43, 0.62, 0.05);
const TAN: Mat = mat(L_FLAT, 0xa58a5e, 0.72, 0.05);
const WOOD: Mat = mat(L_WOOD, 0xc07a42, 0.5, 0.0).tile(7.0);
const OLIVE: Mat = mat(L_FLAT, 0x3d4a2b, 0.62, 0.25);
const RUBBER: Mat = mat(L_FLAT, 0x151517, 0.9, 0.0);
const BRASS: Mat = mat(L_METAL, 0xc9a24a, 0.3, 1.0);
const FABRIC: Mat = mat(L_FABRIC, 0x2c3626, 0.0, 0.0).tile(12.0);
const GLOVE: Mat = mat(L_FLAT, 0x212428, 0.82, 0.0);
const MAG: Mat = mat(L_FLAT, 0x3a3f47, 0.22, 0.1);
const DOT: Mat = mat(L_FLAT, 0xff2a1a, 0.5, 0.0).glow(1.0);
const LENS: Mat = mat(L_FLAT, 0x2a7fc0, 0.08, 0.0).glow(0.25);
const EXHAUST: Mat = mat(L_FLAT, 0xffb060, 0.5, 0.0).glow(1.0);
const BEANS: [u32; 6] = [0xff3b4e, 0x3fd45c, 0xffc21f, 0xff8a1f, 0xff5fb8, 0x52e0ff];
fn bean(i: usize) -> Mat { mat(L_FLAT, BEANS[i % BEANS.len()], 0.3, 0.0).glow(0.12) }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Wpn { Pistol, Knife, Rifle, Sniper, Rpg, Minigun }

/// Holographic sight body; returns the height of its sight line.
fn optic(k: &mut Kit, z: f32, y: f32) -> f32 {
    k.bx(STEEL, 0.03, 0.012, 0.056, 0.0, y + 0.006, z);
    k.bx(STEEL, 0.004, 0.038, 0.04, -0.02, y + 0.03, z).bx(STEEL, 0.004, 0.038, 0.04, 0.02, y + 0.03, z).bx(STEEL, 0.044, 0.004, 0.04, 0.0, y + 0.051, z);
    k.rod(STEEL, 0.004, [-0.024, y + 0.012, z - 0.012], [-0.031, y + 0.012, z - 0.012], 8, false);
    y + 0.03
}
/// Gloved hands and sleeves for the first-person view (the shooter's right is -X).
fn lead_arm(k: &mut Kit, f: [f32; 3]) {
    k.ball(GLOVE, 0.034, f[0], f[1], f[2], 1.1, 1.0, 1.35);
    k.rod(FABRIC, 0.037, [f[0] + 0.015, f[1] - 0.02, f[2] - 0.03], [0.24, f[1] - 0.2, f[2] - 0.38], 10, true);
}
fn arms(k: &mut Kit, g: [f32; 3], fore: Option<[f32; 3]>) {
    k.ball(GLOVE, 0.034, g[0] - 0.004, g[1], g[2], 1.0, 1.25, 1.1);
    k.rod(FABRIC, 0.037, [g[0] - 0.012, g[1] - 0.02, g[2] - 0.03], [-0.15, g[1] - 0.2, g[2] - 0.36], 10, true);
    if let Some(f) = fore { lead_arm(k, f); }
}

// ───────────────────────── player weapons ─────────────────────────
fn carbine(k: &mut Kit) -> (f32, f32) {
    k.tube(STEEL_L, 0.0085, 0.2, 0.47, 0.0, 0.0, 14, 0.0085).tube(STEEL, 0.0125, 0.47, 0.525, 0.0, 0.0, 10, 0.0125).tube(RUBBER, 0.0062, 0.52, 0.5265, 0.0, 0.0, 8, 0.0062);
    for i in 0..3 { k.bx(RUBBER, 0.028, 0.004, 0.008, 0.0, 0.0, 0.485 + i as f32 * 0.013); }
    k.tube(POLY, 0.026, 0.088, 0.375, 0.0, 0.0, 8, 0.026);
    let mut z = 0.12; while z < 0.36 { k.bx(RUBBER, 0.054, 0.009, 0.03, 0.0, 0.0, z).bx(RUBBER, 0.009, 0.054, 0.03, 0.0, -0.002, z); z += 0.046; }
    k.prof(STEEL, &[&[-0.13, -0.012], &[0.09, -0.012], &[0.09, 0.03], &[0.06, 0.036], &[-0.11, 0.036], &[-0.13, 0.02]], 0.04, 0.0, 0.004);
    k.prof(STEEL, &[&[-0.13, -0.012], &[0.09, -0.012], &[0.092, -0.075], &[0.044, -0.082], &[0.04, -0.036], &[-0.13, -0.036]], 0.036, 0.0, 0.004);
    k.bx(STEEL, 0.008, 0.004, 0.09, 0.0, -0.052, -0.004).bx(STEEL, 0.008, 0.02, 0.004, 0.0, -0.044, -0.048);
    k.bx(RUBBER, 0.004, 0.016, 0.056, -0.0205, 0.012, 0.03).bx(STEEL_L, 0.003, 0.014, 0.05, -0.0215, 0.012, 0.03);
    k.rod(STEEL, 0.006, [-0.02, 0.012, -0.05], [-0.03, 0.012, -0.05], 8, false);
    k.bx(STEEL, 0.034, 0.008, 0.022, 0.0, 0.033, -0.128).bxr(STEEL_L, 0.005, 0.02, 0.006, 0.0, -0.043, -0.004, 0.3, 0.0, 0.0);
    k.prof(TAN, &[&[-0.078, -0.036], &[-0.03, -0.036], &[-0.05, -0.088], &[-0.064, -0.142], &[-0.108, -0.136], &[-0.092, -0.08]], 0.03, 0.0, 0.007);
    k.tube(STEEL, 0.014, -0.31, -0.13, 0.0, 0.0, 14, 0.014);
    k.prof(TAN, &[&[-0.19, 0.022], &[-0.33, 0.022], &[-0.338, -0.09], &[-0.31, -0.096], &[-0.27, -0.035], &[-0.19, -0.03]], 0.034, 0.0, 0.006);
    k.bx(RUBBER, 0.036, 0.116, 0.01, 0.0, -0.035, -0.34);
    k.rail(STEEL, -0.11, 0.37, 0.039);
    k.prof(MAG, &[&[0.047, -0.07], &[0.09, -0.07], &[0.105, -0.16], &[0.112, -0.212], &[0.068, -0.218], &[0.058, -0.16]], 0.024, 0.0, 0.003);
    for i in 0..9 { let f = i as f32; k.ellipsoid(bean(i), Vec3::new(0.004, 0.0062, 0.0085), Vec3::new(0.0125, -0.085 - f * 0.0145, 0.07 + f * 0.0035), 8, false); } // jellybeans showing through the magazine window
    let sight = optic(k, 0.0, 0.045);
    k.bx(TAN, 0.03, 0.022, 0.07, 0.0, 0.056, 0.29).tube(LENS, 0.006, 0.324, 0.3265, 0.006, 0.058, 8, 0.006);
    k.bx(STEEL, 0.006, 0.02, 0.004, 0.0, 0.055, 0.355);
    k.rod(TAN, 0.0135, [0.0, -0.03, 0.2], [0.0, -0.108, 0.192], 10, true);
    arms(k, [0.0, -0.095, -0.07], Some([0.004, -0.085, 0.196]));
    (sight, 0.53)
}
fn shotgun(k: &mut Kit) -> (f32, f32) {
    k.tube(STEEL_L, 0.0115, 0.1, 0.66, 0.0, 0.0, 14, 0.0115).tube(RUBBER, 0.0088, 0.655, 0.662, 0.0, 0.0, 10, 0.0088);
    k.tube(STEEL, 0.0115, 0.1, 0.56, 0.0, -0.027, 14, 0.0115).tube(STEEL_L, 0.013, 0.56, 0.578, 0.0, -0.027, 14, 0.013);
    k.bx(STEEL, 0.016, 0.052, 0.014, 0.0, -0.013, 0.5);
    k.prof(STEEL, &[&[-0.11, 0.024], &[0.1, 0.024], &[0.1, -0.046], &[0.0, -0.052], &[-0.11, -0.04]], 0.04, 0.0, 0.004);
    k.bx(RUBBER, 0.004, 0.02, 0.062, -0.0205, 0.004, 0.02).bx(STEEL_L, 0.003, 0.016, 0.03, -0.0215, 0.004, 0.005);
    k.bx(STEEL, 0.008, 0.004, 0.074, 0.0, -0.082, -0.06).bx(STEEL, 0.008, 0.034, 0.004, 0.0, -0.066, -0.097).bx(STEEL, 0.008, 0.034, 0.004, 0.0, -0.066, -0.023).bxr(STEEL_L, 0.005, 0.02, 0.006, 0.0, -0.062, -0.055, 0.3, 0.0, 0.0);
    k.prof(POLY, &[&[-0.11, 0.02], &[-0.2, 0.014], &[-0.4, -0.012], &[-0.405, -0.115], &[-0.37, -0.12], &[-0.2, -0.055], &[-0.165, -0.055], &[-0.185, -0.132], &[-0.14, -0.137], &[-0.11, -0.045]], 0.036, 0.0, 0.008);
    k.bx(RUBBER, 0.038, 0.108, 0.012, 0.0, -0.064, -0.409);
    k.ball(BRASS, 0.0042, 0.0, 0.0165, 0.645, 1.0, 1.0, 1.0).bx(STEEL, 0.02, 0.008, 0.03, 0.0, 0.028, -0.07).bx(STEEL, 0.004, 0.012, 0.004, -0.008, 0.036, -0.07).bx(STEEL, 0.004, 0.012, 0.004, 0.008, 0.036, -0.07);
    k.bx(POLY, 0.008, 0.034, 0.118, 0.024, -0.006, 0.0);
    for i in 0..5 { let z = -0.044 + i as f32 * 0.022; k.rod(bean(i), 0.0098, [0.032, -0.03, z], [0.032, 0.012, z], 10, false).rod(BRASS, 0.0102, [0.032, 0.012, z], [0.032, 0.02, z], 10, false); } // gumball shells in the saddle
    arms(k, [0.0, -0.1, -0.165], None);
    (0.036, 0.665)
}
fn pump(k: &mut Kit) {
    k.tube(POLY, 0.026, 0.22, 0.4, 0.0, -0.02, 12, 0.026);
    let mut z = 0.235; while z < 0.39 { k.tube(POLY, 0.0285, z, z + 0.009, 0.0, -0.02, 12, 0.0285); z += 0.022; }
    lead_arm(k, [0.004, -0.058, 0.31]);
}
fn launcher(k: &mut Kit) -> (f32, f32) {
    k.tube(STEEL, 0.03, 0.1, 0.3, 0.0, 0.0, 16, 0.03).tube(STEEL_L, 0.033, 0.285, 0.302, 0.0, 0.0, 16, 0.033).tube(RUBBER, 0.0245, 0.298, 0.3035, 0.0, 0.0, 14, 0.0245);
    k.tube(STEEL_L, 0.062, -0.03, 0.094, 0.0, -0.03, 18, 0.062);
    for i in 0..6 { let a = i as f32 / 6.0 * TAU + 0.52; k.tube(RUBBER, 0.017, 0.092, 0.0955, a.cos() * 0.036, -0.03 + a.sin() * 0.036, 10, 0.017).bx(STEEL, 0.006, 0.006, 0.1, (a + 0.52).cos() * 0.06, -0.03 + (a + 0.52).sin() * 0.06, 0.032); }
    k.prof(STEEL, &[&[-0.12, 0.05], &[0.12, 0.05], &[0.12, 0.034], &[-0.04, 0.034], &[-0.04, -0.06], &[-0.12, -0.06]], 0.04, 0.0, 0.004);
    k.bx(STEEL, 0.05, 0.15, 0.012, 0.0, -0.03, 0.102).bx(STEEL, 0.028, 0.016, 0.16, 0.0, -0.1, 0.03);
    k.prof(TAN, &[&[-0.112, -0.06], &[-0.06, -0.06], &[-0.085, -0.168], &[-0.132, -0.162]], 0.03, 0.0, 0.007);
    k.bx(STEEL, 0.007, 0.004, 0.05, 0.0, -0.1, -0.03).bxr(STEEL_L, 0.005, 0.02, 0.006, 0.0, -0.075, -0.045, 0.3, 0.0, 0.0);
    k.bx(STEEL, 0.02, 0.012, 0.15, 0.0, -0.037, 0.2).rod(TAN, 0.014, [0.0, -0.04, 0.2], [0.0, -0.122, 0.19], 10, true);
    k.tube(STEEL, 0.012, -0.36, -0.12, 0.0, 0.02, 14, 0.012).prof(TAN, &[&[-0.33, 0.045], &[-0.375, 0.045], &[-0.38, -0.085], &[-0.345, -0.09], &[-0.33, -0.02]], 0.034, 0.0, 0.006);
    k.rail(STEEL, -0.1, 0.11, 0.053);
    let sight = optic(k, 0.0, 0.059);
    arms(k, [0.0, -0.12, -0.105], Some([0.004, -0.1, 0.194]));
    (sight, 0.31)
}

// ───────────────────────── bear weapons ─────────────────────────
fn pistol(k: &mut Kit) {
    k.prof(STEEL, &[&[-0.075, 0.03], &[0.11, 0.03], &[0.115, 0.022], &[0.115, 0.002], &[-0.075, 0.002]], 0.027, 0.0, 0.003);
    for i in 0..5 { k.bx(RUBBER, 0.028, 0.02, 0.0025, 0.0, 0.016, -0.066 + i as f32 * 0.006); }
    k.bx(RUBBER, 0.003, 0.012, 0.036, -0.0135, 0.018, 0.03).tube(STEEL_L, 0.0065, 0.1, 0.119, 0.0, 0.016, 8, 0.0065);
    k.bx(STEEL, 0.004, 0.006, 0.006, 0.0, 0.034, 0.1).bx(STEEL, 0.014, 0.006, 0.006, 0.0, 0.034, -0.066);
    k.prof(POLY, &[&[-0.075, 0.002], &[0.11, 0.002], &[0.11, -0.016], &[0.046, -0.02], &[0.0, -0.022], &[-0.03, -0.03], &[-0.048, -0.118], &[-0.098, -0.11], &[-0.08, -0.03]], 0.028, 0.0, 0.004);
    k.bx(POLY, 0.006, 0.004, 0.05, 0.0, -0.047, 0.02).bx(POLY, 0.006, 0.028, 0.004, 0.0, -0.034, 0.044).bxr(STEEL_L, 0.004, 0.016, 0.005, 0.0, -0.03, 0.014, 0.3, 0.0, 0.0);
}
fn ak(k: &mut Kit) {
    k.tube(STEEL_L, 0.008, 0.1, 0.6, 0.0, 0.0, 12, 0.008).tube(STEEL, 0.0115, 0.575, 0.618, 0.0, 0.0, 10, 0.0115).tube(STEEL, 0.0085, 0.1, 0.34, 0.0, 0.024, 10, 0.0085);
    k.bx(STEEL, 0.02, 0.04, 0.02, 0.0, 0.012, 0.345).bx(STEEL, 0.016, 0.05, 0.016, 0.0, 0.022, 0.535).bx(STEEL, 0.004, 0.022, 0.004, 0.0, 0.055, 0.535);
    k.prof(STEEL, &[&[-0.15, 0.028], &[-0.13, 0.036], &[0.08, 0.036], &[0.1, 0.028], &[0.1, -0.03], &[-0.15, -0.03]], 0.036, 0.0, 0.004);
    k.bx(STEEL, 0.018, 0.012, 0.05, 0.0, 0.042, 0.075).bx(STEEL_L, 0.014, 0.008, 0.02, -0.024, 0.012, 0.03).bx(RUBBER, 0.003, 0.014, 0.07, -0.0185, 0.012, 0.0);
    k.prof(WOOD, &[&[0.1, 0.012], &[0.32, 0.01], &[0.32, -0.028], &[0.1, -0.032]], 0.04, 0.0, 0.007);
    k.prof(WOOD, &[&[0.11, 0.04], &[0.3, 0.038], &[0.3, 0.014], &[0.11, 0.014]], 0.032, 0.0, 0.007);
    k.prof(STEEL, &[&[0.015, -0.03], &[0.07, -0.03], &[0.1, -0.11, 0.135, -0.19], &[0.09, -0.205], &[0.058, -0.12, 0.015, -0.03]], 0.024, 0.0, 0.003); // banana magazine
    k.prof(WOOD, &[&[-0.085, -0.03], &[-0.04, -0.03], &[-0.065, -0.125], &[-0.11, -0.12]], 0.028, 0.0, 0.007);
    k.bx(STEEL, 0.006, 0.004, 0.075, 0.0, -0.075, -0.005).bx(STEEL, 0.006, 0.045, 0.004, 0.0, -0.053, 0.03).bxr(STEEL_L, 0.004, 0.02, 0.005, 0.0, -0.045, -0.01, 0.3, 0.0, 0.0);
    k.prof(WOOD, &[&[-0.15, 0.022], &[-0.4, -0.002], &[-0.405, -0.1], &[-0.375, -0.104], &[-0.15, -0.03]], 0.034, 0.0, 0.008);
    k.bx(STEEL, 0.036, 0.1, 0.006, 0.0, -0.05, -0.407);
}
fn sniper(k: &mut Kit) {
    k.tube(STEEL_L, 0.011, 0.12, 0.74, 0.0, 0.012, 12, 0.008).bx(STEEL, 0.026, 0.02, 0.05, 0.0, 0.012, 0.762).bx(RUBBER, 0.03, 0.007, 0.01, 0.0, 0.012, 0.752).bx(RUBBER, 0.03, 0.007, 0.01, 0.0, 0.012, 0.772);
    k.tube(STEEL, 0.016, -0.13, 0.13, 0.0, 0.012, 14, 0.016).rod(STEEL_L, 0.004, [-0.018, 0.012, -0.07], [-0.05, -0.006, -0.075], 8, false).ball(STEEL_L, 0.008, -0.05, -0.006, -0.075, 1.0, 1.0, 1.0);
    k.prof(OLIVE, &[&[0.34, 0.0], &[0.34, -0.034], &[0.02, -0.046], &[-0.02, -0.05], &[-0.045, -0.13], &[-0.09, -0.125], &[-0.085, -0.05], &[-0.16, -0.045], &[-0.36, -0.07], &[-0.42, -0.07], &[-0.425, 0.035], &[-0.36, 0.04], &[-0.3, 0.012], &[-0.13, 0.0]], 0.044, 0.0, 0.008);
    k.bx(RUBBER, 0.046, 0.11, 0.012, 0.0, -0.018, -0.431).bx(POLY, 0.03, 0.02, 0.1, 0.0, 0.05, -0.33).bx(STEEL, 0.024, 0.06, 0.05, 0.0, -0.07, 0.05);
    k.tube(STEEL, 0.015, -0.1, 0.16, 0.0, 0.068, 14, 0.015).tube(STEEL, 0.015, 0.16, 0.2, 0.0, 0.068, 14, 0.025).tube(STEEL, 0.025, 0.2, 0.262, 0.0, 0.068, 14, 0.025).tube(STEEL, 0.02, -0.17, -0.1, 0.0, 0.068, 14, 0.02);
    k.rod(STEEL, 0.009, [0.0, 0.08, 0.03], [0.0, 0.1, 0.03], 8, false).rod(STEEL, 0.009, [-0.014, 0.068, 0.03], [-0.036, 0.068, 0.03], 8, false).bx(STEEL, 0.016, 0.03, 0.014, 0.0, 0.045, -0.04).bx(STEEL, 0.016, 0.03, 0.014, 0.0, 0.045, 0.11);
    k.ball(LENS, 0.022, 0.0, 0.068, 0.26, 1.0, 1.0, 0.12);
    k.rod(STEEL, 0.005, [0.0, -0.034, 0.3], [0.06, -0.2, 0.36], 6, false).rod(STEEL, 0.005, [0.0, -0.034, 0.3], [-0.06, -0.2, 0.36], 6, false);
}
fn rpg(k: &mut Kit) {
    k.tube(STEEL, 0.021, -0.42, 0.42, 0.0, 0.0, 14, 0.021).tube(STEEL_L, 0.026, 0.4, 0.43, 0.0, 0.0, 14, 0.026).tube(STEEL, 0.05, -0.52, -0.42, 0.0, 0.0, 14, 0.021);
    k.tube(WOOD, 0.029, -0.22, 0.02, 0.0, 0.0, 14, 0.029);
    k.prof(WOOD, &[&[0.1, -0.02], &[0.14, -0.02], &[0.125, -0.11], &[0.085, -0.105]], 0.026, 0.0, 0.006).prof(WOOD, &[&[0.0, -0.02], &[0.04, -0.02], &[0.02, -0.1], &[-0.02, -0.095]], 0.026, 0.0, 0.006);
    k.bx(STEEL, 0.008, 0.004, 0.07, 0.0, -0.06, 0.09).bx(STEEL, 0.03, 0.05, 0.09, 0.036, 0.045, 0.12).tube(LENS, 0.011, 0.164, 0.167, 0.036, 0.048, 10, 0.011);
    k.bx(STEEL, 0.004, 0.03, 0.004, 0.0, 0.035, 0.36).bx(STEEL, 0.012, 0.025, 0.004, 0.0, 0.033, 0.2);
}
fn warhead(k: &mut Kit) {
    k.tube(OLIVE, 0.012, 0.3, 0.46, 0.0, 0.0, 10, 0.012).tube(OLIVE, 0.022, 0.46, 0.52, 0.0, 0.0, 14, 0.043).tube(OLIVE, 0.043, 0.52, 0.6, 0.0, 0.0, 14, 0.043).tube(OLIVE, 0.043, 0.6, 0.72, 0.0, 0.0, 14, 0.012).tube(STEEL_L, 0.012, 0.72, 0.765, 0.0, 0.0, 10, 0.007);
}
fn minigun(k: &mut Kit) {
    k.tube(STEEL, 0.055, -0.1, 0.12, 0.0, 0.0, 16, 0.055).bx(STEEL, 0.1, 0.1, 0.16, 0.0, 0.0, -0.17).bx(OLIVE, 0.09, 0.13, 0.12, 0.0, -0.115, -0.15);
    k.rod(STEEL, 0.008, [0.0, 0.05, -0.2], [0.0, 0.1, -0.2], 8, false).rod(STEEL, 0.008, [0.0, 0.1, -0.2], [0.0, 0.1, 0.04], 8, false).rod(STEEL, 0.008, [0.0, 0.1, 0.04], [0.0, 0.05, 0.04], 8, false);
    k.rod(POLY, 0.012, [-0.04, -0.02, -0.25], [-0.04, -0.02, -0.34], 8, false).rod(POLY, 0.012, [0.04, -0.02, -0.25], [0.04, -0.02, -0.34], 8, false);
    for i in 0..7 { let f = i as f32; k.bx(BRASS, 0.05, 0.012, 0.012, 0.055, -0.07 + f * 0.012, -0.15 + (f * 0.5).sin() * 0.03); }
}
fn barrels(k: &mut Kit) {
    for i in 0..6 { let a = i as f32 / 6.0 * TAU; k.tube(STEEL_L, 0.008, 0.12, 0.62, a.cos() * 0.028, a.sin() * 0.028, 8, 0.008); }
    k.tube(STEEL, 0.04, 0.3, 0.315, 0.0, 0.0, 14, 0.04).tube(STEEL, 0.04, 0.5, 0.515, 0.0, 0.0, 14, 0.04).tube(STEEL, 0.04, 0.6, 0.612, 0.0, 0.0, 14, 0.04).tube(STEEL, 0.012, 0.12, 0.6, 0.0, 0.0, 10, 0.012);
}
fn knife(k: &mut Kit) {
    k.prof(STEEL_L, &[&[0.01, -0.014], &[0.01, 0.014], &[0.15, 0.014], &[0.2, -0.004], &[0.185, -0.014]], 0.004, 0.0, 0.001);
    k.bx(STEEL, 0.012, 0.05, 0.008, 0.0, 0.0, 0.008).prof(POLY, &[&[-0.11, -0.014], &[-0.11, 0.012], &[0.0, 0.014], &[0.0, -0.016]], 0.022, 0.0, 0.006);
}
fn build(f: impl Fn(&mut Kit), s: f32) -> Mesh { let mut k = Kit::new(); f(&mut k); k.m.scaled(s) }

/// A weapon as a bear carries it: scaled up to paw size, with moving parts split out.
pub struct BearGun { pub body: Mesh, pub warhead: Option<Mesh>, pub barrels: Option<Mesh>, pub muzzle: Vec3 }
pub fn bear_gun(w: Wpn) -> BearGun {
    let (s, mz): (f32, [f32; 3]) = match w { Wpn::Pistol => (2.0, [0.0, 0.016, 0.12]), Wpn::Rifle => (1.4, [0.0, 0.0, 0.62]), Wpn::Sniper => (1.35, [0.0, 0.012, 0.79]), Wpn::Rpg => (1.35, [0.0, 0.0, 0.78]), Wpn::Minigun => (1.5, [0.0, 0.0, 0.64]), Wpn::Knife => (2.2, [0.0, 0.0, 0.2]) };
    let body = match w { Wpn::Pistol => build(pistol, s), Wpn::Rifle => build(ak, s), Wpn::Sniper => build(sniper, s), Wpn::Rpg => build(rpg, s), Wpn::Minigun => build(minigun, s), Wpn::Knife => build(knife, s) };
    BearGun { body, warhead: (w == Wpn::Rpg).then(|| build(warhead, s)), barrels: (w == Wpn::Minigun).then(|| build(barrels, s)), muzzle: Vec3::from(mz) * s }
}
/// Where a bear's weapon sits relative to its paw.
pub const PAW_OFFSET: Vec3 = Vec3::new(0.0, 0.1, -0.04);
pub fn rocket() -> Mesh {
    let mut k = Kit::new();
    warhead(&mut k);
    k.bx(OLIVE, 0.12, 0.004, 0.06, 0.0, 0.0, 0.32).bx(OLIVE, 0.004, 0.12, 0.06, 0.0, 0.0, 0.32).ball(EXHAUST, 0.03, 0.0, 0.0, 0.27, 1.0, 1.0, 2.6);
    let mut m = k.m.scaled(1.35);
    for v in &mut m.v { v.pos[2] -= 0.7; }
    m
}

/// A weapon as you hold it. Meshes are in the gun's own frame (+Z forward); the game flips them to face away from the camera.
pub struct ViewModel { pub body: Mesh, pub pump: Option<Mesh>, pub ball: Option<Mesh>, pub barrels: Option<Mesh>, pub dot_z: Option<f32>, pub sight_y: f32, pub hip: Vec3, pub ads_z: f32, pub mz: f32 }
fn vm(f: impl Fn(&mut Kit) -> (f32, f32), hip: [f32; 3], ads_z: f32) -> ViewModel {
    let mut k = Kit::new();
    let (sight_y, mz) = f(&mut k);
    ViewModel { body: k.m, pump: None, ball: None, barrels: None, dot_z: None, sight_y, hip: hip.into(), ads_z, mz }
}
pub fn player_guns() -> [ViewModel; 3] {
    let mut c = vm(carbine, [0.135, -0.118, -0.27], -0.17); c.dot_z = Some(0.012);
    let mut s = vm(shotgun, [0.14, -0.105, -0.3], -0.3); s.pump = Some(build(pump, 1.0));
    let mut l = vm(launcher, [0.15, -0.13, -0.3], -0.2); l.dot_z = Some(0.012);
    let mut bk = Kit::new(); for i in 0..8 { let a = i as f32 / 8.0 * PI; bk.ellipsoid(mat(L_FLAT, [0xff3b4e, 0xffffff, 0xffc21f, 0x3fd45c, 0xffffff, 0x52e0ff, 0x9b4dff, 0xff5fb8][i], 0.2, 0.0).glow(0.06), Vec3::new(0.0255 * a.sin().max(0.2), 0.0255 / 8.0 * 1.25, 0.0255 * a.sin().max(0.2)), Vec3::new(0.0, 0.0255 * a.cos(), 0.292), 12, false); }
    l.ball = Some(bk.m);
    [c, s, l]
}
/// First-person versions of the weapons you take off the bears.
pub fn loot_vm(w: Wpn) -> ViewModel {
    match w {
        Wpn::Pistol => vm(|k| { pistol(k); arms(k, [0.0, -0.075, -0.06], None); (0.042, 0.12) }, [0.11, -0.085, -0.36], -0.32),
        Wpn::Sniper => vm(|k| { sniper(k); arms(k, [0.0, -0.1, -0.07], Some([0.004, -0.075, 0.2])); (0.068, 0.79) }, [0.135, -0.12, -0.27], -0.15),
        Wpn::Rpg => vm(|k| { rpg(k); warhead(k); arms(k, [0.0, -0.08, 0.11], Some([0.004, -0.07, 0.02])); (0.06, 0.78) }, [0.16, -0.09, -0.2], -0.2),
        Wpn::Minigun => { let mut v = vm(|k| { minigun(k); arms(k, [-0.04, -0.03, -0.3], Some([0.004, 0.09, -0.08])); (0.12, 0.64) }, [0.14, -0.25, -0.5], -0.5); v.barrels = Some(build(barrels, 1.0)); v }
        _ => vm(|k| { ak(k); arms(k, [0.0, -0.085, -0.065], Some([0.004, -0.065, 0.2])); (0.07, 0.62) }, [0.135, -0.118, -0.27], -0.2),
    }
}
/// The floating reticle of the holographic sight: a dot and a ring.
pub fn reticle() -> Mesh { let mut k = Kit::new(); k.ball(DOT, 0.0011, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0).torus(DOT, 0.0052, 0.00032, Vec3::ZERO, Vec3::Z, 28); k.m }

// ───────────────────────── what bears wear and carry ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Gear { Helmet, Shades, Beret, Bandana, Crown }
/// Headgear, in head-centred coordinates.
pub fn gear(g: Gear) -> Mesh {
    let mut k = Kit::new();
    let (olive, red, gold, black) = (mat(L_FLAT, 0x4d5a34, 0.6, 0.1), mat(L_FABRIC, 0xd9263c, 0.0, 0.0).tile(6.0), mat(L_METAL, 0xffc21f, 0.25, 0.9), mat(L_FLAT, 0x101114, 0.5, 0.3));
    match g {
        Gear::Helmet => { k.ellipsoid(olive, Vec3::new(0.47, 0.43, 0.47), Vec3::new(0.0, 0.06, 0.0), 20, true).cyl(olive, 0.5, 0.5, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.0, 0.09, 0.0), 20, true).bx(black, 0.04, 0.3, 0.03, 0.4, -0.1, 0.1).bx(black, 0.04, 0.3, 0.03, -0.4, -0.1, 0.1); }
        Gear::Shades => { let m = mat(L_FLAT, 0x050508, 0.08, 1.0); k.bx(m, 0.54, 0.12, 0.05, 0.0, 0.1, 0.37).bx(m, 0.03, 0.04, 0.4, 0.27, 0.11, 0.18).bx(m, 0.03, 0.04, 0.4, -0.27, 0.11, 0.18); }
        Gear::Beret => { k.ellipsoid(red, Vec3::new(0.4, 0.1, 0.4), Vec3::new(0.07, 0.37, 0.0), 16, false).ball(red, 0.04, 0.02, 0.47, 0.0, 1.0, 1.0, 1.0); }
        Gear::Bandana => { k.torus(red, 0.4, 0.05, Vec3::new(0.0, 0.19, 0.0), Vec3::Y, 24).bxr(red, 0.07, 0.03, 0.26, 0.06, 0.14, -0.5, 0.4, 0.0, 0.0).bxr(red, 0.07, 0.03, 0.22, -0.06, 0.1, -0.48, 0.7, 0.0, 0.0); }
        Gear::Crown => {
            k.cyl(gold, 0.26, 0.29, Vec3::new(0.0, 0.36, 0.0), Vec3::new(0.0, 0.48, 0.0), 12, true);
            for i in 0..5 { let a = i as f32 / 5.0 * TAU; k.cyl(gold, 0.07, 0.0, Vec3::new(a.cos() * 0.22, 0.46, a.sin() * 0.22), Vec3::new(a.cos() * 0.22, 0.66, a.sin() * 0.22), 8, false); }
            k.ball(mat(L_FLAT, 0xff2060, 0.2, 0.0).glow(0.5), 0.045, 0.0, 0.5, 0.27, 1.0, 1.0, 1.0);
        }
    }
    k.m
}
/// Riot shield, in the frame of the left paw.
pub fn shield() -> Mesh {
    let mut k = Kit::new();
    let (navy, steel) = (mat(L_METAL, 0x1b2a4a, 0.3, 0.5), mat(L_METAL, 0xc2c9d2, 0.22, 1.0));
    k.bx(navy, 0.95, 1.3, 0.06, 0.0, 0.0, 0.0).bx(steel, 1.0, 0.05, 0.07, 0.0, 0.65, 0.0).bx(steel, 1.0, 0.05, 0.07, 0.0, -0.65, 0.0).bx(mat(L_FLAT, 0x9fd8ff, 0.05, 0.0), 0.52, 0.16, 0.07, 0.0, 0.36, 0.0).bx(mat(L_FLAT, 0xf2f2f2, 0.6, 0.0), 0.7, 0.14, 0.065, 0.0, -0.12, 0.0);
    k.m
}
pub const SHIELD_OFFSET: Vec3 = Vec3::new(-0.36, -0.22, 0.14);
/// The gold diamond over a marked target.
pub fn marker() -> Mesh {
    let mut k = Kit::new();
    let m = mat(L_FLAT, 0xffc21f, 0.3, 0.0).glow(1.0);
    k.cyl(m, 0.22, 0.0, Vec3::ZERO, Vec3::new(0.0, 0.26, 0.0), 4, false).cyl(m, 0.22, 0.0, Vec3::ZERO, Vec3::new(0.0, -0.26, 0.0), 4, false);
    k.m
}
/// Small shared shapes: a unit sphere (tinted per instance) and a jellybean.
pub fn unit_sphere() -> Mesh { let mut k = Kit::new(); k.ellipsoid(mat(L_FLAT, 0xffffff, 0.25, 0.0), Vec3::ONE, Vec3::ZERO, 12, false); k.m }
pub fn jellybean() -> Mesh { let mut k = Kit::new(); k.capsule(mat(L_FLAT, 0xffffff, 0.3, 0.0), 0.075, Vec3::new(0.0, 0.0, -0.065), Vec3::new(0.0, 0.0, 0.065), 10); k.m }
/// Parachute canopy and lines, hanging point at the origin.
pub fn parachute() -> Mesh {
    let mut k = Kit::new();
    let (seg, rings) = (16, 6);
    for i in 0..seg {
        let c = if i == 5 { 0xff5fb8 } else if i % 2 == 0 { 0x48552e } else { 0x5d6b3c };
        let m = mat(L_FABRIC, c, 0.0, 0.0).tile(2.0);
        for j in 0..rings {
            let p = |ii: usize, jj: usize| { let (a, t) = (ii as f32 / seg as f32 * TAU, jj as f32 / rings as f32 * PI * 0.5); Vec3::new(a.cos() * t.sin() * 4.8, 5.4 + t.cos() * 2.6, a.sin() * t.sin() * 4.8) };
            k.quad(m, p(i, j + 1), p(i + 1, j + 1), p(i + 1, j), p(i, j));
        }
    }
    for i in 0..12 { let a = i as f32 / 12.0 * TAU; k.cyl(mat(L_FLAT, 0x22241c, 0.8, 0.0), 0.012, 0.012, Vec3::new(a.cos() * 4.7, 5.5, a.sin() * 4.7), Vec3::new(if i % 2 == 0 { 0.25 } else { -0.25 }, 0.0, 0.0), 4, false); }
    k.m
}
/// Diver propulsion scooter, nose along +Z.
pub fn scooter() -> Mesh {
    let mut k = Kit::new();
    let hull = mat(L_METAL, 0x1b1d21, 0.4, 0.6);
    k.tube(hull, 0.17, -0.55, 0.5, 0.0, 0.0, 16, 0.17).ball(hull, 0.17, 0.0, 0.0, 0.5, 1.0, 1.0, 1.6).tube(mat(L_METAL, 0xd9a441, 0.5, 0.3), 0.175, 0.1, 0.16, 0.0, 0.0, 16, 0.175)
        .rod(hull, 0.02, [-0.34, 0.02, -0.4], [0.34, 0.02, -0.4], 6, false).tube(hull, 0.23, -0.72, -0.55, 0.0, 0.0, 16, 0.23).ball(mat(L_FLAT, 0xfff2c0, 0.3, 0.0).glow(1.0), 0.05, 0.0, 0.0, 0.77, 1.0, 1.0, 1.0);
    k.m
}
