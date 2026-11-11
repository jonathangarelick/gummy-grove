//! The Metal renderer: deferred shading, MPS ray-traced shadows + ambient occlusion, ray-marched gummies, post.
use crate::mesh::{Mesh, Vertex};
use crate::tex::{Atlas, FontAtlas, Materials, LAYERS, TEX};
use crate::world::{Palette, WATER_Y};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use metal::mps::*;
use metal::*;
use objc::rc::autoreleasepool;
use objc::runtime::Object;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    view_proj: Mat4, inv_view_proj: Mat4, prev_view_proj: Mat4, vm_view_proj: Mat4, inv_vm_view_proj: Mat4, cur_view_proj: Mat4,
    cam_pos: Vec4, prev_cam: Vec4, sun_dir: Vec4, sun_color: Vec4, sky_top: Vec4, sky_mid: Vec4, sky_bot: Vec4, fog: Vec4, ambient: Vec4, water: Vec4,
    res: Vec4, misc: Vec4, post: Vec4, post2: Vec4, counts: Vec4,
    lights: [[Vec4; 2]; 32], spheres: [Vec4; 96],
}
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Inst { pub model: Mat4, pub tint: Vec4, pub misc: Vec4 }
impl Inst { pub fn new(model: Mat4) -> Inst { Inst { model, tint: Vec4::ONE, misc: Vec4::ZERO } } }
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GummyU { pub inv_model: Mat4, pub model: Mat4, pub color: Vec4, pub params: Vec4, pub eyes: [Vec4; 3], pub prims: [Vec4; 32] }
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Sprite { pub pos: [f32; 3], pub size: f32, pub color: [f32; 4], pub axis: [f32; 3], pub kind: f32 }
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UiVert { pub pos: [f32; 2], pub uv: [f32; 2], pub color: [u8; 4], pub shape: [f32; 4] }

pub struct Draw { pub mesh: u32, pub inst: Inst }
pub struct Post { pub saturation: f32, pub rush: f32, pub hurt: f32, pub ca: f32, pub vignette: f32 }
pub struct Frame<'a> {
    pub cam_pos: Vec3, pub view: Mat4, pub fov: f32, pub vm_fov: f32,
    pub pal: Palette, pub time: f32, pub cut: bool,
    pub draws: &'a [Draw], pub vm_draws: &'a [Draw], pub gummies: &'a [GummyU], pub sprites: &'a [Sprite],
    pub lights: &'a [[Vec4; 2]], pub spheres: &'a [[Vec4; 2]], pub decals: &'a Mesh, pub ui: &'a [UiVert], pub post: Post, pub vm_sun: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Blend { Off, Alpha, Premul, DecalAlbedo, RoughUp, Wet, Masked }
const RW: MTLTextureUsage = MTLTextureUsage::from_bits_truncate(MTLTextureUsage::ShaderRead.bits() | MTLTextureUsage::ShaderWrite.bits() | MTLTextureUsage::RenderTarget.bits());

fn tex2d(device: &Device, fmt: MTLPixelFormat, w: u64, h: u64, usage: MTLTextureUsage, shared: bool) -> Texture {
    let d = TextureDescriptor::new();
    d.set_pixel_format(fmt); d.set_width(w.max(1)); d.set_height(h.max(1)); d.set_usage(usage);
    d.set_storage_mode(if shared { MTLStorageMode::Shared } else { MTLStorageMode::Private });
    device.new_texture(&d)
}
fn pipeline(device: &Device, lib: &Library, vs: &str, fs: &str, colors: &[(MTLPixelFormat, Blend)], depth: bool) -> RenderPipelineState {
    let d = RenderPipelineDescriptor::new();
    d.set_vertex_function(Some(&lib.get_function(vs, None).unwrap_or_else(|e| panic!("{vs}: {e}"))));
    d.set_fragment_function(Some(&lib.get_function(fs, None).unwrap_or_else(|e| panic!("{fs}: {e}"))));
    for (i, (fmt, b)) in colors.iter().enumerate() {
        let a = d.color_attachments().object_at(i as u64).unwrap();
        a.set_pixel_format(*fmt);
        if *b == Blend::Off { continue; }
        if *b == Blend::Masked { a.set_write_mask(MTLColorWriteMask::empty()); continue; }
        a.set_blending_enabled(true);
        a.set_rgb_blend_operation(MTLBlendOperation::Add); a.set_alpha_blend_operation(MTLBlendOperation::Add);
        use MTLBlendFactor::*;
        let (sr, dr, sa, da) = match b {
            Blend::Alpha => (SourceAlpha, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            Blend::Premul => (One, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            Blend::DecalAlbedo => (One, OneMinusSourceAlpha, Zero, One),          // keep metallic in alpha
            Blend::RoughUp => (Zero, One, One, OneMinusSourceAlpha),              // paint: leave the normal, roughen
            _ => (SourceAlpha, OneMinusSourceAlpha, Zero, OneMinusSourceAlpha),   // wet: flatten the normal, gloss up
        };
        a.set_source_rgb_blend_factor(sr); a.set_destination_rgb_blend_factor(dr); a.set_source_alpha_blend_factor(sa); a.set_destination_alpha_blend_factor(da);
    }
    if depth { d.set_depth_attachment_pixel_format(MTLPixelFormat::Depth32Float); }
    device.new_render_pipeline_state(&d).unwrap_or_else(|e| panic!("pipeline {vs}/{fs}: {e}"))
}
fn depth_state(device: &Device, cmp: MTLCompareFunction, write: bool) -> DepthStencilState {
    let d = DepthStencilDescriptor::new(); d.set_depth_compare_function(cmp); d.set_depth_write_enabled(write); device.new_depth_stencil_state(&d)
}
fn pass<'a>(colors: &[(&TextureRef, Option<[f64; 4]>)], depth: Option<(&TextureRef, Option<f64>, bool)>) -> &'a RenderPassDescriptorRef {
    let rp = RenderPassDescriptor::new();
    for (i, (t, clear)) in colors.iter().enumerate() {
        let a = rp.color_attachments().object_at(i as u64).unwrap();
        a.set_texture(Some(t)); a.set_store_action(MTLStoreAction::Store);
        match clear { Some(c) => { a.set_load_action(MTLLoadAction::Clear); a.set_clear_color(MTLClearColor::new(c[0], c[1], c[2], c[3])); } None => a.set_load_action(MTLLoadAction::Load) }
    }
    if let Some((t, clear, store)) = depth {
        let a = rp.depth_attachment().unwrap();
        a.set_texture(Some(t)); a.set_store_action(if store { MTLStoreAction::Store } else { MTLStoreAction::DontCare });
        match clear { Some(c) => { a.set_load_action(MTLLoadAction::Clear); a.set_clear_depth(c); } None => a.set_load_action(MTLLoadAction::Load) }
    }
    rp
}
fn halton(mut i: u32, b: u32) -> f32 { let (mut f, mut r) = (1.0, 0.0); while i > 0 { f /= b as f32; r += f * (i % b) as f32; i /= b; } r }

struct Targets {
    w: u64, h: u64, ow: u64, oh: u64, rw: u64, rh: u64,
    g_a: Texture, g_n: Texture, g_e: Texture, depth: Texture, hdr: Texture, lin: Texture, scene: Texture, taa: [Texture; 2],
    bloom: Vec<(Texture, Texture)>, ldr: Texture, fin: Texture, rt_acc: [Texture; 2], rt_tmp: Texture, rt_filt: Texture, rays: Buffer, hits: Buffer,
}
impl Targets {
    fn new(dev: &Device, w: u64, h: u64, ow: u64, oh: u64, rt_scale: u64) -> Targets {
        use MTLPixelFormat::*;
        let rt = MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead;
        let (rw, rh) = (w / rt_scale, h / rt_scale);
        let f16 = |w, h| tex2d(dev, RGBA16Float, w, h, RW, false);
        Targets {
            w, h, ow, oh, rw, rh,
            g_a: tex2d(dev, RGBA8Unorm_sRGB, w, h, rt, false), g_n: tex2d(dev, RGBA16Float, w, h, rt, false), g_e: tex2d(dev, RGBA16Float, w, h, rt, false),
            depth: tex2d(dev, Depth32Float, w, h, rt, false), hdr: f16(w, h), lin: tex2d(dev, R32Float, w, h, rt, false), scene: f16(w, h), taa: [f16(w, h), f16(w, h)],
            bloom: (1..=4).map(|i| (f16(w >> i, h >> i), f16(w >> i, h >> i))).collect(),
            ldr: tex2d(dev, BGRA8Unorm, w, h, RW, false), fin: tex2d(dev, BGRA8Unorm, ow, oh, RW, true),
            rt_acc: [f16(rw, rh), f16(rw, rh)], rt_tmp: f16(rw, rh), rt_filt: f16(rw, rh),
            rays: dev.new_buffer(rw * rh * 2 * 32, MTLResourceOptions::StorageModePrivate), hits: dev.new_buffer(rw * rh * 2 * 4, MTLResourceOptions::StorageModePrivate),
        }
    }
}

pub struct Renderer {
    pub device: Device, queue: CommandQueue,
    p_g: RenderPipelineState, p_dm: RenderPipelineState, p_dw: RenderPipelineState, p_light: RenderPipelineState, p_water: RenderPipelineState, p_gum: RenderPipelineState,
    p_sprite: RenderPipelineState, p_taa: RenderPipelineState, p_down: RenderPipelineState, p_final: RenderPipelineState, p_ui: RenderPipelineState, p_copy: RenderPipelineState,
    c_gen: ComputePipelineState, c_resolve: ComputePipelineState, c_blur: ComputePipelineState,
    ds_write: DepthStencilState, ds_read: DepthStencilState, ds_off: DepthStencilState,
    tex_a: Texture, tex_n: Texture, atlas: Texture, font: Texture, t: Targets,
    world_vb: Option<Buffer>, world_ib: Option<Buffer>, world_ic: u64,
    decal_vb: Option<Buffer>, decal_ib: Option<Buffer>, decal_matte: u64, decal_wet: u64,
    lib_v: Vec<Vertex>, lib_i: Vec<u32>, lib_ranges: Vec<(u32, u32, i32)>, lib_vb: Option<Buffer>, lib_ib: Option<Buffer>, lib_dirty: bool,
    arena: Buffer, arena_off: usize,
    accel: Option<TriangleAccelerationStructure>, isect: RayIntersector, blurs: Vec<*mut Object>, scaler: *mut Object,
    frame: u32, prev_vp: Mat4, prev_cam: Vec3, history: bool, rt_scale: u64, pub scale: f32, pub gpu_ms: f32,
}

impl Renderer {
    pub fn new(mats: &Materials, atlas: &Atlas, font: &FontAtlas, ow: u32, oh: u32, scale: f32) -> Renderer {
        let device = Device::system_default().expect("this game needs a Metal GPU");
        assert!(mps_supports_device(&device), "Metal Performance Shaders are not supported on this GPU");
        let src = include_str!("shaders.metal");
        let lib = device.new_library_with_source(src, &CompileOptions::new()).unwrap_or_else(|e| panic!("shader compile failed:\n{e}"));
        use MTLPixelFormat::*;
        let g = [(RGBA8Unorm_sRGB, Blend::Off), (RGBA16Float, Blend::Off), (RGBA16Float, Blend::Off)];
        let p = |vs, fs, c: &[(MTLPixelFormat, Blend)], d| pipeline(&device, &lib, vs, fs, c, d);
        let kernel = |n: &str| device.new_compute_pipeline_state_with_function(&lib.get_function(n, None).unwrap()).unwrap_or_else(|e| panic!("{n}: {e}"));

        let array = |fmt: MTLPixelFormat, data: &Vec<Vec<u8>>| {
            let d = TextureDescriptor::new();
            d.set_texture_type(MTLTextureType::D2Array); d.set_pixel_format(fmt); d.set_width(TEX as u64); d.set_height(TEX as u64); d.set_array_length(LAYERS as u64);
            d.set_mipmap_level_count(10); d.set_usage(MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget);
            let t = device.new_texture(&d);
            let region = MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width: TEX as u64, height: TEX as u64, depth: 1 } };
            for (i, l) in data.iter().enumerate() { t.replace_region_in_slice(region, 0, i as u64, l.as_ptr() as *const _, (TEX * 4) as u64, 0); }
            t
        };
        let single = |fmt: MTLPixelFormat, w: usize, h: usize, bpp: usize, data: &[u8]| {
            let d = TextureDescriptor::new();
            d.set_pixel_format(fmt); d.set_width(w as u64); d.set_height(h as u64); d.set_mipmap_level_count(8); d.set_usage(MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget);
            let t = device.new_texture(&d);
            t.replace_region(MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width: w as u64, height: h as u64, depth: 1 } }, 0, data.as_ptr() as *const _, (w * bpp) as u64);
            t
        };
        let (tex_a, tex_n) = (array(RGBA8Unorm_sRGB, &mats.albedo), array(RGBA8Unorm, &mats.nr));
        let atlas_t = single(RGBA8Unorm_sRGB, atlas.size, atlas.size, 4, &atlas.px);
        let font_t = single(R8Unorm, font.w, font.h, 1, &font.px);
        let queue = device.new_command_queue();
        let cb = queue.new_command_buffer();
        let blit = cb.new_blit_command_encoder();
        for t in [&tex_a, &tex_n, &atlas_t, &font_t] { blit.generate_mipmaps(t); }
        blit.end_encoding(); cb.commit();

        let isect = RayIntersector::from_device(&device).expect("MPS ray intersector unavailable");
        isect.set_ray_data_type(MPSRayDataType::OriginMinDistanceDirectionMaxDistance); isect.set_ray_stride(32);
        isect.set_intersection_data_type(MPSIntersectionDataType::Distance); isect.set_intersection_stride(4);
        let (blurs, scaler) = unsafe {
            let blurs = [2.5f32, 3.0, 3.5, 4.0].iter().map(|s| {
                let o: *mut Object = msg_send![class!(MPSImageGaussianBlur), alloc];
                let o: *mut Object = msg_send![o, initWithDevice: &*device sigma: *s];
                let _: () = msg_send![o, setEdgeMode: 1u64];
                o
            }).collect();
            let s: *mut Object = msg_send![class!(MPSImageLanczosScale), alloc];
            let s: *mut Object = msg_send![s, initWithDevice: &*device];
            (blurs, s)
        };
        let (w, h) = (((ow as f32 * scale) as u64 / 2 * 2).max(2), ((oh as f32 * scale) as u64 / 2 * 2).max(2));
        let rt_scale = 2;
        Renderer {
            p_g: p("g_vs", "g_fs", &g, true),
            p_dm: p("d_vs", "d_fs", &[(RGBA8Unorm_sRGB, Blend::DecalAlbedo), (RGBA16Float, Blend::RoughUp), (RGBA16Float, Blend::Masked)], true),
            p_dw: p("d_vs", "d_fs", &[(RGBA8Unorm_sRGB, Blend::DecalAlbedo), (RGBA16Float, Blend::Wet), (RGBA16Float, Blend::Masked)], true),
            p_light: p("fs_vs", "light_fs", &[(RGBA16Float, Blend::Off), (R32Float, Blend::Off)], false),
            p_water: p("fs_vs", "water_fs", &[(RGBA16Float, Blend::Alpha)], false),
            p_gum: p("gum_vs", "gum_fs", &[(RGBA16Float, Blend::Off), (R32Float, Blend::Off)], true),
            p_sprite: p("sp_vs", "sp_fs", &[(RGBA16Float, Blend::Premul)], true),
            p_taa: p("fs_vs", "taa_fs", &[(RGBA16Float, Blend::Off)], false),
            p_down: p("fs_vs", "down_fs", &[(RGBA16Float, Blend::Off)], false),
            p_final: p("fs_vs", "final_fs", &[(BGRA8Unorm, Blend::Off)], false),
            p_ui: p("ui_vs", "ui_fs", &[(BGRA8Unorm, Blend::Premul)], false),
            p_copy: p("fs_vs", "copy_fs", &[(BGRA8Unorm, Blend::Off)], false),
            c_gen: kernel("rt_gen"), c_resolve: kernel("rt_resolve"), c_blur: kernel("rt_blur"),
            ds_write: depth_state(&device, MTLCompareFunction::GreaterEqual, true), ds_read: depth_state(&device, MTLCompareFunction::GreaterEqual, false), ds_off: depth_state(&device, MTLCompareFunction::Always, false),
            tex_a, tex_n, atlas: atlas_t, font: font_t, t: Targets::new(&device, w, h, ow as u64, oh as u64, rt_scale),
            world_vb: None, world_ib: None, world_ic: 0, decal_vb: None, decal_ib: None, decal_matte: 0, decal_wet: 0,
            lib_v: Vec::new(), lib_i: Vec::new(), lib_ranges: Vec::new(), lib_vb: None, lib_ib: None, lib_dirty: false,
            arena: device.new_buffer(12 << 20, MTLResourceOptions::StorageModeShared), arena_off: 0,
            accel: None, isect, blurs, scaler, frame: 0, prev_vp: Mat4::IDENTITY, prev_cam: Vec3::ZERO, history: false, rt_scale, scale, gpu_ms: 0.0,
            queue, device,
        }
    }
    pub fn size(&self) -> (u32, u32) { (self.t.ow as u32, self.t.oh as u32) }
    pub fn internal_size(&self) -> (u32, u32) { (self.t.w as u32, self.t.h as u32) }
    pub fn resize(&mut self, ow: u32, oh: u32, scale: f32) {
        if ow == 0 || oh == 0 { return; }
        let (w, h) = (((ow as f32 * scale) as u64 / 2 * 2).max(2), ((oh as f32 * scale) as u64 / 2 * 2).max(2));
        self.scale = scale;
        self.t = Targets::new(&self.device, w, h, ow as u64, oh as u64, self.rt_scale);
        self.history = false;
    }
    /// Register a mesh that will be drawn with per-instance transforms. Returns its id.
    pub fn add_mesh(&mut self, m: &Mesh) -> u32 {
        self.lib_ranges.push((self.lib_i.len() as u32, m.i.len() as u32, self.lib_v.len() as i32));
        self.lib_v.extend_from_slice(&m.v); self.lib_i.extend_from_slice(&m.i); self.lib_dirty = true;
        (self.lib_ranges.len() - 1) as u32
    }
    fn buf<T: Pod>(&self, data: &[T]) -> Buffer {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        if bytes.is_empty() { return self.device.new_buffer(16, MTLResourceOptions::StorageModeShared); }
        self.device.new_buffer_with_data(bytes.as_ptr() as *const _, bytes.len() as u64, MTLResourceOptions::StorageModeShared)
    }
    /// Upload the level: everything solid goes to the GPU and into the MPS acceleration structure that shadow and AO rays hit.
    pub fn set_world(&mut self, parts: &[&Mesh], decals: &[&Mesh]) {
        let (mut v, mut i) = (Vec::new(), Vec::new());
        for m in parts { let base = v.len() as u32; v.extend_from_slice(&m.v); i.extend(m.i.iter().map(|x| x + base)); }
        let (vb, ib) = (self.buf(&v), self.buf(&i));
        let accel = TriangleAccelerationStructure::from_device(&self.device).expect("MPS acceleration structure unavailable");
        accel.set_vertex_buffer(Some(&vb)); accel.set_vertex_stride(std::mem::size_of::<Vertex>() as u64);
        accel.set_index_buffer(Some(&ib)); accel.set_index_type(MPSDataType::UInt32); accel.set_triangle_count((i.len() / 3) as u64);
        accel.rebuild();
        self.world_ic = i.len() as u64; self.world_vb = Some(vb); self.world_ib = Some(ib); self.accel = Some(accel);
        // decals: matte paint first, then anything wet or glossy
        let (mut dv, mut matte, mut wet) = (Vec::new(), Vec::new(), Vec::new());
        for m in decals {
            let base = dv.len() as u32; dv.extend_from_slice(&m.v);
            for tri in m.i.chunks_exact(3) { let dst = if m.v[tri[0] as usize].mat[1] < 77 { &mut wet } else { &mut matte }; dst.extend(tri.iter().map(|x| x + base)); }
        }
        self.decal_matte = matte.len() as u64; self.decal_wet = wet.len() as u64;
        matte.extend_from_slice(&wet);
        self.decal_vb = Some(self.buf(&dv)); self.decal_ib = Some(self.buf(&matte));
        self.history = false;
    }
    fn push<T: Pod>(&mut self, data: &[T]) -> u64 {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        let off = (self.arena_off + 255) & !255;
        assert!(off + bytes.len() <= self.arena.length() as usize, "frame arena overflow");
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), (self.arena.contents() as *mut u8).add(off), bytes.len()); }
        self.arena_off = off + bytes.len();
        off as u64
    }

    /// Draw one frame. With a drawable the result is presented; without, it stays in the final texture for `read_pixels`.
    pub fn render(&mut self, f: &Frame, drawable: Option<&MetalDrawableRef>) {
        autoreleasepool(|| self.render_inner(f, drawable));
    }
    fn render_inner(&mut self, f: &Frame, drawable: Option<&MetalDrawableRef>) {
        let t0 = std::time::Instant::now();
        if self.lib_dirty { self.lib_vb = Some(self.buf(&self.lib_v)); self.lib_ib = Some(self.buf(&self.lib_i)); self.lib_dirty = false; }
        self.arena_off = 0;
        if f.cut { self.history = false; }
        let (w, h) = (self.t.w, self.t.h);
        let aspect = w as f32 / h as f32;
        let (jx, jy) = ((halton(self.frame % 16 + 1, 2) - 0.5) * 2.0 / w as f32, (halton(self.frame % 16 + 1, 3) - 0.5) * 2.0 / h as f32);
        let jm = Mat4::from_translation(Vec3::new(jx, jy, 0.0));
        let proj = Mat4::perspective_infinite_reverse_rh(f.fov.to_radians(), aspect, 0.08);
        let (vp_nj, vp) = (proj * f.view, jm * proj * f.view);
        let vm_vp = jm * Mat4::perspective_infinite_reverse_rh(f.vm_fov.to_radians(), aspect, 0.01) * f.view;
        let inv_view = f.view.inverse();
        let p = &f.pal;
        let under = f.cam_pos.y < WATER_Y;
        let mut g = Globals::zeroed();
        g.view_proj = vp; g.inv_view_proj = vp.inverse(); g.prev_view_proj = self.prev_vp; g.vm_view_proj = vm_vp; g.inv_vm_view_proj = vm_vp.inverse(); g.cur_view_proj = vp_nj;
        g.cam_pos = f.cam_pos.extend(f.time); g.prev_cam = self.prev_cam.extend(0.08);
        g.sun_dir = p.sun_dir.extend(p.sun_i); g.sun_color = p.sun_color.extend(p.night);
        g.sky_top = p.top.extend(p.clouds); g.sky_mid = p.mid.extend(0.0); g.sky_bot = p.bot.extend(0.0);
        g.fog = p.fog.extend(p.fog_density); g.ambient = p.ambient.extend(WATER_Y);
        g.water = if under { p.uw.extend(1.0) } else { p.water.extend(0.0) };
        g.res = Vec4::new(w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32);
        g.misc = Vec4::new(self.frame as f32, self.rt_scale as f32, p.exposure, p.bloom);
        g.post = Vec4::new(f.post.saturation, f.post.rush, f.post.hurt, f.post.ca);
        g.post2 = Vec4::new(f.post.vignette, if self.history { 1.0 } else { 0.0 }, 0.1, f.vm_sun);
        let (nl, ns) = (f.lights.len().min(32), f.spheres.len().min(48));
        g.counts = Vec4::new(nl as f32, ns as f32, 0.0, 0.0);
        g.lights[..nl].copy_from_slice(&f.lights[..nl]);
        for i in 0..ns { g.spheres[2 * i] = f.spheres[i][0]; g.spheres[2 * i + 1] = f.spheres[i][1]; }
        let g_off = self.push(&[g]);

        // instances: identity for the level, then dynamic meshes grouped by mesh, then the viewmodel
        let mut insts = vec![Inst::new(Mat4::IDENTITY)];
        let mut runs: Vec<(u32, u32, u32)> = Vec::new(); // mesh, first instance, count
        let mut order: Vec<usize> = (0..f.draws.len()).collect();
        order.sort_by_key(|&i| f.draws[i].mesh);
        for &i in &order {
            let d = &f.draws[i];
            match runs.last_mut() { Some(r) if r.0 == d.mesh => r.2 += 1, _ => runs.push((d.mesh, insts.len() as u32, 1)) }
            insts.push(d.inst);
        }
        for d in f.vm_draws { let mut i = d.inst; i.model = inv_view * i.model; i.misc.y = 1.0; runs.push((d.mesh, insts.len() as u32, 1)); insts.push(i); }
        let i_off = self.push(&insts);
        let (dd_v, dd_i) = (self.push(&f.decals.v), self.push(&f.decals.i));
        let sp_off = self.push(f.sprites);
        let gum_offs: Vec<u64> = f.gummies.iter().map(|gu| self.push(&[*gu])).collect();
        let ui_off = self.push(f.ui);

        let t = &self.t;
        let cb = self.queue.new_command_buffer();
        let tri = MTLPrimitiveType::Triangle;
        // ── 1. G-buffer ──
        {
            let enc = cb.new_render_command_encoder(pass(&[(&t.g_a, Some([0.0; 4])), (&t.g_n, Some([0.0, 1.0, 0.0, 1.0])), (&t.g_e, Some([0.0; 4]))], Some((&t.depth, Some(0.0), true))));
            enc.set_render_pipeline_state(&self.p_g); enc.set_depth_stencil_state(&self.ds_write); enc.set_cull_mode(MTLCullMode::None);
            enc.set_vertex_buffer(1, Some(&self.arena), i_off); enc.set_vertex_buffer(2, Some(&self.arena), g_off); enc.set_fragment_buffer(2, Some(&self.arena), g_off);
            enc.set_fragment_texture(0, Some(&self.tex_a)); enc.set_fragment_texture(1, Some(&self.tex_n));
            if let (Some(vb), Some(ib)) = (&self.world_vb, &self.world_ib) { enc.set_vertex_buffer(0, Some(vb), 0); enc.draw_indexed_primitives(tri, self.world_ic, MTLIndexType::UInt32, ib, 0); }
            if let (Some(vb), Some(ib)) = (&self.lib_vb, &self.lib_ib) {
                enc.set_vertex_buffer(0, Some(vb), 0);
                for &(mesh, first, count) in &runs { let (fi, ic, bv) = self.lib_ranges[mesh as usize]; enc.draw_indexed_primitives_instanced_base_instance(tri, ic as u64, MTLIndexType::UInt32, ib, fi as u64 * 4, count as u64, bv as i64, first as u64); }
            }
            enc.set_depth_stencil_state(&self.ds_read); enc.set_depth_bias(2.0, 2.0, 0.0);
            enc.set_fragment_texture(0, Some(&self.atlas));
            if let (Some(vb), Some(ib)) = (&self.decal_vb, &self.decal_ib) {
                enc.set_vertex_buffer(0, Some(vb), 0);
                if self.decal_matte > 0 { enc.set_render_pipeline_state(&self.p_dm); enc.draw_indexed_primitives(tri, self.decal_matte, MTLIndexType::UInt32, ib, 0); }
                if self.decal_wet > 0 { enc.set_render_pipeline_state(&self.p_dw); enc.draw_indexed_primitives(tri, self.decal_wet, MTLIndexType::UInt32, ib, self.decal_matte * 4); }
            }
            if !f.decals.i.is_empty() { enc.set_render_pipeline_state(&self.p_dw); enc.set_vertex_buffer(0, Some(&self.arena), dd_v); enc.draw_indexed_primitives(tri, f.decals.i.len() as u64, MTLIndexType::UInt32, &self.arena, dd_i); }
            enc.end_encoding();
        }
        // ── 2. MPS: shadow and ambient-occlusion rays ──
        let (acc_out, acc_hist) = (&t.rt_acc[(self.frame % 2) as usize], &t.rt_acc[((self.frame + 1) % 2) as usize]);
        if let Some(accel) = &self.accel {
            let groups = MTLSize { width: (t.rw + 7) / 8, height: (t.rh + 7) / 8, depth: 1 };
            let per = MTLSize { width: 8, height: 8, depth: 1 };
            let enc = cb.new_compute_command_encoder();
            enc.set_compute_pipeline_state(&self.c_gen); enc.set_buffer(0, Some(&self.arena), g_off); enc.set_buffer(1, Some(&t.rays), 0);
            enc.set_texture(0, Some(&t.depth)); enc.set_texture(1, Some(&t.g_n)); enc.dispatch_thread_groups(groups, per); enc.end_encoding();
            self.isect.encode_intersection_to_command_buffer(cb, MPSIntersectionType::Any, &t.rays, 0, &t.hits, 0, t.rw * t.rh * 2, accel);
            let enc = cb.new_compute_command_encoder();
            enc.set_compute_pipeline_state(&self.c_resolve); enc.set_buffer(0, Some(&self.arena), g_off); enc.set_buffer(1, Some(&t.hits), 0);
            enc.set_texture(0, Some(&t.depth)); enc.set_texture(1, Some(acc_hist)); enc.set_texture(2, Some(acc_out)); enc.dispatch_thread_groups(groups, per);
            enc.set_compute_pipeline_state(&self.c_blur);
            for (step, src, dst) in [(1.0f32, acc_out, &t.rt_tmp), (2.0, &t.rt_tmp, &t.rt_filt)] {
                let pr = [step, 0.0, 0.0, 0.0];
                enc.set_bytes(0, 16, pr.as_ptr() as *const _); enc.set_texture(0, Some(src)); enc.set_texture(1, Some(dst)); enc.dispatch_thread_groups(groups, per);
            }
            enc.end_encoding();
        } else {
            cb.new_render_command_encoder(pass(&[(&t.rt_filt, Some([1.0, 1.0, 4000.0, 1.0]))], None)).end_encoding();
        }
        // ── 3. lighting, sky and fog ──
        {
            let enc = cb.new_render_command_encoder(pass(&[(&t.hdr, Some([0.0; 4])), (&t.lin, Some([4000.0, 0.0, 0.0, 0.0]))], None));
            enc.set_render_pipeline_state(&self.p_light); enc.set_fragment_buffer(0, Some(&self.arena), g_off);
            enc.set_fragment_textures(0, &[Some(&t.g_a), Some(&t.g_n), Some(&t.g_e), Some(&t.depth), Some(&t.rt_filt)]);
            enc.draw_primitives(tri, 0, 3); enc.end_encoding();
        }
        // ── 4. water, then a copy of the scene for the gummies to refract ──
        {
            let enc = cb.new_render_command_encoder(pass(&[(&t.hdr, None)], None));
            enc.set_render_pipeline_state(&self.p_water); enc.set_fragment_buffer(0, Some(&self.arena), g_off); enc.set_fragment_texture(0, Some(&t.lin));
            enc.draw_primitives(tri, 0, 3); enc.end_encoding();
            let blit = cb.new_blit_command_encoder();
            blit.copy_from_texture(&t.hdr, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 }, MTLSize { width: w, height: h, depth: 1 }, &t.scene, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 });
            blit.end_encoding();
        }
        // ── 5. gummies ──
        if !f.gummies.is_empty() {
            let enc = cb.new_render_command_encoder(pass(&[(&t.hdr, None), (&t.lin, None)], Some((&t.depth, None, true))));
            enc.set_render_pipeline_state(&self.p_gum); enc.set_depth_stencil_state(&self.ds_write);
            enc.set_front_facing_winding(MTLWinding::CounterClockwise); enc.set_cull_mode(MTLCullMode::Front);
            enc.set_vertex_buffer(0, Some(&self.arena), g_off); enc.set_fragment_buffer(0, Some(&self.arena), g_off); enc.set_fragment_texture(0, Some(&t.scene));
            for &off in &gum_offs { enc.set_vertex_buffer(1, Some(&self.arena), off); enc.set_fragment_buffer(1, Some(&self.arena), off); enc.draw_primitives(tri, 0, 36); }
            enc.end_encoding();
        }
        // ── 6. sprites ──
        if !f.sprites.is_empty() {
            let enc = cb.new_render_command_encoder(pass(&[(&t.hdr, None)], Some((&t.depth, None, false))));
            enc.set_render_pipeline_state(&self.p_sprite); enc.set_depth_stencil_state(&self.ds_read); enc.set_cull_mode(MTLCullMode::None);
            enc.set_vertex_buffer(0, Some(&self.arena), g_off); enc.set_vertex_buffer(1, Some(&self.arena), sp_off); enc.set_fragment_texture(0, Some(&t.lin));
            enc.draw_primitives_instanced(tri, 0, 6, f.sprites.len() as u64); enc.end_encoding();
        }
        // ── 7. temporal anti-aliasing ──
        let (taa_out, taa_hist) = (&t.taa[(self.frame % 2) as usize], &t.taa[((self.frame + 1) % 2) as usize]);
        {
            let enc = cb.new_render_command_encoder(pass(&[(taa_out, Some([0.0; 4]))], None));
            enc.set_render_pipeline_state(&self.p_taa); enc.set_fragment_buffer(0, Some(&self.arena), g_off);
            enc.set_fragment_textures(0, &[Some(&t.hdr), Some(taa_hist), Some(&t.lin), Some(&t.depth)]);
            enc.draw_primitives(tri, 0, 3); enc.end_encoding();
        }
        // ── 8. bloom: downsample, then MPS Gaussian blur at each level ──
        for i in 0..t.bloom.len() {
            let src: &TextureRef = if i == 0 { taa_out } else { &t.bloom[i - 1].1 };
            let enc = cb.new_render_command_encoder(pass(&[(&t.bloom[i].0, Some([0.0; 4]))], None));
            let pr = [if i == 0 { 1.0f32 } else { 0.0 }, 1.1, 0.0, 0.0];
            enc.set_render_pipeline_state(&self.p_down); enc.set_fragment_bytes(0, 16, pr.as_ptr() as *const _); enc.set_fragment_texture(0, Some(src));
            enc.draw_primitives(tri, 0, 3); enc.end_encoding();
            unsafe { let _: () = msg_send![self.blurs[i], encodeToCommandBuffer: cb sourceTexture: &*t.bloom[i].0 destinationTexture: &*t.bloom[i].1]; }
        }
        // ── 9. tone map and grade ──
        {
            let enc = cb.new_render_command_encoder(pass(&[(&t.ldr, Some([0.0; 4]))], None));
            enc.set_render_pipeline_state(&self.p_final); enc.set_fragment_buffer(0, Some(&self.arena), g_off);
            enc.set_fragment_textures(0, &[Some(taa_out), Some(&t.bloom[0].1), Some(&t.bloom[1].1), Some(&t.bloom[2].1), Some(&t.bloom[3].1)]);
            enc.draw_primitives(tri, 0, 3); enc.end_encoding();
        }
        // ── 10. up to window size (MPS Lanczos), interface on top, present ──
        if (w, h) == (t.ow, t.oh) {
            let enc = cb.new_render_command_encoder(pass(&[(&t.fin, Some([0.0; 4]))], None));
            enc.set_render_pipeline_state(&self.p_copy); enc.set_fragment_texture(0, Some(&t.ldr)); enc.draw_primitives(tri, 0, 3); enc.end_encoding();
        } else {
            unsafe { let _: () = msg_send![self.scaler, encodeToCommandBuffer: cb sourceTexture: &*t.ldr destinationTexture: &*t.fin]; }
        }
        if !f.ui.is_empty() {
            let enc = cb.new_render_command_encoder(pass(&[(&t.fin, None)], None));
            let screen = [t.ow as f32, t.oh as f32, 1.0 / t.ow as f32, 1.0 / t.oh as f32];
            enc.set_render_pipeline_state(&self.p_ui); enc.set_vertex_buffer(0, Some(&self.arena), ui_off); enc.set_vertex_bytes(1, 16, screen.as_ptr() as *const _);
            enc.set_fragment_texture(0, Some(&self.font)); enc.draw_primitives(tri, 0, f.ui.len() as u64); enc.end_encoding();
        }
        if let Some(d) = drawable {
            let blit = cb.new_blit_command_encoder();
            blit.copy_from_texture(&t.fin, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 }, MTLSize { width: t.ow, height: t.oh, depth: 1 }, d.texture(), 0, 0, MTLOrigin { x: 0, y: 0, z: 0 });
            blit.end_encoding();
            cb.present_drawable(d);
        }
        cb.commit();
        cb.wait_until_completed();
        self.prev_vp = vp_nj; self.prev_cam = f.cam_pos; self.history = true; self.frame += 1;
        self.gpu_ms = self.gpu_ms * 0.9 + t0.elapsed().as_secs_f32() * 100.0;
    }
    /// RGBA pixels of the last frame.
    pub fn read_pixels(&self) -> (u32, u32, Vec<u8>) {
        let (w, h) = (self.t.ow as usize, self.t.oh as usize);
        let mut px = vec![0u8; w * h * 4];
        self.t.fin.get_bytes(px.as_mut_ptr() as *mut _, (w * 4) as u64, MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width: w as u64, height: h as u64, depth: 1 } }, 0);
        for p in px.chunks_exact_mut(4) { p.swap(0, 2); p[3] = 255; }
        (w as u32, h as u32, px)
    }
}
