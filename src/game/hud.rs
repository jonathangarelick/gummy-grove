//! Heads-up display and menu screens, drawn with the immediate-mode `Ui`.
use super::*;
use crate::ui::{fade, Col, GLASS, HOT, INK, LINE, MINT, PINK, RED, SUN, WHITE};

const BLUE: Col = [96, 176, 255, 255];
const BLACK: Col = [0, 0, 0, 255];
const DIM: Col = [255, 255, 255, 150];
const SCRIM: Col = [20, 5, 15, 255];
const SHEET: Col = [30, 8, 22, 238];
const RES: [(f32, &str); 5] = [(0.0, "Auto"), (0.5, "50%"), (0.67, "67%"), (0.75, "75%"), (1.0, "Native")];

fn commas(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { out.push(','); } out.push(ch); }
    out
}
fn ease(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }
fn stars(ui: &mut Ui, x: f32, y: f32, r: f32, n: u32) { for i in 0..3 { ui.star(x + i as f32 * r * 2.7, y, r, i < n); } }
fn h3(ui: &mut Ui, x: f32, y: f32, t: &str) -> f32 { ui.text(x, y, 11.5, SUN, t); 22.0 }
fn slider(ui: &mut Ui, inp: &Input, x: f32, y: f32, w: f32, label: &str, v: &mut f32, lo: f32, hi: f32) -> bool {
    ui.text(x, y, 14.0, WHITE, label);
    ui.text_r(x + w, y, 14.0, DIM, &format!("{:.2}", *v));
    let (ty, th) = (y + 28.0, 8.0);
    let f = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0);
    ui.rect(x, ty, w, th, 4.0, [255, 255, 255, 40]);
    ui.rect(x, ty, (w * f).max(th), th, 4.0, HOT);
    let hov = ui.hover(x - 10.0, ty - 12.0, w + 20.0, th + 24.0);
    if hov { ui.hot = true; }
    ui.rect(x + w * f - 9.0, ty - 5.0, 18.0, 18.0, 9.0, WHITE);
    if hov && inp.lmb {
        let nv = lo + ((ui.mx - x) / w).clamp(0.0, 1.0) * (hi - lo);
        if (nv - *v).abs() > 1e-4 { *v = nv; return true; }
    }
    false
}

impl Game {
    pub fn draw_ui(&mut self, pw: u32, ph: u32, inp: &Input) {
        let mut ui = std::mem::replace(&mut self.ui, Ui::blank());
        ui.begin(pw, ph, inp.mouse, inp.click);
        if self.run.is_some() && matches!(self.state, State::Playing | State::Paused | State::Dead) { self.hud(&mut ui); }
        if let Some(&scr) = self.screens.last() { self.screen(&mut ui, scr, inp); }
        self.ui = ui;
    }
    /// World position to logical screen position and distance; None when behind the camera.
    fn project(&self, ui: &Ui, p: Vec3) -> Option<(f32, f32, f32)> {
        let v = self.view().transform_point3(p);
        if v.z > -0.4 { return None; }
        let t = (self.fov.to_radians() * 0.5).tan();
        Some((ui.w * 0.5 * (1.0 + v.x / (-v.z * t * ui.w / ui.h)), ui.h * 0.5 * (1.0 - v.y / (-v.z * t)), -v.z))
    }

    // ───────────────────────── in-game HUD ─────────────────────────
    fn hud(&self, ui: &mut Ui) {
        let (w, h, cx, cy) = (ui.w, ui.h, ui.w / 2.0, ui.h / 2.0);
        let (intro, playing) = (self.in_intro(), self.state == State::Playing);
        let gi = self.gi.min(self.guns.len() - 1);
        let g = &self.guns[gi];
        // speech bubbles over the bears
        for b in &self.bubbles {
            let Some((x, y, d)) = self.project(ui, b.pos + Vec3::Y * (2.25 * b.s + 0.15)) else { continue };
            if d > 75.0 { continue; }
            let a = (b.t / 0.12).min(1.0) * ((2.0 - b.t) / 0.3).clamp(0.0, 1.0);
            let size = 15.0 * (14.0 / d.max(6.0)).clamp(0.6, 1.25);
            let tw = ui.text_w(&b.text, size);
            let (bw, bh) = (tw + size * 1.5, size * 1.9);
            let top = y - bh - size * 0.5 - (1.0 - ease(b.t / 0.2)) * 8.0;
            ui.rect_rot(x, top + bh - 1.0, size * 0.75, size * 0.75, 0.785, 2.0, fade([255, 255, 255, 240], a));
            ui.rect(x - bw / 2.0, top, bw, bh, bh * 0.45, fade([255, 255, 255, 240], a));
            ui.text(x - tw / 2.0, top + (bh - size) / 2.0 - 1.0, size, fade(INK, a), &b.text);
        }
        // damage numbers and score pops
        for f in &self.floats {
            let k = f.t / f.life;
            let Some((x, y, d)) = self.project(ui, f.pos + Vec3::Y * (0.35 + k * 1.1)) else { continue };
            let a = ((1.0 - k) / 0.35).min(1.0);
            let (col, size) = match f.cls { 1 => (SUN, 27.0), 2 => (MINT, 19.0), _ => (WHITE, 19.0) };
            let size = size * (16.0 / d.max(8.0)).clamp(0.6, 1.2) * (1.0 + (1.0 - (f.t / 0.12).min(1.0)) * 0.5);
            ui.label(x, y - size, size, fade(col, a), &f.text, 1);
        }
        // sniper scope
        if self.scoped {
            let (r, t) = (h * 0.45, w.max(h) * 2.0);
            ui.ring(cx - r - t, cy - r - t, 2.0 * (r + t), 2.0 * (r + t), r + t, t, BLACK);
            ui.ring(cx - r - 3.0, cy - r - 3.0, 2.0 * r + 6.0, 2.0 * r + 6.0, r + 3.0, 5.0, [0, 0, 0, 170]);
            ui.rect(cx - r, cy - 0.75, r * 2.0, 1.5, 0.0, [0, 0, 0, 230]);
            ui.rect(cx - 0.75, cy - r, 1.5, r * 2.0, 0.0, [0, 0, 0, 230]);
            for i in 1..5 { let o = i as f32 * 26.0; for (dx, dy) in [(o, 0.0), (-o, 0.0), (0.0, o)] { ui.rect(cx + dx - 2.0, cy + dy - 2.0, 4.0, 4.0, 2.0, [0, 0, 0, 230]); } }
        }
        // crosshair
        if playing && !intro && !self.scoped {
            let a = (if g.no_ads { 1.0 } else { 1.0 - self.ads_t * 1.7 }).clamp(0.0, 1.0) * (1.0 - self.sprint_k * 0.75);
            if a > 0.02 {
                let sp = self.player.vel.x.hypot(self.player.vel.z);
                let gap = 5.0 + g.spread * 250.0 + self.cross_bloom * 16.0 + (sp / 9.8).min(1.0) * 7.0 + if self.player.on_ground { 0.0 } else { 6.0 };
                let (c, o) = (fade(WHITE, 0.92 * a), fade(BLACK, 0.45 * a));
                for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let (px, py) = (cx + dx * (gap + 4.5), cy + dy * (gap + 4.5));
                    let (rw, rh) = if dx != 0.0 { (9.0, 2.0) } else { (2.0, 9.0) };
                    ui.rect(px - rw / 2.0 - 1.0, py - rh / 2.0 - 1.0, rw + 2.0, rh + 2.0, 1.5, o);
                    ui.rect(px - rw / 2.0, py - rh / 2.0, rw, rh, 0.5, c);
                }
                ui.rect(cx - 1.5, cy - 1.5, 3.0, 3.0, 1.5, c);
            }
        }
        // hit marker
        let (age, crit, kill) = self.hit_mark;
        if age < 0.22 {
            let c = fade(if kill { RED } else if crit { SUN } else { WHITE }, 1.0 - age / 0.22);
            let r = 9.0 + age * 30.0 + if kill { 3.0 } else { 0.0 };
            for (dx, dy) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] { ui.rect_rot(cx + dx * r, cy + dy * r, 11.0, if kill { 3.2 } else { 2.2 }, if dx * dy > 0.0 { 0.785 } else { -0.785 }, 1.0, c); }
        }
        // where the damage came from
        for &(ang, age) in &self.dmg_dirs { ui.rect_rot(cx + ang.sin() * 120.0, cy - ang.cos() * 120.0, 88.0, 9.0, ang, 4.5, fade(RED, (1.0 - age / 1.1).clamp(0.0, 1.0) * 0.85)); }
        // insertion: letterbox
        if intro {
            ui.rect(0.0, 0.0, w, h * 0.09, 0.0, BLACK); ui.rect(0.0, h * 0.91, w, h * 0.09, 0.0, BLACK);
            ui.text_c(cx, h * 0.91 + 22.0, 13.0, fade(WHITE, 0.6), "SPACE  skip");
        }
        // mission card
        let (kicker, title, sub, left) = (&self.card.0, &self.card.1, &self.card.2, self.card.3);
        if left > 0.0 {
            let (a, y) = ((left / 0.5).min(1.0), if intro { h * 0.26 } else { 92.0 });
            ui.label(cx, y, 14.0, fade(SUN, a), &kicker.to_uppercase(), 1);
            ui.label(cx, y + 20.0, 46.0, fade(WHITE, a), title, 1);
            if !sub.is_empty() { ui.label(cx, y + 76.0, 16.0, fade(WHITE, a * 0.85), sub, 1); }
        }
        // announcements
        let (text, cls, age) = (&self.announce.0, self.announce.1, self.announce.2);
        if age < 1.7 && !text.is_empty() && !intro {
            let (a, pop) = (((1.7 - age) / 0.4).min(1.0), 1.0 + (1.0 - ease(age / 0.18)) * 0.5);
            let (col, size) = match cls { 1 => (SUN, 38.0), 2 => (MINT, 26.0), 3 => (PINK, 46.0), _ => (WHITE, 30.0) };
            ui.label(cx, h * 0.31 - size * pop / 2.0, size * pop, fade(col, a), text, 1);
        }
        if intro { return; }
        // objective, score, combo
        let obj = self.obj_text();
        let tw = ui.text_w(&obj, 14.0);
        ui.panel(24.0, 22.0, tw + 46.0, 34.0, 10.0);
        ui.rect(37.0, 35.0, 8.0, 8.0, 4.0, SUN);
        ui.text(54.0, 30.5, 14.0, WHITE, &obj);
        let score = commas(self.score);
        ui.label(26.0, 66.0, 30.0, WHITE, &score, 0);
        if self.combo > 1 {
            let sx = 26.0 + ui.text_w(&score, 30.0) + 12.0;
            ui.label(sx, 73.0, 21.0, SUN, &format!("x{}", self.combo.min(8)), 0);
            ui.bar(26.0, 104.0, 120.0, 4.0, self.combo_t / 4.0, SUN);
        }
        if let Some(run) = &self.run { ui.label(26.0, 114.0, 13.0, PINK, &format!("+{} candy", run.candy), 0); }
        // boss
        if let Some(f) = self.boss_hp() {
            ui.label(cx, 22.0, 13.0, WHITE, "KING GUMMY", 1);
            ui.rect(cx - 222.0, 42.0, 444.0, 16.0, 8.0, [20, 5, 15, 200]);
            if f > 0.004 { ui.rect(cx - 220.0, 44.0, (440.0 * f).max(12.0), 12.0, 6.0, HOT); }
        }
        // kill feed
        for (i, (t, age)) in self.feed.iter().enumerate() {
            let a = ((4.2 - age) / 0.6).min(1.0) * (age / 0.15).min(1.0);
            let tw = ui.text_w(t, 13.0);
            ui.rect(w - 44.0 - tw, 24.0 + i as f32 * 28.0, tw + 20.0, 23.0, 7.0, fade(GLASS, a));
            ui.text(w - 34.0 - tw, 28.5 + i as f32 * 28.0, 13.0, fade(WHITE, a), t);
        }
        // health, armor, slide charges
        {
            let (x, y) = (24.0, h - 100.0);
            ui.panel(x, y, 308.0, 76.0, 14.0);
            let (hp, frac) = (self.player.hp.max(0.0), (self.player.hp / self.m.max_hp).clamp(0.0, 1.0));
            let low = frac < 0.3;
            let hw = ui.text(x + 16.0, y + 7.0, 34.0, if low { RED } else { WHITE }, &format!("{}", hp.ceil() as i32));
            ui.text(x + 22.0 + hw, y + 24.0, 12.0, DIM, "HP");
            if self.player.armor > 0.5 { ui.text(x + 52.0 + hw, y + 24.0, 12.0, BLUE, &format!("+{} ARMOR", self.player.armor.ceil() as i32)); }
            let n = self.m.dash_max as usize;
            for i in 0..n {
                let (px, fill) = (x + 308.0 - 18.0 - (n - i) as f32 * 17.0 + 4.0, (self.player.dash - i as f32).clamp(0.0, 1.0));
                ui.rect(px, y + 14.0, 13.0, 13.0, 3.5, [255, 255, 255, 34]);
                if fill > 0.05 { ui.rect(px, y + 14.0 + 13.0 * (1.0 - fill), 13.0, 13.0 * fill, 3.5, fade(MINT, if fill >= 1.0 { 1.0 } else { 0.5 })); }
            }
            let (bx, by, bw) = (x + 16.0, y + 52.0, 276.0);
            ui.rect(bx, by, bw, 10.0, 5.0, [255, 255, 255, 30]);
            if self.hp_ghost > frac + 0.004 { ui.rect(bx, by, (bw * self.hp_ghost).max(10.0), 10.0, 5.0, [255, 255, 255, 130]); }
            if frac > 0.004 { ui.rect(bx, by, (bw * frac).max(10.0), 10.0, 5.0, if low { RED } else { MINT }); }
            if self.player.armor > 0.5 { ui.rect(bx, by - 6.0, (bw * self.player.armor / 100.0).max(4.0), 3.0, 1.5, BLUE); }
        }
        // Sugar Rush meter
        {
            let (bw, by) = (300.0, h - 46.0);
            let (rush, full) = (self.player.rush > 0.0, self.player.sugar >= 100.0);
            let pulse = 0.65 + 0.35 * (self.clock * 9.0).sin();
            ui.rect(cx - bw / 2.0 - 2.0, by - 2.0, bw + 4.0, 14.0, 7.0, [20, 5, 15, 170]);
            let f = (self.player.sugar / 100.0).clamp(0.0, 1.0);
            if f > 0.01 { ui.rect(cx - bw / 2.0, by, (bw * f).max(10.0), 10.0, 5.0, if rush { SUN } else if full { fade(HOT, pulse) } else { PINK }); }
            if full && !rush {
                let tw = ui.text_w("SUGAR RUSH READY", 13.0);
                ui.rect(cx - tw / 2.0 - 30.0, by - 28.0, 20.0, 20.0, 5.0, fade(WHITE, pulse));
                ui.text_c(cx - tw / 2.0 - 20.0, by - 25.0, 14.0, INK, "F");
                ui.label(cx - tw / 2.0, by - 25.0, 13.0, fade(SUN, pulse), "SUGAR RUSH READY", 0);
            } else { ui.label(cx, by - 23.0, 11.5, if rush { SUN } else { fade(WHITE, 0.6) }, "SUGAR RUSH", 1); }
        }
        // ammo and weapons
        {
            let (rx, y) = (w - 24.0, h - 100.0);
            ui.panel(rx - 252.0, y, 252.0, 76.0, 14.0);
            let free = self.player.rush > 0.0 && g.loot.is_none();
            let rest = if g.loot.is_some() { "no spare mags".to_string() } else if free { "free fire".to_string() } else { format!("/ {}", g.clip) };
            let aw = ui.text_w(&g.ammo.to_string(), 40.0);
            ui.text(rx - 236.0, y + 4.0, 40.0, if !free && g.ammo * 4 <= g.clip { RED } else { WHITE }, &g.ammo.to_string());
            ui.text(rx - 228.0 + aw, y + 25.0, 14.0, DIM, &rest);
            ui.text(rx - 236.0, y + 52.0, 12.0, fade(WHITE, 0.75), g.name);
            let mut sx = rx;
            for i in (0..self.guns.len()).rev() {
                let s = &self.guns[i];
                if !s.owned { continue; }
                let label = format!("{}  {}", i + 1, s.short);
                let sw = ui.text_w(&label, 12.0) + 18.0;
                sx -= sw;
                ui.rect(sx, y - 34.0, sw, 26.0, 8.0, if i == gi { fade(HOT, 0.9) } else { GLASS });
                ui.text(sx + 9.0, y - 27.5, 12.0, fade(WHITE, if i == gi { 1.0 } else { 0.7 }), &label);
                sx -= 6.0;
            }
            if self.reload_t > 0.0 && playing {
                let total = (g.reload * self.m.reload).max(0.01);
                ui.label(cx, cy + 42.0, 11.5, fade(WHITE, 0.85), "RELOADING", 1);
                ui.bar(cx - 50.0, cy + 60.0, 100.0, 5.0, 1.0 - self.reload_t / total, WHITE);
            } else if playing && g.loot.is_none() && !free && g.ammo * 4 <= g.clip {
                ui.label(cx, cy + 46.0, 12.0, fade(RED, 0.65 + 0.35 * (self.clock * 8.0).sin()), if g.ammo == 0 { "R  RELOAD" } else { "LOW AMMO" }, 1);
            }
        }
        // pick-up prompt
        if let (Some(name), true) = (self.loot_name(), playing) {
            let t = format!("Pick up {name}");
            let tw = ui.text_w(&t, 15.0);
            let (px, py) = (cx - (tw + 54.0) / 2.0, h * 0.635);
            ui.panel(px, py, tw + 54.0, 36.0, 10.0);
            ui.rect(px + 8.0, py + 7.0, 22.0, 22.0, 5.0, WHITE);
            ui.text_c(px + 19.0, py + 10.0, 15.0, INK, "E");
            ui.text(px + 40.0, py + 9.5, 15.0, WHITE, &t);
        }
        // radio
        let (msg, left) = self.radio;
        if left > 0.0 && !msg.is_empty() {
            let a = (left / 0.4).min(1.0) * ((5.0 - left) / 0.25).clamp(0.0, 1.0);
            let (x, wd) = (24.0, 400.0);
            let ph = ui.wrap_h(wd - 62.0, 13.5, msg) + 18.0;
            let y = h - 112.0 - ph;
            ui.rect(x, y, wd, ph, 12.0, fade(GLASS, a));
            ui.rect(x, y + 8.0, 3.0, ph - 16.0, 1.5, fade(MINT, a));
            ui.text(x + 14.0, y + 11.0, 11.0, fade(MINT, a), "HQ");
            ui.wrap(x + 46.0, y + 9.0, wd - 62.0, 13.5, fade(WHITE, a), msg);
        }
    }

    // ───────────────────────── menus ─────────────────────────
    fn back(&mut self) {
        self.audio.ui(); self.reset_armed = false; self.profile.save();
        if self.screens.len() > 1 { self.screens.pop(); } else if self.state == State::Paused { self.resume(); } else if self.state != State::Menu { self.to_menu(); }
    }
    fn screen(&mut self, ui: &mut Ui, scr: Screen, inp: &Input) {
        let (w, h) = (ui.w, ui.h);
        let dim = match scr { Screen::Menu => 0.0, Screen::Pause => 0.55, Screen::Results => 0.6, _ => 0.72 };
        if dim > 0.0 { ui.rect(0.0, 0.0, w, h, 0.0, fade(SCRIM, dim)); }
        if inp.hit(KeyCode::Escape) {
            match scr { Screen::Menu | Screen::Results => {} Screen::Pause => { self.audio.ui(); self.resume(); return; } _ => { self.back(); return; } }
        }
        match scr {
            Screen::Menu => self.scr_menu(ui),
            Screen::Map => self.scr_map(ui),
            Screen::Brief(id) => self.scr_brief(ui, id),
            Screen::Armory => self.scr_armory(ui),
            Screen::Settings => self.scr_settings(ui, inp),
            Screen::Pause => self.scr_pause(ui),
            Screen::Results => self.scr_results(ui),
        }
    }
    fn chip(&self, ui: &mut Ui, x: f32, y: f32, right: bool) {
        let p = &self.profile;
        let (lv, rank, candy) = (format!("Lv {}", p.level), rank_name(p.level), format!("{} candy", commas(p.candy)));
        let pts = if p.perk_pts > 0 { format!("{} perk pt{}", p.perk_pts, if p.perk_pts == 1 { "" } else { "s" }) } else { String::new() };
        let wd = 16.0 + ui.text_w(&lv, 15.0) + 10.0 + ui.text_w(rank, 13.0) + 12.0 + 70.0 + 14.0 + ui.text_w(&candy, 14.0) + if pts.is_empty() { 0.0 } else { 14.0 + ui.text_w(&pts, 13.0) } + 16.0;
        let x0 = if right { x - wd } else { x };
        ui.rect(x0, y, wd, 34.0, 17.0, [255, 255, 255, 20]); ui.ring(x0, y, wd, 34.0, 17.0, 1.0, LINE);
        let mut pen = x0 + 16.0;
        pen += ui.text(pen, y + 9.0, 15.0, WHITE, &lv) + 10.0;
        pen += ui.text(pen, y + 10.5, 13.0, DIM, rank) + 12.0;
        ui.bar(pen, y + 14.0, 70.0, 6.0, p.xp as f32 / xp_for(p.level) as f32, MINT);
        pen += 84.0;
        pen += ui.text(pen, y + 9.5, 14.0, PINK, &candy);
        if !pts.is_empty() { ui.text(pen + 14.0, y + 10.5, 13.0, SUN, &pts); }
    }
    fn sheet(&self, ui: &mut Ui, want_w: f32, hgt: f32) -> (f32, f32, f32, f32) {
        let pw = want_w.min(ui.w - 48.0);
        let (px, py) = ((ui.w - pw) / 2.0, (ui.h - hgt) / 2.0);
        ui.rect(px, py, pw, hgt, 24.0, SHEET); ui.ring(px, py, pw, hgt, 24.0, 1.0, LINE);
        (px, py, pw, hgt)
    }
    /// Back button, title and the profile chip. Returns true when back was clicked.
    fn header(&self, ui: &mut Ui, x: f32, y: f32, w: f32, title: &str, kicker: &str) -> bool {
        let back = ui.button_c(x, y, 44.0, 44.0, "<", false, true);
        if kicker.is_empty() { ui.text(x + 60.0, y + 5.0, 32.0, WHITE, title); } else { ui.text(x + 60.0, y - 3.0, 11.5, SUN, kicker); ui.text(x + 60.0, y + 13.0, 30.0, WHITE, title); }
        self.chip(ui, x + w, y + 5.0, true);
        back
    }
    fn scr_menu(&mut self, ui: &mut Ui) {
        let h = ui.h;
        for i in 0..36 { ui.rect(0.0, 0.0, 400.0 + i as f32 * 6.0, h, 0.0, fade(SCRIM, 0.03)); }
        let x = 84.0;
        ui.label(x, 62.0, 100.0, PINK, "Gummy", 0);
        ui.label(x, 148.0, 100.0, WHITE, "Grove", 0);
        ui.text(x + 4.0, 264.0, 17.0, fade(WHITE, 0.82), "They have guns. You have jellybeans.");
        self.chip(ui, x, 302.0, false);
        let (nm, started) = (self.profile.next_mission(), !self.profile.done.is_empty());
        let (label, sub) = if !started { ("New Campaign".to_string(), String::new()) } else if let Some(m) = nm { ("Continue".to_string(), format!("Mission {} · {}", m.id, m.title)) } else { ("Campaign Complete".to_string(), "replay missions".to_string()) };
        let (bw, mut y) = (390.0, 360.0);
        if ui.button(x, y, bw, 60.0, &label, &sub, true, true) { self.audio.ui(); self.screens.push(match nm { Some(m) => Screen::Brief(m.id), None => Screen::Map }); return; }
        y += 76.0;
        if ui.button(x, y, bw, 48.0, "Missions", "", false, true) { self.audio.ui(); self.screens.push(Screen::Map); return; }
        y += 56.0;
        let best = if self.profile.best > 0 { format!("best {}", commas(self.profile.best)) } else { "endless waves".to_string() };
        if ui.button(x, y, bw, 48.0, "Arcade", &best, false, true) { self.audio.ui(); self.start_run(0); return; }
        y += 56.0;
        if ui.button(x, y, bw, 48.0, "Armory", if self.profile.perk_pts > 0 { "perk point ready" } else { "" }, false, true) { self.audio.ui(); self.screens.push(Screen::Armory); return; }
        y += 56.0;
        if ui.button(x, y, (bw - 10.0) / 2.0, 48.0, "Settings", "", false, true) { self.audio.ui(); self.screens.push(Screen::Settings); return; }
        if ui.button(x + (bw + 10.0) / 2.0, y, (bw - 10.0) / 2.0, 48.0, "Quit", "", false, true) { self.profile.save(); self.quit = true; }
    }
    fn scr_map(&mut self, ui: &mut Ui) {
        let (px, py, pw, _) = self.sheet(ui, 1080.0, 566.0);
        if self.header(ui, px + 36.0, py + 30.0, pw - 72.0, "Campaign", "") { self.back(); return; }
        let (gx, gy, gap) = (px + 36.0, py + 104.0, 16.0);
        let (cw, ch) = ((pw - 72.0 - gap * 3.0) / 4.0, 198.0);
        for (i, m) in MISSIONS.iter().enumerate() {
            let (x, y0) = (gx + (i % 4) as f32 * (cw + gap), gy + (i / 4) as f32 * (ch + gap));
            let open = self.profile.unlocked(m.id);
            let hov = open && ui.hover(x, y0, cw, ch);
            if hov { ui.hot = true; }
            let tint: Col = match m.palette { "sunset" => [255, 120, 80, 255], "overcast" => [150, 170, 190, 255], "night" => [96, 104, 230, 255], _ => [255, 190, 110, 255] };
            let y = y0 - if hov { 4.0 } else { 0.0 };
            ui.rect(x, y, cw, ch, 16.0, fade(tint, if !open { 0.05 } else if hov { 0.36 } else { 0.2 }));
            ui.ring(x, y, cw, ch, 16.0, 1.0, LINE);
            ui.text(x + 16.0, y + 12.0, 44.0, fade(WHITE, if open { 0.3 } else { 0.1 }), &format!("{:02}", m.id));
            if open { ui.wrap(x + 16.0, y + 72.0, cw - 30.0, 18.0, WHITE, m.title); } else { ui.text(x + 16.0, y + 72.0, 18.0, fade(WHITE, 0.35), "Locked"); }
            ui.text(x + 16.0, y + ch - 60.0, 12.0, fade(WHITE, if open { 0.6 } else { 0.25 }), &format!("{} · {}", m.obj_name(), m.palette));
            stars(ui, x + 26.0, y + ch - 26.0, 9.0, self.profile.stars(m.id));
            if hov && ui.click { self.audio.ui(); self.screens.push(Screen::Brief(m.id)); return; }
        }
    }
    fn scr_brief(&mut self, ui: &mut Ui, id: u32) {
        let Some(m) = MISSIONS.iter().find(|m| m.id == id) else { self.back(); return };
        let (px, py, pw, ph) = self.sheet(ui, 940.0, 600.0);
        let (x, w) = (px + 40.0, pw - 80.0);
        if self.header(ui, x, py + 30.0, w, m.title, &format!("MISSION {} · {}", m.id, m.palette.to_uppercase())) { self.back(); return; }
        stars(ui, x + 70.0, py + 96.0, 8.0, self.profile.stars(m.id));
        let mut y = py + 124.0;
        y += ui.wrap(x, y, w, 16.0, fade(WHITE, 0.88), m.brief) + 20.0;
        let cw = (w - 40.0) / 2.0;
        let mut ly = y;
        ly += h3(ui, x, ly, "OBJECTIVE");
        ly += ui.wrap(x, ly, cw, 14.0, WHITE, &m.obj_text()) + 12.0;
        ly += h3(ui, x, ly, "HOSTILES");
        let threats = m.threats.iter().map(|k| k.plural()).collect::<Vec<_>>().join(" · ");
        ly += ui.wrap(x, ly, cw, 14.0, WHITE, &threats) + 12.0;
        ly += h3(ui, x, ly, "INSERTION");
        ui.wrap(x, ly, cw, 14.0, WHITE, if m.insert == Insert::Sdv { "Underwater, by diver scooter, up the boat ramp." } else { "Parachute drop over the bay." });
        let (rx, mut ry) = (x + cw + 40.0, y);
        ry += h3(ui, rx, ry, "STARS");
        for t in ["Complete the mission".to_string(), "Finish with 50%+ health".to_string(), format!("{} headshots", m.heads)] { ui.star(rx + 8.0, ry + 8.0, 6.0, true); ui.text(rx + 24.0, ry, 14.0, WHITE, &t); ry += 24.0; }
        ry += 10.0;
        ry += h3(ui, rx, ry, "REWARD");
        let pen = rx + ui.text(rx, ry, 16.0, PINK, &format!("{} candy", m.candy)) + 14.0;
        ui.text(pen, ry + 1.0, 15.0, WHITE, &format!("{} XP", m.xp));
        if let (Some(u), true) = (m.unlock, self.profile.stars(m.id) == 0) { ui.text(rx, ry + 28.0, 14.0, SUN, &format!("Unlocks {u}")); }
        let fy = py + ph - 80.0;
        if ui.button_c(x, fy, 150.0, 48.0, "Armory", false, true) { self.audio.ui(); self.screens.push(Screen::Armory); return; }
        if ui.button_c(x + w - 230.0, fy, 230.0, 50.0, "Deploy", true, true) { self.audio.ui(); self.start_run(m.id); }
    }
    fn scr_armory(&mut self, ui: &mut Ui) {
        let (px, py, pw, _) = self.sheet(ui, 1120.0, 672.0);
        let (x, w) = (px + 36.0, pw - 72.0);
        if self.header(ui, x, py + 26.0, w, "Armory", "") { self.back(); return; }
        let mut y = py + 90.0;
        y += h3(ui, x, y, "UPGRADES");
        let gap = 12.0;
        let (cw, ch) = ((w - gap * 3.0) / 4.0, 128.0);
        for (i, u) in UPGRADES.iter().enumerate() {
            let (cx, cy) = (x + (i % 4) as f32 * (cw + gap), y + (i / 4) as f32 * (ch + gap));
            let t = self.profile.up(u.id);
            let (locked, max, cost) = (u.needs > 0 && self.profile.stars(u.needs) == 0, t >= 3, TIER_COST[(t as usize).min(2)]);
            let a = if locked { 0.45 } else { 1.0 };
            ui.rect(cx, cy, cw, ch, 14.0, [255, 255, 255, if locked { 8 } else { 18 }]); ui.ring(cx, cy, cw, ch, 14.0, 1.0, LINE);
            ui.text(cx + 14.0, cy + 11.0, 10.5, fade(SUN, a), &u.group.to_uppercase());
            ui.text(cx + 14.0, cy + 27.0, 16.0, fade(WHITE, a), u.name);
            let desc = if locked { format!("Unlocks after Mission {}.", u.needs) } else { u.desc.to_string() };
            ui.wrap(cx + 14.0, cy + 51.0, cw - 28.0, 12.0, fade(WHITE, 0.62 * a), &desc);
            for k in 0..3 { ui.rect(cx + 14.0 + k as f32 * 20.0, cy + ch - 25.0, 15.0, 7.0, 3.5, if k < t { MINT } else { [255, 255, 255, 40] }); }
            let can = !locked && !max && self.profile.candy >= cost;
            if ui.button_c(cx + cw - 94.0, cy + ch - 42.0, 82.0, 30.0, &if max { "Maxed".to_string() } else { commas(cost) }, can, can) {
                self.profile.candy -= cost; self.profile.up.insert(u.id.to_string(), t + 1); self.profile.save(); self.m = mods(&self.profile); self.audio.buy();
            }
        }
        y += 2.0 * (ch + gap) + 12.0;
        let pp = self.profile.perk_pts;
        let pen = x + ui.text(x, y, 11.5, SUN, "PERKS") + 12.0;
        ui.text(pen, y, 11.5, DIM, &format!("{pp} point{} to spend · earn one every level", if pp == 1 { "" } else { "s" }));
        y += 22.0;
        let (cw, ch) = ((w - gap * 2.0) / 3.0, 82.0);
        for (i, p) in PERKS.iter().enumerate() {
            let (cx, cy) = (x + (i % 3) as f32 * (cw + gap), y + (i / 3) as f32 * (ch + gap));
            let has = self.profile.perk(p.id);
            ui.rect(cx, cy, cw, ch, 14.0, if has { fade(MINT, 0.14) } else { [255, 255, 255, 18] }); ui.ring(cx, cy, cw, ch, 14.0, 1.0, LINE);
            ui.text(cx + 14.0, cy + 12.0, 16.0, WHITE, p.name);
            ui.wrap(cx + 14.0, cy + 36.0, cw - 132.0, 12.0, fade(WHITE, 0.62), p.desc);
            let can = !has && pp >= p.cost;
            if ui.button_c(cx + cw - 102.0, cy + (ch - 30.0) / 2.0, 90.0, 30.0, &if has { "Equipped".to_string() } else { format!("{} pt{}", p.cost, if p.cost > 1 { "s" } else { "" }) }, can, can) {
                self.profile.perk_pts -= p.cost; self.profile.perks.push(p.id.to_string()); self.profile.save(); self.m = mods(&self.profile); self.audio.buy();
            }
        }
    }
    fn scr_settings(&mut self, ui: &mut Ui, inp: &Input) {
        let (px, py, pw, _) = self.sheet(ui, 760.0, 606.0);
        let (x, w) = (px + 40.0, pw - 80.0);
        if self.header(ui, x, py + 30.0, w, "Settings", "") { self.back(); return; }
        let mut y = py + 108.0;
        let mut sens = self.profile.sens;
        if slider(ui, inp, x, y, w, "Mouse sensitivity", &mut sens, 0.3, 2.5) { self.profile.sens = sens; }
        y += 62.0;
        let mut vol = self.profile.volume;
        if slider(ui, inp, x, y, w, "Volume", &mut vol, 0.0, 1.0) { self.profile.volume = vol; self.audio.set_volume(vol); }
        y += 70.0;
        let ri = RES.iter().position(|r| (r.0 - self.profile.scale).abs() < 0.01).unwrap_or(0);
        let hw = (w - 16.0) / 2.0;
        if ui.button(x, y, hw, 50.0, &format!("Resolution: {}", RES[ri].1), "", false, true) { self.audio.ui(); self.profile.scale = RES[(ri + 1) % RES.len()].0; self.scale_changed = true; self.profile.save(); }
        if ui.button(x + hw + 16.0, y, hw, 50.0, if self.reset_armed { "Click again to wipe everything" } else { "Reset progress" }, "", false, true) {
            if self.reset_armed { self.profile.reset(); self.profile.save(); self.m = mods(&self.profile); self.reset_armed = false; self.audio.deny(); } else { self.reset_armed = true; self.audio.ui(); }
        }
        y += 74.0;
        y += h3(ui, x, y, "CONTROLS") + 2.0;
        let keys = [("WASD", "move"), ("Shift", "sprint"), ("C", "crouch · slide while moving"), ("Space", "jump · climb ledges"), ("Click", "fire"), ("Right click", "aim down sights"), ("V", "melee"), ("E", "pick up gear"),
            ("1 2 3 4", "weapons"), ("Q", "last weapon"), ("R", "reload"), ("F", "Sugar Rush"), ("Esc", "pause"), ("F11", "fullscreen")];
        for (i, (k, d)) in keys.iter().enumerate() {
            let (kx, ky) = (x + (i % 2) as f32 * (w / 2.0), y + (i / 2) as f32 * 30.0);
            let kw = ui.text_w(k, 12.0) + 16.0;
            ui.rect(kx, ky, kw, 22.0, 6.0, [255, 255, 255, 36]);
            ui.text(kx + 8.0, ky + 5.0, 12.0, WHITE, k);
            ui.text(kx + kw + 10.0, ky + 4.5, 13.0, DIM, d);
        }
    }
    fn scr_pause(&mut self, ui: &mut Ui) {
        let (pw, ph) = (400.0, 426.0);
        let (px, py, cx) = ((ui.w - pw) / 2.0, (ui.h - ph) / 2.0, ui.w / 2.0);
        ui.rect(px, py, pw, ph, 24.0, SHEET); ui.ring(px, py, pw, ph, 24.0, 1.0, LINE);
        ui.text_c(cx, py + 30.0, 44.0, WHITE, "Paused");
        ui.text_c(cx, py + 88.0, 14.0, DIM, "The bears are politely waiting.");
        let (bx, bw, mut y) = (px + 36.0, pw - 72.0, py + 128.0);
        if ui.button(bx, y, bw, 56.0, "Resume", "", true, true) { self.audio.ui(); self.resume(); return; }
        y += 74.0;
        if ui.button(bx, y, bw, 48.0, "Restart", "", false, true) { let id = self.run.as_ref().and_then(|r| r.mission).map(|m| m.id).unwrap_or(0); self.audio.ui(); self.start_run(id); return; }
        y += 58.0;
        if ui.button(bx, y, bw, 48.0, "Settings", "", false, true) { self.audio.ui(); self.screens.push(Screen::Settings); return; }
        y += 58.0;
        if ui.button(bx, y, bw, 48.0, "Abandon to menu", "", false, true) { self.audio.ui(); self.to_menu(); }
    }
    fn scr_results(&mut self, ui: &mut Ui) {
        let Some(r) = self.results.as_ref() else { self.to_menu(); return };
        let (win, mission, score, wave, stats, candy, xp, n_stars, level_ups, new_best, age, unlock) = (r.win, r.mission, r.score, r.wave, r.stats, r.candy, r.xp, r.stars, r.level_ups, r.new_best, r.age, r.unlock);
        let bonus = r.bonus.clone();
        let starred = win && mission.is_some();
        let (px, py, pw, ph) = self.sheet(ui, 860.0, if starred { 624.0 } else { 440.0 });
        let (cx, mut y) = (px + pw / 2.0, py + 34.0);
        ui.text_c(cx, y, 46.0, if win { SUN } else { RED }, if win { "Mission Complete" } else if mission.is_some() { "Mission Failed" } else { "Chewed Up" });
        y += 62.0;
        let sub = match mission { Some(m) => format!("Mission {} · {}", m.id, m.title), None => format!("Arcade · reached wave {wave}{}", if new_best { " · new best" } else { "" }) };
        ui.text_c(cx, y, 15.0, DIM, &sub);
        y += 38.0;
        if starred {
            for i in 0..3u32 {
                let at = 0.5 + i as f32 * 0.38;
                let on = i < n_stars && age > at;
                let pop = if on { 1.0 + (1.0 - ((age - at) / 0.25).min(1.0)) * 0.6 } else { 1.0 };
                ui.star(cx + (i as f32 - 1.0) * 76.0, y + 30.0, 24.0 * pop, on);
            }
            y += 78.0;
            for (t, ok) in &bonus {
                let tw = ui.text_w(t, 14.0);
                ui.star(cx - tw / 2.0 - 12.0, y + 8.0, 6.0, *ok);
                ui.text(cx - tw / 2.0 + 4.0, y, 14.0, fade(WHITE, if *ok { 1.0 } else { 0.45 }), t);
                y += 23.0;
            }
            y += 14.0;
        }
        let acc = if stats.shots > 0 { ((stats.hits as f32 / stats.shots as f32) * 100.0).round().min(100.0) as u32 } else { 0 };
        let cols = [(commas(score), "Score"), (stats.kills.to_string(), "Kills"), (stats.heads.to_string(), "Headshots"), (stats.disarms.to_string(), "Disarms"), (format!("{acc}%"), "Accuracy")];
        let sw = (pw - 80.0) / 5.0;
        ui.rect(px + 40.0, y, pw - 80.0, 84.0, 16.0, [255, 255, 255, 14]);
        for (i, (v, l)) in cols.iter().enumerate() { let sx = px + 40.0 + sw * (i as f32 + 0.5); ui.text_c(sx, y + 14.0, 28.0, WHITE, v); ui.text_c(sx, y + 53.0, 12.0, DIM, l); }
        y += 104.0;
        let (a, b) = (format!("+{} candy", commas(candy)), format!("+{xp} XP"));
        let (aw, bw) = (ui.text_w(&a, 20.0), ui.text_w(&b, 17.0));
        ui.text(cx - (aw + bw + 18.0) / 2.0, y, 20.0, PINK, &a);
        ui.text(cx - (aw + bw + 18.0) / 2.0 + aw + 18.0, y + 2.0, 17.0, WHITE, &b);
        y += 34.0;
        if level_ups > 0 { ui.text_c(cx, y, 14.0, MINT, &format!("Level up · Lv {} {} · perk point earned", self.profile.level, rank_name(self.profile.level))); y += 24.0; }
        if let Some(u) = unlock { ui.text_c(cx, y, 14.0, SUN, &format!("New weapon · {u}")); }
        let (fy, fx) = (py + ph - 78.0, px + 40.0);
        if ui.button_c(fx, fy, 130.0, 48.0, "Menu", false, true) { self.audio.ui(); self.to_menu(); return; }
        if ui.button_c(fx + 142.0, fy, 130.0, 48.0, "Armory", false, true) { self.audio.ui(); self.screens.push(Screen::Armory); return; }
        let nm = self.profile.next_mission();
        let label = if win { if nm.is_some() { "Next Mission" } else { "Play Arcade" } } else if mission.is_some() { "Retry" } else { "Play Again" };
        if ui.button_c(px + pw - 40.0 - 230.0, fy, 230.0, 50.0, label, true, true) {
            self.audio.ui();
            if win { match nm { Some(m) => self.screens.push(Screen::Brief(m.id)), None => self.start_run(0) } } else { self.start_run(mission.map(|m| m.id).unwrap_or(0)); }
        }
    }
}
