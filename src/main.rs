//! Gummy Grove: a first-person shooter on a native Metal renderer with Metal Performance Shaders ray tracing.
#![allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
#[macro_use]
extern crate objc;
mod audio;
mod bear;
mod game;
mod gfx;
mod guns;
mod mesh;
mod meta;
mod tex;
mod ui;
mod world;

use game::{Game, Input, Results, Screen, State, Stats};
use glam::Vec3;
use metal::foreign_types::ForeignType;
use metal::{MTLPixelFormat, MetalDrawableRef, MetalLayer};
use objc::rc::autoreleasepool;
use objc::runtime::{Object, YES};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{CursorGrabMode, CursorIcon, Fullscreen, Window, WindowId};

pub fn save_png(path: &str, w: u32, h: u32, px: &[u8]) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).expect("cannot create screenshot file"));
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(px).unwrap();
}
/// Internal render scale: the saved preference, or about two megapixels' worth on big displays (MPS Lanczos brings it up to the window).
fn auto_scale(pref: f32, ow: u32, oh: u32) -> f32 { if pref > 0.0 { pref } else { (2.1e6 / (ow as f32 * oh as f32).max(1.0)).sqrt().clamp(0.5, 1.0) } }

/// Renderer + game + input: everything that is the same with or without a window.
struct Core { r: gfx::Renderer, game: Game, inp: Input, time: f32, cut_prev: bool }
fn boot(ow: u32, oh: u32, scale: f32, sound: bool) -> Core {
    let t = Instant::now();
    let mats = tex::build_materials();
    let fonts = tex::load_fonts();
    let atlas = tex::build_atlas(&fonts);
    let fa = tex::build_font_atlas(&fonts.ui, 64.0);
    let mut r = gfx::Renderer::new(&mats, &atlas, &fa, ow, oh, scale);
    let audio = if sound { audio::Audio::new() } else { audio::Audio::silent() };
    let game = Game::new(&mut r, atlas, audio, ui::Ui::new(&fa));
    eprintln!("startup {:?}", t.elapsed());
    Core { r, game, inp: Input::default(), time: 0.0, cut_prev: true }
}
impl Core {
    fn step(&mut self, dt: f32) { self.time += dt; self.game.update(dt, &mut self.inp); }
    fn draw(&mut self, drawable: Option<&MetalDrawableRef>) {
        let (ow, oh) = self.r.size();
        self.game.draw_ui(ow, oh, &self.inp);
        if self.game.world_dirty {
            let w = &self.game.world;
            self.r.set_world(&[&w.static_mesh, &w.arena_mesh], &[&w.static_decals, &w.arena_decals]);
            self.game.world_dirty = false;
        }
        self.game.build_scene();
        let g = &self.game;
        let f = gfx::Frame {
            cam_pos: g.cam_pos, view: g.view(), fov: g.fov, vm_fov: 50.0, pal: g.pal, time: self.time, cut: g.cut || self.cut_prev,
            draws: &g.draws, vm_draws: &g.vm_draws, gummies: &g.gummies, sprites: &g.sprites, lights: &g.lights, spheres: &g.spheres, decals: &g.decals, ui: &g.ui.v, post: g.post(), vm_sun: g.vm_sun,
        };
        self.r.render(&f, drawable);
        self.cut_prev = self.game.cut;
        self.game.cut = false;
        self.inp.end_frame();
    }
}

// ───────────────────────── windowed ─────────────────────────
struct App { core: Option<Core>, window: Option<Window>, layer: Option<MetalLayer>, last: Instant, captured: bool, pointer: bool, wheel: f32, show_fps: bool, fps_n: u32, fps_t: Instant, smoke: Option<u32>, frames: u32, slowest: f32, t0: Instant, attached: bool }
impl App {
    fn capture(&mut self, on: bool) {
        let Some(w) = &self.window else { return };
        if on { if w.set_cursor_grab(CursorGrabMode::Locked).is_err() { let _ = w.set_cursor_grab(CursorGrabMode::Confined); } w.set_cursor_visible(false); }
        else { let _ = w.set_cursor_grab(CursorGrabMode::None); w.set_cursor_visible(true); }
        self.captured = on;
        if let Some(c) = self.core.as_mut() { c.inp.lmb = false; c.inp.rmb = false; c.inp.dx = 0.0; c.inp.dy = 0.0; }
    }
    fn resize(&mut self, w: u32, h: u32) {
        let (Some(core), Some(layer)) = (self.core.as_mut(), self.layer.as_ref()) else { return };
        if w < 2 || h < 2 { return; }
        let mut s = layer.drawable_size();
        s.width = w as f64; s.height = h as f64;
        layer.set_drawable_size(s);
        if let Some(win) = &self.window { layer.set_contents_scale(win.scale_factor()); }
        core.r.resize(w, h, auto_scale(core.game.profile.scale, w, h));
        core.cut_prev = true;
    }
    fn frame(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        let mut fix: Option<(u32, u32)> = None;
        self.frames += 1;
        if let Some(n) = self.smoke { // self-test: menu, then a mission, then out
            if self.frames > 20 { self.slowest = self.slowest.max(dt); }
            if let Some(c) = self.core.as_mut() {
                if self.frames == n / 3 { c.game.start_run(1); }
                if self.frames == n / 3 + 20 { c.game.skip_intro(); }
                if self.frames > n / 2 { c.inp.down.insert(KeyCode::KeyW); c.inp.lmb = self.frames % 30 < 20; c.inp.dx += 3.0; }
                if self.frames >= n {
                    let (iw, ih) = c.r.internal_size();
                    let (_, _, px) = c.r.read_pixels();
                    let mean = px.chunks_exact(4).map(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) as f64).sum::<f64>() / (px.len() as f64 / 4.0 * 3.0);
                    println!("layer attached to the window's view: {} · mean brightness of the last presented frame: {:.0}/255", self.attached, mean);
                    println!("smoke ok · {} frames in {:.2}s · {:.1} fps · gpu {:.1} ms · slowest frame {:.1} ms · window {:?} · internal {}x{} · captured {}", n, self.t0.elapsed().as_secs_f32(), n as f32 / self.t0.elapsed().as_secs_f32(), c.r.gpu_ms, self.slowest * 1000.0, c.r.size(), iw, ih, self.captured);
                    self.smoke = None; el.exit(); return;
                }
            }
        }
        {
            let (Some(core), Some(layer)) = (self.core.as_mut(), self.layer.as_ref()) else { return };
            core.step(dt);
            autoreleasepool(|| {
                let d = layer.next_drawable();
                if let Some(d) = d {
                    let t = d.texture();
                    if (t.width() as u32, t.height() as u32) != core.r.size() { fix = Some((t.width() as u32, t.height() as u32)); }
                }
                if fix.is_none() { core.draw(d); } else { core.inp.end_frame(); }
            });
        }
        if let Some((w, h)) = fix { self.resize(w, h); }
        let Some(core) = self.core.as_mut() else { return };
        let (want, quit, hot) = (core.game.want_capture, core.game.quit, core.game.ui.hot);
        if core.game.scale_changed { core.game.scale_changed = false; let (w, h) = core.r.size(); core.r.resize(w, h, auto_scale(core.game.profile.scale, w, h)); core.cut_prev = true; }
        let gpu = core.r.gpu_ms;
        let (iw, ih) = core.r.internal_size();
        if want != self.captured { self.capture(want); }
        if hot != self.pointer { self.pointer = hot; if let Some(w) = &self.window { w.set_cursor(if hot { CursorIcon::Pointer } else { CursorIcon::Default }); } }
        self.fps_n += 1;
        if self.fps_t.elapsed().as_secs_f32() >= 1.0 {
            if let (true, Some(w)) = (self.show_fps, &self.window) { w.set_title(&format!("Gummy Grove · {} fps · {:.1} ms · {}x{}", self.fps_n, gpu, iw, ih)); }
            self.fps_n = 0; self.fps_t = Instant::now();
        }
        if quit { el.exit(); }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes().with_title("Gummy Grove").with_inner_size(LogicalSize::new(1440.0, 900.0)).with_min_inner_size(LogicalSize::new(960.0, 600.0));
        let window = el.create_window(attrs).expect("could not open a window");
        let sz = window.inner_size();
        let (w, h) = (sz.width.max(2), sz.height.max(2));
        let core = boot(w, h, auto_scale(meta::Profile::load().scale, w, h), self.smoke.is_none());
        let layer = MetalLayer::new();
        layer.set_device(&core.r.device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);
        layer.set_framebuffer_only(false);
        layer.set_opaque(true);
        layer.set_contents_scale(window.scale_factor());
        let mut s = layer.drawable_size();
        s.width = w as f64; s.height = h as f64;
        layer.set_drawable_size(s);
        if let Ok(RawWindowHandle::AppKit(hnd)) = window.window_handle().map(|h| h.as_raw()) {
            unsafe {
                let view = hnd.ns_view.as_ptr() as *mut Object;
                let _: () = msg_send![view, setWantsLayer: YES];
                let _: () = msg_send![view, setLayer: layer.as_ptr() as *mut Object];
                let now: *mut Object = msg_send![view, layer];
                self.attached = now == layer.as_ptr() as *mut Object;
            }
        }
        self.core = Some(core); self.layer = Some(layer); self.window = Some(window);
        self.last = Instant::now(); self.t0 = Instant::now();
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => { if let Some(c) = &self.core { c.game.profile.save(); } el.exit(); }
            WindowEvent::Resized(sz) => self.resize(sz.width, sz.height),
            WindowEvent::ScaleFactorChanged { .. } => { if let Some(w) = &self.window { let sz = w.inner_size(); self.resize(sz.width, sz.height); } }
            WindowEvent::Focused(false) => {
                if let Some(c) = self.core.as_mut() { c.inp.down.clear(); c.game.pause(); }
                self.capture(false);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(k) = event.physical_key else { return };
                let down = event.state == ElementState::Pressed;
                if down && !event.repeat {
                    if k == KeyCode::F11 { if let Some(w) = &self.window { w.set_fullscreen(if w.fullscreen().is_some() { None } else { Some(Fullscreen::Borderless(None)) }); } return; }
                    if k == KeyCode::F3 { self.show_fps = !self.show_fps; if let (false, Some(w)) = (self.show_fps, &self.window) { w.set_title("Gummy Grove"); } return; }
                }
                let Some(c) = self.core.as_mut() else { return };
                if down { if !event.repeat { c.inp.pressed.push(k); } c.inp.down.insert(k); } else { c.inp.down.remove(&k); }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(c) = self.core.as_mut() else { return };
                let down = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => { c.inp.lmb = down; if down { c.inp.click = true; } }
                    MouseButton::Right => c.inp.rmb = down,
                    MouseButton::Middle => { if down { c.inp.mmb = true; } }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => { if let Some(c) = self.core.as_mut() { c.inp.mouse = (position.x as f32, position.y as f32); } }
            WindowEvent::MouseWheel { delta, .. } => {
                self.wheel += match delta { MouseScrollDelta::LineDelta(_, y) => y, MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0 };
                if self.wheel.abs() >= 1.0 { if let Some(c) = self.core.as_mut() { c.inp.wheel = -self.wheel.signum(); } self.wheel = 0.0; }
            }
            WindowEvent::RedrawRequested => self.frame(el),
            _ => {}
        }
    }
    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, ev: DeviceEvent) {
        if let (DeviceEvent::MouseMotion { delta }, true, Some(c)) = (ev, self.captured, self.core.as_mut()) { c.inp.dx += delta.0 as f32; c.inp.dy += delta.1 as f32; }
    }
    fn about_to_wait(&mut self, _el: &ActiveEventLoop) { if let Some(w) = &self.window { w.request_redraw(); } }
}

// ───────────────────────── headless: screenshots and the bot ─────────────────────────
/// A crude but tireless player: looks at the nearest bear it can see and holds the trigger.
fn bot(g: &mut Game, inp: &mut Input, n: u32, god: bool) {
    use KeyCode::*;
    inp.down.clear(); inp.lmb = false; inp.rmb = false;
    if g.state != State::Playing { return; }
    if g.in_intro() { g.skip_intro(); return; }
    if god { g.player.hp = g.m.max_hp; }
    let eye = g.player.pos + Vec3::Y * g.player.h;
    let (mut seen, mut near): (Option<(f32, Vec3)>, Option<(f32, Vec3)>) = (None, None);
    for b in &g.bears {
        let c = b.zone(0.0, if (n / 300) % 2 == 0 { 1.6 } else { 0.85 }, 0.02);
        let d = c.distance(eye);
        if near.map_or(true, |x| d < x.0) { near = Some((d, c)); }
        if g.world.seg_blocked(eye, c) < 0.0 && seen.map_or(true, |x| d < x.0) { seen = Some((d, c)); }
    }
    let look = |g: &mut Game, c: Vec3| { let d = c - eye; g.player.yaw = (-d.x).atan2(-d.z); g.player.pitch = d.y.atan2(d.x.hypot(d.z)).clamp(-1.4, 1.4); };
    match (seen, near) {
        (Some((d, c)), _) => {
            look(g, c);
            inp.lmb = true;
            if d > 30.0 { inp.down.insert(KeyW); }
            if d < 2.6 && n % 30 == 0 { inp.pressed.push(KeyV); }
            inp.down.insert(if (n / 90) % 2 == 0 { KeyA } else { KeyD });
        }
        (None, Some((_, c))) => {
            look(g, c);
            inp.down.insert(KeyW); inp.down.insert(ShiftLeft);
            if n % 50 == 0 { inp.pressed.push(Space); }
            if n % 240 == 120 { inp.pressed.push(KeyC); }
        }
        _ => { let c = Vec3::new(0.0, 1.5, 30.0); look(g, c); if g.player.pos.distance(c) > 6.0 { inp.down.insert(KeyW); } }
    }
    if g.near_pick.is_some() && n % 20 == 0 { inp.pressed.push(KeyE); }
    if g.player.sugar >= 100.0 { inp.pressed.push(KeyF); }
    if n % 900 == 450 { inp.pressed.push([Digit1, Digit2, Digit3, Digit4][(n / 900 % 4) as usize]); }
}
fn headless(args: &[String]) {
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let num = |k: &str, d: f32| arg(k).and_then(|v| v.parse().ok()).unwrap_or(d);
    let has = |k: &str| args.iter().any(|a| a == k);
    let save = arg("--save").unwrap_or_else(|| std::env::temp_dir().join("gummy-grove-test-save.json").to_string_lossy().into_owned());
    if !has("--keep-save") { let _ = std::fs::remove_file(&save); }
    std::env::set_var("GUMMY_SAVE", &save);
    let (w, h) = (num("--w", 1280.0) as u32, num("--h", 720.0) as u32);
    let mut c = boot(w, h, num("--scale", 1.0), has("--sound"));
    let god = !has("--mortal");
    if has("--rich") { // a mid-campaign profile for menu screenshots
        let p = &mut c.game.profile;
        p.level = 6; p.xp = 900; p.candy = 2480; p.perk_pts = 2; p.best = 48250;
        for (m, s) in [(1, 3), (2, 2), (3, 3), (4, 1)] { p.done.insert(m, s); }
        p.up.insert("sour".into(), 2); p.up.insert("vital".into(), 1); p.perks.push("head".into());
        c.game.m = meta::mods(&c.game.profile);
    }

    if has("--uitest") { // drive the menus with synthetic clicks and keys
        fn click(c: &mut Core, x: f32, y: f32) { c.inp.mouse = (x, y); c.inp.click = true; c.step(1.0 / 60.0); autoreleasepool(|| c.draw(None)); }
        fn key(c: &mut Core, k: KeyCode) { c.inp.pressed.push(k); c.step(1.0 / 60.0); autoreleasepool(|| c.draw(None)); }
        let mut ok = true;
        let mut check = |what: &str, pass: bool| { println!("{} {what}", if pass { "ok  " } else { "FAIL" }); ok &= pass; };
        c.step(1.0 / 60.0); autoreleasepool(|| c.draw(None));
        click(&mut c, 280.0, 390.0); let next = c.game.profile.next_mission().map(|m| m.id).unwrap_or(1);
        check("the top button opens the next mission's briefing", c.game.screens.last() == Some(&Screen::Brief(next)));
        key(&mut c, KeyCode::Escape); check("Escape goes back to the menu", c.game.screens.last() == Some(&Screen::Menu));
        click(&mut c, 280.0, 460.0); check("Missions opens the campaign map", c.game.screens.last() == Some(&Screen::Map));
        click(&mut c, 250.0, 280.0); check("mission card opens its briefing", c.game.screens.last() == Some(&Screen::Brief(1)));
        click(&mut c, 950.0, 605.0); check("Deploy starts the mission", c.game.state == State::Playing && c.game.in_intro() && c.game.want_capture);
        c.game.skip_intro(); for _ in 0..30 { c.step(1.0 / 60.0); c.inp.end_frame(); }
        key(&mut c, KeyCode::Escape); check("Escape pauses", c.game.state == State::Paused && !c.game.want_capture);
        key(&mut c, KeyCode::Escape); check("Escape resumes", c.game.state == State::Playing && c.game.want_capture);
        key(&mut c, KeyCode::Escape); click(&mut c, 640.0, 421.0); check("Settings opens from pause", c.game.screens.last() == Some(&Screen::Settings));
        click(&mut c, 322.0, 109.0); check("back returns to pause", c.game.screens.last() == Some(&Screen::Pause));
        click(&mut c, 640.0, 303.0); check("Resume button resumes", c.game.state == State::Playing);
        key(&mut c, KeyCode::Escape); click(&mut c, 640.0, 489.0); check("Abandon returns to the menu", c.game.state == State::Menu && c.game.screens.last() == Some(&Screen::Menu));
        click(&mut c, 280.0, 572.0); check("Armory opens", c.game.screens.last() == Some(&Screen::Armory));
        let before = (c.game.profile.candy, c.game.profile.up("hopper"));
        click(&mut c, 580.0, 236.0); check("buying an upgrade spends candy", c.game.profile.up("hopper") == before.1 + 1 && c.game.profile.candy == before.0 - 200);
        key(&mut c, KeyCode::Escape); click(&mut c, 280.0, 516.0); check("Arcade starts an endless run", c.game.state == State::Playing && c.game.run.as_ref().is_some_and(|r| r.mission.is_none()));
        println!("{}", if ok { "UI OK" } else { "UI FAILED" });
        return;
    }
    if has("--bot") { // play everything through and report
        let list: Vec<u32> = match arg("--mission").and_then(|v| v.parse().ok()) { Some(m) => vec![m], None => vec![1, 2, 3, 4, 5, 6, 7, 8, 0] };
        let mut failed = 0;
        for id in list {
            c.game.start_run(id);
            let (t0, limit) = (Instant::now(), if id == 0 { 60 * 300 } else { 60 * 900 });
            let (mut n, mut outcome, mut low) = (0u32, "TIMEOUT".to_string(), f32::MAX);
            while n < limit {
                bot(&mut c.game, &mut c.inp, n, god);
                c.step(1.0 / 60.0);
                low = low.min(c.game.player.hp);
                if n % 600 == 0 { autoreleasepool(|| c.draw(None)); } else { if n % 12 == 0 { c.game.draw_ui(w, h, &c.inp); c.game.build_scene(); } c.inp.end_frame(); }
                n += 1;
                if c.game.state == State::Results { let r = c.game.results.as_ref().unwrap(); outcome = if r.win { format!("WIN {} stars", r.stars) } else { "LOST".into() }; break; }
                if id == 0 && c.game.wave >= 9 { outcome = "reached wave 9".into(); break; }
            }
            if outcome == "TIMEOUT" || outcome == "LOST" { failed += 1; }
            let s = c.game.stats;
            println!("mission {id}: {outcome} · {:.0}s sim · {} kills {} heads {} disarms · score {} · wave {} · bears left {} · lowest hp {low:.0} · {:.1}s real", n as f32 / 60.0, s.kills, s.heads, s.disarms, c.game.score, c.game.wave, c.game.bears.len(), t0.elapsed().as_secs_f32());
            c.game.to_menu();
        }
        println!("{}", if failed == 0 { "ALL OK".to_string() } else { format!("{failed} FAILED") });
        return;
    }

    // one screenshot
    if let Some(p) = arg("--pal") { c.game.pal = world::palette(&p); }
    if let Some(m) = arg("--mission").and_then(|v| v.parse::<u32>().ok()) {
        c.game.start_run(m);
        if let Some(p) = arg("--pal") { c.game.pal = world::palette(&p); }
        if !has("--intro") { c.game.skip_intro(); }
    }
    let (t, frames) = (num("--t", 0.0), num("--frames", 40.0) as u32);
    if let Some(want) = arg("--freeze-when").and_then(|v| v.parse::<usize>().ok()) {
        // play with the bot until at least `want` bears are in clear view, then stop time and take the picture
        let (mut n, start) = (0u32, (t * 60.0) as u32);
        loop {
            bot(&mut c.game, &mut c.inp, n, god);
            if n >= start { c.inp.lmb = false; c.inp.pressed.clear(); } // hold fire and let them come
            c.step(1.0 / 60.0);
            let (view, eye) = (c.game.view(), c.game.cam_pos);
            let seen = c.game.bears.iter().filter(|b| {
                let p = b.zone(0.0, 0.9, 0.0);
                let v = view.transform_point3(p);
                b.spawn >= 1.0 && b.flash <= 0.0 && -v.z > 3.5 && -v.z < 22.0 && (v.x / v.z).abs() < 0.8 && (v.y / v.z).abs() < 0.38 && c.game.world.seg_blocked(eye, p) < 0.0
            }).count();
            n += 1;
            if (n >= start && seen >= want) || n >= start + 60 * 60 || c.game.state != State::Playing { println!("froze at {:.1}s with {seen} bears in view", n as f32 / 60.0); break; }
            c.inp.end_frame();
        }
        c.game.hurt_flash = 0.0; c.game.trauma = 0.0;
        for _ in 0..frames { c.inp.down.clear(); c.inp.lmb = false; c.step(0.0); autoreleasepool(|| c.draw(None)); }
        let (pw, ph, px) = c.r.read_pixels();
        let out = arg("--shot").unwrap_or("shot.png".into());
        save_png(&out, pw, ph, &px);
        println!("wrote {out}");
        return;
    }
    if has("--lineup") { c.game.debug_lineup(Vec3::new(num("--lx", 0.0), num("--ly", 0.0), num("--lz", 72.0))); }
    if let Some(which) = arg("--inspect") { // weapon models on a table, for looking at from outside
        let at = glam::Mat4::from_translation(Vec3::new(num("--ix", 0.0), num("--iy", 1.5), num("--iz", 70.0))) * glam::Mat4::from_rotation_y(num("--iyaw", 1.5708));
        let loot = match which.as_str() { "ak" => Some(guns::Wpn::Rifle), "pistol" => Some(guns::Wpn::Pistol), "sniper" => Some(guns::Wpn::Sniper), "rpg" => Some(guns::Wpn::Rpg), "minigun" => Some(guns::Wpn::Minigun), _ => None };
        c.game.inspect(which.parse::<usize>().unwrap_or(1).saturating_sub(1), loot, at);
    }
    let total = (t * 60.0) as u32 + frames;
    let give = arg("--give").map(|g| match g.as_str() { "pistol" => guns::Wpn::Pistol, "sniper" => guns::Wpn::Sniper, "rpg" => guns::Wpn::Rpg, "minigun" => guns::Wpn::Minigun, _ => guns::Wpn::Rifle });
    for n in 0..total {
        if has("--play") { bot(&mut c.game, &mut c.inp, n, god); } else if god && c.game.state == State::Playing { c.game.player.hp = c.game.m.max_hp; }
        if has("--ads") { c.inp.rmb = true; }
        if has("--fire") { c.inp.lmb = n % 20 < 12; }
        if has("--fwd") { c.inp.down.insert(KeyCode::KeyW); }
        if n == 30 { if let Some(w) = give { c.game.equip_loot(w); } }
        if n == 30 { if let Some(g) = arg("--gun").and_then(|v| v.parse::<usize>().ok()) { for gun in c.game.guns.iter_mut().take(3) { gun.owned = true; } c.game.switch_to(g - 1); } }
        if has("--rush") && n == 40 { c.game.player.sugar = 100.0; c.game.start_rush(); }
        c.inp.mouse = (num("--mx", -100.0), num("--my", -100.0));
        c.step(1.0 / 60.0);
        if n + frames == total { // the frozen moment: set up whichever screen was asked for
            match arg("--screen").as_deref() {
                Some("map") => c.game.screens.push(Screen::Map),
                Some("brief") => c.game.screens.push(Screen::Brief(num("--brief", 3.0) as u32)),
                Some("armory") => c.game.screens.push(Screen::Armory),
                Some("settings") => c.game.screens.push(Screen::Settings),
                Some("pause") => c.game.pause(),
                Some("results") => {
                    let m = meta::MISSIONS.iter().find(|m| m.id == 3);
                    c.game.results = Some(Results { win: true, mission: m, score: 18450, wave: 3, stats: Stats { kills: 31, heads: 12, disarms: 4, shots: 420, hits: 268, ff: 2 }, candy: 512, xp: 640, stars: 3,
                        bonus: vec![("Mission complete".into(), true), ("Finish with 50%+ health".into(), true), ("8 headshots (12)".into(), true)], unlock: Some("GB-12 Gumball Shotgun"), level_ups: 1, new_best: false, age: 5.0 });
                    c.game.state = State::Results; c.game.screens = vec![Screen::Results];
                }
                _ => {}
            }
        }
        if has("--noui") { c.game.screens.clear(); }
        if has("--cam") { c.game.cam_pos = Vec3::new(num("--x", 0.0), num("--y", 1.7), num("--z", 60.0)); c.game.cam_rot = Vec3::new(num("--pitch", 0.0), num("--yaw", 0.0), 0.0); c.game.fov = num("--fov", 75.0); }
        if n + frames >= total { autoreleasepool(|| c.draw(None)); } else { c.inp.end_frame(); }
    }
    let (pw, ph, px) = c.r.read_pixels();
    let out = arg("--shot").unwrap_or("shot.png".into());
    save_png(&out, pw, ph, &px);
    println!("wrote {out} · {:.1} ms/frame gpu · internal {:?}", c.r.gpu_ms, c.r.internal_size());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--shot" || a == "--bot" || a == "--uitest") { headless(&args); return; }
    let smoke = args.iter().position(|a| a == "--smoke").map(|i| args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(300));
    if smoke.is_some() { std::env::set_var("GUMMY_SAVE", std::env::temp_dir().join("gummy-grove-test-save.json")); }
    let el = EventLoop::new().expect("could not start the event loop");
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App { core: None, window: None, layer: None, last: Instant::now(), captured: false, pointer: false, wheel: 0.0, show_fps: false, fps_n: 0, fps_t: Instant::now(), smoke, frames: 0, slowest: 0.0, t0: Instant::now(), attached: false };
    el.run_app(&mut app).expect("event loop failed");
}
