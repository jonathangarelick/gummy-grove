// Menu screens: main menu, campaign map, briefing, armory, settings, pause and results.
import { settings, saveSettings } from './core.js';
import { audio } from './audio.js';
import { profile, save, resetProfile, MISSIONS, UPGRADES, PERKS, TIER_COST, BEAR_NAMES, OBJ_NAMES, missionUnlocked, nextMission, rankName, xpFor } from './meta.js';

const $ = id => document.getElementById(id);
const overlay = $('overlay'), panel = $('panel');
let H = {}, cur = null, data = null, stack = [], resetArmed = false;

const stars = n => `<span class="stars">${[0, 1, 2].map(i => `<i class="${i < n ? 'on' : ''}">★</i>`).join('')}</span>`;
const chip = () => `<div class="chip"><b>Lv ${profile.level}</b><span>${rankName(profile.level)}</span><span class="xp"><i style="width:${Math.round((profile.xp / xpFor(profile.level)) * 100)}%"></i></span><em class="cz">${profile.candy.toLocaleString()}</em>${profile.perkPts ? `<em class="pp">${profile.perkPts} perk pt${profile.perkPts > 1 ? 's' : ''}</em>` : ''}</div>`;
const head = (title, extra = '') => `<header><button class="back" data-act="back" aria-label="Back">←</button><h2>${title}</h2>${extra}</header>`;
function objText(m) {
  const o = m.obj;
  return o.type === 'survive' ? `Survive for ${o.time} seconds until extraction.` : o.type === 'hunt' ? `Eliminate ${o.count} ${o.label}, marked with a gold diamond.` : o.type === 'boss' ? `Fight through ${o.waves.length} waves and defeat the King.` : `Eliminate all hostiles across ${o.waves.length} waves.`;
}

const screens = {
  menu() {
    const nm = nextMission(), started = Object.keys(profile.done).length > 0;
    return `<div class="col menu"><div class="logo"><span>Gummy</span><span>Grove</span></div><p class="tag">They have guns. You have jellybeans.</p>${chip()}
      <button class="primary" data-act="${nm ? 'brief' : 'go'}" data-arg="${nm ? nm.id : 'map'}">${!started ? 'New Campaign' : nm ? `Continue <small>Mission ${nm.id} · ${nm.title}</small>` : 'Campaign Complete <small>replay missions</small>'}</button>
      <button data-act="go" data-arg="map">Missions</button>
      <button data-act="play" data-arg="0">Arcade <small>${settings.best ? 'best ' + settings.best.toLocaleString() : 'endless waves'}</small></button>
      <button data-act="go" data-arg="armory">Armory${profile.perkPts ? ' <small class="hot">perk point ready</small>' : ''}</button>
      <button data-act="go" data-arg="settings">Settings</button></div>`;
  },
  map() {
    return `<div class="wide">${head('Campaign', chip())}<div class="grid missions">${MISSIONS.map(m => {
      const open = missionUnlocked(m.id);
      return `<button class="mcard pal-${m.palette} ${open ? '' : 'locked'}" data-act="brief" data-arg="${m.id}" ${open ? '' : 'disabled'}><span class="num">${String(m.id).padStart(2, '0')}</span><b>${open ? m.title : 'Locked'}</b><small>${OBJ_NAMES[m.obj.type]} · ${m.palette}</small>${stars(profile.done[m.id] || 0)}</button>`;
    }).join('')}</div></div>`;
  },
  brief() {
    const m = data;
    return `<div class="wide brief pal-${m.palette}">${head(`<small>Mission ${m.id} · ${m.palette}</small>${m.title}`, stars(profile.done[m.id] || 0))}
      <p class="story">${m.brief}</p>
      <div class="cols"><div><h3>Objective</h3><p>${objText(m)}</p><h3>Hostiles</h3><p>${m.threats.map(t => BEAR_NAMES[t]).join(' · ')}</p></div>
      <div><h3>Stars</h3><ul class="checks"><li>Complete the mission</li><li>Finish with 50%+ health</li><li>${m.bonus.heads} headshots</li></ul>
      <h3>Reward</h3><p><em class="cz">${m.reward.candy}</em> &nbsp;${m.reward.xp} XP${m.reward.unlock && !profile.done[m.id] ? ` &nbsp;<b class="unlock">Unlocks ${m.reward.unlock}</b>` : ''}</p></div></div>
      <footer><button data-act="go" data-arg="armory">Armory</button><button class="primary" data-act="play" data-arg="${m.id}">Deploy</button></footer></div>`;
  },
  armory() {
    return `<div class="wide">${head('Armory', chip())}<h3>Upgrades</h3><div class="grid ups">${UPGRADES.map(u => {
      const t = profile.up[u.id] || 0, locked = u.needs && !profile.done[u.needs], max = t >= 3, cost = TIER_COST[t];
      return `<div class="ucard ${locked ? 'locked' : ''}"><small>${u.gun}</small><b>${u.name}</b><p>${locked ? `Unlocks after Mission ${u.needs}.` : u.desc}</p><span class="pips">${[0, 1, 2].map(i => `<i class="${i < t ? 'on' : ''}"></i>`).join('')}</span>
        <button data-act="buy" data-arg="${u.id}" ${locked || max || profile.candy < cost ? 'disabled' : ''}>${max ? 'Maxed' : `<em class="cz">${cost}</em>`}</button></div>`;
    }).join('')}</div>
    <h3>Perks <small>${profile.perkPts} point${profile.perkPts === 1 ? '' : 's'} to spend · earn one every level</small></h3><div class="grid ups">${PERKS.map(p => {
      const has = profile.perks[p.id];
      return `<div class="ucard perk ${has ? 'owned' : ''}"><b>${p.name}</b><p>${p.desc}</p><button data-act="perk" data-arg="${p.id}" ${has || profile.perkPts < p.cost ? 'disabled' : ''}>${has ? 'Equipped' : `${p.cost} pt${p.cost > 1 ? 's' : ''}`}</button></div>`;
    }).join('')}</div></div>`;
  },
  settings() {
    const k = (a, b) => `<div><kbd>${a}</kbd>${b}</div>`;
    return `<div class="wide narrow">${head('Settings')}
      <label>Mouse sensitivity<input type="range" id="sens" min="0.3" max="2.5" step="0.05" value="${settings.sens}"></label>
      <label>Volume<input type="range" id="vol" min="0" max="1" step="0.05" value="${settings.volume}"></label>
      <div class="row"><button data-act="gfx">Graphics: ${settings.quality === 'high' ? 'High' : 'Low'} <small>reloads the page</small></button><button class="danger" data-act="reset">${resetArmed ? 'Click again to wipe everything' : 'Reset progress'}</button></div>
      <h3>Controls</h3><div class="keys">${k('WASD', 'move')}${k('Mouse', 'aim')}${k('Click', 'fire')}${k('Right click', 'aim down sights')}${k('Space', 'jump · double jump')}${k('Shift', 'dash')}${k('1 2 3', 'weapons')}${k('Q', 'last weapon')}${k('R', 'reload')}${k('F', 'Sugar Rush')}${k('Esc', 'pause')}${k('M', 'mute')}</div></div>`;
  },
  pause() {
    return `<div class="col"><h2 class="big">Paused</h2><p class="tag">The bears are politely waiting.</p><button class="primary" data-act="resume">Resume</button><button data-act="restart">Restart</button><button data-act="go" data-arg="settings">Settings</button><button data-act="quit">Abandon to menu</button></div>`;
  },
  results() {
    const r = data, m = r.m, acc = r.stats.shots ? Math.round((r.stats.hits / r.stats.shots) * 100) : 0, nm = nextMission();
    const st = (v, l) => `<div><b>${v}</b>${l}</div>`;
    let btns;
    if (r.win) btns = `<button data-act="quit">Menu</button><button data-act="go" data-arg="armory">Armory</button>${nm ? `<button class="primary" data-act="brief" data-arg="${nm.id}">Next Mission</button>` : `<button class="primary" data-act="play" data-arg="0">Play Arcade</button>`}`;
    else btns = `<button data-act="quit">Menu</button><button data-act="go" data-arg="armory">Armory</button><button class="primary" data-act="play" data-arg="${r.id}">${m ? 'Retry' : 'Play Again'}</button>`;
    return `<div class="wide results ${r.win ? 'win' : 'lose'}"><h2 class="big">${r.win ? 'Mission Complete' : m ? 'Mission Failed' : 'Chewed Up'}</h2>
      <p class="sub">${m ? `Mission ${m.id} · ${m.title}` : `Arcade · reached wave ${r.wave}${r.newBest ? ' · new best' : ''}`}</p>
      ${r.win ? `<div class="bigstars">${stars(r.stars)}</div><ul class="checks done">${r.bonus.map(([t, ok]) => `<li class="${ok ? 'ok' : 'no'}">${t}</li>`).join('')}</ul>` : ''}
      <div class="statrow">${st(r.score.toLocaleString(), 'Score')}${st(r.stats.kills, 'Kills')}${st(r.stats.heads, 'Headshots')}${st(r.stats.disarms, 'Disarms')}${st(acc + '%', 'Accuracy')}</div>
      <div class="loot"><em class="cz">+${r.candy.toLocaleString()}</em><span>+${r.xp} XP</span>${r.levelUps ? `<b class="lvl">Level up · Lv ${profile.level} ${rankName(profile.level)} · perk point earned</b>` : ''}${r.unlock ? `<b class="unlock">New weapon · ${r.unlock}</b>` : ''}</div>
      <footer>${btns}</footer></div>`;
  },
};

function render() {
  overlay.hidden = false; overlay.className = 'scr-' + cur; panel.innerHTML = screens[cur](); overlay.scrollTop = 0;
  const f = panel.querySelector('button.primary') || panel.querySelector('button'); if (f) f.focus({ preventScroll: true });
}
function show(screen, d = null, root = false) {
  if (root) stack = []; else if (cur) stack.push([cur, data]);
  cur = screen; data = d; resetArmed = false; render();
  if (screen === 'results' && d.win) for (let i = 0; i < d.stars; i++) setTimeout(() => audio.sfx.star(i), 500 + i * 380);
}
function hide() { overlay.hidden = true; cur = null; stack = []; }
function back() { const p = stack.pop(); if (p) { [cur, data] = p; resetArmed = false; render(); } else H.quit(); }

function act(a, arg) {
  switch (a) {
    case 'go': show(arg); break;
    case 'back': back(); break;
    case 'brief': show('brief', MISSIONS.find(m => m.id === +arg)); break;
    case 'play': hide(); H.play(+arg); break;
    case 'resume': hide(); H.resume(); break;
    case 'restart': hide(); H.restart(); break;
    case 'quit': H.quit(); break;
    case 'buy': {
      const t = profile.up[arg] || 0, cost = TIER_COST[t];
      if (t < 3 && profile.candy >= cost) { profile.candy -= cost; profile.up[arg] = t + 1; save(); audio.sfx.buy(); render(); } else audio.sfx.deny();
      break;
    }
    case 'perk': {
      const p = PERKS.find(x => x.id === arg);
      if (p && !profile.perks[arg] && profile.perkPts >= p.cost) { profile.perkPts -= p.cost; profile.perks[arg] = true; save(); audio.sfx.buy(); render(); } else audio.sfx.deny();
      break;
    }
    case 'gfx': settings.quality = settings.quality === 'high' ? 'low' : 'high'; saveSettings(); location.reload(); break;
    case 'reset': if (resetArmed) { resetProfile(); resetArmed = false; } else resetArmed = true; render(); break;
  }
}

export const ui = {
  init(handlers) {
    H = handlers;
    panel.addEventListener('click', e => {
      const t = e.target.closest('[data-act]'); if (!t || t.disabled) return;
      audio.init(); if (t.dataset.act !== 'buy' && t.dataset.act !== 'perk') audio.sfx.ui();
      act(t.dataset.act, t.dataset.arg);
    });
    panel.addEventListener('input', e => {
      if (e.target.id === 'sens') settings.sens = +e.target.value;
      if (e.target.id === 'vol') { settings.volume = +e.target.value; audio.setVolume(settings.volume); }
      saveSettings();
    });
    addEventListener('keydown', e => { if (e.code === 'Escape' && cur && cur !== 'menu' && cur !== 'results' && stack.length) { e.stopPropagation(); back(); } });
  },
  show, hide,
  get screen() { return cur; },
};
