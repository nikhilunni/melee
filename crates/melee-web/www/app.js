// The web app runtime: HTML menus, file access, keyboard and the frame
// loop. Every rule lives in libmelee (WebApp, from crates/melee-web).
import init, { WebApp, catalog, header_len, fst_range, create_surface } from './pkg/melee_web.js';

const SCREENS = ['disc', 'characters', 'stages', 'loading', 'match', 'results'];
const ACTIONS = { left: 0, right: 1, up: 2, down: 3, attack: 4, special: 5, jump: 6, shield: 7, grab: 8 };
// Physical keys (KeyboardEvent.code), as in the macOS app.
const KEYS = {
  KeyA: [0, 'left'], KeyD: [0, 'right'], KeyW: [0, 'up'], KeyS: [0, 'down'], KeyK: [0, 'attack'],
  KeyJ: [0, 'special'], Space: [0, 'jump'], KeyL: [0, 'shield'], KeyI: [0, 'grab'],
  ArrowLeft: [1, 'left'], ArrowRight: [1, 'right'], ArrowUp: [1, 'up'], ArrowDown: [1, 'down'],
  KeyN: [1, 'attack'], KeyM: [1, 'special'], Comma: [1, 'jump'], Period: [1, 'shield'], Slash: [1, 'grab'],
};

const $ = (id) => document.getElementById(id);
const canvas = $('game');
let app, cat, discFile = null, picking = 0, lastFrame = null;
// One load at a time; a new token abandons the previous one.
let loadActive = false, loadToken = 0;

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  for (const child of children) node.append(child);
  return node;
}
const characterName = (id) => cat.characters[id]?.name ?? '';
const randomSeed = () => crypto.getRandomValues(new Uint32Array(1))[0];

function message(title, text, actions = []) {
  const dialog = $('message');
  $('message-title').textContent = title;
  $('message-text').textContent = text;
  const row = $('message-actions');
  row.replaceChildren(...actions.map(([label, run]) =>
    el('button', { className: 'big', textContent: label, onclick: () => { dialog.close(); run(); } })),
    el('button', { className: 'big prominent', textContent: 'OK', onclick: () => dialog.close() }));
  if (!dialog.open) dialog.showModal();
}
/** Run a core call; a thrown error becomes a message. */
function attempt(run, title = 'Melee') {
  try { run(); } catch (e) { message(title, e.message ?? String(e)); }
  render();
}

// ---- Disc access: Blob.slice reads only the bytes asked for.

const readRange = async (file, start, end) => new Uint8Array(await file.slice(start, end).arrayBuffer());

async function openDisc(file) {
  try {
    const header = await readRange(file, 0, header_len());
    const [start, end] = fst_range(header);
    const fst = await readRange(file, start, end);
    app.open_disc(header, fst, file.size);
    discFile = file;
  } catch (e) {
    message('That disc image cannot be used', e.message ?? String(e));
  }
  render();
}

// The last disc, where the browser can keep a file handle (Chromium's File
// System Access API). The handle is stored, not the 1.4 GB file.
const remembered = {
  async db() {
    return new Promise((resolve, reject) => {
      const open = indexedDB.open('melee', 1);
      open.onupgradeneeded = () => open.result.createObjectStore('handles');
      open.onsuccess = () => resolve(open.result);
      open.onerror = () => reject(open.error);
    });
  },
  async put(handle) {
    try {
      const db = await this.db();
      db.transaction('handles', 'readwrite').objectStore('handles').put(handle, 'disc');
    } catch { /* storage unavailable: nothing to remember */ }
  },
  async get() {
    try {
      const db = await this.db();
      return await new Promise((resolve) => {
        const get = db.transaction('handles').objectStore('handles').get('disc');
        get.onsuccess = () => resolve(get.result ?? null);
        get.onerror = () => resolve(null);
      });
    } catch { return null; }
  },
};
async function openHandle(handle) {
  if ((await handle.queryPermission?.({ mode: 'read' })) !== 'granted'
      && (await handle.requestPermission?.({ mode: 'read' })) !== 'granted') {
    return message('Permission needed', 'Allow the page to read the disc image to continue.');
  }
  await remembered.put(handle);
  await openDisc(await handle.getFile());
}
async function chooseDisc() {
  if (window.showOpenFilePicker) {
    try {
      const [handle] = await window.showOpenFilePicker({
        types: [{ description: 'GameCube disc image', accept: { 'application/octet-stream': ['.iso', '.gcm'] } }],
      });
      return openHandle(handle);
    } catch (e) {
      if (e.name === 'AbortError') return;
    }
  }
  $('disc-input').click();
}
$('disc-input').addEventListener('change', (e) => e.target.files[0] && openDisc(e.target.files[0]));
$('choose-disc').addEventListener('click', chooseDisc);

const dropzone = $('dropzone');
for (const target of [dropzone, document.body]) {
  target.addEventListener('dragover', (e) => {
    e.preventDefault();
    if (app?.screen() === 0) dropzone.classList.add('targeted');
  });
  target.addEventListener('dragleave', () => dropzone.classList.remove('targeted'));
  target.addEventListener('drop', async (e) => {
    e.preventDefault();
    dropzone.classList.remove('targeted');
    if (![0, 1, 2, 5].includes(app.screen())) return;
    const item = [...e.dataTransfer.items].find((i) => i.kind === 'file');
    const handle = await item?.getAsFileSystemHandle?.();
    if (handle?.kind === 'file') return openHandle(handle);
    const file = e.dataTransfer.files[0];
    if (file) openDisc(file);
  });
}

// ---- Loading: read each pending file, then build the match.

async function load() {
  const run = ++loadToken;
  try {
    for (const { name, start, end } of app.pending_files()) {
      const bytes = await readRange(discFile, start, end);
      if (run !== loadToken || app.screen() !== 3) return;
      app.provide_file(name, bytes);
      renderLoading();
    }
    // Let the full bar paint before the match build blocks briefly.
    await new Promise((resolve) => setTimeout(resolve, 0));
    if (run !== loadToken || app.screen() !== 3) return;
    try { app.finish_loading(); } catch { /* the notice explains */ }
  } catch (e) {
    app.fail_loading(`Could not read the disc: ${e.message ?? e}. Choose the disc again if it moved.`);
  }
  lastFrame = null;
  render();
}

// ---- Rendering the screens from the core's state.

function render() {
  const screen = SCREENS[app.screen()];
  for (const name of SCREENS) $(`screen-${name}`).hidden = name !== screen;
  canvas.style.visibility = screen === 'match' || screen === 'results' ? 'visible' : 'hidden';
  const notice = app.take_notice();
  if (notice) {
    const faulted = app.faulted();
    message(faulted ? 'The match stopped on a fault' : 'Melee', notice,
      faulted ? [['Save Replay', saveReplay], ['Quit to Character Select', () => attempt(() => app.quit_to_menu())]] : []);
  }
  ({ disc: renderDisc, characters: renderCharacters, stages: renderStages, loading: renderLoading,
     match: renderMatch, results: renderResults })[screen]();
  if (screen === 'loading' && !loadActive) {
    loadActive = true;
    load().finally(() => {
      loadActive = false;
      if (app.screen() === 3) render();
    });
  }
}

async function renderDisc() {
  const row = $('disc-actions');
  row.replaceChildren();
  const info = app.disc_info();
  if (info && discFile) {
    row.append(el('button', { className: 'big', textContent: `Continue with ${info.gameId}`,
      onclick: () => attempt(() => app.resume_disc()) }));
  }
  const handle = await remembered.get();
  if (handle && !discFile) {
    row.append(el('button', { className: 'big', textContent: `Open ${handle.name}`, onclick: () => openHandle(handle) }));
  }
}

function renderCharacters() {
  const selection = app.selection();
  $('stocks').textContent = selection.stocks;
  const grid = $('character-grid');
  const rows = [];
  for (const c of cat.characters) (rows[c.row] ??= []).push(c);
  grid.replaceChildren(...rows.map((row) => el('div', { className: 'character-row' },
    ...row.sort((a, b) => a.column - b.column).map((c) => {
      const by = [0, 1].filter((p) => selection.players[p].character === c.id);
      const cell = el('button', { className: `character ${by.length ? `p${by[0] + 1}` : ''}`, title: c.name,
        onclick: () => choose(c.id) }, c.name);
      cell.append(el('span', { className: 'badges' },
        ...by.map((p) => el('span', { className: `badge p${p + 1}`, textContent: `P${p + 1}` }))));
      return cell;
    }))));
  for (const panel of document.querySelectorAll('.player-panel')) {
    const player = Number(panel.dataset.player);
    const slot = selection.players[player];
    const other = selection.players[1 - player];
    const character = cat.characters[slot.character];
    panel.classList.toggle('active', picking === player);
    panel.onclick = () => { picking = player; renderCharacters(); };
    const title = el('div', { className: 'player-title' },
      el('span', { className: 'tag', textContent: `P${player + 1}`, style: `color: var(--p${player + 1})` }),
      el('span', { textContent: character?.name ?? 'Choose a character', className: character ? '' : 'muted' }));
    if (character) {
      title.append(el('button', { className: 'clear', textContent: 'Clear', onclick: (e) => {
        e.stopPropagation(); picking = player; attempt(() => app.choose_character(player, -1));
      } }));
    }
    const body = character
      ? el('div', { className: 'costumes' }, el('span', { className: 'muted', textContent: 'Costume' }),
          ...Array.from({ length: character.costumes }, (_, costume) => el('button', {
            className: `costume ${slot.costume === costume ? `p${player + 1}-bg` : ''}`,
            textContent: costume + 1,
            disabled: other.character === slot.character && other.costume === costume,
            onclick: (e) => { e.stopPropagation(); attempt(() => app.set_costume(player, costume)); },
          })))
      : el('p', { className: 'muted', textContent: picking === player ? 'Click a character above' : 'Waiting…' });
    panel.replaceChildren(title, body);
  }
  $('confirm-characters').disabled = !selection.ready;
}
function choose(id) {
  attempt(() => app.choose_character(picking, id));
  if (app.selection().players[1 - picking].character < 0) picking = 1 - picking;
  renderCharacters();
}

function renderStages() {
  const selection = app.selection();
  const names = selection.players.map((p) => characterName(p.character));
  $('matchup').textContent = `${names[0]} vs ${names[1]} · ${selection.stocks} stocks`;
  $('stage-grid').replaceChildren(...cat.stages.map((s) => el('button', { className: 'stage', textContent: s.name,
    onclick: () => attempt(() => app.choose_stage(s.id, randomSeed())) })));
}

function renderLoading() {
  const p = app.load_progress();
  const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;
  $('load-bar').value = p.bytesTotal ? p.bytesDone / p.bytesTotal : 1;
  $('load-detail').textContent = p.filesTotal
    ? `${p.filesDone} of ${p.filesTotal} files · ${mb(p.bytesDone)} of ${mb(p.bytesTotal)}`
    : 'Starting the match…';
}

function card(player) {
  const tag = `p${Math.min(player.port, 1) + 1}`;
  return el('div', { className: 'card' },
    el('span', { className: 'tag', textContent: `P${player.port + 1}`, style: `color: var(--${tag})` }),
    el('div', { className: 'percent', textContent: `${Math.floor(player.percent)}%` }),
    el('div', { className: 'name', textContent: characterName(player.character) }),
    el('div', { className: 'stocks-row' }, ...Array.from({ length: Math.min(player.stocks, 12) },
      () => el('span', { className: `stock ${tag}-bg` }))));
}
let hudKey = '';
function renderMatch() {
  const hud = app.hud();
  const key = hud ? hud.players.map((p) => `${Math.floor(p.percent)}/${p.stocks}`).join() : '';
  if (key !== hudKey) {
    hudKey = key;
    $('hud').replaceChildren(...(hud?.players.map(card) ?? []));
  }
  $('pause').hidden = !(app.is_paused() && !app.faulted() && document.hasFocus());
}
function renderResults() {
  const results = app.results();
  if (!results) return;
  const winner = results.hud.players[results.winner];
  $('results-title').textContent = winner ? `P${winner.port + 1} ${characterName(winner.character)} Wins!` : 'Draw';
  $('results-players').replaceChildren(...results.hud.players.map(card));
}

function saveReplay() {
  try {
    const bytes = app.replay_bytes();
    const link = el('a', { href: URL.createObjectURL(new Blob([bytes], { type: 'application/json' })),
      download: `melee-replay-${new Date().toISOString().replaceAll(':', '-')}.json` });
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 10000);
  } catch (e) {
    message('Could not save the replay', e.message ?? String(e));
  }
}

// ---- Buttons with data-action.

const ACTIONS_BY_NAME = {
  back: () => { loadToken++; app.back(); },
  confirm: () => app.confirm_characters(),
  'stocks-down': () => app.set_stocks(Math.max(1, app.selection().stocks - 1)),
  'stocks-up': () => app.set_stocks(Math.min(99, app.selection().stocks + 1)),
  resume: () => { app.set_paused(false); lastFrame = null; },
  restart: () => { app.restart(); lastFrame = null; },
  quit: () => app.quit_to_menu(),
  rematch: () => app.rematch(randomSeed()),
  'save-replay': saveReplay,
};
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action]')?.dataset.action;
  if (action) attempt(ACTIONS_BY_NAME[action]);
});

// ---- Keyboard.

document.addEventListener('keydown', (e) => {
  if ($('message').open || e.metaKey || e.ctrlKey || e.altKey) return;
  const screen = SCREENS[app.screen()];
  if (screen === 'match') {
    if (e.code === 'Escape') {
      e.preventDefault();
      if (!e.repeat && !app.faulted()) { app.set_paused(!app.is_paused()); lastFrame = null; render(); }
      return;
    }
    const binding = KEYS[e.code];
    if (binding) {
      e.preventDefault();
      if (!e.repeat) app.action(binding[0], ACTIONS[binding[1]], true);
    }
    return;
  }
  if (e.code === 'Escape' && ['characters', 'stages', 'loading'].includes(screen)) attempt(ACTIONS_BY_NAME.back);
  if (e.code === 'Enter' && screen === 'characters' && app.selection().ready) attempt(ACTIONS_BY_NAME.confirm);
  if (e.code === 'Enter' && screen === 'results') attempt(ACTIONS_BY_NAME.rematch);
});
document.addEventListener('keyup', (e) => {
  const binding = KEYS[e.code];
  if (binding && app?.screen() === 4) app.action(binding[0], ACTIONS[binding[1]], false);
});
window.addEventListener('blur', () => { app?.set_focused(false); lastFrame = null; render(); });
window.addEventListener('focus', () => { app?.set_focused(true); lastFrame = null; render(); });

// ---- Frame loop.

let sized = '';
function fitCanvas() {
  const ratio = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.round(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.round(canvas.clientHeight * ratio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  return `${width}x${height}`;
}
// ?perf shows frame rate, time spent in the core per frame, simulation
// ticks per second and the canvas size (diagnosing slow machines).
const perf = new URLSearchParams(location.search).has('perf') ? {
  element: Object.assign(document.body.appendChild(document.createElement('pre')), { className: 'perf' }),
  frames: 0, busy: 0, since: performance.now(), tick: 0,
  sample(start) {
    this.frames++;
    this.busy += performance.now() - start;
    const elapsed = performance.now() - this.since;
    if (elapsed < 500) return;
    const tick = app.hud()?.tick ?? 0;
    this.element.textContent = `${(this.frames * 1000 / elapsed).toFixed(1)} fps  ` +
      `${(this.busy / this.frames).toFixed(2)} ms/frame in core  ` +
      `${(Math.max(0, tick - this.tick) * 1000 / elapsed).toFixed(0)} ticks/s  ${canvas.width}x${canvas.height}`;
    Object.assign(this, { frames: 0, busy: 0, since: performance.now(), tick });
  },
} : null;
function frame(now) {
  if (window.meleeCrashed) return;
  requestAnimationFrame(frame);
  const screen = app.screen();
  const size = fitCanvas();
  const resized = size !== sized;
  if (screen === 4 || (screen === 5 && resized)) {
    if (screen === 4 && !app.needs_frame() && !resized) { lastFrame = null; return; }
    const elapsed = lastFrame === null ? 0 : now - lastFrame;
    lastFrame = now;
    sized = size;
    try {
      const start = performance.now();
      app.frame(elapsed, canvas.width, canvas.height);
      perf?.sample(start);
    } catch (e) {
      // After a wasm abort the module is unusable: stop (the panic hook has
      // already shown the crash). Other errors are faults with a notice.
      if (e instanceof WebAssembly.RuntimeError) {
        if (!window.meleeCrashed) window.meleeFatal('The game crashed', String(e));
        return;
      }
    }
    if (app.screen() !== screen || app.faulted()) render();
    else if (screen === 4) renderMatch();
  } else {
    lastFrame = null;
  }
}

// ---- Development: ?disc=<url> reads an image served with HTTP Range
// (tools/run-web.sh --disc); ?autostart=P1:P2[:Stage] walks the menus.

class RemoteFile {
  constructor(url, size) { Object.assign(this, { url, size, name: url.split('/').pop() }); }
  static async open(url) {
    const head = await fetch(url, { headers: { Range: 'bytes=0-0' } });
    const size = Number(head.headers.get('Content-Range')?.split('/')[1]);
    if (!head.ok || !size) throw new Error(`${url} is not served with HTTP Range`);
    return new RemoteFile(url, size);
  }
  slice(start, end) {
    return { arrayBuffer: async () => {
      const response = await fetch(this.url, { headers: { Range: `bytes=${start}-${end - 1}` } });
      if (response.status !== 206) throw new Error(`range ${start}-${end} of ${this.url}: ${response.status}`);
      return response.arrayBuffer();
    } };
  }
}
async function developmentStart() {
  const params = new URLSearchParams(location.search);
  if (!params.get('disc')) return;
  await openDisc(await RemoteFile.open(params.get('disc')));
  const [p1, p2, stage] = (params.get('autostart') ?? '').split(':');
  const key = (list, name) => list.find((x) => x.key === name)?.id;
  if (p1 && p2) {
    attempt(() => { app.choose_character(0, key(cat.characters, p1)); app.choose_character(1, key(cat.characters, p2)); });
  }
  if (stage) attempt(() => { app.confirm_characters(); app.choose_stage(key(cat.stages, stage), randomSeed()); });
}

// ---- Start.

async function main() {
  if (!navigator.gpu) {
    return window.meleeFatal('WebGPU is required',
      'This browser does not offer WebGPU. Use a current Chrome or Edge (113+), or Safari 26+.');
  }
  await init();
  cat = catalog();
  app = new WebApp();
  fitCanvas();
  try {
    app.set_surface(await create_surface(canvas));
  } catch (e) {
    return window.meleeFatal('WebGPU is unavailable', e.message ?? String(e));
  }
  render();
  requestAnimationFrame(frame);
  await developmentStart();
}
main();
