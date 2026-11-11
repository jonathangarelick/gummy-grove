//! Campaign, progression and armory data, plus the save file.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind { Grunt, Knife, Rifle, Shield, Rpg, Sniper, Boss }
impl Kind {
    pub fn plural(self) -> &'static str { match self { Kind::Grunt => "Pistol Bears", Kind::Knife => "Stabby Runts", Kind::Rifle => "Rifle Bears", Kind::Shield => "Shield Bears", Kind::Rpg => "Rocket Bears", Kind::Sniper => "Sniper Bears", Kind::Boss => "King Gummy" } }
    pub fn tip(self) -> &'static str {
        match self {
            Kind::Grunt => "They have pistols. Real ones. Keep moving and their shots will trail behind you.",
            Kind::Knife => "Stabby runts incoming. Too small to bother shooting. Press V to club them.",
            Kind::Rifle => "Rifle bears fire in bursts. Shoot the gun arm off, then press E to take the AK for yourself.",
            Kind::Shield => "Riot shields stop jellybeans cold. Aim over the top for the head, or get behind them.",
            Kind::Rpg => "Rocket bears! You can shoot a rocket out of the air. Their friends will not enjoy it.",
            Kind::Sniper => "Sniper lasers. When the beam turns white you have a heartbeat to move.",
            Kind::Boss => "That is King Gummy. He brought a minigun. Get behind a wall when it spins up.",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Profile {
    pub xp: u32, pub level: u32, pub candy: u32, pub perk_pts: u32,
    pub up: HashMap<String, u32>, pub perks: Vec<String>, pub done: HashMap<u32, u32>,
    pub best: u32, pub sens: f32, pub volume: f32, pub scale: f32,
}
impl Default for Profile {
    fn default() -> Self { Profile { xp: 0, level: 1, candy: 0, perk_pts: 0, up: HashMap::new(), perks: Vec::new(), done: HashMap::new(), best: 0, sens: 1.0, volume: 0.8, scale: 0.0 } }
}
fn save_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("GUMMY_SAVE") { return p.into(); } // tests and headless runs keep away from the real save
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join("Library/Application Support/GummyGrove/save.json")
}
impl Profile {
    pub fn load() -> Profile { std::fs::read_to_string(save_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default() }
    pub fn save(&self) {
        let p = save_path();
        if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
        if let Ok(s) = serde_json::to_string_pretty(self) { let _ = std::fs::write(p, s); }
    }
    pub fn reset(&mut self) { let keep = (self.sens, self.volume, self.scale); *self = Profile::default(); (self.sens, self.volume, self.scale) = keep; }
    pub fn up(&self, id: &str) -> u32 { self.up.get(id).copied().unwrap_or(0) }
    pub fn perk(&self, id: &str) -> bool { self.perks.iter().any(|p| p == id) }
    pub fn stars(&self, mission: u32) -> u32 { self.done.get(&mission).copied().unwrap_or(0) }
    /// Adds XP; returns how many levels were gained (each grants a perk point).
    pub fn add_xp(&mut self, n: u32) -> u32 {
        self.xp += n;
        let mut ups = 0;
        while self.xp >= xp_for(self.level) { self.xp -= xp_for(self.level); self.level += 1; self.perk_pts += 1; ups += 1; }
        ups
    }
    pub fn unlocked(&self, mission: u32) -> bool { mission == 1 || self.stars(mission - 1) > 0 }
    pub fn next_mission(&self) -> Option<&'static Mission> { MISSIONS.iter().find(|m| self.stars(m.id) == 0) }
}
pub const RANKS: [&str; 11] = ["Sweet Tooth", "Sugar Scout", "Bean Counter", "Gumshoe", "Sour Soldier", "Taffy Trooper", "Fudge Sergeant", "Caramel Captain", "Licorice Lieutenant", "Candy Commando", "Grove Legend"];
pub fn rank_name(level: u32) -> &'static str { RANKS[(level as usize - 1).min(RANKS.len() - 1)] }
pub fn xp_for(level: u32) -> u32 { (320.0 * (level as f32).powf(1.3)).round() as u32 }

pub const TIER_COST: [u32; 3] = [200, 450, 900];
pub struct Upgrade { pub id: &'static str, pub group: &'static str, pub name: &'static str, pub desc: &'static str, pub needs: u32 }
pub const UPGRADES: [Upgrade; 8] = [
    Upgrade { id: "sour", group: "Carbine", name: "Sour Rounds", desc: "+25% JB-4 carbine damage per tier.", needs: 0 },
    Upgrade { id: "hopper", group: "Carbine", name: "Extended Mags", desc: "+8 jellybeans per magazine per tier.", needs: 0 },
    Upgrade { id: "gumballs", group: "Shotgun", name: "Extra Gumballs", desc: "+2 pellets per shot per tier.", needs: 2 },
    Upgrade { id: "boom", group: "Launcher", name: "Bigger Boom", desc: "Larger, harder-hitting jawbreaker blasts.", needs: 4 },
    Upgrade { id: "vital", group: "Body", name: "Vitality", desc: "+25 max health per tier.", needs: 0 },
    Upgrade { id: "metab", group: "Body", name: "Sugar Metabolism", desc: "Sugar Rush fills 25% faster and lasts 1s longer per tier.", needs: 0 },
    Upgrade { id: "tooth", group: "Body", name: "Sweet Tooth", desc: "Gummy bits heal more and fly to you from further away.", needs: 0 },
    Upgrade { id: "hands", group: "Body", name: "Quick Hands", desc: "15% faster reloads per tier.", needs: 0 },
];
pub struct Perk { pub id: &'static str, pub cost: u32, pub name: &'static str, pub desc: &'static str }
pub const PERKS: [Perk; 6] = [
    Perk { id: "dash", cost: 1, name: "Triple Slide", desc: "Carry a third slide charge." },
    Perk { id: "head", cost: 1, name: "Headhunter", desc: "Headshots deal 2.5x instead of 2x." },
    Perk { id: "smile", cost: 1, name: "Disarming Smile", desc: "Disarmed bears drop extra bits and triple disarm score." },
    Perk { id: "jumper", cost: 2, name: "Rocket Jumper", desc: "Your own explosions launch you but never hurt you." },
    Perk { id: "chain", cost: 2, name: "Chain Reaction", desc: "Bears killed during Sugar Rush explode." },
    Perk { id: "wind", cost: 2, name: "Second Wind", desc: "Once per mission, cheat death and trigger a free Sugar Rush." },
];

/// Everything combat needs to know about the player's build.
#[derive(Clone, Copy)]
pub struct Mods {
    pub blaster_dmg: f32, pub blaster_clip: u32, pub pellets: u32, pub boom_r: f32, pub boom_dmg: f32, pub max_hp: f32,
    pub rush_gain: f32, pub rush_time: f32, pub bit_heal: f32, pub magnet: f32, pub reload: f32, pub dash_max: f32, pub head_mul: f32,
    pub smile: bool, pub no_self: bool, pub chain: bool, pub wind: bool, pub guns: [bool; 3],
}
pub fn mods(p: &Profile) -> Mods {
    let u = |id: &str| p.up(id) as f32;
    Mods {
        blaster_dmg: 1.0 + 0.25 * u("sour"), blaster_clip: 24 + 8 * p.up("hopper"), pellets: 9 + 2 * p.up("gumballs"), boom_r: 5.5 + 0.8 * u("boom"), boom_dmg: 9.0 + 2.0 * u("boom"),
        max_hp: 100.0 + 25.0 * u("vital"), rush_gain: 1.0 + 0.25 * u("metab"), rush_time: 6.5 + u("metab"), bit_heal: 2.0 + u("tooth"), magnet: 8.0 + 3.0 * u("tooth"), reload: 1.0 - 0.15 * u("hands"),
        dash_max: if p.perk("dash") { 3.0 } else { 2.0 }, head_mul: if p.perk("head") { 2.5 } else { 2.0 },
        smile: p.perk("smile"), no_self: p.perk("jumper"), chain: p.perk("chain"), wind: p.perk("wind"), guns: [true, p.stars(2) > 0, p.stars(4) > 0],
    }
}

pub type Mix = &'static [(Kind, u32)];
pub enum Obj {
    Clear(&'static [Mix]),
    Survive { time: f32, cap: usize, mix: Mix },
    Hunt { target: Kind, count: u32, label: &'static str, cap: usize, mix: Mix },
    Boss(&'static [Mix]),
}
#[derive(Clone, Copy, PartialEq)]
pub enum Insert { Para, Sdv }
pub struct Mission {
    pub id: u32, pub insert: Insert, pub title: &'static str, pub palette: &'static str, pub seed: u32, pub layout: &'static [(&'static str, u32)],
    pub brief: &'static str, pub obj: Obj, pub threats: &'static [Kind], pub heads: u32, pub candy: u32, pub xp: u32, pub unlock: Option<&'static str>, pub radio: &'static [&'static str],
}
impl Mission {
    pub fn obj_name(&self) -> &'static str { match self.obj { Obj::Clear(_) => "Eliminate", Obj::Survive { .. } => "Survive", Obj::Hunt { .. } => "Hunt", Obj::Boss(_) => "Boss" } }
    pub fn obj_line(&self) -> String {
        match &self.obj {
            Obj::Survive { time, .. } => format!("Survive for {} seconds", *time as u32),
            Obj::Hunt { count, label, .. } => format!("Eliminate {count} {label}"),
            Obj::Boss(_) => "Defeat the King".into(),
            Obj::Clear(w) => format!("Eliminate all hostiles · {} waves", w.len()),
        }
    }
    pub fn obj_text(&self) -> String {
        match &self.obj {
            Obj::Survive { time, .. } => format!("Survive for {} seconds until extraction.", *time as u32),
            Obj::Hunt { count, label, .. } => format!("Eliminate {count} {label}, marked with a gold diamond."),
            Obj::Boss(w) => format!("Fight through {} waves and defeat the King.", w.len()),
            Obj::Clear(w) => format!("Eliminate all hostiles across {} waves.", w.len()),
        }
    }
}
use Kind::*;
pub const MISSIONS: [Mission; 8] = [
    Mission { id: 1, insert: Insert::Para, title: "Operation Sweet Tooth", palette: "morning", seed: 3, layout: &[],
        brief: "Agent, the Gummy Armed Forces have seized the Grove. Intel confirms they are carrying pistols. Real ones. Supply got you a proper carbine, but the only ammunition left in the Grove is jellybeans. Clear the square and try not to get eaten.",
        obj: Obj::Clear(&[&[(Grunt, 4)], &[(Grunt, 6)]]), threats: &[Grunt], heads: 3, candy: 150, xp: 220, unlock: None,
        radio: &["Boots down, Agent. Shift to sprint, C to slide, right-click to aim.", "Eat the gummy bits they drop. They heal you and fill your Sugar Rush."] },
    Mission { id: 2, insert: Insert::Sdv, title: "Knife Party", palette: "morning", seed: 8, layout: &[("walls", 5), ("crates", 8)],
        brief: "The runts have found the cutlery drawer. They are small, fast and extremely stabby. Hold the square. R&D says a new toy is waiting for you if you make it back.",
        obj: Obj::Clear(&[&[(Grunt, 3), (Knife, 4)], &[(Grunt, 4), (Knife, 6)], &[(Grunt, 5), (Knife, 7)]]), threats: &[Grunt, Knife], heads: 5, candy: 220, xp: 320, unlock: Some("GB-12 Gumball Shotgun"),
        radio: &["Runts in the alleys. Jump at a ledge and you will climb it. The rooftops are high ground."] },
    Mission { id: 3, insert: Insert::Para, title: "Automatic Sweetness", palette: "sunset", seed: 14, layout: &[("walls", 9), ("drops", 5)],
        brief: "Rifle squads are sweeping the Grove at dusk. Extraction is ninety seconds out. You do not need to win, Agent. You need to not lose until the clock runs out.",
        obj: Obj::Survive { time: 90.0, cap: 8, mix: &[(Grunt, 2), (Knife, 2), (Rifle, 3)] }, threats: &[Rifle, Knife, Grunt], heads: 8, candy: 300, xp: 420, unlock: None,
        radio: &["Ninety seconds, Agent. When the Sugar Rush meter is full, hit F and everything slows down but you."] },
    Mission { id: 4, insert: Insert::Sdv, title: "Riot Act", palette: "overcast", seed: 21, layout: &[("walls", 8), ("crates", 9), ("pops", 6)],
        brief: "S.W.E.E.T. teams are locking down the Grove behind riot shields. Three captains run the operation. Remove the captains and the line folds. Jellybeans will not go through those shields.",
        obj: Obj::Hunt { target: Shield, count: 3, label: "Captains", cap: 6, mix: &[(Grunt, 2), (Knife, 2), (Rifle, 2)] }, threats: &[Shield, Rifle, Knife], heads: 8, candy: 380, xp: 520, unlock: Some("JW-40 Jawbreaker Launcher"),
        radio: &["Captains are marked with a gold diamond. Everyone else is just in the way."] },
    Mission { id: 5, insert: Insert::Sdv, title: "Long Live the King", palette: "night", seed: 34, layout: &[("walls", 9), ("statues", 3)],
        brief: "We found him. King Gummy is holding court in the square tonight, and he has a minigun the size of a canoe. End the monarchy. Bring explosives.",
        obj: Obj::Boss(&[&[(Rifle, 3), (Knife, 4)], &[(Boss, 1), (Grunt, 3), (Knife, 3)]]), threats: &[Boss, Rifle, Knife], heads: 6, candy: 500, xp: 700, unlock: None,
        radio: &["Night op. Their tracers give them away. So do yours."] },
    Mission { id: 6, insert: Insert::Para, title: "Fire in the Hole", palette: "sunset", seed: 41, layout: &[("walls", 10), ("mallows", 7)],
        brief: "With the King melted, the generals have escalated. Rocket teams are moving in. On the bright side, rockets do not care who they hit.",
        obj: Obj::Clear(&[&[(Rpg, 2), (Grunt, 4), (Knife, 3)], &[(Rpg, 3), (Rifle, 3), (Knife, 4)], &[(Rpg, 4), (Rifle, 4), (Shield, 2)]]), threats: &[Rpg, Rifle, Shield], heads: 10, candy: 560, xp: 800, unlock: None,
        radio: &["Rockets are slow. Shoot them down, or let them land on their own side."] },
    Mission { id: 7, insert: Insert::Para, title: "Overwatch", palette: "overcast", seed: 55, layout: &[("walls", 11), ("crates", 8), ("drops", 8)],
        brief: "Four marksmen have the Grove zeroed from the treeline. Nothing moves without their say. Break line of sight, close the distance and take them off the board.",
        obj: Obj::Hunt { target: Sniper, count: 4, label: "Marksmen", cap: 7, mix: &[(Rifle, 3), (Knife, 3), (Shield, 1)] }, threats: &[Sniper, Rifle, Shield], heads: 10, candy: 640, xp: 900, unlock: None,
        radio: &["Stay behind brick. A slide makes you untouchable for a moment, use it when the laser locks."] },
    Mission { id: 8, insert: Insert::Sdv, title: "The Last Bite", palette: "night", seed: 77, layout: &[("walls", 10), ("crates", 8), ("statues", 3)],
        brief: "This is everything they have left. Rifles, rockets, shields, snipers, and the royal twins, heirs to the gummy throne. Finish it, Agent. The Grove is counting on you.",
        obj: Obj::Boss(&[&[(Rifle, 4), (Shield, 2), (Rpg, 2), (Knife, 4)], &[(Sniper, 2), (Rpg, 2), (Rifle, 4), (Knife, 4)], &[(Boss, 2), (Rifle, 3), (Knife, 4)]]), threats: &[Boss, Sniper, Rpg, Shield], heads: 14, candy: 1000, xp: 1400, unlock: None,
        radio: &["All of them, Agent. Make it count."] },
];

/// Arcade: scripted opening waves, then a growing random budget with a king every fifth wave.
pub fn arcade_wave(n: u32, rnd: &mut dyn FnMut() -> f32) -> (&'static str, Vec<(Kind, u32)>) {
    match n {
        1 => ("First Contact", vec![(Grunt, 5)]),
        2 => ("Knife Party", vec![(Grunt, 4), (Knife, 5)]),
        3 => ("Automatic Sweetness", vec![(Grunt, 3), (Knife, 3), (Rifle, 4)]),
        4 => ("Riot Control", vec![(Grunt, 3), (Rifle, 3), (Shield, 3), (Knife, 3)]),
        5 => ("King Gummy", vec![(Boss, 1), (Grunt, 3), (Knife, 4)]),
        6 => ("Fire in the Hole", vec![(Rpg, 3), (Rifle, 4), (Knife, 4), (Grunt, 3)]),
        7 => ("Overwatch", vec![(Sniper, 3), (Rifle, 4), (Shield, 2), (Knife, 4)]),
        _ => {
            const NAMES: [&str; 6] = ["Sugar Crash", "Sticky Situation", "Gelatin Offensive", "Total Recoil", "Chewy Reckoning", "No Sweet Surrender"];
            let cost = [(Grunt, 1.0), (Knife, 1.0), (Rifle, 2.0), (Shield, 3.0), (Rpg, 3.0), (Sniper, 3.0)];
            let mut mix: Vec<(Kind, u32)> = Vec::new();
            let mut budget = 10.0 + n as f32 * 3.0;
            if n % 5 == 0 { mix.push((Boss, if n >= 15 { 2 } else { 1 })); budget *= 0.6; }
            while budget > 0.0 {
                let (k, c) = cost[((rnd() * cost.len() as f32) as usize).min(cost.len() - 1)];
                match mix.iter_mut().find(|m| m.0 == k) { Some(m) => m.1 += 1, None => mix.push((k, 1)) }
                budget -= c;
            }
            (if n % 5 == 0 { "The King Returns" } else { NAMES[n as usize % NAMES.len()] }, mix)
        }
    }
}
