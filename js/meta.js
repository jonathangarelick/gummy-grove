// Campaign, progression and armory data. Everything here persists in localStorage via core.settings.
import { settings, saveSettings, C } from './core.js';

const fresh = () => ({ xp: 0, level: 1, candy: 0, perkPts: 0, up: {}, perks: {}, done: {} });
settings.profile = Object.assign(fresh(), settings.profile || {});
export const profile = settings.profile;
export const save = saveSettings;
export function resetProfile() { for (const k of Object.keys(profile)) delete profile[k]; Object.assign(profile, fresh()); settings.best = 0; save(); }

export const RANKS = ['Sweet Tooth', 'Sugar Scout', 'Bean Counter', 'Gumshoe', 'Sour Soldier', 'Taffy Trooper', 'Fudge Sergeant', 'Caramel Captain', 'Licorice Lieutenant', 'Candy Commando', 'Grove Legend'];
export const rankName = lvl => RANKS[Math.min(lvl - 1, RANKS.length - 1)];
export const xpFor = lvl => Math.round(320 * Math.pow(lvl, 1.3)); // XP needed to leave this level
export function addXp(n) {
  profile.xp += n; let ups = 0;
  while (profile.xp >= xpFor(profile.level)) { profile.xp -= xpFor(profile.level); profile.level++; profile.perkPts++; ups++; }
  return ups;
}

export const TIER_COST = [200, 450, 900];
export const UPGRADES = [
  { id: 'sour', gun: 'Carbine', name: 'Sour Rounds', desc: '+25% JB-4 carbine damage per tier.' },
  { id: 'hopper', gun: 'Carbine', name: 'Extended Mags', desc: '+8 jellybeans per magazine per tier.' },
  { id: 'gumballs', gun: 'Shotgun', name: 'Extra Gumballs', desc: '+2 pellets per shot per tier.', needs: 2 },
  { id: 'boom', gun: 'Launcher', name: 'Bigger Boom', desc: 'Larger, harder-hitting jawbreaker blasts.', needs: 4 },
  { id: 'vital', gun: 'Body', name: 'Vitality', desc: '+25 max health per tier.' },
  { id: 'metab', gun: 'Body', name: 'Sugar Metabolism', desc: 'Sugar Rush fills 25% faster and lasts 1s longer per tier.' },
  { id: 'tooth', gun: 'Body', name: 'Sweet Tooth', desc: 'Gummy bits heal more and fly to you from further away.' },
  { id: 'hands', gun: 'Body', name: 'Quick Hands', desc: '15% faster reloads per tier.' },
];
export const PERKS = [
  { id: 'dash', cost: 1, name: 'Triple Dash', desc: 'Carry a third dash charge.' },
  { id: 'head', cost: 1, name: 'Headhunter', desc: 'Headshots deal 2.5× instead of 2×.' },
  { id: 'smile', cost: 1, name: 'Disarming Smile', desc: 'Disarmed bears drop extra bits and triple disarm score.' },
  { id: 'jumper', cost: 2, name: 'Rocket Jumper', desc: 'Your own explosions launch you but never hurt you.' },
  { id: 'chain', cost: 2, name: 'Chain Reaction', desc: 'Bears killed during Sugar Rush explode.' },
  { id: 'wind', cost: 2, name: 'Second Wind', desc: 'Once per mission, cheat death and trigger a free Sugar Rush.' },
];

// Everything the combat code needs to know about the player's build.
export function mods() {
  const u = id => profile.up[id] || 0, p = id => !!profile.perks[id];
  return {
    blasterDmg: 1 + 0.25 * u('sour'), blasterClip: 24 + 8 * u('hopper'), pellets: 9 + 2 * u('gumballs'),
    boomR: 5.5 + 0.8 * u('boom'), boomDmg: 9 + 2 * u('boom'), maxHp: 100 + 25 * u('vital'),
    rushGain: 1 + 0.25 * u('metab'), rushTime: 6.5 + u('metab'), bitHeal: 2 + u('tooth'), magnet: 8 + 3 * u('tooth'), reload: 1 - 0.15 * u('hands'),
    dashMax: p('dash') ? 3 : 2, headMul: p('head') ? 2.5 : 2, smile: p('smile'), noSelf: p('jumper'), chain: p('chain'), wind: p('wind'),
    guns: [true, !!profile.done[2], !!profile.done[4]],
  };
}

export const BEAR_NAMES = { grunt: 'Pistol Bears', knife: 'Stabby Runts', rifle: 'Rifle Bears', shield: 'Shield Bears', rpg: 'Rocket Bears', sniper: 'Sniper Bears', boss: 'King Gummy' };
export const TIPS = {
  grunt: 'They have pistols. Real ones. Keep moving and their shots will trail behind you.',
  knife: 'Stabby runts incoming. They are fast, so jump or dash clear. Shift to dash.',
  rifle: 'Rifle bears fire in bursts. Shoot the gun arm clean off and they have to bite instead.',
  shield: 'Riot shields stop jellybeans cold. Aim over the top for the head, or get behind them.',
  rpg: 'Rocket bears! You can shoot a rocket out of the air. Their friends will not enjoy it.',
  sniper: 'Sniper lasers. When the beam turns white you have a heartbeat to move.',
  boss: 'That is King Gummy. He brought a minigun. Stay behind chocolate when it spins up.',
};
export const OBJ_NAMES = { clear: 'Eliminate', survive: 'Survive', hunt: 'Hunt', boss: 'Boss' };

export const MISSIONS = [
  { id: 1, title: 'Operation Sweet Tooth', palette: 'morning', seed: 3, layout: {},
    brief: 'Agent, the Gummy Armed Forces have seized the Grove. Intel confirms they are carrying pistols. Real ones. Supply got you a proper carbine, but the only ammunition left in the Grove is jellybeans. Clear the landing zone and try not to get eaten.',
    obj: { type: 'clear', waves: [{ grunt: 4 }, { grunt: 6 }] }, threats: ['grunt'], bonus: { heads: 3 }, reward: { candy: 150, xp: 220 },
    radio: ['Agent, you are on the ground. WASD to move, click to fire, right-click to aim.', 'Eat the gummy bits they drop. They heal you and fill your Sugar Rush.'] },
  { id: 2, title: 'Knife Party', palette: 'morning', seed: 8, layout: { walls: 5, crates: 8 },
    brief: 'The runts have found the cutlery drawer. They are small, fast and extremely stabby. Hold the picnic grounds. R&D says a new toy is waiting for you if you make it back.',
    obj: { type: 'clear', waves: [{ grunt: 3, knife: 4 }, { grunt: 4, knife: 6 }, { grunt: 5, knife: 7 }] }, threats: ['grunt', 'knife'], bonus: { heads: 5 }, reward: { candy: 220, xp: 320, unlock: 'GB-12 Gumball Shotgun' },
    radio: ['Runts on the perimeter. Space to jump, twice to double jump. The cake is high ground.'] },
  { id: 3, title: 'Automatic Sweetness', palette: 'sunset', seed: 14, layout: { walls: 9, drops: 5 },
    brief: 'Rifle squads are sweeping the Grove at dusk. Extraction is ninety seconds out. You do not need to win, Agent. You need to not lose until the clock runs out.',
    obj: { type: 'survive', time: 90, cap: 8, mix: { grunt: 2, knife: 2, rifle: 3 } }, threats: ['rifle', 'knife', 'grunt'], bonus: { heads: 8 }, reward: { candy: 300, xp: 420 },
    radio: ['Ninety seconds, Agent. When the Sugar Rush meter is full, hit F and everything slows down but you.'] },
  { id: 4, title: 'Riot Act', palette: 'overcast', seed: 21, layout: { walls: 8, crates: 9, pops: 6 },
    brief: 'S.W.E.E.T. teams are locking down the Grove behind riot shields. Three captains run the operation. Remove the captains and the line folds. Jellybeans will not go through those shields.',
    obj: { type: 'hunt', target: 'shield', count: 3, label: 'Captains', cap: 6, mix: { grunt: 2, knife: 2, rifle: 2 } }, threats: ['shield', 'rifle', 'knife'], bonus: { heads: 8 }, reward: { candy: 380, xp: 520, unlock: 'JW-40 Jawbreaker Launcher' },
    radio: ['Captains are marked with a gold diamond. Everyone else is just in the way.'] },
  { id: 5, title: 'Long Live the King', palette: 'night', seed: 34, layout: { walls: 9, statues: 3 },
    brief: 'We found him. King Gummy is holding court at the cake tonight, and he has a minigun the size of a canoe. End the monarchy. Bring explosives.',
    obj: { type: 'boss', waves: [{ rifle: 3, knife: 4 }, { boss: 1, grunt: 3, knife: 3 }] }, threats: ['boss', 'rifle', 'knife'], bonus: { heads: 6 }, reward: { candy: 500, xp: 700 },
    radio: ['Night op. Their tracers give them away. So do yours.'] },
  { id: 6, title: 'Fire in the Hole', palette: 'sunset', seed: 41, layout: { walls: 10, mallows: 7 },
    brief: 'With the King melted, the generals have escalated. Rocket teams are moving in. On the bright side, rockets do not care who they hit.',
    obj: { type: 'clear', waves: [{ rpg: 2, grunt: 4, knife: 3 }, { rpg: 3, rifle: 3, knife: 4 }, { rpg: 4, rifle: 4, shield: 2 }] }, threats: ['rpg', 'rifle', 'shield'], bonus: { heads: 10 }, reward: { candy: 560, xp: 800 },
    radio: ['Rockets are slow. Shoot them down, or let them land on their own side.'] },
  { id: 7, title: 'Overwatch', palette: 'overcast', seed: 55, layout: { walls: 11, crates: 8, drops: 8 },
    brief: 'Four marksmen have the Grove zeroed from the treeline. Nothing moves without their say. Break line of sight, close the distance and take them off the board.',
    obj: { type: 'hunt', target: 'sniper', count: 4, label: 'Marksmen', cap: 7, mix: { rifle: 3, knife: 3, shield: 1 } }, threats: ['sniper', 'rifle', 'shield'], bonus: { heads: 10 }, reward: { candy: 640, xp: 900 },
    radio: ['Stay behind chocolate. A dash makes you untouchable for a moment, use it when the laser locks.'] },
  { id: 8, title: 'The Last Bite', palette: 'night', seed: 77, layout: { walls: 10, crates: 8, statues: 3 },
    brief: 'This is everything they have left. Rifles, rockets, shields, snipers, and the royal twins, heirs to the gummy throne. Finish it, Agent. The Grove is counting on you.',
    obj: { type: 'boss', waves: [{ rifle: 4, shield: 2, rpg: 2, knife: 4 }, { sniper: 2, rpg: 2, rifle: 4, knife: 4 }, { boss: 2, rifle: 3, knife: 4 }] }, threats: ['boss', 'sniper', 'rpg', 'shield'], bonus: { heads: 14 }, reward: { candy: 1000, xp: 1400 },
    radio: ['All of them, Agent. Make it count.'] },
];
export const missionUnlocked = id => id === 1 || !!profile.done[id - 1];
export const nextMission = () => MISSIONS.find(m => !profile.done[m.id]) || null;

// Arcade: scripted intro waves, then a growing random budget with a king every fifth wave.
const ARCADE = [null,
  { title: 'First Contact', mix: { grunt: 5 } }, { title: 'Knife Party', mix: { grunt: 4, knife: 5 } }, { title: 'Automatic Sweetness', mix: { grunt: 3, knife: 3, rifle: 4 } },
  { title: 'Riot Control', mix: { grunt: 3, rifle: 3, shield: 3, knife: 3 } }, { title: 'King Gummy', mix: { boss: 1, grunt: 3, knife: 4 } },
  { title: 'Fire in the Hole', mix: { rpg: 3, rifle: 4, knife: 4, grunt: 3 } }, { title: 'Overwatch', mix: { sniper: 3, rifle: 4, shield: 2, knife: 4 } },
];
const NAMES = ['Sugar Crash', 'Sticky Situation', 'Gelatin Offensive', 'Total Recoil', 'Chewy Reckoning', 'No Sweet Surrender'];
export function arcadeWave(n) {
  if (ARCADE[n]) return ARCADE[n];
  const cost = { grunt: 1, knife: 1, rifle: 2, shield: 3, rpg: 3, sniper: 3 }, types = Object.keys(cost), mix = {};
  let budget = 10 + n * 3;
  if (n % 5 === 0) { mix.boss = n >= 15 ? 2 : 1; budget *= 0.6; }
  while (budget > 0) { const t = types[(Math.random() * types.length) | 0]; mix[t] = (mix[t] || 0) + 1; budget -= cost[t]; }
  return { title: n % 5 === 0 ? 'The King Returns' : NAMES[n % NAMES.length], mix };
}
export { C };
