//! Combat, bear AI, missions and the per-frame scene. A fairly direct port of the web version's game loop.
use crate::audio::{Audio, HERE};
use crate::bear::{self, BearPose};
use crate::gfx::{Draw, GummyU, Inst, Renderer, Sprite};
use crate::guns::{self, Gear, ViewModel, Wpn};
use crate::mesh::{Kit, Mat as SurfMat, Mesh};
use crate::meta::*;
use crate::tex::Atlas;
use crate::ui::Ui;
use crate::world::{lin, palette, Palette, World, ARENA_R, CAKE, SPAWN, WATER_Y};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::f32::consts::{PI, TAU};
use winit::keyboard::KeyCode;

mod hud;

// ───────────────────────── small helpers ─────────────────────────
thread_local! { static RNG: Cell<u32> = const { Cell::new(0x9E37_79B9) }; }
pub fn rnd() -> f32 { RNG.with(|r| { let mut x = r.get(); x ^= x << 13; x ^= x >> 17; x ^= x << 5; r.set(x); (x & 0xff_ffff) as f32 / 16_777_216.0 }) }
pub fn rr(a: f32, b: f32) -> f32 { a + rnd() * (b - a) }
pub fn pick<T: Copy>(a: &[T]) -> T { a[((rnd() * a.len() as f32) as usize).min(a.len() - 1)] }
pub fn chance(p: f32) -> bool { rnd() < p }
pub fn damp(a: f32, b: f32, k: f32, dt: f32) -> f32 { b + (a - b) * (-k * dt).exp() }
pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn v3(x: f32, y: f32, z: f32) -> Vec3 { Vec3::new(x, y, z) }
fn seg_sphere(a: Vec3, b: Vec3, c: Vec3, r: f32) -> Option<Vec3> {
    let ab = b - a;
    let t = ((c - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    let p = a + ab * t;
    (p.distance_squared(c) < r * r).then_some(p)
}
const CANDY: [u32; 7] = [0xff3b4e, 0x3fd45c, 0xffc21f, 0xff8a1f, 0xff5fb8, 0x52e0ff, 0x9b4dff];

#[derive(Default)]
pub struct Input {
    pub down: HashSet<KeyCode>, pub pressed: Vec<KeyCode>, pub dx: f32, pub dy: f32,
    pub lmb: bool, pub rmb: bool, pub click: bool, pub mmb: bool, pub wheel: f32, pub mouse: (f32, f32),
}
impl Input {
    pub fn key(&self, k: KeyCode) -> bool { self.down.contains(&k) }
    pub fn hit(&self, k: KeyCode) -> bool { self.pressed.contains(&k) }
    pub fn end_frame(&mut self) { self.pressed.clear(); self.dx = 0.0; self.dy = 0.0; self.click = false; self.mmb = false; self.wheel = 0.0; }
}

// ───────────────────────── data ─────────────────────────
pub struct BearType {
    pub name: &'static str, pub s: f32, pub hp: f32, pub speed: f32, pub weapon: Wpn, pub melee: bool, pub mdmg: f32, pub two: bool, pub shield: bool, pub laser: bool, pub hitscan: bool,
    pub range: (f32, f32), pub aim: f32, pub burst: u32, pub burst_gap: f32, pub cool: (f32, f32), pub b_speed: f32, pub dmg: f32, pub acc: f32, pub track: f32, pub aim_move: f32,
    pub pts: u32, pub colors: &'static [u32], pub gear: &'static [Gear], pub say: Option<&'static [&'static str]>, pub arm_hp: f32,
}
const BT: BearType = BearType { name: "", s: 1.0, hp: 4.0, speed: 3.6, weapon: Wpn::Pistol, melee: false, mdmg: 9.0, two: false, shield: false, laser: false, hitscan: false, range: (8.0, 17.0), aim: 0.55, burst: 1, burst_gap: 0.3, cool: (1.1, 2.0), b_speed: 30.0, dmg: 8.0, acc: 0.03, track: 5.0, aim_move: 0.45, pts: 100, colors: &[0xff3b4e], gear: &[], say: None, arm_hp: 2.0 };
static GRUNT: BearType = BearType { name: "Pistol Bear", colors: &[0xff3b4e, 0xff8a1f, 0xffc21f, 0x3fd45c], gear: &[Gear::Shades], ..BT };
static KNIFE: BearType = BearType { name: "Stabby Runt", s: 0.62, hp: 2.0, speed: 6.8, weapon: Wpn::Knife, melee: true, mdmg: 11.0, pts: 120, colors: &[0xff5fb8, 0x52e0ff], gear: &[Gear::Bandana], arm_hp: 1.0, ..BT };
static RIFLE: BearType = BearType { name: "Rifle Bear", s: 1.05, hp: 6.0, speed: 3.2, weapon: Wpn::Rifle, two: true, range: (11.0, 24.0), aim: 0.7, burst: 4, burst_gap: 0.11, cool: (1.6, 2.6), b_speed: 40.0, dmg: 5.0, acc: 0.035, track: 4.0, pts: 200, colors: &[0x3fd45c, 0xffc21f, 0xff8a1f], gear: &[Gear::Helmet], say: Some(&L_RIFLE), arm_hp: 3.0, ..BT };
static SHIELD: BearType = BearType { name: "Shield Bear", s: 1.25, hp: 9.0, speed: 2.7, shield: true, range: (4.5, 10.0), aim: 0.6, burst: 2, burst_gap: 0.28, cool: (1.3, 2.2), aim_move: 0.8, pts: 300, colors: &[0x2f7bff], gear: &[Gear::Helmet], say: Some(&L_SHIELD), arm_hp: 3.0, ..BT };
static RPG: BearType = BearType { name: "Rocket Bear", s: 1.15, hp: 6.0, speed: 2.7, weapon: Wpn::Rpg, two: true, laser: true, range: (17.0, 34.0), aim: 1.5, burst_gap: 1.0, cool: (3.2, 4.8), b_speed: 17.0, dmg: 40.0, acc: 0.01, track: 3.0, aim_move: 0.2, pts: 350, colors: &[0x9b4dff], gear: &[Gear::Beret], say: Some(&L_RPG), arm_hp: 3.0, ..BT };
static SNIPER: BearType = BearType { name: "Sniper Bear", speed: 2.9, weapon: Wpn::Sniper, two: true, laser: true, hitscan: true, range: (24.0, 60.0), aim: 2.0, burst_gap: 1.0, cool: (2.4, 3.6), dmg: 32.0, acc: 0.0, track: 8.0, aim_move: 0.0, pts: 350, colors: &[0x52e0ff, 0x9fd4ff], gear: &[Gear::Beret], say: Some(&L_SNIPER), ..BT };
static BOSS: BearType = BearType { name: "King Gummy", s: 3.2, hp: 150.0, speed: 2.5, weapon: Wpn::Minigun, two: true, range: (7.0, 28.0), aim: 1.1, burst: 34, burst_gap: 0.065, cool: (2.0, 3.0), b_speed: 36.0, dmg: 4.0, acc: 0.05, track: 2.2, aim_move: 0.5, pts: 3000, gear: &[Gear::Crown, Gear::Shades], say: Some(&L_BOSS), arm_hp: 999.0, ..BT };
pub fn bt(k: Kind) -> &'static BearType { match k { Kind::Grunt => &GRUNT, Kind::Knife => &KNIFE, Kind::Rifle => &RIFLE, Kind::Shield => &SHIELD, Kind::Rpg => &RPG, Kind::Sniper => &SNIPER, Kind::Boss => &BOSS } }

const L_SPOT: [&str; 5] = ["CONTACT!", "THERE HE IS!", "CANDY THIEF, 12 O'CLOCK!", "WEAPONS FREE!", "GET THE BIG ONE!"];
const L_HIT: [&str; 5] = ["OW! MY GELATIN!", "I'M HIT!", "MEDIC!", "THAT'LL LEAVE A DENT!", "RUDE!"];
const L_DISARM: [&str; 5] = ["MY ARM!", "I NEEDED THAT!", "FINE. I'LL BITE.", "WHO NEEDS ARMS?!", "THAT WAS MY GOOD ARM!"];
const L_ALLY: [&str; 5] = ["TEDDY, NOOO!", "MAN DOWN!", "HE WAS SO SWEET!", "THEY GOT GARY!", "AVENGE HIM!"];
const L_RIFLE: [&str; 3] = ["SUPPRESSING FIRE!", "COVER ME!", "LIGHT HIM UP!"];
const L_RPG: [&str; 3] = ["FIRE IN THE HOLE!", "ROCKET OUT!", "SAY CHEESE!"];
const L_SNIPER: [&str; 3] = ["HOLD STILL...", "STEADY...", "ONE SHOT..."];
const L_SHIELD: [&str; 3] = ["ADVANCE!", "PUSH! PUSH!", "S.W.E.E.T. TEAM, GO!"];
const L_MELEE: [&str; 3] = ["NOM NOM!", "CHOMP!", "COME HERE!"];
const L_BOSS: [&str; 3] = ["KNEEL BEFORE YOUR KING!", "YOU CALL THOSE BULLETS?", "I AM THE ONE TRUE GUMMY!"];
const L_FF: [&str; 3] = ["WATCH YOUR FIRE!", "WRONG TEAM, STEVE!", "FRIENDLY! FRIENDLY!"];
const L_RUSH: [&str; 3] = ["WHY IS HE SO FAST?!", "HE'S ON A SUGAR HIGH!", "RETREAT?!"];
const L_DIE: [&str; 4] = ["TELL MY WIFE... I'M DELICIOUS", "SO... CHEWY...", "I REGRET NOTHING", "AVENGE... ME..."];

#[derive(Clone)]
pub struct Gun {
    pub name: &'static str, pub short: &'static str, pub clip: u32, pub rate: f32, pub reload: f32, pub pellets: u32, pub spread: f32, pub ads: f32, pub speed: f32, pub dmg: f32, pub kick: f32, pub grav: f32, pub radius: f32,
    pub ammo: u32, pub owned: bool, pub loot: Option<Wpn>, pub lead: bool, pub rocket: bool, pub scope: bool, pub no_ads: bool,
}
const G0: Gun = Gun { name: "", short: "", clip: 24, rate: 0.105, reload: 1.1, pellets: 1, spread: 0.008, ads: 0.0015, speed: 85.0, dmg: 1.0, kick: 1.0, grav: 3.0, radius: 0.0, ammo: 0, owned: false, loot: None, lead: false, rocket: false, scope: false, no_ads: false };
fn base_guns() -> Vec<Gun> {
    vec![
        Gun { name: "JB-4 Jellybean Carbine", short: "Carbine", ..G0 },
        Gun { name: "GB-12 Gumball Shotgun", short: "Shotgun", clip: 6, rate: 0.6, reload: 1.5, pellets: 9, spread: 0.075, ads: 0.045, speed: 70.0, kick: 2.8, grav: 6.0, ..G0 },
        Gun { name: "JW-40 Jawbreaker Launcher", short: "Launcher", clip: 2, rate: 0.75, reload: 2.0, spread: 0.002, ads: 0.0, speed: 42.0, dmg: 9.0, radius: 5.5, kick: 3.4, grav: 10.0, ..G0 },
    ]
}
/// Weapons taken off the bears fire real lead. One magazine, then you throw them away.
fn loot_gun(w: Wpn) -> Gun {
    let g = match w {
        Wpn::Pistol => Gun { name: "Captured Pistol", short: "Pistol", clip: 12, rate: 0.17, spread: 0.01, ads: 0.003, speed: 150.0, dmg: 2.0, kick: 1.5, grav: 0.0, lead: true, ..G0 },
        Wpn::Sniper => Gun { name: "Captured Sniper Rifle", short: "Sniper", clip: 5, rate: 1.1, spread: 0.03, ads: 0.0, speed: 260.0, dmg: 14.0, kick: 4.5, grav: 0.0, lead: true, scope: true, ..G0 },
        Wpn::Rpg => Gun { name: "Captured RPG", short: "RPG", clip: 1, rate: 1.0, spread: 0.004, ads: 0.002, speed: 34.0, dmg: 14.0, radius: 6.5, kick: 4.0, grav: 2.0, rocket: true, ..G0 },
        Wpn::Minigun => Gun { name: "The King's Minigun", short: "Minigun", clip: 200, rate: 0.045, spread: 0.03, ads: 0.03, speed: 150.0, dmg: 1.5, kick: 0.5, grav: 0.0, lead: true, no_ads: true, ..G0 },
        _ => Gun { name: "Captured AK", short: "AK", clip: 30, rate: 0.095, spread: 0.014, ads: 0.004, speed: 160.0, dmg: 2.0, kick: 1.5, grav: 0.0, lead: true, ..G0 },
    };
    Gun { ammo: g.clip, owned: true, loot: Some(w), ..g }
}
fn streak(combo: u32) -> Option<&'static str> { match combo { 2 => Some("DOUBLE SCOOP"), 3 => Some("TRIPLE SCOOP"), 5 => Some("SWEET STREAK"), 8 => Some("CANDY CARNAGE"), 12 => Some("TOTAL MELTDOWN"), _ => None } }

// ───────────────────────── entities ─────────────────────────
#[derive(Clone, Copy, PartialEq)]
pub enum Fsm { Idle, Aim, Fire }
#[derive(Clone, Copy)]
pub struct Bear {
    pub id: u32, pub kind: Kind, pub t: &'static BearType, pub color: u32, pub pos: Vec3, pub yaw: f32, pub s: f32, pub hp: f32, pub max_hp: f32, pub speed: f32, pub mdmg: f32, pub reach: f32, pub arm_hp: f32,
    pub spawn: f32, pub think_t: f32, pub los: bool, pub seen: bool, pub strafe: f32, pub side: f32, pub stuck: f32,
    pub fsm: Fsm, pub cool_t: f32, pub aim_t: f32, pub burst: u32, pub burst_t: f32, pub lost_t: f32, pub has_token: bool, pub locked: bool, pub lock: Vec3, pub track: Vec3,
    pub kx: f32, pub kz: f32, pub cool: f32, pub seed: f32, pub minion_t: f32, pub elite: bool, pub armed: bool, pub melee: bool, pub head: bool,
    pub phase: f32, pub hop: f32, pub j: f32, pub jv: f32, pub flash: f32, pub kick: f32, pub lunge: f32, pub gy: f32, pub squash: Vec3, pub lean: f32, pub arm_x: f32, pub arm_l: f32, pub head_x: f32,
    pub die_t: f32, pub sun: f32, pub spin: f32, pub warhead: bool,
}
impl Bear {
    pub fn new(kind: Kind, id: u32) -> Bear {
        let t = bt(kind);
        Bear {
            id, kind, t, color: pick(t.colors), pos: Vec3::ZERO, yaw: 0.0, s: t.s, hp: t.hp, max_hp: t.hp, speed: t.speed, mdmg: t.mdmg, reach: 0.75 * t.s + 0.6, arm_hp: t.arm_hp,
            spawn: 1.0, think_t: rr(0.0, 0.4), los: false, seen: false, strafe: if chance(0.5) { -1.0 } else { 1.0 }, side: if chance(0.5) { -1.0 } else { 1.0 }, stuck: 0.0,
            fsm: Fsm::Idle, cool_t: rr(0.8, 1.8), aim_t: 0.0, burst: 0, burst_t: 0.0, lost_t: 0.0, has_token: false, locked: false, lock: Vec3::ZERO, track: Vec3::ZERO,
            kx: 0.0, kz: 0.0, cool: 0.6, seed: rr(0.0, 10.0), minion_t: 7.0, elite: false, armed: true, melee: t.melee, head: true,
            phase: rr(0.0, TAU), hop: rr(7.0, 9.0) / t.s.sqrt(), j: 0.0, jv: 0.0, flash: 0.0, kick: 0.0, lunge: 0.0, gy: 0.0, squash: Vec3::ONE, lean: 0.0, arm_x: 0.0, arm_l: 0.0, head_x: 0.0,
            die_t: 0.0, sun: 1.0, spin: 0.0, warhead: true,
        }
    }
    pub fn scale_now(&self) -> f32 { if self.spawn >= 1.0 { self.s } else { self.s * (1.0 - (1.0 - self.spawn).powi(3)) * (1.0 + 0.25 * (self.spawn * PI).sin()) } }
    pub fn pose(&self) -> BearPose {
        BearPose {
            pos: self.pos, yaw: self.yaw, scale: self.scale_now().max(0.001), tilt_x: 0.0, hop: self.gy, lean: self.lean, squash: self.squash,
            arm_r: self.armed.then_some((self.arm_x, 0.0)), arm_l: Some((self.arm_l, if self.t.two && self.armed { -0.5 } else { 0.0 })), head: self.head.then_some(self.head_x),
        }
    }
    /// A point in the bear's own frame, in world space (used for hit zones).
    pub fn zone(&self, lx: f32, ly: f32, lz: f32) -> Vec3 {
        let (c, sn) = (self.yaw.cos(), self.yaw.sin());
        v3(self.pos.x + (lx * c + lz * sn) * self.s, self.pos.y + (self.gy + ly) * self.s, self.pos.z + (-lx * sn + lz * c) * self.s)
    }
    fn muzzle(&self, local: Vec3) -> Vec3 { (self.pose().hand_xf(true) * Mat4::from_translation(guns::PAW_OFFSET)).transform_point3(local) }
}
#[derive(Clone, Copy)]
pub struct Shot { pos: Vec3, vel: Vec3, prev: Vec3, from: Vec3, fresh: bool, life: f32, color: u32, dmg: f32, grav: f32, boom: Option<(f32, f32, f32)>, kind: u8, spin: f32 }
#[derive(Clone, Copy)]
pub struct EBullet { pos: Vec3, vel: Vec3, prev: Vec3, life: f32, dmg: f32, rocket: bool, whiz: bool, shot_down: bool, s: f32 }
#[derive(Clone, Copy)]
struct Bit { pos: Vec3, vel: Vec3, t: f32, color: u32 }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Loot { Gun(Wpn), Armor }
#[derive(Clone, Copy)]
enum DebrisKind { Arm(u32, Option<Wpn>), Head(u32, &'static [Gear]), Gun(Wpn), Shield }
#[derive(Clone, Copy)]
pub struct Debris { kind: DebrisKind, pub pos: Vec3, vel: Vec3, rot: Vec3, w: Vec3, life: f32, r: f32, s: f32, shrink: f32, pub loot: Option<Loot> }
#[derive(Clone, Copy)]
struct Chunk { pos: Vec3, vel: Vec3, size: f32, life: f32, max: f32, color: u32 }
#[derive(Clone, Copy)]
struct Spark { pos: Vec3, vel: Vec3, size: f32, life: f32, max: f32, color: Vec3, grav: f32, smoke: bool }
struct Splat { x: f32, z: f32, color: u32, size: f32, rot: f32, life: f32 }
struct Beam { a: Vec3, b: Vec3, t: f32, max: f32, r: f32, color: Vec3 }
struct Boom { pos: Vec3, r: f32, t: f32, color: Vec3 }
struct Flash { pos: Vec3, t: f32, size: f32 }
struct TempLight { pos: Vec3, color: Vec3, range: f32 }
pub struct Float { pub pos: Vec3, pub text: String, pub cls: u8, pub t: f32, pub life: f32 }
pub struct Bubble { pub text: String, pub bear: u32, pub pos: Vec3, pub s: f32, pub t: f32 }
struct Mantle { t: f32, dur: f32, from: Vec3, to: Vec3 }
struct Intro { sdv: bool, t: f32, dur: f32, wet: bool }
#[derive(Clone, Copy, Default)]
pub struct Stats { pub kills: u32, pub heads: u32, pub disarms: u32, pub shots: u32, pub hits: u32, pub ff: u32 }
#[derive(Clone, Copy, PartialEq)]
pub enum State { Menu, Playing, Paused, Dead, Results }
#[derive(Clone, Copy, PartialEq)]
pub enum Screen { Menu, Map, Brief(u32), Armory, Settings, Pause, Results }
#[derive(Clone, Copy, PartialEq)]
enum RunType { Clear, Survive, Hunt, Boss, Arcade }
#[derive(Clone, Copy, PartialEq)]
enum Src { Shot, Boom, Ff, Melee }
#[derive(Clone, Copy, PartialEq)]
enum Zone { Head, Arm, Body }
pub struct Run {
    pub mission: Option<&'static Mission>, typ: RunType, waves: &'static [Mix], mix: Mix, cap: usize, target: Kind, pub count: u32, pub label: &'static str,
    pub wave_i: i32, pub queue: Vec<Kind>, spawn_t: f32, break_t: f32, pub timer: f32, pub killed: u32, spawned: u32, target_t: f32,
    pub candy: u32, pub xp: u32, pub level_ups: u32, pub done: bool, end_t: f32, wind: bool, seen: HashSet<Kind>, pub t: f32,
}
pub struct Results { pub win: bool, pub mission: Option<&'static Mission>, pub score: u32, pub wave: u32, pub stats: Stats, pub candy: u32, pub xp: u32, pub stars: u32, pub bonus: Vec<(String, bool)>, pub unlock: Option<&'static str>, pub level_ups: u32, pub new_best: bool, pub age: f32 }
pub struct Player { pub pos: Vec3, pub vel: Vec3, pub yaw: f32, pub pitch: f32, pub hp: f32, pub armor: f32, pub h: f32, pub on_ground: bool, pub crouch: bool, pub iframes: f32, pub sugar: f32, pub rush: f32, pub dash: f32 }

struct VmIds { body: u32, pump: Option<u32>, ball: Option<u32>, barrels: Option<u32>, dot_z: Option<f32>, sight_y: f32, hip: Vec3, ads_z: f32 }
struct GunIds { body: u32, warhead: Option<u32>, barrels: Option<u32>, muzzle: Vec3 }
struct Ids { sphere: u32, bean: u32, rocket: u32, marker: u32, shield: u32, chute: u32, scooter: u32, reticle: u32, gear: HashMap<Gear, u32>, gun: HashMap<Wpn, GunIds>, vm: Vec<VmIds>, loot_vm: HashMap<Wpn, VmIds> }
fn vm_ids(r: &mut Renderer, v: &ViewModel) -> VmIds {
    VmIds { body: r.add_mesh(&v.body), pump: v.pump.as_ref().map(|m| r.add_mesh(m)), ball: v.ball.as_ref().map(|m| r.add_mesh(m)), barrels: v.barrels.as_ref().map(|m| r.add_mesh(m)), dot_z: v.dot_z, sight_y: v.sight_y, hip: v.hip, ads_z: v.ads_z }
}

pub struct Game {
    pub state: State, pub screens: Vec<Screen>, pub profile: Profile, pub m: Mods, pub world: World, pub atlas: Atlas, pub audio: Audio, pub ui: Ui, ids: Ids,
    pub pal: Palette, pub world_dirty: bool, pub want_capture: bool, pub quit: bool, pub cut: bool, pub reset_armed: bool, pub scale_changed: bool,
    pub player: Player, pub run: Option<Run>, pub results: Option<Results>, pub stats: Stats,
    pub score: u32, pub wave: u32, tokens: u32, pub combo: u32, pub combo_t: f32, hitstop: f32, slow_t: f32, ts_w: f32, ts_p: f32, pub trauma: f32, pub hurt_flash: f32, pub hp_ghost: f32, pub clock: f32,
    kick_pitch: f32, kick_yaw: f32, land_dip: f32, bob_t: f32, roll: f32, turn_v: f32, step_off: f32, pub sprint_k: f32, pub slide_k: f32, slide_t: f32, melee_t: f32, chute_t: f32, chute_pos: Vec3, dead_t: f32, menu_t: f32, heart_t: f32, muffled: bool,
    mantle: Option<Mantle>, intro: Option<Intro>, pub near_pick: Option<usize>,
    pub guns: Vec<Gun>, pub gi: usize, last_gi: usize, sw_t: f32, sw_to: Option<usize>, pub reload_t: f32, fire_cool: f32, pub ads_t: f32, recoil: f32, pump_t: f32, sway_x: f32, sway_y: f32, pub cross_bloom: f32, barrel_spin: f32,
    bark_cd: f32, chain_n: u32, chain_t: f32, next_id: u32, rush_fx: f32,
    pub bears: Vec<Bear>, dying: Vec<Bear>, props: Vec<Bear>, shots: Vec<Shot>, pub ebullets: Vec<EBullet>, bits: Vec<Bit>, pub debris: Vec<Debris>, chunks: Vec<Chunk>, sparks: Vec<Spark>, splats: Vec<Splat>, beams: Vec<Beam>, booms: Vec<Boom>, flashes: Vec<Flash>, lights_tmp: Vec<TempLight>, pending_booms: Vec<Vec3>,
    pub floats: Vec<Float>, pub bubbles: Vec<Bubble>, pub feed: Vec<(String, f32)>, pub announce: (String, u8, f32), pub card: (String, String, String, f32), pub radio_q: Vec<&'static str>, pub radio: (&'static str, f32), pub hit_mark: (f32, bool, bool), pub dmg_dirs: Vec<(f32, f32)>,
    // camera and per-frame scene
    pub vm_sun: f32, sun_t: f32, sun_hit: f32, pub cam_pos: Vec3, pub cam_rot: Vec3, pub fov: f32, cam_fwd: Vec3, cam_right: Vec3, cam_up: Vec3, gun_pos: Vec3, gun_rot: Vec3, dot_off: Vec2, pub scoped: bool,
    pub draws: Vec<Draw>, pub vm_draws: Vec<Draw>, pub gummies: Vec<GummyU>, pub sprites: Vec<Sprite>, pub lights: Vec<[Vec4; 2]>, pub spheres: Vec<[Vec4; 2]>, pub decals: Mesh, pub extra: Vec<Draw>,
}

impl Game {
    pub fn new(r: &mut Renderer, atlas: Atlas, audio: Audio, ui: Ui) -> Game {
        let profile = Profile::load();
        let world = World::new(&atlas);
        let mut gear = HashMap::new();
        for g in [Gear::Helmet, Gear::Shades, Gear::Beret, Gear::Bandana, Gear::Crown] { gear.insert(g, r.add_mesh(&guns::gear(g))); }
        let mut gun = HashMap::new();
        for w in [Wpn::Pistol, Wpn::Knife, Wpn::Rifle, Wpn::Sniper, Wpn::Rpg, Wpn::Minigun] { let g = guns::bear_gun(w); gun.insert(w, GunIds { body: r.add_mesh(&g.body), warhead: g.warhead.as_ref().map(|m| r.add_mesh(m)), barrels: g.barrels.as_ref().map(|m| r.add_mesh(m)), muzzle: g.muzzle }); }
        let mut loot_vm = HashMap::new();
        for w in [Wpn::Pistol, Wpn::Rifle, Wpn::Sniper, Wpn::Rpg, Wpn::Minigun] { loot_vm.insert(w, vm_ids(r, &guns::loot_vm(w))); }
        let ids = Ids {
            sphere: r.add_mesh(&guns::unit_sphere()), bean: r.add_mesh(&guns::jellybean()), rocket: r.add_mesh(&guns::rocket()), marker: r.add_mesh(&guns::marker()), shield: r.add_mesh(&guns::shield()),
            chute: r.add_mesh(&guns::parachute()), scooter: r.add_mesh(&guns::scooter()), reticle: r.add_mesh(&guns::reticle()), gear, gun, vm: guns::player_guns().iter().map(|v| vm_ids(r, v)).collect(), loot_vm,
        };
        let m = mods(&profile);
        let mut g = Game {
            state: State::Menu, screens: vec![Screen::Menu], profile, m, world, atlas, audio, ui, ids, pal: palette("morning"), world_dirty: true, want_capture: false, quit: false, cut: true, reset_armed: false, scale_changed: false,
            player: Player { pos: Vec3::ZERO, vel: Vec3::ZERO, yaw: 0.0, pitch: 0.0, hp: 100.0, armor: 0.0, h: 1.7, on_ground: true, crouch: false, iframes: 0.0, sugar: 0.0, rush: 0.0, dash: 2.0 },
            run: None, results: None, stats: Stats::default(), score: 0, wave: 0, tokens: 0, combo: 0, combo_t: 0.0, hitstop: 0.0, slow_t: 0.0, ts_w: 1.0, ts_p: 1.0, trauma: 0.0, hurt_flash: 0.0, hp_ghost: 1.0, clock: 0.0,
            kick_pitch: 0.0, kick_yaw: 0.0, land_dip: 0.0, bob_t: 0.0, roll: 0.0, turn_v: 0.0, step_off: 0.0, sprint_k: 0.0, slide_k: 0.0, slide_t: 0.0, melee_t: 0.0, chute_t: 0.0, chute_pos: Vec3::ZERO, dead_t: 0.0, menu_t: 0.0, heart_t: 0.0, muffled: false,
            mantle: None, intro: None, near_pick: None, guns: base_guns(), gi: 0, last_gi: 0, sw_t: 1.0, sw_to: None, reload_t: 0.0, fire_cool: 0.0, ads_t: 0.0, recoil: 0.0, pump_t: 0.0, sway_x: 0.0, sway_y: 0.0, cross_bloom: 0.0, barrel_spin: 0.0,
            bark_cd: 0.0, chain_n: 0, chain_t: 0.0, next_id: 1, rush_fx: 0.0,
            bears: Vec::new(), dying: Vec::new(), props: Vec::new(), shots: Vec::new(), ebullets: Vec::new(), bits: Vec::new(), debris: Vec::new(), chunks: Vec::new(), sparks: Vec::new(), splats: Vec::new(), beams: Vec::new(), booms: Vec::new(), flashes: Vec::new(), lights_tmp: Vec::new(), pending_booms: Vec::new(),
            floats: Vec::new(), bubbles: Vec::new(), feed: Vec::new(), announce: (String::new(), 0, 9.0), card: (String::new(), String::new(), String::new(), 0.0), radio_q: Vec::new(), radio: ("", 0.0), hit_mark: (9.0, false, false), dmg_dirs: Vec::new(),
            vm_sun: 1.0, sun_t: 0.0, sun_hit: 1.0, cam_pos: v3(0.0, 5.0, 15.0), cam_rot: Vec3::ZERO, fov: 55.0, cam_fwd: Vec3::NEG_Z, cam_right: Vec3::X, cam_up: Vec3::Y, gun_pos: Vec3::ZERO, gun_rot: Vec3::ZERO, dot_off: Vec2::ZERO, scoped: false,
            draws: Vec::new(), vm_draws: Vec::new(), gummies: Vec::new(), sprites: Vec::new(), lights: Vec::new(), spheres: Vec::new(), decals: Mesh::default(), extra: Vec::new(),
        };
        g.audio.set_volume(g.profile.volume);
        g.to_menu();
        g
    }
    pub fn view(&self) -> Mat4 { (Mat4::from_translation(self.cam_pos) * Mat4::from_euler(EulerRot::YXZ, self.cam_rot.y, self.cam_rot.x, self.cam_rot.z)).inverse() }
    fn bear_idx(&self, id: u32) -> Option<usize> { self.bears.iter().position(|b| b.id == id) }

    // ───────────────────────── effects ─────────────────────────
    fn burst(&mut self, pos: Vec3, color: u32, n: u32, size: f32, power: f32) {
        for _ in 0..n { self.chunks.push(Chunk { pos, vel: v3(rr(-1.0, 1.0), rr(0.2, 1.3), rr(-1.0, 1.0)) * power, size: size * rr(0.6, 1.4), life: rr(0.6, 1.3), max: 1.3, color }); }
        if self.chunks.len() > 500 { let cut = self.chunks.len() - 500; self.chunks.drain(0..cut); }
    }
    fn spark(&mut self, pos: Vec3, color: u32, n: u32, power: f32, size: f32, life: f32, boost: f32) {
        for _ in 0..n { self.sparks.push(Spark { pos, vel: v3(rr(-1.0, 1.0), rr(-0.3, 1.0), rr(-1.0, 1.0)) * power, size: size * rr(0.6, 1.4), life: life * rr(0.6, 1.3), max: life * 1.3, color: lin(color) * boost, grav: 7.0, smoke: false }); }
        if self.sparks.len() > 500 { let cut = self.sparks.len() - 500; self.sparks.drain(0..cut); }
    }
    fn smoke(&mut self, pos: Vec3, vel: Vec3, size: f32, life: f32, color: u32) { self.sparks.push(Spark { pos, vel, size, life, max: life, color: lin(color), grav: -1.2, smoke: true }); }
    fn splat(&mut self, x: f32, z: f32, color: u32, size: f32) {
        if x.abs().max(z.abs()) < CAKE[0].0 + size * 0.5 { return; }
        self.splats.push(Splat { x, z, color, size, rot: rr(0.0, TAU), life: 26.0 });
        if self.splats.len() > 36 { self.splats.remove(0); }
    }
    fn light(&mut self, pos: Vec3, color: Vec3, range: f32) { self.lights_tmp.push(TempLight { pos, color, range }); }
    pub fn say(&mut self, bear: &Bear, text: &str, force: bool) {
        if !force && self.bark_cd > 0.0 { return; }
        self.bark_cd = 1.4;
        if self.bubbles.len() >= 3 { self.bubbles.remove(0); }
        self.bubbles.push(Bubble { text: text.to_string(), bear: bear.id, pos: bear.pos, s: bear.s, t: 0.0 });
        let sp = self.audio.sp(bear.pos.x, bear.pos.z);
        self.audio.babble(sp, 1.25 / bear.s.sqrt());
    }
    pub fn announce(&mut self, text: &str, cls: u8) { self.announce = (text.to_string(), cls, 0.0); }
    fn float_text(&mut self, pos: Vec3, text: String, cls: u8, life: f32) { self.floats.push(Float { pos, text, cls, t: 0.0, life }); if self.floats.len() > 24 { self.floats.remove(0); } }
    fn show_card(&mut self, kicker: &str, title: &str, sub: &str) { self.card = (kicker.to_string(), title.to_string(), sub.to_string(), 3.4); }
    fn dmg_dir(&mut self, sx: f32, sz: f32) {
        let (dx, dz, sy, cy) = (sx - self.player.pos.x, sz - self.player.pos.z, self.player.yaw.sin(), self.player.yaw.cos());
        self.dmg_dirs.push(((dx * cy - dz * sy).atan2(-dx * sy - dz * cy), 0.0));
    }

    // ───────────────────────── player weapons ─────────────────────────
    fn config_guns(&mut self) {
        self.guns = base_guns();
        self.guns[0].dmg = self.m.blaster_dmg; self.guns[0].clip = self.m.blaster_clip; self.guns[1].pellets = self.m.pellets; self.guns[2].radius = self.m.boom_r; self.guns[2].dmg = self.m.boom_dmg;
        for (i, g) in self.guns.iter_mut().enumerate() { g.ammo = g.clip; g.owned = self.m.guns[i]; }
    }
    pub fn switch_to(&mut self, i: usize) {
        if self.state != State::Playing || self.intro.is_some() || i == self.gi || Some(i) == self.sw_to || i >= self.guns.len() || !self.guns[i].owned { return; }
        self.sw_to = Some(i); self.sw_t = 0.0; self.reload_t = 0.0; self.audio.swap();
    }
    fn start_reload(&mut self) {
        let g = &self.guns[self.gi];
        if g.loot.is_some() || self.reload_t > 0.0 || g.ammo == g.clip || self.state != State::Playing || self.sw_t < 1.0 { return; }
        self.reload_t = g.reload * self.m.reload; self.audio.reload(self.reload_t);
    }
    pub fn equip_loot(&mut self, w: Wpn) {
        if let Some(cur) = self.guns.get_mut(3) { if cur.owned && cur.loot == Some(w) { cur.ammo = (cur.ammo + cur.clip).min(cur.clip * 2); self.audio.pickup(); self.announce("+ AMMO", 2); return; } }
        let g = loot_gun(w);
        let name = g.name.to_uppercase();
        if self.guns.len() > 3 { self.guns[3] = g; } else { self.guns.push(g); }
        self.audio.pickup(); self.announce(&name, 2);
        if self.gi == 3 { self.sw_t = 0.45; self.sw_to = None; } else { self.sw_to = None; self.switch_to(3); }
    }
    fn drop_loot(&mut self) {
        if self.guns.len() < 4 || !self.guns[3].owned { return; }
        self.guns[3].owned = false;
        let name = format!("{} EMPTY", self.guns[3].short.to_uppercase());
        self.announce(&name, 0);
        let back = if self.last_gi < 3 && self.guns[self.last_gi].owned { self.last_gi } else { 0 };
        self.switch_to(back);
    }
    pub fn fire(&mut self) {
        let gi = self.gi;
        if self.sw_t < 1.0 || self.reload_t > 0.0 || self.fire_cool > 0.0 || self.melee_t > 0.0 || self.sprint_k > 0.5 || self.mantle.is_some() || !self.guns[gi].owned { return; }
        if self.guns[gi].ammo == 0 { if self.guns[gi].loot.is_some() { self.drop_loot(); } else { self.start_reload(); } return; }
        let rushing = self.player.rush > 0.0;
        if !rushing || self.guns[gi].loot.is_some() { self.guns[gi].ammo -= 1; }
        let g = self.guns[gi].clone();
        self.fire_cool = g.rate / if rushing { 1.5 } else { 1.0 };
        self.recoil = (self.recoil + g.kick * 0.55).min(1.6); self.kick_pitch += g.kick * 0.011; self.kick_yaw += rr(-1.0, 1.0) * g.kick * 0.004; self.cross_bloom = (self.cross_bloom + g.kick * 0.3).min(1.0);
        self.trauma = (self.trauma + g.kick * 0.035).min(1.0);
        if g.rocket { self.audio.egun(3, HERE); } else if g.lead { self.audio.egun(match g.loot { Some(Wpn::Pistol) => 0, Some(Wpn::Sniper) => 2, _ => 1 }, HERE); }
        else if gi == 0 { self.audio.blaster(); } else if gi == 1 { self.audio.scatter(); self.pump_t = 0.5; } else { self.audio.cannon(); }
        let (fwd, right, up, cam) = (self.cam_fwd, self.cam_right, self.cam_up, self.cam_pos);
        let origin = cam + right * 0.2 * (1.0 - self.ads_t) - up * 0.15 + fwd * 0.75;
        self.flashes.push(Flash { pos: origin + fwd * 0.25, t: 0.05, size: 0.16 + g.kick * 0.03 });
        self.light(origin, v3(1.0, 0.8, 0.5) * 4.0, 9.0);
        if gi == 0 || g.lead { let pv = self.player.vel; self.chunks.push(Chunk { pos: origin + right * 0.12 - fwd * 0.45 + up * 0.06, vel: right * 3.5 + v3(pv.x, rr(2.0, 3.4), pv.z), size: 0.022, life: 0.9, max: 0.9, color: 0xd4a84a }); }
        let move_k = (self.player.vel.x.hypot(self.player.vel.z) / 9.0).min(1.0);
        let spread = lerp(g.spread, g.ads, self.ads_t) * if self.player.crouch { 0.7 } else { 1.0 } * (1.0 + move_k * 0.7 + if self.slide_t > 0.0 { 0.6 } else { 0.0 });
        for _ in 0..g.pellets {
            let (a, r) = (rr(0.0, TAU), rnd().sqrt() * spread);
            let dir = (fwd + right * (a.cos() * r) + up * (a.sin() * r)).normalize();
            let aim = (cam + dir * 60.0 - origin).normalize();
            let color = pick(&CANDY);
            let (kind, boom) = if g.rocket { (4, Some((g.radius, g.dmg, if self.m.no_self { 0.0 } else { 30.0 }))) } else if g.lead { (3, None) } else if gi == 2 { (2, Some((g.radius, g.dmg, if self.m.no_self { 0.0 } else { 18.0 }))) } else { (gi as u8, None) };
            self.stats.shots += 1;
            self.shots.push(Shot { pos: origin, vel: aim * g.speed * rr(0.96, 1.04), prev: origin, from: cam, fresh: true, life: 1.8, color: if g.lead { 0xffd27a } else { color }, dmg: g.dmg, grav: g.grav, boom, kind, spin: 0.0 });
        }
        if self.guns[gi].ammo == 0 { if g.loot.is_some() { self.drop_loot(); } else if !rushing { self.start_reload(); } }
    }
    /// Rifle butt: short reach, hits everything in front, built for the small ones.
    pub fn melee_hit(&mut self) {
        let (fwd, cam) = (self.cam_fwd, self.cam_pos);
        let mut any = false;
        let ids: Vec<u32> = self.bears.iter().map(|b| b.id).collect();
        for id in ids {
            let Some(i) = self.bear_idx(id) else { continue };
            let c = self.bears[i].zone(0.0, 0.9, 0.0);
            let d = c - cam;
            if d.length() - 0.6 * self.bears[i].s < 2.5 && d.normalize().dot(fwd) > 0.4 {
                any = true;
                let s = self.bears[i].s;
                self.bears[i].kx += fwd.x * 14.0 / s; self.bears[i].kz += fwd.z * 14.0 / s;
                self.hit_bear(i, Zone::Body, 3.0, fwd, c, Src::Melee);
            }
        }
        if any { self.hitstop = self.hitstop.max(0.07); self.trauma = (self.trauma + 0.25).min(1.0); self.audio.thwack(); }
    }
    fn update_shots(&mut self, dt: f32) {
        let mut i = self.shots.len();
        while i > 0 {
            i -= 1;
            let mut s = self.shots[i];
            if s.fresh { s.prev = s.from; s.fresh = false; } else { s.prev = s.pos; }
            s.vel.y -= s.grav * dt; s.pos += s.vel * dt; s.life -= dt; s.spin += dt * 9.0;
            if s.kind == 2 && chance(0.6) { self.sparks.push(Spark { pos: s.pos, vel: v3(rr(-1.0, 1.0), rr(0.0, 2.0), rr(-1.0, 1.0)), size: 0.05, life: 0.35, max: 0.35, color: lin(pick(&CANDY)) * 2.5, grav: 7.0, smoke: false }); }
            if s.kind == 4 && chance(0.8) { self.smoke(s.prev, v3(rr(-0.4, 0.4), rr(0.0, 0.8), rr(-0.4, 0.4)), rr(0.14, 0.26), rr(0.5, 0.9), 0xd8d0d8); }
            let mut dead = s.life <= 0.0;
            let tw = self.world.seg_blocked(s.prev, s.pos);
            let end = if tw >= 0.0 { s.prev.lerp(s.pos, tw) } else { s.pos };
            if !dead { for r in self.ebullets.iter_mut() { if r.rocket && seg_sphere(s.prev, end, r.pos, 0.6).is_some() { r.life = 0.0; r.shot_down = true; dead = true; break; } } }
            if !dead {
                for j in 0..self.bears.len() {
                    let b = self.bears[j];
                    let (fx, fz, boom) = (b.yaw.sin(), b.yaw.cos(), s.boom.is_some());
                    if b.head { if let Some(h) = seg_sphere(s.prev, end, b.zone(0.0, 1.6, 0.02), 0.44 * b.s + 0.1) { s.pos = h; if !boom { self.hit_bear(j, Zone::Head, s.dmg, s.vel, h, Src::Shot); } dead = true; break; } }
                    if b.t.shield {
                        let vl = s.vel.x.hypot(s.vel.z).max(1e-4);
                        if (s.vel.x * fx + s.vel.z * fz) / vl < -0.3 {
                            if let Some(h) = seg_sphere(s.prev, end, b.zone(0.12, 0.95, 0.5), 0.7 * b.s) {
                                s.pos = h;
                                if !boom { self.spark(h, 0xffffff, 6, 5.0, 0.04, 0.2, 4.0); let sp = self.audio.sp(h.x, h.z); self.audio.clang(sp); self.bears[j].jv += 2.0; }
                                dead = true; break;
                            }
                        }
                    }
                    if b.armed && b.arm_hp < 99.0 { if let Some(h) = seg_sphere(s.prev, end, b.zone(-0.5, 1.18, 0.5), 0.24 * b.s + 0.08) { s.pos = h; if !boom { self.hit_bear(j, Zone::Arm, s.dmg, s.vel, h, Src::Shot); } dead = true; break; } }
                    if let Some(h) = seg_sphere(s.prev, end, b.zone(0.0, 0.82, 0.0), 0.68 * b.s + 0.1) { s.pos = h; if !boom { self.hit_bear(j, Zone::Body, s.dmg, s.vel, h, Src::Shot); } dead = true; break; }
                }
            }
            if !dead && (tw >= 0.0 || s.pos.y < 0.04) {
                dead = true;
                if tw >= 0.0 { s.pos = end; } else { s.pos.y = 0.04; }
                if s.boom.is_none() { self.spark(s.pos, s.color, 4, 3.0, 0.05, 0.28, 2.2); }
            }
            if dead { self.shots.remove(i); if let Some((r, d, sd)) = s.boom { self.explode(s.pos, r, d, sd, true, false); } } else { self.shots[i] = s; }
            if self.state != State::Playing { return; }
        }
    }

    // ───────────────────────── bears ─────────────────────────
    fn max_tokens(&self) -> u32 { (2 + self.wave / 3).min(5) }
    fn end_attack(&mut self, b: &mut Bear, cool: f32) { b.fsm = Fsm::Idle; b.cool_t = cool; b.locked = false; b.warhead = true; if b.has_token { self.tokens = self.tokens.saturating_sub(1); b.has_token = false; } }
    pub fn spawn_bear(&mut self, kind: Kind, at: Option<(f32, f32)>, elite: bool) -> u32 {
        let pp = self.player.pos;
        let (mut x, mut z) = at.unwrap_or((0.0, 0.0));
        if at.is_none() {
            for i in 0..16 { // somewhere 28-48 m off, preferably round a corner
                let (a, d) = (rr(0.0, TAU), rr(28.0, 48.0));
                x = pp.x + a.cos() * d; z = pp.z + a.sin() * d;
                let r = x.hypot(z);
                if r > ARENA_R - 4.0 { x *= (ARENA_R - 4.0) / r; z *= (ARENA_R - 4.0) / r; }
                if (x - pp.x).hypot(z - pp.z) < 22.0 || self.world.floor_at(x, z, 99.0) > 0.0 { continue; }
                if i > 10 || self.world.seg_blocked(v3(x, 1.5, z), v3(pp.x, pp.y + 1.4, pp.z)) >= 0.0 { break; }
            }
        }
        let mut b = Bear::new(kind, self.next_id);
        self.next_id += 1;
        let hp_mul = (if kind == Kind::Boss { 1.0 + (self.wave as f32 / 5.0 - 1.0).max(0.0) * 0.5 } else { 1.0 + (self.wave as f32 - 7.0).max(0.0) * 0.07 }) * if elite { 2.2 } else { 1.0 };
        b.pos = v3(x, 0.0, z); b.hp = (b.t.hp * hp_mul).round(); b.max_hp = b.hp; b.speed = b.t.speed * rr(0.92, 1.08) * (1.0 + (self.wave as f32 * 0.015).min(0.3));
        b.spawn = 0.0; b.track = pp; b.elite = elite; if elite { b.arm_hp = 999.0; }
        self.spark(v3(x, 0.3, z), b.color, 18, 5.0, 0.07, 0.5, 2.0);
        if kind == Kind::Boss { self.say(&b, pick(&L_BOSS), true); self.audio.boss(); self.trauma = (self.trauma + 0.4).min(1.0); }
        if let Some(run) = self.run.as_mut() { if run.seen.insert(kind) { self.radio_q.push(kind.tip()); } }
        let id = b.id;
        self.bears.push(b);
        id
    }
    fn animate(b: &mut Bear, dt: f32, moving: bool, elev: f32, aiming: bool, clock: f32) {
        b.phase += dt * b.hop * if moving { 1.0 } else { 0.45 };
        let h = b.phase.sin().abs();
        b.gy = h * if moving { if aiming { 0.12 } else { 0.28 } } else { 0.05 };
        b.jv += (-200.0 * b.j - 9.0 * b.jv) * dt; b.j += b.jv * dt;
        let sq = (h - 0.5) * if moving { 0.14 } else { 0.04 } + b.j.clamp(-0.6, 0.6) * 0.5;
        b.squash = v3(1.0 - sq * 0.5, 1.0 + sq, 1.0 - sq * 0.5); b.lean = b.phase.sin() * if moving { 0.09 } else { 0.03 };
        b.kick = damp(b.kick, 0.0, 12.0, dt); b.lunge = damp(b.lunge, 0.0, 7.0, dt);
        if b.armed { b.arm_x = -elev - b.kick * 0.35 + if b.melee { (b.phase * 2.0).sin() * 0.35 - b.lunge * 1.1 } else { 0.0 }; b.arm_l = if b.t.two { b.arm_x } else { 0.0 }; }
        else { b.arm_l = (b.phase * 2.2).sin() * 0.6 - 0.4; }
        b.head_x = -elev * 0.5;
        if b.fsm != Fsm::Idle { b.spin += dt * 26.0; }
        if b.flash > 0.0 { b.flash = (b.flash - dt).max(0.0); }
        let _ = clock;
    }
    fn enemy_shoot(&mut self, b: &mut Bear) {
        let t = b.t;
        let m = b.muzzle(self.ids.gun[&t.weapon].muzzle);
        b.kick = 1.0;
        self.flashes.push(Flash { pos: m, t: 0.06, size: 0.28 * b.s });
        self.light(m, v3(1.0, 0.75, 0.45) * 5.0, 10.0);
        let sp = self.audio.sp(m.x, m.z);
        self.audio.egun(match t.weapon { Wpn::Rifle | Wpn::Minigun => 1, Wpn::Sniper => 2, Wpn::Rpg => 3, _ => 0 }, sp);
        for _ in 0..4 { self.sparks.push(Spark { pos: m, vel: v3(rr(-2.0, 2.0), rr(0.0, 3.0), rr(-2.0, 2.0)), size: 0.04, life: 0.15, max: 0.15, color: lin(0xffd27a) * 3.0, grav: 7.0, smoke: false }); }
        if t.hitscan { // sniper: a line to wherever the laser locked
            let d = (b.lock - m).normalize();
            let mut end = m + d * 95.0;
            let tb = self.world.seg_blocked(m, end);
            if tb >= 0.0 { end = m.lerp(end, tb); }
            let c = self.player.pos + Vec3::Y * self.player.h * 0.6;
            let hit = seg_sphere(m, end, c, 0.62).is_some() && self.player.iframes <= 0.0;
            self.beams.push(Beam { a: m, b: end, t: 0.16, max: 0.16, r: 0.05, color: v3(7.0, 6.0, 5.0) });
            if hit { self.hurt(t.dmg, m.x, m.z); } else { self.audio.whiz(); }
            return;
        }
        let (dist, rpg) = (m.distance(b.track), t.weapon == Wpn::Rpg);
        let target = v3(b.track.x + rr(-1.0, 1.0) * t.acc * dist, b.track.y + if rpg { 0.15 } else { self.player.h * 0.65 } + rr(-1.0, 1.0) * t.acc * dist * 0.6, b.track.z + rr(-1.0, 1.0) * t.acc * dist);
        let d = (target - m).normalize();
        if rpg { b.warhead = false; }
        self.ebullets.push(EBullet { pos: m, vel: d * t.b_speed, prev: m, life: if rpg { 4.5 } else { 3.0 }, dmg: t.dmg, rocket: rpg, whiz: false, shot_down: false, s: b.s });
    }
    fn update_bears(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.bears.len() {
            let mut b = self.bears[i];
            let (pp, t) = (self.player.pos, b.t);
            if b.spawn < 1.0 { b.spawn = (b.spawn + dt * 2.2).min(1.0); }
            let (mut dx, mut dz) = (pp.x - b.pos.x, pp.z - b.pos.z);
            let dist = dx.hypot(dz).max(1e-4);
            dx /= dist; dz /= dist;
            b.track.x = damp(b.track.x, pp.x, t.track, dt); b.track.y = damp(b.track.y, pp.y, 6.0, dt); b.track.z = damp(b.track.z, pp.z, t.track, dt);
            b.think_t -= dt;
            if b.think_t <= 0.0 {
                b.think_t = rr(0.3, 0.55);
                b.los = self.world.seg_blocked(v3(b.pos.x, 1.5 * b.s, b.pos.z), v3(pp.x, pp.y + self.player.h * 0.82, pp.z)) < 0.0;
                if chance(0.22) { b.strafe = -b.strafe; }
                if b.los && !b.seen { b.seen = true; if chance(0.5) { self.say(&b, pick(&L_SPOT), false); } }
                let c = v3(b.pos.x, b.s, b.pos.z);
                let lit = self.world.seg_blocked(c, c + self.pal.sun_dir * 140.0) < 0.0;
                b.sun = damp(b.sun, if lit { 1.0 } else { 0.0 }, 4.0, 0.4);
            }
            // movement intent
            let (mut mx, mut mz, mut sp) = (0.0, 0.0, b.speed);
            if b.melee {
                if dist > b.reach { let zig = if dist > 5.0 { (self.clock * 3.0 + b.seed).sin() * 0.5 } else { 0.0 }; mx = dx - dz * zig; mz = dz + dx * zig; }
            } else {
                let (rmin, rmax) = t.range;
                if !b.los || dist > rmax { mx = dx; mz = dz; }
                else if dist < rmin { mx = -dx - dz * b.strafe * 0.5; mz = -dz + dx * b.strafe * 0.5; }
                else { mx = -dz * b.strafe; mz = dx * b.strafe; sp *= 0.7; }
                if b.fsm != Fsm::Idle { sp *= t.aim_move; }
            }
            let ml = f32::hypot(mx, mz);
            let moving = ml > 0.01 && sp > 0.05;
            if moving { b.pos.x += mx / ml * sp * dt; b.pos.z += mz / ml * sp * dt; }
            b.pos.x += b.kx * dt; b.pos.z += b.kz * dt;
            let kd = (-8.0 * dt).exp(); b.kx *= kd; b.kz *= kd;
            let (bx, bz) = (b.pos.x, b.pos.z);
            self.world.push_out(&mut b.pos, 0.5 * b.s, 0.0);
            let (nx, nz) = (b.pos.x - bx, b.pos.z - bz);
            let nl = nx.hypot(nz);
            if nl > 1e-5 { // slide around whatever we bumped into
                b.pos.x += -nz / nl * b.side * sp * dt * 0.8; b.pos.z += nx / nl * b.side * sp * dt * 0.8; b.stuck += dt;
                if b.stuck > 1.6 { b.side = -b.side; b.strafe = -b.strafe; b.stuck = 0.0; }
            } else { b.stuck = (b.stuck - dt * 0.5).max(0.0); }
            let (r, lim) = (b.pos.x.hypot(b.pos.z), ARENA_R - 1.5);
            if r > lim { b.pos.x *= lim / r; b.pos.z *= lim / r; b.strafe = -b.strafe; }
            let da = dx.atan2(dz) - b.yaw;
            b.yaw += da.sin().atan2(da.cos()) * (dt * 8.0).min(1.0);

            // weapon
            if b.melee {
                b.cool -= dt;
                if dist < b.reach + 0.45 && b.cool <= 0.0 && b.spawn >= 1.0 && pp.y < 1.8 * b.s { b.cool = 0.9; b.lunge = 1.0; b.jv += 4.0; if chance(0.3) { self.say(&b, pick(&L_MELEE), false); } self.hurt(b.mdmg, b.pos.x, b.pos.z); }
            } else if b.fsm == Fsm::Idle {
                b.cool_t -= dt;
                if b.cool_t <= 0.0 && b.los && b.spawn >= 1.0 && dist < t.range.1 * 1.3 && (b.kind == Kind::Boss || self.tokens < self.max_tokens()) {
                    b.fsm = Fsm::Aim; b.aim_t = t.aim; b.lost_t = 0.0; b.locked = false;
                    if b.kind != Kind::Boss { self.tokens += 1; b.has_token = true; }
                    if let Some(lines) = t.say { if chance(0.4) { self.say(&b, pick(lines), false); } }
                    if t.laser { let sp2 = self.audio.sp(b.pos.x, b.pos.z); self.audio.charge(sp2); }
                }
            } else if b.fsm == Fsm::Aim {
                b.aim_t -= dt;
                if !b.los { b.lost_t += dt; if b.lost_t > 0.5 { self.end_attack(&mut b, 0.6); } }
                if b.fsm == Fsm::Aim && t.hitscan && !b.locked && b.aim_t <= 0.34 { b.locked = true; b.lock = v3(pp.x, pp.y + self.player.h * 0.62, pp.z); let sp2 = self.audio.sp(b.pos.x, b.pos.z); self.audio.lock(sp2); }
                if b.fsm == Fsm::Aim && b.aim_t <= 0.0 { b.fsm = Fsm::Fire; b.burst = t.burst; b.burst_t = 0.0; }
            } else {
                b.burst_t -= dt;
                while b.burst_t <= 0.0 && b.burst > 0 && self.state == State::Playing { self.enemy_shoot(&mut b); b.burst -= 1; b.burst_t += t.burst_gap; }
                if b.burst == 0 { let c = rr(t.cool.0, t.cool.1); self.end_attack(&mut b, c); }
            }
            if b.kind == Kind::Boss && b.spawn >= 1.0 {
                b.minion_t -= dt;
                if b.minion_t <= 0.0 && self.bears.len() < 14 { b.minion_t = 8.0; for _ in 0..2 { self.spawn_bear(Kind::Knife, Some((b.pos.x + rr(-2.0, 2.0), b.pos.z + rr(-2.0, 2.0))), false); } }
            }
            let (elev, aiming) = ((pp.y + self.player.h * 0.65 - 1.18 * b.s).atan2(dist), b.fsm != Fsm::Idle);
            Self::animate(&mut b, dt, moving, elev, aiming, self.clock);
            self.bears[i] = b;
            // keep bears from stacking
            let (head, tail) = self.bears.split_at_mut(i + 1);
            let a = &mut head[i];
            for o in tail.iter_mut() {
                let (ex, ez) = (o.pos.x - a.pos.x, o.pos.z - a.pos.z);
                let (d, min) = (ex.hypot(ez).max(1e-4), (a.s + o.s) * 0.55);
                if d < min { let (ux, uz, push, wb) = (ex / d, ez / d, min - d, o.s / (a.s + o.s)); a.pos.x -= ux * push * wb; a.pos.z -= uz * push * wb; o.pos.x += ux * push * (1.0 - wb); o.pos.z += uz * push * (1.0 - wb); }
            }
            if self.state != State::Playing { return; }
            i += 1;
        }
    }
    fn hit_bear(&mut self, i: usize, zone: Zone, dmg: f32, dir: Vec3, pos: Vec3, src: Src) {
        if src == Src::Shot { self.stats.hits += 1; }
        let mut b = self.bears[i];
        b.jv += 7.0; b.seen = true;
        let dl = dir.x.hypot(dir.z).max(1e-4);
        let k = match src { Src::Shot => 3.5, Src::Melee => 2.0, _ => 9.0 } / b.s;
        b.kx += dir.x / dl * k; b.kz += dir.z / dl * k;
        if zone == Zone::Arm {
            b.arm_hp -= dmg; self.bears[i] = b;
            self.burst(pos, b.color, 3, 0.07, 4.0); self.hit_mark = (0.0, false, false); self.audio.hit();
            if b.arm_hp <= 0.0 { self.disarm(i, dir); }
            return;
        }
        let crit = zone == Zone::Head;
        let d = dmg * if crit { self.m.head_mul } else { 1.0 };
        b.hp -= d; b.flash = 0.12; self.bears[i] = b;
        self.burst(pos, b.color, if crit { 5 } else { 3 }, 0.09 * b.s.sqrt(), 4.5);
        if src != Src::Ff {
            self.float_text(pos, format!("{}", (d * 10.0).round() as i32), if crit { 1 } else { 0 }, 0.75);
            if src == Src::Shot || src == Src::Melee { self.hit_mark = (0.0, crit, b.hp <= 0.0); if crit { self.audio.head(); } else { self.audio.hit(); } }
        }
        if b.hp <= 0.0 { self.kill_bear(i, crit && src == Src::Shot, dir, src); }
        else if chance(0.12) { self.say(&b, pick(if src == Src::Ff { &L_FF[..] } else { &L_HIT[..] }), false); }
    }
    fn disarm(&mut self, i: usize, dir: Vec3) {
        let mut b = self.bears[i];
        let dl = dir.x.hypot(dir.z).max(1e-4);
        b.armed = false; b.melee = true; b.speed *= 1.55;
        self.end_attack(&mut b, 0.0);
        self.bears[i] = b;
        let arm = b.zone(-0.5, 1.18, 0.2);
        let loot = (b.t.weapon != Wpn::Knife).then_some(Loot::Gun(b.t.weapon));
        self.add_debris(DebrisKind::Arm(b.color, Some(b.t.weapon)), arm, v3(dir.x / dl * 3.0 + rr(-2.0, 2.0), rr(6.0, 9.0), dir.z / dl * 3.0 + rr(-2.0, 2.0)), 4.5, 0.2 * b.s, b.s, loot);
        self.stats.disarms += 1;
        let pts = if self.m.smile { 150 } else { 50 };
        self.score += pts; self.announce("DISARMED", 2); self.audio.disarm(); self.say(&b, pick(&L_DISARM), true); self.hitstop = self.hitstop.max(0.05);
        self.burst(arm, b.color, 6, 0.1, 5.0); self.float_text(arm, format!("+{pts}"), 2, 1.0);
        if self.m.smile { self.drop_bits(b.pos, b.color, 3); }
    }
    fn kill_bear(&mut self, i: usize, crit: bool, dir: Vec3, src: Src) {
        let mut b = self.bears.remove(i);
        self.end_attack(&mut b, 0.0);
        let (p, dl, t) = (b.pos, dir.x.hypot(dir.z).max(1e-4), b.t);
        self.stats.kills += 1; if crit { self.stats.heads += 1; } if src == Src::Ff { self.stats.ff += 1; }
        self.combo += 1; self.combo_t = 4.0;
        let pts = (t.pts as f32 * self.combo.min(8) as f32 * if crit { 1.5 } else { 1.0 } * if b.elite { 2.0 } else { 1.0 }).round() as u32;
        self.score += pts;
        if self.player.rush <= 0.0 { self.player.sugar = (self.player.sugar + if crit { 14.0 } else { 9.0 } * self.m.rush_gain).min(100.0); }
        let xp = (t.pts as f32 / 8.0).round() as u32 * if b.elite { 3 } else { 1 };
        let ups = self.profile.add_xp(xp);
        if let Some(run) = self.run.as_mut() { run.xp += xp; run.candy += (t.pts as f32 / 20.0).round() as u32 * if b.elite { 3 } else { 1 }; run.level_ups += ups; }
        if ups > 0 { let msg = format!("LEVEL UP · {}", rank_name(self.profile.level).to_uppercase()); self.announce(&msg, 1); self.audio.levelup(); self.player.hp = (self.player.hp + 40.0).min(self.m.max_hp); }
        else if let Some(sname) = streak(self.combo) { self.announce(sname, if self.combo >= 5 { 1 } else { 0 }); }
        let tag = match src { Src::Ff => "FRIENDLY FIRE  ", Src::Melee => "MELEE  ", _ if crit => "HEADSHOT  ", _ => "" };
        self.feed.insert(0, (format!("{tag}{}{}  +{pts}", if b.elite { "Target " } else { "" }, t.name), 0.0));
        self.feed.truncate(5);
        self.float_text(v3(p.x, 1.5 * b.s, p.z), format!("+{pts}"), 2, 1.1);
        self.hitstop = self.hitstop.max(if b.kind == Kind::Boss { 0.25 } else if crit { 0.07 } else { 0.045 });
        self.trauma = (self.trauma + if b.kind == Kind::Boss { 0.7 } else { 0.12 }).min(1.0);
        if self.m.chain && self.player.rush > 0.0 { self.pending_booms.push(v3(p.x, 0.9 * b.s, p.z)); }
        if chance(0.14) { self.bark_cd = 0.0; self.say(&b, pick(&L_DIE), true); }
        else if !self.bears.is_empty() && chance(0.3) { let o = pick(&self.bears); self.say(&o, pick(&L_ALLY), false); }
        if b.elite { if let Some(run) = self.run.as_mut() { run.killed += 1; let msg = format!("TARGET DOWN  {}/{}", run.killed, run.count); self.announce(&msg, 1); } }
        if b.kind == Kind::Boss { self.announce("THE KING IS JELLY", 1); self.slow_t = self.slow_t.max(1.2); }
        let last = self.bears.is_empty() && self.run.as_ref().is_some_and(|r| r.queue.is_empty() && r.typ != RunType::Survive && r.typ != RunType::Hunt);
        if last { self.slow_t = self.slow_t.max(0.8); }
        if crit && b.kind != Kind::Boss && b.head { // the head pops clean off; the body staggers on for a moment
            b.head = false;
            self.add_debris(DebrisKind::Head(b.color, t.gear), b.zone(0.0, 1.6, 0.0), v3(dir.x / dl * 4.0 + rr(-1.0, 1.0), rr(7.0, 10.0), dir.z / dl * 4.0 + rr(-1.0, 1.0)), 3.5, 0.4 * b.s, b.s, None);
            let sp = self.audio.sp(p.x, p.z); self.audio.pop(sp);
            self.burst(v3(p.x, 1.5 * b.s, p.z), b.color, 8, 0.08, 6.0);
            b.die_t = 0.55; self.dying.push(b);
        } else { self.burst_bear(&b, src != Src::Shot); }
    }
    fn burst_bear(&mut self, b: &Bear, big: bool) {
        let (p, s) = (b.pos, b.s);
        if b.armed { let loot = (b.t.weapon != Wpn::Knife).then_some(Loot::Gun(b.t.weapon)); self.add_debris(DebrisKind::Gun(b.t.weapon), b.zone(-0.5, 1.2, 0.5), v3(rr(-2.0, 2.0), rr(4.0, 7.0), rr(-2.0, 2.0)), 5.0, 0.12 * s, s, loot); }
        if b.t.shield { self.add_debris(DebrisKind::Shield, b.zone(0.2, 1.0, 0.5), v3(rr(-2.0, 2.0), rr(3.0, 6.0), rr(-2.0, 2.0)), 5.0, 0.3 * s, s, Some(Loot::Armor)); }
        self.burst(v3(p.x, 0.9 * s, p.z), b.color, (14.0 * s).round() as u32 + if big { 10 } else { 0 }, 0.17 * s.sqrt(), if big { 11.0 } else { 7.0 } + s);
        self.splat(p.x, p.z, b.color, rr(1.1, 1.7) * s);
        let n = (if b.kind == Kind::Boss { 16 } else { 2 + (rnd() * 3.0) as u32 }) * if self.player.rush > 0.0 { 2 } else { 1 };
        self.drop_bits(p, b.color, n);
        let sp = self.audio.sp(p.x, p.z); self.audio.kill(sp);
    }
    fn update_dying(&mut self, dt: f32) {
        let mut i = self.dying.len();
        while i > 0 {
            i -= 1;
            let c = self.clock;
            let b = &mut self.dying[i];
            b.die_t -= dt; b.lean = (c * 34.0).sin() * 0.22; b.arm_l = (c * 40.0).sin() * 0.9; b.arm_x = (c * 40.0).cos() * 0.9;
            b.pos.x += b.yaw.sin() * dt * 1.5; b.pos.z += b.yaw.cos() * dt * 1.5;
            if b.die_t <= 0.0 { let b = self.dying.remove(i); self.burst_bear(&b, false); }
        }
    }
    fn add_debris(&mut self, kind: DebrisKind, pos: Vec3, vel: Vec3, life: f32, r: f32, s: f32, loot: Option<Loot>) {
        self.debris.push(Debris { kind, pos, vel, rot: v3(rr(0.0, TAU), rr(0.0, TAU), rr(0.0, TAU)), w: v3(rr(-9.0, 9.0), rr(-9.0, 9.0), rr(-9.0, 9.0)), life: if loot.is_some() { 26.0 } else { life }, r, s, shrink: 1.0, loot });
    }
    fn update_debris(&mut self, dt: f32) {
        self.near_pick = None;
        let (mut best, pp) = (2.7, self.player.pos);
        let mut i = self.debris.len();
        while i > 0 {
            i -= 1;
            let d = &mut self.debris[i];
            d.vel.y -= 26.0 * dt; d.pos += d.vel * dt; d.rot += d.w * dt;
            if d.pos.y < d.r { d.pos.y = d.r; d.vel.y *= -0.42; d.vel.x *= 0.6; d.vel.z *= 0.6; d.w *= 0.55; }
            d.life -= dt;
            if d.life < 0.4 { d.shrink = (d.shrink * (1.0 - dt * 6.0)).max(0.0); }
            if d.loot.is_some() { let dd = (d.pos.x - pp.x).hypot(d.pos.z - pp.z); if dd < best && (pp.y - d.pos.y).abs() < 2.6 { best = dd; self.near_pick = Some(i); } }
            if d.life <= 0.0 { self.debris.remove(i); if let Some(n) = self.near_pick.as_mut() { if *n > i { *n -= 1; } else if *n == i { self.near_pick = None; } } }
        }
    }
    pub fn take_loot(&mut self) {
        let Some(i) = self.near_pick else { return };
        if i >= self.debris.len() { return; }
        match self.debris[i].loot { Some(Loot::Armor) => { self.player.armor = (self.player.armor + 50.0).min(100.0); self.announce("+50 ARMOR", 2); self.audio.pickup(); } Some(Loot::Gun(w)) => self.equip_loot(w), None => return }
        self.debris.remove(i); self.near_pick = None;
    }

    // ───────────────────────── enemy fire, explosions, damage ─────────────────────────
    fn update_ebullets(&mut self, dt: f32) {
        let pp = self.player.pos;
        let mut i = self.ebullets.len();
        while i > 0 {
            i -= 1;
            let mut e = self.ebullets[i];
            e.prev = e.pos; e.pos += e.vel * dt; e.life -= dt;
            let mut dead = e.life <= 0.0;
            if e.rocket && chance(0.8) { self.smoke(e.prev, v3(rr(-0.4, 0.4), rr(0.0, 0.8), rr(-0.4, 0.4)), rr(0.14, 0.26), rr(0.5, 0.9), 0xd8d0d8); }
            if !dead {
                let t = self.world.seg_blocked(e.prev, e.pos);
                if t >= 0.0 { e.pos = e.prev.lerp(e.pos, t); dead = true; } else if e.pos.y < 0.05 { e.pos.y = 0.05; dead = true; }
                if dead && !e.rocket { self.spark(e.pos, 0xffd27a, 4, 4.0, 0.04, 0.2, 3.0); }
            }
            if !dead {
                let half = self.player.h * 0.56;
                let c = v3(pp.x, pp.y + half, pp.z);
                let ab = e.pos - e.prev;
                let q = e.prev + ab * ((c - e.prev).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
                let (hd, vd) = ((q.x - c.x).hypot(q.z - c.z), (q.y - c.y).abs());
                if hd < if e.rocket { 1.2 } else { 0.45 } && vd < if e.rocket { 1.6 } else { half } {
                    if e.rocket { dead = true; } else if self.player.iframes <= 0.0 { dead = true; self.hurt(e.dmg, e.prev.x - e.vel.x, e.prev.z - e.vel.z); }
                } else if !e.whiz && !e.rocket && hd < 2.2 && vd < 2.0 { e.whiz = true; self.audio.whiz(); }
            }
            if dead { self.ebullets.remove(i); if e.rocket { self.explode(e.pos, 5.0, 5.0, 40.0, e.shot_down, e.shot_down); } } else { self.ebullets[i] = e; }
            if self.state != State::Playing { return; }
        }
    }
    pub fn explode(&mut self, pos: Vec3, radius: f32, bear_dmg: f32, player_dmg: f32, by_player: bool, shot_down: bool) {
        let sp = self.audio.sp(pos.x, pos.z);
        self.audio.boom(sp);
        let pinkish = by_player && !shot_down;
        self.booms.push(Boom { pos, r: radius, t: 0.0, color: if pinkish { v3(4.0, 1.6, 3.2) } else { v3(4.5, 2.4, 0.8) } });
        for _ in 0..46 { self.sparks.push(Spark { pos, vel: v3(rr(-1.0, 1.0), rr(-0.2, 1.0), rr(-1.0, 1.0)) * 14.0, size: rr(0.05, 0.14), life: rr(0.3, 0.9), max: 0.9, color: lin(if pinkish { pick(&CANDY) } else { 0xffb347 }) * 3.5, grav: 7.0, smoke: false }); }
        for _ in 0..10 { self.smoke(pos + v3(rr(-1.0, 1.0), rr(0.0, 1.0), rr(-1.0, 1.0)), v3(rr(-1.0, 1.0), rr(1.0, 3.0), rr(-1.0, 1.0)), rr(0.5, 1.1), rr(0.8, 1.6), if pinkish { 0xffc4e4 } else { 0x5a4a52 }); }
        self.light(pos, if by_player { v3(1.0, 0.6, 0.9) } else { v3(1.0, 0.7, 0.4) } * 14.0, radius * 4.0);
        self.trauma = (self.trauma + (1.2 - pos.distance(self.cam_pos) / 22.0).clamp(0.0, 0.8)).min(1.0);
        if pos.y < 2.0 { self.splat(pos.x, pos.z, if pinkish { pick(&CANDY) } else { 0x3a2030 }, radius * 0.45); }
        if shot_down { self.score += 75; self.announce("INTERCEPTED", 2); }
        let c = self.player.pos + Vec3::Y * 0.9;
        let d = c.distance(pos);
        if d < radius {
            let f = 1.0 - d / radius;
            let n = (c - pos).normalize_or_zero();
            self.player.vel.x += n.x * 16.0 * f; self.player.vel.z += n.z * 16.0 * f; self.player.vel.y += (5.0 + 7.0 * f) * f + 2.0; self.player.on_ground = false;
            if player_dmg > 0.0 { self.hurt(player_dmg * f, pos.x, pos.z); }
        }
        let ids: Vec<u32> = self.bears.iter().map(|b| b.id).collect();
        for id in ids {
            let Some(i) = self.bear_idx(id) else { continue };
            let bc = self.bears[i].zone(0.0, 0.85, 0.0);
            let bd = bc.distance(pos) - 0.5 * self.bears[i].s;
            if bd < radius { self.hit_bear(i, Zone::Body, (bear_dmg * (1.0 - bd / radius).clamp(0.0, 1.0)).round().max(1.0), bc - pos, bc, if by_player { Src::Boom } else { Src::Ff }); }
        }
    }
    pub fn hurt(&mut self, dmg: f32, sx: f32, sz: f32) {
        if self.state != State::Playing || self.player.iframes > 0.0 || self.intro.is_some() || self.run.as_ref().is_some_and(|r| r.done) { return; }
        let mut d = dmg * if self.player.rush > 0.0 { 0.5 } else { 1.0 };
        if self.player.armor > 0.0 { let a = self.player.armor.min(d * 0.7); self.player.armor -= a; d -= a; }
        self.player.hp -= d; self.player.iframes = 0.12;
        self.hurt_flash = (self.hurt_flash + 0.35 + dmg / 40.0).min(1.0); self.trauma = (self.trauma + 0.25 + dmg / 60.0).min(1.0); self.kick_pitch += rr(-0.02, 0.02);
        self.dmg_dir(sx, sz); self.audio.hurt();
        if self.player.hp <= 0.0 {
            let wind = self.m.wind && self.run.as_ref().is_some_and(|r| !r.wind);
            if wind { if let Some(r) = self.run.as_mut() { r.wind = true; } self.player.hp = 30.0; self.player.sugar = 100.0; self.player.iframes = 1.0; self.start_rush(); self.announce("SECOND WIND", 1); }
            else { self.player.hp = 0.0; self.state = State::Dead; self.dead_t = 2.2; self.audio.over(); self.audio.music(0.15); let m = self.run.as_ref().is_some_and(|r| r.mission.is_some()); self.announce(if m { "MISSION FAILED" } else { "CHEWED UP" }, 0); }
        }
    }
    fn drop_bits(&mut self, p: Vec3, color: u32, n: u32) { for _ in 0..n { self.bits.push(Bit { pos: v3(p.x, 0.8, p.z), vel: v3(rr(-4.0, 4.0), rr(5.0, 9.0), rr(-4.0, 4.0)), t: 0.0, color }); } }
    fn update_bits(&mut self, dt: f32) {
        let c = self.player.pos + Vec3::Y;
        let range = if self.player.rush > 0.0 { self.m.magnet * 2.0 } else { self.m.magnet };
        let mut i = self.bits.len();
        while i > 0 {
            i -= 1;
            let b = &mut self.bits[i];
            b.t += dt;
            let d = b.pos.distance(c);
            if b.t > 0.5 && d < range { let want = (c - b.pos) * (20.0 / d.max(1e-3)); b.vel = b.vel.lerp(want, (dt * 7.0).min(1.0)); }
            else { b.vel.y -= 24.0 * dt; if b.pos.y < 0.15 { b.pos.y = 0.15; b.vel.y *= -0.4; b.vel.x *= 0.7; b.vel.z *= 0.7; } }
            b.pos += b.vel * dt;
            if d < 1.1 && b.t > 0.3 {
                self.player.hp = (self.player.hp + self.m.bit_heal).min(self.m.max_hp);
                if self.player.rush <= 0.0 { self.player.sugar = (self.player.sugar + 2.5 * self.m.rush_gain).min(100.0); }
                if let Some(r) = self.run.as_mut() { r.candy += 1; }
                self.audio.bit(self.chain_n); self.chain_n += 1; self.chain_t = 0.7; self.bits.remove(i);
            } else if b.t > 16.0 { self.bits.remove(i); }
        }
    }

    // ───────────────────────── run / objectives ─────────────────────────
    fn clear_all(&mut self) {
        self.bears.clear(); self.dying.clear(); self.props.clear(); self.shots.clear(); self.ebullets.clear(); self.bits.clear(); self.debris.clear(); self.chunks.clear(); self.sparks.clear(); self.splats.clear();
        self.beams.clear(); self.booms.clear(); self.flashes.clear(); self.lights_tmp.clear(); self.pending_booms.clear(); self.floats.clear(); self.bubbles.clear(); self.feed.clear(); self.radio_q.clear(); self.radio = ("", 0.0); self.dmg_dirs.clear();
        self.tokens = 0; self.near_pick = None;
    }
    pub fn obj_text(&self) -> String {
        let Some(run) = &self.run else { return String::new() };
        let left = self.bears.len() + run.queue.len();
        if run.done { return "MISSION COMPLETE".into(); }
        match run.typ {
            RunType::Arcade => format!("WAVE {} · {left} HOSTILES", self.wave.max(1)),
            RunType::Survive => { let t = run.timer.max(0.0).ceil() as u32; format!("SURVIVE · {}:{:02}", t / 60, t % 60) }
            RunType::Hunt => format!("{} · {}/{}", run.label.to_uppercase(), run.killed, run.count),
            _ => format!("WAVE {}/{} · {left} HOSTILES", self.wave.max(1), run.waves.len()),
        }
    }
    pub fn start_run(&mut self, id: u32) {
        let m = MISSIONS.iter().find(|x| x.id == id);
        self.screens.clear(); self.clear_all();
        self.pal = palette(m.map(|x| x.palette).unwrap_or(pick(&["morning", "sunset", "night", "overcast"])));
        self.world.build_arena(&self.atlas, m.map(|x| x.seed).unwrap_or(1 + (rnd() * 999.0) as u32), m.map(|x| x.layout).unwrap_or(&[]));
        self.world_dirty = true; self.cut = true;
        self.m = mods(&self.profile); self.config_guns();
        self.player = Player { pos: v3(SPAWN.0, 0.0, SPAWN.1), vel: Vec3::ZERO, yaw: 0.0, pitch: 0.0, hp: self.m.max_hp, armor: 0.0, h: 1.7, on_ground: true, crouch: false, iframes: 0.0, sugar: 0.0, rush: 0.0, dash: self.m.dash_max };
        self.hp_ghost = 1.0; self.score = 0; self.wave = 0; self.combo = 0; self.combo_t = 0.0; self.hitstop = 0.0; self.slow_t = 0.0; self.trauma = 0.0; self.hurt_flash = 0.0; self.kick_pitch = 0.0; self.kick_yaw = 0.0;
        self.land_dip = 0.0; self.roll = 0.0; self.turn_v = 0.0; self.step_off = 0.0; self.sprint_k = 0.0; self.slide_k = 0.0; self.slide_t = 0.0; self.melee_t = 0.0; self.chute_t = 0.0; self.ts_w = 1.0; self.ts_p = 1.0; self.clock = 0.0;
        self.mantle = None; self.stats = Stats::default(); self.results = None;
        self.gi = 0; self.last_gi = 0; self.sw_t = 1.0; self.sw_to = None; self.reload_t = 0.0; self.fire_cool = 0.0; self.recoil = 0.0; self.ads_t = 0.0; self.cross_bloom = 0.0;
        let (typ, waves, mix, cap, target, count, label, timer): (RunType, &'static [Mix], Mix, usize, Kind, u32, &'static str, f32) = match m.map(|x| &x.obj) {
            Some(Obj::Clear(w)) => (RunType::Clear, w, &[], 10, Kind::Grunt, 0, "", 0.0),
            Some(Obj::Boss(w)) => (RunType::Boss, w, &[], 10, Kind::Grunt, 0, "", 0.0),
            Some(Obj::Survive { time, cap, mix }) => (RunType::Survive, &[], mix, *cap, Kind::Grunt, 0, "", *time),
            Some(Obj::Hunt { target, count, label, cap, mix }) => (RunType::Hunt, &[], mix, *cap, *target, *count, label, 0.0),
            None => (RunType::Arcade, &[], &[], 14, Kind::Grunt, 0, "", 0.0),
        };
        self.run = Some(Run { mission: m, typ, waves, mix, cap, target, count, label, wave_i: -1, queue: Vec::new(), spawn_t: 2.5, break_t: 2.4, timer, killed: 0, spawned: 0, target_t: 3.5, candy: 0, xp: 0, level_ups: 0, done: false, end_t: 0.0, wind: false, seen: HashSet::new(), t: 0.0 });
        let sdv = match m { Some(x) => x.insert == Insert::Sdv, None => chance(0.5) };
        self.intro = Some(Intro { sdv, t: 0.0, dur: if sdv { 9.5 } else { 8.0 }, wet: false });
        if sdv { self.player.pitch = 0.04; self.audio.dive(); } else { self.player.pitch = -0.5; self.audio.wind(); }
        self.state = State::Playing; self.want_capture = true;
        match m { Some(x) => { self.show_card(&format!("MISSION {}", x.id), x.title, &x.obj_line()); self.radio_q.extend(x.radio.iter()); } None => self.show_card("ARCADE", "Endless", "How long can you last?") }
        self.card.3 = self.card.3.max(self.intro.as_ref().unwrap().dur - 2.5);
        self.audio.wave(); self.audio.music(0.6); self.audio.muffle(false); self.muffled = false;
    }
    fn next_wave(&mut self) {
        let Some(run) = self.run.as_mut() else { return };
        run.wave_i += 1; self.wave = (run.wave_i + 1) as u32;
        let arcade = run.typ == RunType::Arcade;
        let (title, mix): (&str, Vec<(Kind, u32)>) = if arcade { arcade_wave(self.wave, &mut rnd) } else { ("", run.waves[run.wave_i as usize].to_vec()) };
        let mut q: Vec<Kind> = Vec::new();
        for &(k, c) in &mix { if k != Kind::Boss { for _ in 0..c { q.push(k); } } }
        for i in (1..q.len()).rev() { let j = (rnd() * (i + 1) as f32) as usize; q.swap(i, j.min(i)); }
        for &(k, c) in &mix { if k == Kind::Boss { for _ in 0..c { let at = q.len().saturating_sub(2); q.insert(at, k); } } }
        run.queue = q; run.spawn_t = 1.2;
        let (wi, n) = (run.wave_i, run.waves.len());
        if arcade { self.show_card(&format!("WAVE {}", self.wave), title, ""); self.audio.wave(); }
        else if wi > 0 { self.show_card(&format!("WAVE {} / {n}", self.wave), if wi as usize == n - 1 { "Final Wave" } else { "Reinforcements" }, ""); self.audio.wave(); }
    }
    fn update_run(&mut self, dt: f32) {
        let Some(run) = self.run.as_mut() else { return };
        let weighted = |mix: Mix| -> Kind { let n: u32 = mix.iter().map(|m| m.1).sum(); let mut r = rnd() * n as f32; for &(k, c) in mix { r -= c as f32; if r < 0.0 { return k; } } mix[0].0 };
        let plain = self.bears.iter().filter(|b| !b.elite).count();
        match run.typ {
            RunType::Survive | RunType::Hunt => {
                run.spawn_t -= dt;
                let spawn = run.spawn_t <= 0.0 && plain < run.cap;
                if spawn { run.spawn_t = rr(1.1, 2.1); }
                let (mix, typ) = (run.mix, run.typ);
                if typ == RunType::Survive { run.timer -= dt; if run.timer <= 0.0 { self.complete(); return; } }
                else {
                    if run.killed >= run.count { self.complete(); return; }
                    if !self.bears.iter().any(|b| b.elite) && run.spawned < run.count { run.target_t -= dt; if run.target_t <= 0.0 { run.spawned += 1; run.target_t = 3.5; let tk = run.target; self.spawn_bear(tk, None, true); self.announce("TARGET SPOTTED", 1); } }
                }
                if spawn { self.spawn_bear(weighted(mix), None, false); }
            }
            _ => {
                if !run.queue.is_empty() {
                    run.spawn_t -= dt;
                    let cap = if run.typ == RunType::Arcade { (7 + self.wave as usize).min(14) } else { 10 };
                    if run.spawn_t <= 0.0 && self.bears.len() < cap { let k = run.queue.pop().unwrap(); run.spawn_t = (1.0 - self.wave as f32 * 0.05).max(0.35); self.spawn_bear(k, None, false); }
                } else if self.bears.is_empty() && self.dying.is_empty() {
                    if run.wave_i >= 0 && run.typ != RunType::Arcade && run.wave_i as usize >= run.waves.len() - 1 { self.complete(); return; }
                    if run.break_t <= 0.0 {
                        run.break_t = 4.0;
                        let bonus = self.wave * 100; self.score += bonus;
                        let msg = format!("WAVE CLEARED  +{bonus}"); self.announce(&msg, 0); self.audio.clear(); self.player.hp = (self.player.hp + 20.0).min(self.m.max_hp);
                    } else { run.break_t -= dt; if run.break_t <= 0.0 { self.next_wave(); } }
                }
            }
        }
    }
    pub fn complete(&mut self) {
        let Some(run) = self.run.as_mut() else { return };
        if run.done { return; }
        run.done = true; run.end_t = 3.4; self.slow_t = 1.4;
        let bears = std::mem::take(&mut self.bears);
        for b in &bears { self.burst_bear(b, true); }
        self.ebullets.clear(); self.tokens = 0;
        self.announce("MISSION COMPLETE", 1); self.audio.win(); self.audio.music(0.3);
    }
    fn finish_run(&mut self, win: bool) {
        let Some(run) = self.run.as_ref() else { return };
        let m = run.mission;
        let mut r = Results { win, mission: m, score: self.score, wave: self.wave, stats: self.stats, candy: run.candy, xp: run.xp, stars: 0, bonus: Vec::new(), unlock: None, level_ups: run.level_ups, new_best: false, age: 0.0 };
        if let (Some(m), true) = (m, win) {
            let (b1, b2, first) = (self.player.hp >= self.m.max_hp * 0.5, self.stats.heads >= m.heads, self.profile.stars(m.id) == 0);
            r.stars = 1 + b1 as u32 + b2 as u32;
            r.bonus = vec![("Mission complete".into(), true), ("Finish with 50%+ health".into(), b1), (format!("{} headshots ({})", m.heads, self.stats.heads), b2)];
            r.candy += (m.candy as f32 * if first { 1.0 } else { 0.4 } * (1.0 + 0.25 * (r.stars - 1) as f32)).round() as u32;
            let bx = (m.xp as f32 * if first { 1.0 } else { 0.4 }).round() as u32;
            r.xp += bx; r.level_ups += self.profile.add_xp(bx);
            if first { r.unlock = m.unlock; }
            let best = self.profile.stars(m.id).max(r.stars); self.profile.done.insert(m.id, best);
        } else if m.is_none() { r.candy += self.score / 150; if self.score > self.profile.best { self.profile.best = self.score; r.new_best = true; } }
        self.profile.candy += r.candy; self.profile.save();
        self.results = Some(r);
        self.state = State::Results; self.screens = vec![Screen::Results]; self.want_capture = false;
        self.audio.music(0.15); self.audio.muffle(false); self.muffled = false;
    }
    pub fn to_menu(&mut self) {
        self.clear_all(); self.run = None; self.intro = None; self.pal = palette("morning");
        self.world.build_arena(&self.atlas, 1, &[]); self.world_dirty = true; self.cut = true;
        let kinds = [Kind::Rifle, Kind::Grunt, Kind::Shield, Kind::Rpg, Kind::Sniper, Kind::Knife, Kind::Rifle, Kind::Grunt];
        for (i, k) in kinds.iter().enumerate() {
            let (a, up) = (i as f32 / kinds.len() as f32 * TAU, i % 2);
            let r = if up == 1 { 3.4 } else { 5.6 };
            let mut b = Bear::new(*k, self.next_id); self.next_id += 1;
            b.pos = v3(a.sin() * r, CAKE[up].1, a.cos() * r); b.yaw = a;
            self.props.push(b);
        }
        self.state = State::Menu; self.screens = vec![Screen::Menu]; self.want_capture = false;
        self.audio.music(0.15); self.audio.muffle(false);
    }
    pub fn start_rush(&mut self) {
        if self.state != State::Playing || self.player.sugar < 100.0 || self.player.rush > 0.0 { return; }
        self.player.rush = self.m.rush_time; self.audio.rush_on(); self.announce("SUGAR RUSH", 3); self.reload_t = 0.0;
        for g in self.guns.iter_mut() { if g.loot.is_none() { g.ammo = g.clip; } }
        if !self.bears.is_empty() { let b = pick(&self.bears); self.say(&b, pick(&L_RUSH), true); }
    }
    pub fn pause(&mut self) { if self.state != State::Playing { return; } self.state = State::Paused; self.screens = vec![Screen::Pause]; self.want_capture = false; self.audio.muffle(true); }
    pub fn resume(&mut self) { if self.state != State::Paused { return; } self.state = State::Playing; self.screens.clear(); self.want_capture = true; self.audio.muffle(self.player.rush > 0.0); }
    pub fn skip_intro(&mut self) { if let Some(i) = self.intro.as_mut() { i.t = i.t.max(i.dur * 0.94); } }
    pub fn in_intro(&self) -> bool { self.intro.is_some() }

    // ───────────────────────── player ─────────────────────────
    fn update_player(&mut self, dt: f32, rdt: f32, inp: &Input) {
        use KeyCode::*;
        let f = (inp.key(KeyW) || inp.key(ArrowUp)) as i32 as f32 - (inp.key(KeyS) || inp.key(ArrowDown)) as i32 as f32;
        let s = (inp.key(KeyD) || inp.key(ArrowRight)) as i32 as f32 - (inp.key(KeyA) || inp.key(ArrowLeft)) as i32 as f32;
        let (sy, cy) = (self.player.yaw.sin(), self.player.yaw.cos());
        let (mut wx, mut wz) = (-sy * f + cy * s, -cy * f - sy * s);
        let wl = wx.hypot(wz);
        if wl > 0.0 { wx /= wl; wz /= wl; }
        self.turn_v = damp(self.turn_v, if rdt > 0.0 { inp.dx * 0.0022 * self.profile.sens / rdt } else { 0.0 }, 12.0, rdt); // how fast you are turning, for the lean
        let mut sprinting = false;
        let ads = inp.rmb;
        if let Some(mn) = self.mantle.as_mut() { // hauling up onto a ledge: straight up first, then over
            mn.t += dt / mn.dur;
            let t = mn.t.min(1.0);
            let over = ((t - 0.35) / 0.65).clamp(0.0, 1.0);
            self.player.pos = v3(lerp(mn.from.x, mn.to.x, over), lerp(mn.from.y, mn.to.y, (t * 1.5).min(1.0)), lerp(mn.from.z, mn.to.z, over));
            self.player.vel = Vec3::ZERO;
            if t >= 1.0 { self.player.pos.y = mn.to.y; self.player.on_ground = true; self.mantle = None; self.land_dip = -0.08; }
        } else {
            let p = &mut self.player;
            if inp.hit(KeyC) || inp.hit(ControlLeft) {
                let sp = p.vel.x.hypot(p.vel.z);
                if p.on_ground && self.slide_t <= 0.0 && sp > 6.0 && p.dash >= 1.0 { p.dash -= 1.0; self.slide_t = 0.9; let k = (sp + 3.5).max(12.5) / sp; p.vel.x *= k; p.vel.z *= k; p.iframes = p.iframes.max(0.28); self.audio.slide(); }
            }
            p.dash = (p.dash + dt / 1.6).min(self.m.dash_max);
            if self.slide_t > 0.0 { // committed: friction bleeds the speed, you can only nudge the line
                self.slide_t -= dt;
                let fr = (-1.4 * dt).exp();
                p.vel.x = p.vel.x * fr + wx * 2.5 * dt; p.vel.z = p.vel.z * fr + wz * 2.5 * dt; p.crouch = false;
                if p.vel.x.hypot(p.vel.z) < 4.5 || !p.on_ground { self.slide_t = 0.0; }
            } else {
                p.crouch = (inp.key(KeyC) || inp.key(ControlLeft)) && p.on_ground;
                sprinting = (inp.key(ShiftLeft) || inp.key(ShiftRight)) && f > 0.0 && p.on_ground && !ads && !inp.lmb && !p.crouch && self.melee_t <= 0.0;
                let speed = if p.crouch { 3.4 } else if sprinting { 9.8 } else { 6.6 } * (1.0 - self.ads_t * 0.4);
                let acc = (dt * if p.on_ground { 10.0 } else { 2.5 }).min(1.0);
                p.vel.x += (wx * speed - p.vel.x) * acc; p.vel.z += (wz * speed - p.vel.z) * acc;
            }
            if inp.hit(Space) && p.on_ground { p.vel.y = 8.8; p.on_ground = false; self.slide_t = 0.0; self.audio.jump(); }
            p.vel.y -= 30.0 * dt;
            let (was_air, vy0, y_prev) = (!p.on_ground, p.vel.y, p.pos.y);
            p.pos += p.vel * dt;
            let feet = p.pos.y;
            self.world.push_out(&mut p.pos, 0.45, feet);
            if let Some(hi) = self.world.last_hit { // pushing into something climbable: grab the edge
                let hit = self.world.colliders[hi];
                if hit.top && wl > 0.0 && (was_air || inp.key(Space)) {
                    let (dh, tx, tz) = (hit.h - p.pos.y, p.pos.x + wx * 1.05, p.pos.z + wz * 1.05);
                    if dh > 0.42 && dh < 2.5 {
                        let top = self.world.floor_at(tx, tz, hit.h + 0.01);
                        if top >= hit.h - 0.01 { self.mantle = Some(Mantle { t: 0.0, dur: 0.26 + dh * 0.13, from: p.pos, to: v3(tx, top, tz) }); self.slide_t = 0.0; self.audio.mantle(); }
                    }
                }
            }
            let (d, lim) = (p.pos.x.hypot(p.pos.z), ARENA_R - 1.6);
            if d > lim { p.pos.x *= lim / d; p.pos.z *= lim / d; }
            let fl = self.world.floor_at(p.pos.x, p.pos.z, p.pos.y);
            if p.pos.y <= fl {
                if !was_air && fl - y_prev > 0.04 { self.step_off -= fl - y_prev; } // walked up a stair: ease the camera instead of popping
                p.pos.y = fl;
                if was_air && vy0 < -7.0 { self.land_dip = (vy0 * 0.014).clamp(-0.32, 0.0); self.trauma = (self.trauma + 0.08).min(1.0); self.audio.land(); }
                p.vel.y = 0.0; p.on_ground = true;
            } else if p.pos.y > fl + 0.05 { p.on_ground = false; }
        }
        let p = &mut self.player;
        p.iframes -= dt;
        p.h = damp(p.h, if self.slide_t > 0.0 { 0.85 } else if p.crouch { 1.12 } else { 1.7 }, 12.0, rdt);
        self.sprint_k = damp(self.sprint_k, if sprinting { 1.0 } else { 0.0 }, 10.0, rdt); self.slide_k = damp(self.slide_k, if self.slide_t > 0.0 { 1.0 } else { 0.0 }, 12.0, rdt);
        let moving = if p.on_ground && self.slide_t <= 0.0 && self.mantle.is_none() { p.vel.x.hypot(p.vel.z) } else { 0.0 };
        let prev_bob = self.bob_t;
        self.bob_t += dt * moving * 1.2;
        if moving > 1.5 && (self.bob_t / PI).floor() != (prev_bob / PI).floor() { self.audio.step(sprinting); }
        self.land_dip = damp(self.land_dip, 0.0, 9.0, rdt); self.step_off = damp(self.step_off, 0.0, 14.0, rdt); self.kick_pitch = damp(self.kick_pitch, 0.0, 9.0, rdt); self.kick_yaw = damp(self.kick_yaw, 0.0, 9.0, rdt);
        self.roll = damp(self.roll, -s * 0.02 - (self.turn_v * 0.022).clamp(-0.075, 0.075) + if self.slide_t > 0.0 { 0.07 } else { 0.0 }, 9.0, rdt); // lean into strafes and turns
        let amp = (moving / 7.0).min(1.0) * (1.0 - self.ads_t * 0.7) * if sprinting { 1.6 } else { 1.0 };
        let (bob, sway, sh) = (self.bob_t.sin() * 0.032 * amp, (self.bob_t * 0.5).cos() * 0.02 * amp, self.trauma * self.trauma);
        let mk = self.mantle.as_ref().map(|m| (m.t.min(1.0) * PI).sin()).unwrap_or(0.0);
        self.cam_pos = v3(p.pos.x + sway * cy, p.pos.y + p.h + bob + self.land_dip + self.step_off - mk * 0.25, p.pos.z - sway * sy);
        self.cam_rot = v3(p.pitch + self.kick_pitch - mk * 0.25 + rr(-1.0, 1.0) * sh * 0.05, p.yaw + self.kick_yaw + rr(-1.0, 1.0) * sh * 0.05, self.roll + (self.bob_t * 0.5).cos() * 0.004 * amp + rr(-1.0, 1.0) * sh * 0.04);
        let scope = self.guns[self.gi].scope;
        let fov = lerp(75.0, if scope { 16.0 } else { 50.0 }, self.ads_t) + if p.rush > 0.0 { 7.0 } else { 0.0 } + self.sprint_k * 5.0 + self.slide_k * 7.0;
        self.fov = damp(self.fov, fov, 12.0, rdt);
        let q = Quat::from_euler(EulerRot::YXZ, self.cam_rot.y, self.cam_rot.x, self.cam_rot.z);
        self.cam_fwd = q * Vec3::NEG_Z; self.cam_right = q * Vec3::X; self.cam_up = q * Vec3::Y;
    }
    fn update_gun(&mut self, dt: f32, rdt: f32, inp: &Input) {
        self.fire_cool -= dt;
        if self.sw_t < 1.0 { self.sw_t += rdt / 0.34; if self.sw_t >= 0.45 { if let Some(to) = self.sw_to.take() { self.last_gi = self.gi; self.gi = to; } } }
        if self.gi >= self.guns.len() { self.gi = 0; }
        let gi = self.gi;
        if self.reload_t > 0.0 { self.reload_t -= dt; if self.reload_t <= 0.0 { self.reload_t = 0.0; self.guns[gi].ammo = self.guns[gi].clip; } }
        let g = self.guns[gi].clone();
        let want_ads = inp.rmb && !g.no_ads && self.reload_t <= 0.0 && self.sw_t >= 1.0 && self.sprint_k < 0.3 && self.melee_t <= 0.0 && self.mantle.is_none();
        self.ads_t = damp(self.ads_t, if want_ads { 1.0 } else { 0.0 }, 14.0, rdt);
        if (inp.hit(KeyCode::KeyV) || inp.mmb) && self.melee_t <= 0.0 && self.sw_t >= 1.0 && self.mantle.is_none() { self.melee_t = 0.42; self.audio.swing(); }
        if self.melee_t > 0.0 { let was = self.melee_t; self.melee_t -= dt; if was > 0.27 && self.melee_t <= 0.27 { self.melee_hit(); } }
        if inp.lmb { self.fire(); }
        self.recoil = damp(self.recoil, 0.0, 13.0, dt); self.cross_bloom = damp(self.cross_bloom, 0.0, 6.0, rdt);
        if self.pump_t > 0.0 { let was = self.pump_t; self.pump_t -= dt; if was > 0.3 && self.pump_t <= 0.3 { self.audio.pump(); } }
        if g.loot == Some(Wpn::Minigun) && self.fire_cool > -0.25 { self.barrel_spin += rdt * 30.0; }
        // weapon inertia: the gun trails behind where you are looking and catches up
        self.sway_x = damp(self.sway_x, (inp.dx * 0.0009).clamp(-0.06, 0.06), 9.0, rdt); self.sway_y = damp(self.sway_y, (inp.dy * 0.0009).clamp(-0.05, 0.05), 9.0, rdt);
        let p = &self.player;
        let moving = if p.on_ground { (p.vel.x.hypot(p.vel.z) / 8.0).min(1.0) } else { 0.0 };
        let (hip_k, aim_k) = (1.0 - self.ads_t * 0.85, 1.0 - self.ads_t * 0.65);
        let dip = if self.reload_t > 0.0 { ((1.0 - self.reload_t / (g.reload * self.m.reload).max(0.01)) * PI).sin() } else { 0.0 };
        let drop = if self.sw_t < 0.45 { self.sw_t / 0.45 } else if self.sw_t < 1.0 { 1.0 - (self.sw_t - 0.45) / 0.55 } else { 0.0 };
        let mel = if self.melee_t > 0.0 { ((1.0 - self.melee_t / 0.42) * PI).sin() } else { 0.0 };
        let mk = self.mantle.as_ref().map(|m| (m.t.min(1.0) * PI).sin()).unwrap_or(0.0);
        let vm = self.vm(gi);
        let (hip, ads) = (vm.hip, v3(0.0, -vm.sight_y, vm.ads_z));
        let mut pos = hip.lerp(ads, self.ads_t);
        pos.x += (self.bob_t * 0.5).cos() * 0.007 * moving * hip_k - self.sway_x * 0.5 + (self.clock * 1.3).sin() * 0.0006 - self.sprint_k * 0.03 - mel * 0.06;
        pos.y += (self.bob_t * 0.5).sin().abs() * 0.008 * moving * hip_k - dip * 0.13 - drop * 0.2 + self.sway_y * 0.5 - self.land_dip * 0.12 + (self.clock * 2.1 + 1.0).sin() * 0.0005 - self.sprint_k * 0.035 - mk * 0.22 - self.slide_k * 0.02;
        pos.z += self.recoil * 0.035 - mel * 0.16;
        self.gun_pos = pos;
        self.gun_rot = v3(self.recoil * 0.1 - dip * 0.9 - drop * 1.1 + self.sway_y * 1.5 * aim_k - self.sprint_k * 0.3 - mk * 0.9 + mel * 0.15,
            0.11 * hip_k + self.sway_x * 2.0 * aim_k + self.sprint_k * 0.7 + mel * 0.9,
            dip * 0.5 - self.roll * 2.0 - 0.05 * hip_k - (self.turn_v * 0.04).clamp(-0.14, 0.14) + self.sprint_k * 0.22 + self.slide_k * 0.28 - mel * 0.5);
        // the holographic reticle floats: it lags a turn and settles, like the real thing
        self.dot_off = Vec2::new(self.sway_x * 0.09 + (self.clock * 1.3).sin() * 0.0003, self.sway_y * 0.09 + self.bob_t.sin() * 0.0006 * moving);
        self.scoped = g.scope && self.ads_t > 0.85;
    }
    fn vm(&self, gi: usize) -> &VmIds { match self.guns.get(gi).and_then(|g| g.loot) { Some(w) => &self.ids.loot_vm[&w], None => &self.ids.vm[gi.min(2)] } }

    // ───────────────────────── insertion: by parachute or under the water ─────────────────────────
    fn update_intro(&mut self, rdt: f32) {
        let Some(mut it) = self.intro.take() else { return };
        it.t += rdt;
        let (t, p) = ((it.t / it.dur).min(1.0), self.player.pos);
        if !it.sdv { // under canopy from the bay side: fast at first, then the flare
            let (k, sw) = ((1.0 - t).powf(1.7), (it.t * 1.3).sin());
            self.cam_pos = v3(p.x + k * 14.0 + sw * k * 2.5, 105.0 * k + self.player.h, p.z + k * 34.0);
            if t > 0.72 { self.player.pitch += (0.0 - self.player.pitch) * (rdt * 3.0).min(1.0); } // level out for the landing
            self.cam_rot = v3(self.player.pitch, self.player.yaw, sw * 0.07 * (k * 4.0).min(1.0));
            self.chute_pos = self.cam_pos; self.chute_t = 2.0;
        } else { // towed in under the surface, then up the boat ramp
            let u = if t < 0.6 { 0.0 } else { (t - 0.6) / 0.4 };
            let z = if t < 0.6 { lerp(ARENA_R + 52.0, ARENA_R + 15.0, t / 0.6) } else { lerp(ARENA_R + 15.0, SPAWN.1, u * u * (3.0 - 2.0 * u)) };
            let gy = if z > ARENA_R { (-(z - ARENA_R) * 0.205).max(-3.2) } else { 0.0 };
            let eye = lerp(0.5, self.player.h, (u * 1.6).clamp(0.0, 1.0));
            self.cam_pos = v3(p.x + (it.t * 0.8).sin() * 0.25 * (1.0 - u), gy + eye + (it.t * 1.7).sin() * 0.06 * (1.0 - u), z);
            self.cam_rot = v3(self.player.pitch, self.player.yaw, (it.t * 0.9).sin() * 0.04 * (1.0 - u));
            let wet = self.cam_pos.y < WATER_Y;
            if it.wet && !wet { self.audio.splash(); self.trauma = 0.25; }
            it.wet = wet;
            if wet && chance(0.5) { let c = self.cam_pos; self.smoke(c + v3(rr(-0.6, 0.6), -0.5, -rr(0.5, 2.5)), v3(rr(-0.2, 0.2), rr(0.6, 1.4), rr(-0.2, 0.2)), rr(0.03, 0.07), 1.2, 0xcfeff5); }
            if u < 0.3 { let m = Mat4::from_translation(self.cam_pos + v3(0.0, -0.42, -0.95)) * Mat4::from_rotation_y(PI); self.draws.push(Draw { mesh: self.ids.scooter, inst: Inst::new(m) }); }
        }
        self.fov = 75.0;
        if t >= 1.0 {
            self.sw_t = 0.45; self.player.vel = Vec3::ZERO;
            if !it.sdv { self.chute_t = 1.8; self.land_dip = -0.3; self.trauma = 0.35; self.audio.land(); }
        } else { self.intro = Some(it); }
    }

    // ───────────────────────── main update ─────────────────────────
    pub fn update(&mut self, rdt: f32, inp: &mut Input) {
        self.draws.clear();
        self.card.3 = (self.card.3 - rdt).max(0.0);
        self.announce.2 += rdt; self.hit_mark.0 += rdt;
        for d in self.dmg_dirs.iter_mut() { d.1 += rdt; } self.dmg_dirs.retain(|d| d.1 < 1.1);
        for f in self.feed.iter_mut() { f.1 += rdt; } self.feed.retain(|f| f.1 < 4.2);
        for f in self.floats.iter_mut() { f.t += rdt; } self.floats.retain(|f| f.t < f.life);
        for b in self.bubbles.iter_mut() { b.t += rdt; } self.bubbles.retain(|b| b.t < 2.0);
        for i in 0..self.bubbles.len() { if let Some(j) = self.bear_idx(self.bubbles[i].bear) { self.bubbles[i].pos = self.bears[j].pos; } }
        for l in self.lights_tmp.iter_mut() { l.color *= (-20.0 * rdt).exp(); } self.lights_tmp.retain(|l| l.color.max_element() > 0.02);
        for f in self.flashes.iter_mut() { f.t -= rdt; } self.flashes.retain(|f| f.t > 0.0);
        for b in self.beams.iter_mut() { b.t -= rdt; } self.beams.retain(|b| b.t > 0.0);
        for b in self.booms.iter_mut() { b.t += rdt / 0.38; } self.booms.retain(|b| b.t < 1.0);
        self.rush_fx = damp(self.rush_fx, if self.player.rush > 0.0 && self.state != State::Menu { 1.0 } else { 0.0 }, 8.0, rdt);
        match self.state {
            State::Playing => self.tick(rdt, inp),
            State::Menu => {
                self.menu_t += rdt * 0.1;
                self.cam_pos = v3(self.menu_t.sin() * 15.0 + 3.0, 5.2, self.menu_t.cos() * 15.0);
                let f = (v3(-2.5, 2.7, 0.0) - self.cam_pos).normalize();
                self.cam_rot = v3(f.y.asin(), (-f.x).atan2(-f.z), 0.0); self.fov = 55.0;
                let c = self.clock;
                for b in self.props.iter_mut() { Self::animate(b, rdt, false, 0.0, false, c); }
            }
            State::Dead => {
                self.clock += rdt; self.dead_t -= rdt;
                self.cam_pos.y = damp(self.cam_pos.y, self.player.pos.y + 0.45, 4.0, rdt); self.cam_rot.z = damp(self.cam_rot.z, 0.55, 3.0, rdt);
                self.idle_fx(rdt * 0.3); self.hurt_flash = (self.hurt_flash - rdt).max(0.0);
                if self.dead_t <= 0.0 { self.finish_run(false); }
            }
            State::Results => { self.idle_fx(rdt * 0.3); if let Some(r) = self.results.as_mut() { let before = r.age; r.age += rdt; if r.win { for i in 0..r.stars { let at = 0.5 + i as f32 * 0.38; if before < at && r.age >= at { self.audio.star(i); } } } } }
            State::Paused => {}
        }
    }
    fn idle_fx(&mut self, dt: f32) { self.update_dying(dt); self.update_debris(dt); self.update_particles(dt); }
    fn update_particles(&mut self, dt: f32) {
        for c in self.chunks.iter_mut() { c.life -= dt; c.vel.y -= 24.0 * dt; c.pos += c.vel * dt; if c.pos.y < c.size { c.pos.y = c.size; c.vel.y *= -0.45; c.vel.x *= 0.7; c.vel.z *= 0.7; } }
        self.chunks.retain(|c| c.life > 0.0);
        for s in self.sparks.iter_mut() { s.life -= dt; s.vel.y -= s.grav * dt; s.pos += s.vel * dt; if !s.smoke && s.pos.y < s.size { s.pos.y = s.size; s.vel = Vec3::ZERO; } }
        self.sparks.retain(|s| s.life > 0.0);
        for s in self.splats.iter_mut() { s.life -= dt; } self.splats.retain(|s| s.life > 0.0);
    }
    fn tick(&mut self, rdt: f32, inp: &mut Input) {
        use KeyCode::*;
        self.clock += rdt;
        if let Some(r) = self.run.as_mut() { r.t += rdt; }
        if inp.hit(Escape) || inp.hit(KeyP) { inp.pressed.clear(); self.pause(); return; }
        let k = 0.0022 * self.profile.sens * (1.0 - self.ads_t * if self.guns[self.gi].scope { 0.8 } else { 0.35 });
        self.player.yaw -= inp.dx * k; self.player.pitch = (self.player.pitch - inp.dy * k).clamp(-1.5, 1.5);
        if self.intro.is_some() {
            if inp.hit(Space) || inp.hit(Enter) || inp.click { self.skip_intro(); }
            self.update_intro(rdt); self.update_particles(rdt);
            return;
        }
        if self.chute_t > 0.0 { self.chute_t -= rdt; self.chute_pos += v3(0.0, -2.2, 3.0) * rdt; }
        let rushing = self.player.rush > 0.0;
        let (mut ws, mut ps) = (if rushing { 0.35 } else { 1.0 }, if rushing { 0.92 } else { 1.0 });
        if self.slow_t > 0.0 { self.slow_t -= rdt; ws = f32::min(ws, 0.2); ps = f32::min(ps, 0.4); }
        self.ts_w = damp(self.ts_w, ws, 14.0, rdt); self.ts_p = damp(self.ts_p, ps, 14.0, rdt);
        let mut hs = 1.0;
        if self.hitstop > 0.0 { self.hitstop -= rdt; hs = 0.03; }
        let (pdt, wdt) = (rdt * self.ts_p * hs, rdt * self.ts_w * hs);
        if inp.hit(KeyE) { self.take_loot(); }
        if inp.hit(KeyR) { self.start_reload(); }
        if inp.hit(KeyF) { self.start_rush(); }
        if inp.hit(KeyQ) { let l = self.last_gi; self.switch_to(l); }
        if inp.hit(KeyM) { self.audio.toggle_mute(); }
        for (i, kc) in [Digit1, Digit2, Digit3, Digit4].iter().enumerate() { if inp.hit(*kc) { self.switch_to(i); } }
        if inp.wheel != 0.0 {
            let owned: Vec<usize> = (0..self.guns.len()).filter(|&i| self.guns[i].owned).collect();
            if let Some(at) = owned.iter().position(|&i| i == self.sw_to.unwrap_or(self.gi)) { let n = owned.len(); self.switch_to(owned[(at + if inp.wheel > 0.0 { 1 } else { n - 1 }) % n]); }
        }
        self.update_player(pdt, rdt, inp); self.update_gun(pdt, rdt, inp); self.update_shots(pdt);
        self.sun_t -= rdt;
        if self.sun_t <= 0.0 { self.sun_t = 0.12; let e = self.cam_pos; self.sun_hit = if self.world.seg_blocked(e, e + self.pal.sun_dir * 140.0) < 0.0 { 1.0 } else { 0.0 }; }
        self.vm_sun = damp(self.vm_sun, self.sun_hit, 6.0, rdt);
        if self.state == State::Playing { self.update_bears(wdt); }
        if self.state == State::Playing { self.update_ebullets(wdt); }
        if self.state == State::Playing { let pb = std::mem::take(&mut self.pending_booms); for p in pb { self.explode(p, 3.6, 4.0, 0.0, true, false); } }
        self.update_dying(wdt); self.update_bits(pdt); self.update_debris(wdt); self.update_particles(wdt);
        if self.state != State::Playing { return; }
        if self.run.as_ref().is_some_and(|r| r.done) { let end = { let r = self.run.as_mut().unwrap(); r.end_t -= rdt; r.end_t <= 0.0 }; if end { self.finish_run(true); return; } } else { self.update_run(wdt); }
        if self.combo > 0 { self.combo_t -= rdt; if self.combo_t <= 0.0 { self.combo = 0; } }
        if rushing { self.player.rush -= rdt; self.player.sugar = (self.player.rush / self.m.rush_time * 100.0).max(0.0); if self.player.rush <= 0.0 { self.player.rush = 0.0; self.audio.rush_off(); } }
        self.bark_cd -= rdt; self.chain_t -= rdt; if self.chain_t <= 0.0 { self.chain_n = 0; }
        self.hurt_flash = (self.hurt_flash - rdt * 1.6).max(0.0); self.trauma = (self.trauma - rdt * 1.5).max(0.0);
        if self.player.hp < self.m.max_hp * 0.3 { self.heart_t -= rdt; if self.heart_t <= 0.0 { self.heart_t = 0.85; self.audio.heart(); } }
        if self.radio.1 > 0.0 { self.radio.1 -= rdt; } else if !self.radio_q.is_empty() && self.card.3 <= 0.6 { self.radio = (self.radio_q.remove(0), 5.0); self.audio.radio(); }
        let hp_p = (self.player.hp / self.m.max_hp).clamp(0.0, 1.0);
        self.hp_ghost = hp_p.max(damp(self.hp_ghost, hp_p, 2.5, rdt));
        self.audio.set_listener(self.player.pos.x, self.player.pos.z, self.player.yaw); self.audio.rate = lerp(1.0, 0.62, self.rush_fx);
        if rushing != self.muffled { self.muffled = rushing; self.audio.muffle(rushing); }
        if !self.run.as_ref().is_some_and(|r| r.done) { self.audio.music(if self.player.rush > 0.0 { 1.0 } else if !self.bears.is_empty() { (0.55 + self.bears.len() as f32 * 0.07).min(1.0) } else { 0.4 }); }
    }

    // ───────────────────────── scene assembly ─────────────────────────
    pub fn post(&self) -> crate::gfx::Post {
        let low = if self.state == State::Playing && self.player.hp < self.m.max_hp * 0.3 { 0.22 + (self.clock * 6.0).sin() * 0.08 } else { 0.0 };
        crate::gfx::Post {
            saturation: if self.state == State::Dead { 0.3 } else { 1.04 + self.rush_fx * 0.3 }, rush: self.rush_fx,
            hurt: (self.hurt_flash * 0.85).max(low).max(if self.state == State::Dead { 0.6 } else { 0.0 }),
            ca: 0.0008 + self.hurt_flash * 0.003 + self.rush_fx * 0.0016 + self.slide_k * 0.002 + self.trauma * 0.002, vignette: 0.34,
        }
    }
    fn draw_bear(&mut self, b: &Bear, glow: f32) {
        let pose = b.pose();
        self.gummies.push(bear::gummy(&pose, lin(b.color), b.flash / 0.12 * 0.6, b.sun, glow));
        let s = b.scale_now();
        self.spheres.push([v3(b.pos.x, b.pos.y + (b.gy + 0.85) * s, b.pos.z).extend(0.62 * s), (lin(b.color) * 0.85 + 0.05).extend(0.0)]);
        if b.head {
            self.spheres.push([v3(b.pos.x, b.pos.y + (b.gy + 1.6) * s, b.pos.z).extend(0.4 * s), (lin(b.color) * 0.85 + 0.05).extend(0.0)]);
            let hx = pose.head_xf();
            for g in b.t.gear { self.draws.push(Draw { mesh: self.ids.gear[g], inst: Inst::new(hx) }); }
        }
        if b.armed {
            let (gid, hand) = (&self.ids.gun[&b.t.weapon], pose.hand_xf(true) * Mat4::from_translation(guns::PAW_OFFSET));
            self.draws.push(Draw { mesh: gid.body, inst: Inst::new(hand) });
            if let (Some(w), true) = (gid.warhead, b.warhead) { self.draws.push(Draw { mesh: w, inst: Inst::new(hand) }); }
            if let Some(br) = gid.barrels { self.draws.push(Draw { mesh: br, inst: Inst::new(hand * Mat4::from_rotation_z(b.spin)) }); }
        }
        if b.t.shield { self.draws.push(Draw { mesh: self.ids.shield, inst: Inst::new(pose.hand_xf(false) * Mat4::from_translation(guns::SHIELD_OFFSET)) }); }
        if b.elite { let m = Mat4::from_translation(v3(b.pos.x, b.pos.y + (2.75 + (self.clock * 3.0).sin() * 0.1) * s, b.pos.z)) * Mat4::from_rotation_y(self.clock * 3.0) * Mat4::from_scale(Vec3::splat(s)); self.draws.push(Draw { mesh: self.ids.marker, inst: Inst::new(m) }); }
    }
    fn sprite(&mut self, pos: Vec3, size: f32, color: Vec3, alpha: f32, kind: f32, axis: Vec3) { self.sprites.push(Sprite { pos: pos.to_array(), size, color: [color.x, color.y, color.z, alpha], axis: axis.to_array(), kind }); }
    /// Turn the current state into draw lists for the renderer. Call after `update`.
    pub fn build_scene(&mut self) {
        self.vm_draws.clear(); self.gummies.clear(); self.sprites.clear(); self.lights.clear(); self.spheres.clear();
        self.draws.extend(self.extra.iter().map(|d| Draw { mesh: d.mesh, inst: d.inst }));
        let cam = self.cam_pos;
        // statues
        let statues = self.world.statues.clone();
        for (i, st) in statues.iter().enumerate() {
            let mut p = BearPose::idle(st.pos, st.yaw, st.scale);
            if st.pose == 1 { p.arm_r = Some((0.0, -1.45)); p.arm_l = Some((0.0, 1.45)); }
            if st.pose == 2 { p.tilt_x = -PI / 2.0; p.pos.y += (self.clock * 0.9 + i as f32).sin() * 0.08; }
            self.gummies.push(bear::gummy(&p, lin(st.color), 0.0, 1.0, self.pal.night * 0.08));
            if st.pose == 0 { self.spheres.push([(st.pos + Vec3::Y * 0.9 * st.scale).extend(0.62 * st.scale), (lin(st.color) * 0.85 + 0.05).extend(0.0)]); if st.crown { self.draws.push(Draw { mesh: self.ids.gear[&Gear::Crown], inst: Inst::new(p.head_xf()) }); } }
        }
        let list: Vec<Bear> = self.bears.iter().chain(self.dying.iter()).chain(self.props.iter()).copied().collect();
        for b in &list { self.draw_bear(b, 0.0); }
        // lasers
        for b in &list {
            if b.t.laser && b.fsm == Fsm::Aim && b.armed {
                let m = b.muzzle(self.ids.gun[&b.t.weapon].muzzle);
                let end = if b.locked { b.lock } else { self.player.pos + Vec3::Y * if b.t.hitscan { self.player.h * 0.62 } else { 0.2 } };
                let (col, r) = if b.locked { (v3(6.0, 5.4, 5.4), 0.035) } else { (v3(3.2, 0.15, 0.2), 0.018) };
                self.sprite(m, r, col, 0.7, 2.0, end - m);
            }
        }
        // debris
        let debris = self.debris.clone();
        for d in &debris {
            let xf = Mat4::from_translation(d.pos) * Mat4::from_euler(EulerRot::XYZ, d.rot.x, d.rot.y, d.rot.z) * Mat4::from_scale(Vec3::splat(d.s * d.shrink));
            match d.kind {
                DebrisKind::Gun(w) => { self.draws.push(Draw { mesh: self.ids.gun[&w].body, inst: Inst::new(xf) }); }
                DebrisKind::Shield => self.draws.push(Draw { mesh: self.ids.shield, inst: Inst::new(xf) }),
                DebrisKind::Arm(color, w) => {
                    let model = xf * Mat4::from_translation(v3(0.0, -0.6, 0.0));
                    let mut prims = [Vec4::ZERO; 32]; prims[0] = v3(0.0, 0.6, -0.18).extend(1.0); prims[1] = v3(0.0, 0.6, 0.18).extend(0.15);
                    self.gummies.push(GummyU { inv_model: model.inverse(), model, color: lin(color).extend(0.0), params: Vec4::new(1.0, 1.0, 0.0, d.s), eyes: [Vec4::new(0.0, -99.0, 0.0, 0.01); 3], prims });
                    if let Some(w) = w { self.draws.push(Draw { mesh: self.ids.gun[&w].body, inst: Inst::new(xf * Mat4::from_translation(v3(0.0, 0.1, 0.2))) }); }
                }
                DebrisKind::Head(color, gear) => {
                    let model = xf * Mat4::from_translation(v3(0.0, -0.6, 0.0));
                    let c = v3(0.0, 0.6, 0.0);
                    let mut prims = [Vec4::ZERO; 32];
                    for (k, (o, r)) in [(Vec3::ZERO, v3(0.42, 0.4, 0.4)), (v3(-0.3, 0.34, -0.02), v3(0.16, 0.16, 0.11)), (v3(0.3, 0.34, -0.02), v3(0.16, 0.16, 0.11)), (v3(0.0, -0.08, 0.32), v3(0.19, 0.15, 0.15))].iter().enumerate() { prims[2 * k] = (c + *o).extend(0.0); prims[2 * k + 1] = r.extend(0.0); }
                    let eyes = [(c + v3(-0.16, 0.1, 0.36)).extend(0.06), (c + v3(0.16, 0.1, 0.36)).extend(0.06), (c + v3(0.0, -0.03, 0.47)).extend(0.055)];
                    self.gummies.push(GummyU { inv_model: model.inverse(), model, color: lin(color).extend(0.0), params: Vec4::new(4.0, 1.0, 0.0, d.s), eyes, prims });
                    for g in gear { self.draws.push(Draw { mesh: self.ids.gear[g], inst: Inst::new(xf) }); }
                }
            }
            if d.loot.is_some() && (d.life > 4.0 || (d.life * 12.0).sin() > 0.0) { self.sprite(v3(d.pos.x, 0.1, d.pos.z), 0.06, v3(4.0, 3.2, 0.8), 0.45, 2.0, Vec3::Y * 2.6); } // a column of light so dropped gear is easy to spot
        }
        // projectiles
        let shots = self.shots.clone();
        for s in &shots {
            let dir = s.vel.normalize_or_zero();
            let face = Mat4::from_translation(s.pos) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Z, dir));
            let tint = |c: u32, e: f32| Inst { model: Mat4::IDENTITY, tint: lin(c).extend(1.0), misc: Vec4::new(e, 0.0, 0.0, 0.0) };
            match s.kind {
                0 => self.draws.push(Draw { mesh: self.ids.bean, inst: Inst { model: face * Mat4::from_scale(Vec3::splat(1.3)), ..tint(s.color, 1.6) } }),
                1 => self.draws.push(Draw { mesh: self.ids.sphere, inst: Inst { model: face * Mat4::from_scale(Vec3::splat(0.085)), ..tint(s.color, 1.6) } }),
                2 => { self.draws.push(Draw { mesh: self.ids.sphere, inst: Inst { model: face * Mat4::from_rotation_x(s.spin) * Mat4::from_scale(Vec3::splat(0.2)), ..tint(0xffffff, 0.8) } }); self.sprite(s.pos, 0.5, v3(2.5, 1.2, 2.8), 0.5, 0.0, Vec3::ZERO); }
                3 => self.sprite(s.pos - dir * 1.6, 0.035, v3(5.0, 3.6, 1.2), 1.0, 2.0, dir * 1.6),
                _ => self.draws.push(Draw { mesh: self.ids.rocket, inst: Inst::new(face) }),
            }
        }
        let eb = self.ebullets.clone();
        for e in &eb {
            let dir = e.vel.normalize_or_zero();
            if e.rocket { self.draws.push(Draw { mesh: self.ids.rocket, inst: Inst::new(Mat4::from_translation(e.pos) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Z, dir)) * Mat4::from_scale(Vec3::splat(e.s))) }); self.sprite(e.pos - dir * 0.6, 0.35, v3(6.0, 3.0, 0.8), 0.8, 0.0, Vec3::ZERO); }
            else { self.sprite(e.pos - dir * 1.2, 0.045, v3(5.0, 3.6, 1.2), 1.0, 2.0, dir * 1.2); }
        }
        let bits = self.bits.clone();
        for b in &bits { self.draws.push(Draw { mesh: self.ids.sphere, inst: Inst { model: Mat4::from_translation(b.pos) * Mat4::from_scale(Vec3::splat(0.14)), tint: lin(b.color).extend(1.0), misc: Vec4::new(1.5, 0.0, 0.0, 0.0) } }); }
        let chunks = self.chunks.clone();
        for c in &chunks { let sc = c.size * (c.life / (c.max * 0.4)).min(1.0); self.draws.push(Draw { mesh: self.ids.sphere, inst: Inst { model: Mat4::from_translation(c.pos) * Mat4::from_scale(Vec3::splat(sc)), tint: lin(c.color).extend(1.0), misc: Vec4::new(0.25, 0.0, 0.0, 0.0) } }); }
        for i in 0..self.sparks.len() { let s = self.sparks[i]; let k = (s.life / (s.max * 0.4)).min(1.0); if s.smoke { self.sprite(s.pos, s.size * (2.0 - k), s.color, 0.35 * k, 1.0, Vec3::ZERO); } else { self.sprite(s.pos, s.size * k * 2.2, s.color, 1.0, 0.0, Vec3::ZERO); } }
        for i in 0..self.beams.len() { let b = &self.beams[i]; let (a, d, r, c, al) = (b.a, b.b - b.a, b.r, b.color, b.t / b.max); self.sprite(a, r, c, al, 2.0, d); }
        for i in 0..self.booms.len() { let b = &self.booms[i]; let e = 1.0 - (1.0 - b.t).powi(3); let (p, r, c, al) = (b.pos, b.r * (0.4 + 0.9 * e), b.color, 1.0 - b.t); self.sprite(p, r, c, al, 0.0, Vec3::ZERO); self.sprite(p, r * 0.5, c * 2.0, al, 0.0, Vec3::ZERO); }
        for i in 0..self.flashes.len() { let f = &self.flashes[i]; let (p, sz) = (f.pos, f.size); self.sprite(p, sz * 2.4, v3(6.0, 4.0, 1.4), 1.0, 0.0, Vec3::ZERO); }
        // parachute
        if self.chute_t > 0.0 && self.run.is_some() { let k = (self.chute_t / 1.8).min(1.0); let tilt = if self.intro.is_some() { 0.0 } else { (1.0 - k) * 1.2 }; self.draws.push(Draw { mesh: self.ids.chute, inst: Inst::new(Mat4::from_translation(self.chute_pos) * Mat4::from_rotation_x(tilt) * Mat4::from_scale(Vec3::splat(0.2 + 0.8 * k))) }); }
        // goo on the ground
        let mut dk = Kit::new();
        let r = self.atlas.splat;
        for s in &self.splats {
            let c = [(s.color >> 16) as u8, (s.color >> 8) as u8, s.color as u8, ((s.life / 4.0).min(1.0) * 217.0) as u8];
            let mt = SurfMat { layer: 0, color: c, rough: 10, metal: 0, emissive: 0, tile: 1.0 };
            let (co, si, h) = (s.rot.cos() * s.size, s.rot.sin() * s.size, 0.03);
            let p = |x: f32, z: f32| v3(s.x + x * co - z * si, h, s.z + x * si + z * co);
            dk.quad4(mt, [p(-1.0, 1.0), p(1.0, 1.0), p(1.0, -1.0), p(-1.0, -1.0)], [Vec2::new(r.u0, r.v1), Vec2::new(r.u1, r.v1), Vec2::new(r.u1, r.v0), Vec2::new(r.u0, r.v0)]);
        }
        self.decals = dk.m;
        // lights: muzzle flashes and blasts, then the nearest street lamps after dark
        for l in &self.lights_tmp { self.lights.push([l.pos.extend(l.range), l.color.extend(0.0)]); }
        if self.pal.night > 0.25 {
            let mut lamps: Vec<Vec3> = self.world.lamps.clone();
            lamps.sort_by(|a, b| a.distance_squared(cam).total_cmp(&b.distance_squared(cam)));
            for l in lamps.iter().take(22) { self.lights.push([l.extend(24.0), (v3(1.0, 0.68, 0.36) * 6.5 * self.pal.night).extend(0.0)]); }
        }
        self.lights.truncate(32);
        self.spheres.sort_by(|a, b| a[0].truncate().distance_squared(cam).total_cmp(&b[0].truncate().distance_squared(cam)));
        self.spheres.truncate(48);
        // the weapon in your hands
        if self.state != State::Menu && self.intro.is_none() && !self.scoped && self.run.is_some() {
            let gi = self.gi.min(self.guns.len() - 1);
            let base = Mat4::from_translation(self.gun_pos) * Mat4::from_euler(EulerRot::XYZ, self.gun_rot.x, self.gun_rot.y, self.gun_rot.z) * Mat4::from_rotation_y(PI);
            let (body, pump, ball, barrels, dot_z, sight_y) = { let v = self.vm(gi); (v.body, v.pump, v.ball, v.barrels, v.dot_z, v.sight_y) };
            self.vm_draws.push(Draw { mesh: body, inst: Inst::new(base) });
            if let Some(pm) = pump { let z = -((1.0 - self.pump_t / 0.5).clamp(0.0, 1.0) * PI).sin() * 0.075 * if self.pump_t > 0.0 { 1.0 } else { 0.0 }; self.vm_draws.push(Draw { mesh: pm, inst: Inst::new(base * Mat4::from_translation(v3(0.0, 0.0, z))) }); }
            if let Some(bl) = ball { if self.guns[gi].ammo > 0 && self.reload_t <= 0.0 { self.vm_draws.push(Draw { mesh: bl, inst: Inst::new(base) }); } }
            if let Some(br) = barrels { self.vm_draws.push(Draw { mesh: br, inst: Inst::new(base * Mat4::from_rotation_z(self.barrel_spin)) }); }
            if let Some(dz) = dot_z { self.vm_draws.push(Draw { mesh: self.ids.reticle, inst: Inst::new(base * Mat4::from_translation(v3(self.dot_off.x, sight_y + self.dot_off.y, dz))) }); }
        }
    }
    /// Debug: a row of bears in every colour, for judging the jelly shading up close.
    pub fn debug_lineup(&mut self, at: Vec3) {
        self.props.clear();
        let kinds = [Kind::Grunt, Kind::Rifle, Kind::Shield, Kind::Rpg, Kind::Sniper, Kind::Knife, Kind::Grunt];
        for (i, k) in kinds.iter().enumerate() {
            let mut b = Bear::new(*k, self.next_id); self.next_id += 1;
            b.color = CANDY[i]; b.pos = at + v3((i as f32 - 3.0) * 1.9, 0.0, 0.0); b.yaw = 0.0;
            self.props.push(b);
        }
    }
    /// Debug: stand a first-person weapon model up in the world so it can be looked at from outside.
    pub fn inspect(&mut self, slot: usize, loot: Option<Wpn>, at: Mat4) {
        let v = match loot { Some(w) => &self.ids.loot_vm[&w], None => &self.ids.vm[slot.min(2)] };
        for m in [Some(v.body), v.pump, v.ball, v.barrels].into_iter().flatten() { self.extra.push(Draw { mesh: m, inst: Inst::new(at) }); }
    }
    pub fn loot_name(&self) -> Option<&'static str> {
        let d = self.debris.get(self.near_pick?)?;
        match d.loot? { Loot::Armor => Some("Riot shield plating  +50 armor"), Loot::Gun(w) => Some(loot_gun(w).name) }
    }
    pub fn boss_hp(&self) -> Option<f32> { let (mut h, mut m) = (0.0, 0.0); for b in &self.bears { if b.kind == Kind::Boss { h += b.hp.max(0.0); m += b.max_hp; } } (m > 0.0).then_some(h / m) }
}
