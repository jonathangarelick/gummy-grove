//! Gummy bears as signed-distance shapes: a pose becomes a handful of ellipsoids and capsules the shader ray-marches.
use crate::gfx::GummyU;
use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Copy)]
pub struct BearPose {
    pub pos: Vec3, pub yaw: f32, pub scale: f32,
    pub tilt_x: f32,              // whole-body pitch (the inflatable in the bay lies on its back)
    pub hop: f32, pub lean: f32, pub squash: Vec3,
    pub arm_r: Option<(f32, f32)>, pub arm_l: Option<(f32, f32)>, // (pitch, yaw) of each arm, None once shot off
    pub head: Option<f32>,        // head pitch, None once popped
}
impl BearPose {
    pub fn idle(pos: Vec3, yaw: f32, scale: f32) -> BearPose {
        BearPose { pos, yaw, scale, tilt_x: 0.0, hop: 0.0, lean: 0.0, squash: Vec3::ONE, arm_r: Some((0.0, 0.0)), arm_l: Some((0.0, 0.0)), head: Some(0.0) }
    }
    /// Transform of the body (root → hop/lean/squash), shared by the SDF and by rigid attachments.
    pub fn body(&self) -> Mat4 {
        Mat4::from_translation(self.pos) * Mat4::from_rotation_y(self.yaw) * Mat4::from_rotation_x(self.tilt_x) * Mat4::from_scale(Vec3::splat(self.scale))
            * Mat4::from_translation(Vec3::new(0.0, self.hop, 0.0)) * Mat4::from_rotation_z(self.lean) * Mat4::from_scale(self.squash)
    }
    pub fn head_xf(&self) -> Mat4 { self.body() * Mat4::from_translation(HEAD) * Mat4::from_rotation_x(self.head.unwrap_or(0.0)) }
    /// Where a held weapon sits: at the end of the right arm, pointing along it.
    pub fn hand_xf(&self, right: bool) -> Mat4 {
        let (pitch, yaw) = if right { self.arm_r } else { self.arm_l }.unwrap_or((0.0, 0.0));
        self.body() * Mat4::from_translation(Vec3::new(if right { -0.48 } else { 0.48 }, 1.18, 0.06)) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch) * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.42))
    }
}
pub const HEAD: Vec3 = Vec3::new(0.0, 1.58, 0.02);

/// Build the shader block for one bear.
pub fn gummy(p: &BearPose, color: Vec3, flash: f32, sun_vis: f32, glow: f32) -> GummyU {
    let model = p.body();
    let mut prims = [Vec4::ZERO; 32];
    let mut n = 0;
    let mut ell = |c: Vec3, r: Vec3| { prims[2 * n] = c.extend(0.0); prims[2 * n + 1] = r.extend(0.0); n += 1; };
    ell(Vec3::new(0.0, 0.85, 0.0), Vec3::new(0.55, 0.62, 0.48));
    ell(Vec3::new(0.0, 0.8, 0.14), Vec3::new(0.4, 0.42, 0.4));
    ell(Vec3::new(-0.3, 0.26, 0.2), Vec3::new(0.2, 0.2, 0.32));
    ell(Vec3::new(0.3, 0.26, 0.2), Vec3::new(0.2, 0.2, 0.32));
    let mut eyes = [Vec4::new(0.0, -99.0, 0.0, 0.01); 3];
    if let Some(hp) = p.head {
        let hr = Mat4::from_rotation_x(hp);
        let at = |o: Vec3| HEAD + hr.transform_vector3(o);
        ell(HEAD, Vec3::new(0.42, 0.4, 0.4));
        ell(at(Vec3::new(-0.3, 0.34, -0.02)), Vec3::new(0.16, 0.16, 0.11));
        ell(at(Vec3::new(0.3, 0.34, -0.02)), Vec3::new(0.16, 0.16, 0.11));
        ell(at(Vec3::new(0.0, -0.08, 0.32)), Vec3::new(0.19, 0.15, 0.15));
        eyes = [at(Vec3::new(-0.16, 0.1, 0.36)).extend(0.06), at(Vec3::new(0.16, 0.1, 0.36)).extend(0.06), at(Vec3::new(0.0, -0.03, 0.47)).extend(0.055)];
    }
    for (side, arm) in [(-1.0f32, p.arm_r), (1.0, p.arm_l)] {
        if let Some((pitch, yaw)) = arm {
            let piv = Vec3::new(side * 0.48, 1.18, 0.06);
            let dir = (Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch)).transform_vector3(Vec3::Z);
            prims[2 * n] = (piv + dir * 0.04).extend(1.0); prims[2 * n + 1] = (piv + dir * 0.4).extend(0.15); n += 1;
        }
    }
    GummyU { inv_model: model.inverse(), model, color: color.extend(flash), params: Vec4::new(n as f32, sun_vis, glow, p.scale), eyes, prims }
}
