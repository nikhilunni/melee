// The web app runtime: HTML menus, file access, keyboard and the frame
// loop. Every rule lives in libmelee (WebApp, from crates/melee-web); the
// look follows docs/DESIGN.md.
//
// The menus are built once and updated in place (classes, text, image
// sources), so hovers, focus and animations survive state changes.
import init, { WebApp, catalog, header_len, fst_range, create_surface, pad_labels, focus_next } from './pkg/melee_web.js';
import { Pads, FakePads, MENU, renderControllers, updateLive } from './pads.js';

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
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const SPRING = 'cubic-bezier(0.2, 0.9, 0.25, 1.15)';
let app, cat, discFile = null, picking = 0, lastFrame = null;
// Gamepads (pads.js) and their labels; set up in main().
let pads = null, padLabels = null;
// One load at a time; a new token abandons the previous one.
let loadActive = false, loadToken = 0;
// The stage of the match being loaded or played (the loading backdrop).
let chosenStage = null;
// Development (?screen=): show a screen the core is not on, for screenshots.
let devView = null, devResults = null;

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  for (const child of children) if (child != null) node.append(child);
  return node;
}
function svgUse(symbol, className) {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('class', className);
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
  use.setAttribute('href', `#${symbol}`);
  svg.append(use);
  return svg;
}
const characterName = (id) => cat.characters[id]?.name ?? '';
const randomSeed = () => crypto.getRandomValues(new Uint32Array(1))[0];
const portClass = (port) => `port-${Math.min(port, 3)}`;
const SLOTS = 4;
const TEAMS = ['Red', 'Blue', 'Green'];
const CONTROLLER = { off: 0, human: 1, cpu: 2 };
const SPECIALS = ['Off', 'Super Sudden Death', 'Lightning', 'Slo-Mo'];
const FREQUENCIES = ['Off', 'Very low', 'Low', 'Medium', 'High', 'Very high'];
/** The players in the match, in port order. */
const activePlayers = (selection) => selection.players
  .map((slot, player) => ({ ...slot, player })).filter((slot) => slot.controller !== CONTROLLER.off);
/** "4 stock, teams, items": what the rules add up to. */
function rulesSummary(rules) {
  const parts = [rules.mode === 1 ? `Time ${rules.timeMinutes} min` : `${rules.stocks} stock${rules.stocks === 1 ? '' : 's'}`];
  if (rules.teams) parts.push('teams');
  if (rules.itemFrequency >= 0) parts.push('items');
  if (rules.special) parts.push(SPECIALS[rules.special]);
  return parts.join(', ');
}

function message(title, text, actions = [], tone = 'neutral') {
  const dialog = $('message');
  dialog.classList.toggle('danger', tone !== 'neutral');
  dialog.classList.toggle('mono', tone === 'fault');
  $('message-title').textContent = title;
  $('message-text').textContent = text;
  const button = (label, run, className = 'btn') =>
    el('button', { className, onclick: () => { dialog.close(); run?.(); } }, el('span', { textContent: label }));
  const copy = el('button', { className: 'link-button copy', textContent: 'Copy details', onclick: () => {
    navigator.clipboard?.writeText(text);
    copy.textContent = 'Copied';
  } });
  $('message-actions').replaceChildren(...(tone !== 'neutral' ? [copy] : []),
    ...actions.map(([label, run]) => button(label, run)), button('OK', null, 'btn accent'));
  if (!dialog.open) dialog.showModal();
  dialog.querySelector('.btn.accent').focus();
}
/** Run a core call; a thrown error becomes a message. */
function attempt(run, title = 'Melee') {
  try { run(); } catch (e) { message(title, e.message ?? String(e), [], 'danger'); }
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
    art.reset();
    showDiscError(null);
    loadArt(file);
  } catch (e) {
    const text = e.message ?? String(e);
    if (app.screen() === 0) showDiscError(file.name, text);
    else message('That disc image cannot be used', text, [], 'danger');
  }
  render();
}
function showDiscError(name, text) {
  const card = $('disc-error');
  card.hidden = !name;
  if (!name) return;
  card.replaceChildren(
    el('h2', { textContent: 'That disc image cannot be used' }),
    el('p', { textContent: `${name}: ${text}` }),
    el('p', { className: 'hint', textContent: 'Melee needs an uncompressed NTSC-U 1.02 image (.iso or .gcm).' }));
}

// ---- Menu art: the menu archives (about 5 MB), read right after the disc
// opens; menus show our own placeholders until app.art_ready().

async function loadArt(file) {
  try {
    for (const { name, start, end } of app.art_files()) {
      const bytes = await readRange(file, start, end);
      if (file !== discFile) return; // another disc replaced this one
      app.provide_file(name, bytes);
      updateArtStatus();
    }
  } catch (e) {
    console.warn(`Menu art unavailable: ${e.message ?? e}`);
  }
  updateArtStatus();
  art.refresh();
}
function updateArtStatus() {
  const status = $('art-status');
  const ready = !app || app.art_ready() || !discFile;
  status.hidden = ready;
  if (!ready) {
    const p = app.art_progress();
    status.textContent = `Reading menu art · ${Math.round(100 * p.bytesDone / Math.max(1, p.bytesTotal))}%`;
  }
}

// Disc art, decoded once by the core and kept as blob: URLs. Elements ask
// for a piece with data-art="kind:id:costume"; art.hydrate() fills in
// <img> sources, CSS masks (class "mask": intensity art, tinted by its
// background colour) and portrait crops (data-crop), and sets data-state to
// "ready" or "none" (retail has no such image). kind: ART below.
const ART = { portrait: 0, face: 1, stock: 2, characterEmblem: 3, stageIcon: 4, stageName: 5, stageEmblem: 6,
  stagePreview: 7 };
const NONE = 'none', PENDING = 'pending';
// Portrait framings: [aspect of the element (height / width), zoom (the
// portrait's width over the element's)]. Tile art is 1.44x the tile wide
// (it extends under the slant).
const CROPS = { tile: [0.56, 0.8], face: [1, 1.9], row: [1, 1.9] };
const art = {
  urls: new Map(),
  heads: new Map(), // character id -> {x, top}: where the face is in the portrait (fractions)
  queued: false,
  reset() {
    for (const url of this.urls.values()) if (url.startsWith?.('blob:')) URL.revokeObjectURL(url);
    this.urls.clear();
    this.heads.clear();
  },
  /** A blob: URL, NONE, or null while it is not available yet. */
  get(kind, id, costume = 0) {
    if (!app?.art_ready()) return null;
    const key = `${kind}:${id}:${costume}`;
    const known = this.urls.get(key);
    if (known !== undefined) return known === PENDING ? null : known;
    let image;
    try {
      image = app.art(kind, id, costume);
    } catch {
      // A preview still rendering is not missing; ask again later.
      if (kind === ART.stagePreview && previewsPending()) return null;
      this.urls.set(key, NONE);
      return NONE;
    }
    if (!image) return null; // a stage preview not rendered yet
    const data = new ImageData(image.data, image.width, image.height);
    if (kind === ART.portrait && !this.heads.has(id)) this.heads.set(id, findHead(data));
    this.urls.set(key, PENDING);
    const surface = new OffscreenCanvas(image.width, image.height);
    surface.getContext('2d').putImageData(data, 0, 0);
    surface.convertToBlob().then((blob) => {
      if (this.urls.get(key) !== PENDING) return; // reset since
      this.urls.set(key, URL.createObjectURL(blob));
      this.refresh();
    });
    return null;
  },
  /** Hydrate everything on the next animation frame (coalesced). */
  refresh() {
    if (this.queued) return;
    this.queued = true;
    requestAnimationFrame(() => { this.queued = false; this.hydrate(document); });
    // Hidden tabs have no animation frames.
    setTimeout(() => { if (this.queued) { this.queued = false; this.hydrate(document); } }, 100);
  },
  hydrate(root) {
    for (const node of root.querySelectorAll('[data-art]')) this.apply(node);
  },
  apply(node) {
    const spec = node.dataset.art;
    const [kind, id, costume] = spec ? spec.split(':').map(Number) : [];
    const value = spec ? this.get(kind, id, costume) : null;
    const state = value === NONE ? 'none' : value ? 'ready' : '';
    if (state === 'ready') {
      if (node.tagName === 'IMG') {
        if (node.getAttribute('src') !== value) {
          node.src = value;
          if (node.classList.contains('slide-in')) slideIn(node);
        }
      } else if (node.classList.contains('mask')) {
        const image = `url("${value}")`;
        node.style.webkitMaskImage = image;
        node.style.maskImage = image;
      } else {
        node.style.backgroundImage = `url("${value}")`;
        if (node.dataset.crop) Object.assign(node.style, cropStyle(id, ...CROPS[node.dataset.crop]));
      }
    } else if (node.tagName === 'IMG' && node.hasAttribute('src')) {
      node.removeAttribute('src');
    }
    if (node.dataset.state !== state) node.dataset.state = state;
  },
};
// For the console and page tests: meleeArt(ART.portrait, 0, 0) is ImageData.
window.meleeArt = (kind, id, costume = 0) => {
  if (!app?.art_ready()) return null;
  try {
    const { width, height, data } = app.art(kind, id, costume);
    return new ImageData(data, width, height);
  } catch { return null; }
};
window.meleeArtKinds = ART;

/** Where the face is: the top of the figure, centred on the mass of its top third. */
function findHead({ width, height, data }) {
  const opaque = (x, y) => data[(y * width + x) * 4 + 3] > 120;
  let top = 0;
  find: for (; top < height; top++) {
    let count = 0;
    for (let x = 0; x < width; x++) if (opaque(x, top) && ++count >= 4) break find;
  }
  let sum = 0, n = 0;
  const bottom = Math.min(height, top + Math.round(height * 0.3));
  for (let y = top; y < bottom; y++) {
    for (let x = 0; x < width; x++) if (opaque(x, y)) { sum += x; n++; }
  }
  return { x: n ? sum / n / width : 0.5, top: Math.max(0, top / height - 0.015) };
}
/** Background size and position showing the face of `id`'s portrait. */
function cropStyle(id, aspect, zoom) {
  const head = art.heads.get(id) ?? { x: 0.5, top: 0 };
  const clamp = (v) => Math.min(1, Math.max(0, v));
  const tall = zoom * 188 / 136; // portrait height over the element's width
  const x = zoom === 1 ? 0.5 : clamp((0.5 - head.x * zoom) / (1 - zoom));
  const y = tall <= aspect ? 0 : clamp(head.top * tall / (tall - aspect));
  return { backgroundSize: `${zoom * 100}% auto`, backgroundPosition: `${x * 100}% ${y * 100}%` };
}
function slideIn(node) {
  if (reducedMotion.matches) return;
  const from = node.closest('[data-player="1"]') ? -24 : 24;
  node.animate([{ opacity: 0, transform: `translateX(${from}px)` }, { opacity: 1, transform: 'none' }],
    { duration: 360, easing: SPRING });
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
  // A gamepad press is no user activation: the browser would ignore the picker.
  if (navigator.userActivation && !navigator.userActivation.isActive) {
    return message('Open a disc', 'Browsers open files only right after a click, a key press or a drop; a gamepad '
      + 'button does not count. Click Open Disc, press Enter on it, or drop the image on the page. A disc opened '
      + 'before is offered here as Continue or Open (Chrome and Edge remember it).');
  }
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
  target.addEventListener('dragleave', (e) => {
    if (!e.relatedTarget || !dropzone.contains(e.relatedTarget)) dropzone.classList.remove('targeted');
  });
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
      updateLoading();
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

// ---- Screens.

let shownScreen = null;
const MENUS = ['disc', 'characters', 'stages', 'loading'];
/** Show one screen; menus slide the outgoing one away. True when it changed. */
function showScreen(name) {
  if (name === shownScreen) return false;
  const previous = shownScreen;
  shownScreen = name;
  const next = $(`screen-${name}`);
  next.classList.remove('leaving');
  next.hidden = false;
  if (previous) {
    const old = $(`screen-${previous}`);
    if (MENUS.includes(previous) && MENUS.includes(name) && !reducedMotion.matches) {
      old.classList.add('leaving');
      setTimeout(() => {
        if (shownScreen !== previous) { old.hidden = true; old.classList.remove('leaving'); }
      }, 180);
    } else {
      old.hidden = true;
    }
  }
  const menu = MENUS.includes(name);
  $('backdrop').classList.toggle('off', !menu);
  canvas.style.visibility = menu ? 'hidden' : 'visible';
  return true;
}

function render() {
  const real = SCREENS[app.screen()];
  const screen = devView ?? real;
  const entered = showScreen(screen);
  const notice = app.take_notice();
  if (notice) {
    const faulted = app.faulted();
    message(faulted ? 'The match stopped on a fault' : (real === 'stages' ? 'The match could not start' : 'Melee'), notice,
      faulted ? [['Save Replay', saveReplay], ['Quit to Character Select', () => attempt(() => app.quit_to_menu())]] : [],
      faulted ? 'fault' : 'danger');
  }
  ({ disc: updateDisc, characters: updateCharacters, stages: updateStages, loading: updateLoading,
     match: updateMatch, results: updateResults })[screen](entered);
  // A pad drives the menus: a new screen opens with something focused.
  if (entered && document.body.classList.contains('pad-nav') && !anyDialogOpen()) {
    const layer = topLayer();
    if (!layer.contains(document.activeElement)) focusDefault(layer);
  }
  updatePadHints();
  if (real === 'loading' && !loadActive) {
    loadActive = true;
    load().finally(() => {
      loadActive = false;
      if (app.screen() === 3) render();
    });
  }
}

// ---- Disc.

async function updateDisc(entered) {
  if (!entered) return;
  const row = $('disc-actions');
  for (const extra of row.querySelectorAll('.extra')) extra.remove();
  const info = app.disc_info();
  if (info && discFile) {
    row.append(el('button', { className: 'btn extra', onclick: () => attempt(() => app.resume_disc()) },
      el('span', { textContent: `Continue with ${info.gameId}` })));
    return;
  }
  const handle = await remembered.get();
  if (handle && !discFile && !row.querySelector('.extra')) {
    row.append(el('button', { className: 'btn extra', onclick: () => openHandle(handle) },
      el('span', { textContent: `Open ${handle.name}` })));
  }
}

// ---- Character select.

let tiles = []; // by character id
let characterCursor = 10; // Fox
function buildCharacters() {
  const grid = $('character-grid');
  const rows = [];
  for (const c of cat.characters) (rows[c.row] ??= []).push(c);
  for (const row of rows) {
    const line = el('div', { className: 'character-row', role: 'row' });
    for (const c of row.sort((a, b) => a.column - b.column)) {
      const tile = el('button', { className: 'tile', role: 'gridcell', tabIndex: -1,
        onclick: () => choose(c.id), onfocus: () => { characterCursor = c.id; },
        onpointerenter: (e) => e.currentTarget.focus({ preventScroll: true }) },
      el('div', { className: 'art' }),
      el('div', { className: 'fallback' }, svgUse('emblem', ''), el('img', { alt: '' })),
      el('span', { className: 'name', textContent: c.name }),
      el('span', { className: 'coins' }));
      tile.querySelector('.art').dataset.art = `${ART.portrait}:${c.id}:0`;
      tile.querySelector('.art').dataset.crop = 'tile';
      tile.querySelector('.fallback img').dataset.art = `${ART.stock}:${c.id}:0`;
      tile.dataset.id = c.id;
      tiles[c.id] = tile;
      line.append(tile);
    }
    grid.append(line);
  }
  // Portrait parallax: ±4 px following the pointer.
  grid.addEventListener('pointermove', (e) => {
    const tile = e.target.closest('.tile');
    if (!tile) return;
    const box = tile.getBoundingClientRect();
    tile.style.setProperty('--px', `${((e.clientX - box.left) / box.width - 0.5) * 8}px`);
    tile.style.setProperty('--py', `${((e.clientY - box.top) / box.height - 0.5) * 8}px`);
  });

  for (const panel of document.querySelectorAll('.player-panel')) {
    const player = Number(panel.dataset.player);
    const portrait = el('img', { className: 'main slide-in', alt: '' });
    panel.append(
      el('div', { className: 'panel-tag', textContent: `P${player + 1}`, ariaHidden: 'true' }),
      el('div', { className: 'slot-controls', onclick: (e) => e.stopPropagation() }),
      el('p', { className: 'panel-status', textContent: 'Choosing' }),
      el('div', { className: 'panel-emblem mask' }),
      el('div', { className: 'panel-portrait' }, svgUse('emblem', 'placeholder'),
        el('img', { className: 'stock-hero', alt: '' }), portrait),
      el('div', { className: 'panel-info' },
        el('div', { className: 'name-row' }, el('p', { className: 'panel-name' }),
          el('button', { className: 'clear-pick', textContent: '✕', ariaLabel: `Clear player ${player + 1}'s character`,
            onclick: (e) => { e.stopPropagation(); picking = player; attempt(() => app.choose_character(player, -1)); } })),
        el('p', { className: 'panel-sub' }),
        el('div', { className: 'chips', role: 'radiogroup', ariaLabel: `Player ${player + 1} costume` })));
    panel.setAttribute('role', 'group');
    panel.addEventListener('click', () => {
      picking = player;
      updateCharacters();
      tiles[characterCursor]?.focus({ preventScroll: true });
    });
  }
}

/** A panel's HMN/CPU/OFF switch, CPU level, team and handicap. */
function slotControls(player, slot, rules) {
  const step = (label, value, min, max, name, set) => el('div', { className: 'slot-step' },
    el('span', { textContent: label }),
    el('button', { textContent: '−', ariaLabel: `Lower P${player + 1} ${name}`, disabled: value <= min,
      onclick: () => { attempt(() => set(value - 1)); updateCharacters(); } }),
    el('output', { textContent: value === 0 ? '–' : String(value) }),
    el('button', { textContent: '+', ariaLabel: `Raise P${player + 1} ${name}`, disabled: value >= max,
      onclick: () => { attempt(() => set(value + 1)); updateCharacters(); } }));
  const nodes = [el('div', { className: 'segmented', role: 'radiogroup', ariaLabel: `Player ${player + 1}` },
    ...[['HMN', CONTROLLER.human], ['CPU', CONTROLLER.cpu], ['OFF', CONTROLLER.off]].map(([label, value]) => {
      const button = el('button', { className: slot.controller === value ? 'on' : '', textContent: label, role: 'radio',
        ariaChecked: String(slot.controller === value),
        onclick: () => { attempt(() => app.set_controller(player, value)); updateCharacters(); } });
      button.dataset.controller = value;
      return button;
    }))];
  if (slot.controller === CONTROLLER.cpu) {
    nodes.push(step('LV', slot.cpuLevel, 1, 9, 'CPU level', (v) => app.set_cpu_level(player, v)));
  }
  if (slot.controller !== CONTROLLER.off && rules.teams) {
    nodes.push(el('button', { className: 'team-chip', textContent: TEAMS[slot.team] ?? '?',
      ariaLabel: `Player ${player + 1} team ${TEAMS[slot.team]}`,
      onclick: () => { attempt(() => app.set_team(player, (slot.team + 1) % TEAMS.length)); updateCharacters(); } }));
  }
  if (slot.controller !== CONTROLLER.off && handicapShown()) {
    nodes.push(step('HC', slot.handicap, 0, 9, 'handicap', (v) => app.set_handicap(player, v)));
  }
  const pad = pads?.count ? pads.list().find((p) => p.port === player) : null;
  if (slot.controller === CONTROLLER.human && pads?.count) {
    nodes.push(el('span', { className: `pad-chip ${pad ? 'joined' : ''}`, textContent: pad ? pad.name : 'Press a button to join' }));
  }
  return nodes;
}
let handicapRule = false;
const handicapShown = () => handicapRule || app.selection().players.some((p) => p.handicap > 0);

/** A character-select control's identity across rebuilds (its panel and role). */
function controlKey(button) {
  const player = button.closest('[data-player]')?.dataset.player ?? '';
  const role = button.dataset.controller !== undefined ? `controller ${button.dataset.controller}`
    : button.classList.contains('team-chip') ? 'team'
      : button.classList.contains('chip') ? `chip ${[...button.parentElement.children].indexOf(button)}`
        : button.getAttribute('aria-label') ?? button.textContent;
  return `${player}:${role}`;
}
function updateCharacters(entered) {
  // Panels rebuild their controls when they change; focus stays on the same control.
  const before = document.activeElement;
  const lostKey = before?.closest?.('.player-panel') ? controlKey(before) : null;
  const selection = app.selection();
  const rules = app.rules();
  const ready = selection.ready;
  const timed = rules.mode === 1;
  $('stocks').textContent = timed ? `${rules.timeMinutes}′` : rules.stocks;
  $('stocks-label').textContent = timed ? 'Time' : 'Stock';
  const grid = $('character-grid');
  grid.dataset.active = picking;
  for (const tile of tiles) {
    const id = Number(tile.dataset.id);
    const by = [0, 1, 2, 3].filter((p) => selection.players[p].controller !== CONTROLLER.off
      && selection.players[p].character === id);
    tile.classList.toggle('taken-p1', by.includes(0) || by.includes(2));
    tile.classList.toggle('taken-p2', by.includes(1) || by.includes(3));
    const coins = tile.querySelector('.coins');
    const want = by.map((p) => `p${p + 1}`).join();
    if (coins.dataset.by !== want) {
      coins.dataset.by = want;
      coins.replaceChildren(...by.map((p) => el('span', { className: `coin p${p + 1}`, textContent: `P${p + 1}` })));
    }
    tile.setAttribute('aria-label', `${characterName(id)}${by.length ? `, chosen by ${by.map((p) => `P${p + 1}`).join(' and ')}` : ''}`);
  }

  for (const panel of document.querySelectorAll('.player-panel')) {
    const player = Number(panel.dataset.player);
    const slot = selection.players[player];
    const others = selection.players.filter((p, i) => i !== player && p.controller !== CONTROLLER.off);
    const off = slot.controller === CONTROLLER.off;
    const id = off ? -1 : slot.character;
    const character = cat.characters[id];
    panel.classList.toggle('active', picking === player && !ready);
    panel.classList.toggle('off', off);
    if (rules.teams && !off) panel.dataset.team = slot.team; else delete panel.dataset.team;
    const controls = panel.querySelector('.slot-controls');
    const padSignature = pads?.count ? pads.list().map((p) => `${p.handle}:${p.port}`).join() : '';
    const controlSignature = JSON.stringify([slot, rules.teams, handicapShown(), padSignature]);
    if (controls.dataset.signature !== controlSignature) {
      controls.dataset.signature = controlSignature;
      controls.replaceChildren(...slotControls(player, slot, rules));
    }
    panel.setAttribute('aria-label', `Player ${player + 1}${character ? `: ${character.name}` : ''}`);
    panel.querySelector('.panel-portrait .main').dataset.art = character ? `${ART.portrait}:${id}:${slot.costume}` : '';
    panel.querySelector('.stock-hero').dataset.art = character ? `${ART.stock}:${id}:${slot.costume}` : '';
    panel.querySelector('.panel-emblem').dataset.art = character ? `${ART.characterEmblem}:${id}:0` : '';
    panel.querySelector('.clear-pick').hidden = !character;
    const name = panel.querySelector('.panel-name');
    name.textContent = character?.name ?? 'Choose a fighter';
    name.classList.toggle('empty', !character);
    if (off) name.textContent = 'Not playing';
    panel.querySelector('.panel-sub').textContent = off ? 'Pick a fighter or set HMN/CPU to join'
      : character ? `Costume ${slot.costume + 1} of ${character.costumes}`
        : picking === player ? 'Click a fighter or press Enter' : 'Waiting…';
    const chips = panel.querySelector('.chips');
    const takenCostumes = others.filter((o) => o.character === id).map((o) => o.costume);
    const signature = character ? `${id}/${slot.costume}/${takenCostumes}` : '';
    if (chips.dataset.signature !== signature) {
      chips.dataset.signature = signature;
      chips.replaceChildren(...Array.from({ length: character?.costumes ?? 0 }, (_, costume) => {
        const taken = takenCostumes.includes(costume);
        const chip = el('button', {
          className: `chip ${slot.costume === costume ? 'on' : ''}`, role: 'radio', tabIndex: -1,
          ariaChecked: String(slot.costume === costume), ariaLabel: `Costume ${costume + 1}${taken ? ' (taken)' : ''}`,
          disabled: taken,
          onclick: (e) => { e.stopPropagation(); attempt(() => app.set_costume(player, costume)); },
        }, el('img', { alt: '' }));
        chip.firstChild.dataset.art = `${ART.stock}:${id}:${costume}`;
        return chip;
      }));
    }
  }

  const slot = $('screen-characters').querySelector('.ready-slot');
  const wasReady = slot.classList.contains('ready');
  slot.classList.toggle('ready', ready);
  const tag = `<b class="p${picking + 1}">P${picking + 1}</b>`;
  const waiting = selection.players[picking].controller !== CONTROLLER.off && selection.players[picking].character < 0;
  $('pick-hint').innerHTML = ready ? '' : waiting || !selection.notReady ? `${tag} &middot; choose your fighter`
    : `${tag} &middot; ${selection.notReady}`;
  if (ready && !wasReady) $('ready-banner').focus({ preventScroll: true });
  if (!ready && document.activeElement === $('ready-banner')) tiles[characterCursor]?.focus({ preventScroll: true });
  if (entered) {
    const mine = selection.players[picking].character;
    if (mine >= 0) characterCursor = mine;
    (ready ? $('ready-banner') : tiles[characterCursor])?.focus({ preventScroll: true });
  }
  if (lostKey && !before.isConnected) {
    [...$('screen-characters').querySelectorAll('.player-panel button')]
      .find((b) => !b.disabled && controlKey(b) === lostKey)?.focus({ preventScroll: true });
  }
  updateArtStatus();
  art.hydrate($('screen-characters'));
}
function choose(id) {
  attempt(() => app.choose_character(picking, id));
  // On to the next player in the match who still needs a pick.
  const players = app.selection().players;
  for (let step = 1; step < SLOTS; step++) {
    const next = (picking + step) % SLOTS;
    if (players[next].controller !== CONTROLLER.off && players[next].character < 0) { picking = next; break; }
  }
  updateCharacters();
}

// ---- Rules.

function buildRules() {
  const rules = app.rules();
  const set = (change) => { attempt(() => app.set_rules(change)); buildRules(); updateCharacters(); };
  const options = (labels, selected, pick) => el('div', { className: 'options', role: 'radiogroup' },
    ...labels.map((label, i) => el('button', { className: i === selected ? 'on' : '', textContent: label, role: 'radio',
      ariaChecked: String(i === selected), onclick: () => pick(i) })));
  const number = (value, min, max, show, pick) => el('div', { className: 'number' },
    el('button', { textContent: '◀', ariaLabel: 'Less', disabled: value <= min, onclick: () => pick(value - 1) }),
    el('output', { textContent: show(value) }),
    el('button', { textContent: '▶', ariaLabel: 'More', disabled: value >= max, onclick: () => pick(value + 1) }));
  const rows = [
    ['Mode', options(['Stock', 'Time'], rules.mode, (i) => set({ mode: i, timeMinutes: i === 1 && !rules.timeMinutes ? 2 : rules.timeMinutes }))],
  ];
  if (rules.mode === 0) rows.push(['Stocks', number(rules.stocks, 1, 99, String, (v) => set({ stocks: v }))]);
  rows.push([rules.mode === 1 ? 'Time' : 'Time limit', number(rules.timeMinutes, rules.mode === 1 ? 1 : 0, 99,
    (v) => (v ? `${v} min` : 'None'), (v) => set({ timeMinutes: v }))]);
  rows.push(['Teams', options(['Off', 'On'], rules.teams ? 1 : 0, (i) => set({ teams: i === 1 }))]);
  if (rules.teams) rows.push(['Friendly fire', options(['Off', 'On'], rules.friendlyFire ? 1 : 0, (i) => set({ friendlyFire: i === 1 }))]);
  rows.push(['Items', options(FREQUENCIES, rules.itemFrequency + 1, (i) => set({ itemFrequency: i - 1 }))]);
  rows.push(['Damage ratio', number(rules.damageRatio, 5, 20, (v) => `${(v / 10).toFixed(1)}×`, (v) => set({ damageRatio: v }))]);
  rows.push(['Handicap', options(['Off', 'On'], handicapShown() ? 1 : 0, (i) => {
    handicapRule = i === 1;
    if (!handicapRule) app.selection().players.forEach((p, player) => { if (p.handicap) attempt(() => app.set_handicap(player, 0)); });
    buildRules();
    updateCharacters();
  })]);
  rows.push(['Special', options(SPECIALS, rules.special, (i) => set({ special: i }))]);
  // Rebuilt on every change: focus stays on the same row's same button.
  const focused = document.activeElement?.dataset?.focusId;
  for (const [label, control] of rows) {
    control.querySelectorAll('button').forEach((button, i) => { button.dataset.focusId = `rules.${label}.${i}`; });
  }
  $('rules-rows').replaceChildren(...rows.flatMap(([label, control]) => [el('span', { className: 'label', textContent: label }), control]));
  const again = focused && $('rules-rows').querySelector(`[data-focus-id="${focused}"]`);
  if (again && !again.disabled) again.focus({ preventScroll: true });
  else if (focused?.startsWith('rules.')) $('rules-rows').querySelector(`[data-focus-id^="${focused.replace(/\d+$/, '')}"]:not(:disabled)`)?.focus({ preventScroll: true });
}
/** Arrow keys over the grid: the nearest tile in that direction. */
function moveCursor(dx, dy) {
  const from = tiles[characterCursor].getBoundingClientRect();
  const fx = from.left + from.width / 2, fy = from.top + from.height / 2;
  let best = null, bestScore = Infinity;
  for (const tile of tiles) {
    const box = tile.getBoundingClientRect();
    const x = box.left + box.width / 2 - fx, y = box.top + box.height / 2 - fy;
    const along = dx ? x * dx : y * dy;
    const across = Math.abs(dx ? y : x);
    if (along <= 1) continue;
    const score = along + across * 3;
    if (score < bestScore) { bestScore = score; best = tile; }
  }
  if (!best && dx) { // wrap within the row
    const row = tiles[characterCursor].parentElement.children;
    best = dx > 0 ? row[0] : row[row.length - 1];
  }
  best?.focus({ preventScroll: true });
}

// ---- Stage select.

let stageTiles = [];
let stageCursor = 0, heroShown = null, heroFront = 'a', shuffling = false;
let previewPoll = 0;
const RANDOM = 'random';
function buildStages() {
  const row = $('stage-row');
  const options = [...cat.stages.map((s) => s.id), RANDOM];
  for (const [index, id] of options.entries()) {
    const random = id === RANDOM;
    const card = el('div', { className: 'stage-card' });
    if (random) {
      card.append(el('div', { className: 'random-glyph', textContent: '?' }));
    } else {
      const preview = el('img', { className: 'preview', alt: '' });
      preview.dataset.art = `${ART.stagePreview}:${id}:0`;
      const icon = el('img', { className: 'icon', alt: '' });
      icon.dataset.art = `${ART.stageIcon}:${id}:0`;
      card.append(stagePlaceholder(id, icon), preview);
    }
    const tile = el('button', {
      className: 'stage-tile', role: 'option', tabIndex: -1, ariaLabel: random ? 'Random stage' : cat.stages[id].name,
      onclick: () => (random ? shuffleStage() : startStage(id)),
      onfocus: () => { stageCursor = index; markStageCursor(); showHero(id); },
      // One cursor for mouse and keyboard, as in retail.
      onpointerenter: () => { if (!shuffling) tile.focus({ preventScroll: true }); },
    }, card, el('span', { className: 'stage-label', textContent: random ? 'Random' : cat.stages[id].name }));
    tile.dataset.stage = id;
    stageTiles.push(tile);
    row.append(tile);
  }
}
/** The keyboard cursor stays visible after a mouse click (focus rings do not). */
function markStageCursor() {
  stageTiles.forEach((tile, i) => tile.classList.toggle('current', i === stageCursor));
}
function startStage(id) {
  const reason = app.stage_unavailable(id);
  if (reason) return message(cat.stages[id].name, `Not with these players and rules: ${reason}.`);
  chosenStage = id;
  attempt(() => app.choose_stage(id, randomSeed()));
}
/** RANDOM: a quick shuffle across the stages, then the pick. */
function shuffleStage() {
  if (shuffling) return;
  const playable = cat.stages.filter((s) => !app.stage_unavailable(s.id));
  if (!playable.length) return;
  const pick = playable[Math.floor(Math.random() * playable.length)].id;
  if (reducedMotion.matches) return startStage(pick);
  shuffling = true;
  let step = 0;
  const steps = 10 + pick;
  const tick = () => {
    const index = step % cat.stages.length;
    stageTiles.forEach((t, i) => t.classList.toggle('flash', i === index));
    showHero(index);
    if (step++ < steps) return setTimeout(tick, 40 + step * 6);
    setTimeout(() => {
      shuffling = false;
      stageTiles.forEach((t) => t.classList.remove('flash'));
      startStage(pick);
    }, 260);
  };
  tick();
}
// Our own colours for each stage, shown until its rendered preview is
// ready (and behind its icon on a tile). Disc icons are never blurred or
// scaled past 2x (DESIGN.md).
const STAGE_GRADIENTS = {
  Battlefield: ['#5a3fa8', '#1a1240', '#f0a65a'],
  FinalDestination: ['#40206e', '#07061a', '#d05cff'],
  DreamLand: ['#4fa3e8', '#1d4a7a', '#9be36a'],
  FountainOfDreams: ['#3a2a8a', '#0d0a2e', '#ff8ad8'],
  PokemonStadium: ['#3d8fd8', '#123058', '#5ed27a'],
  YoshisStory: ['#7ccf5a', '#245a2a', '#ffd25a'],
  YoshisIsland64: ['#8fd0f0', '#2a6a3a', '#f5e070'],
  KongoJungle64: ['#6a8a3a', '#1e2e10', '#c88a3a'],
  KongoJungle: ['#7a5a2a', '#2a1a08', '#e0a040'],
  JungleJapes: ['#3a7a4a', '#123020', '#7fd0e0'],
  HyruleTemple: ['#8fc0f0', '#3a5a3a', '#f0f0e0'],
  Fourside: ['#2a3070', '#05061a', '#f0d060'],
  BrinstarDepths: ['#a03a10', '#2a0808', '#f08030'],
  FlatZone: ['#c8d8b0', '#3a3a32', '#7a8a6a'],
};
/** A stage's gradient with `children` on it (an emblem, its icon). */
function stagePlaceholder(id, ...children) {
  const [a, b, c] = STAGE_GRADIENTS[cat.stages[id].key] ?? ['#2a2f63', '#141735', '#6a74ff'];
  const node = el('div', { className: 'stage-placeholder' }, ...children);
  node.style.setProperty('--stage-a', a);
  node.style.setProperty('--stage-b', b);
  node.style.setProperty('--stage-c', c);
  return node;
}
function showHero(id) {
  id = id === RANDOM ? RANDOM : Number(id);
  if (heroShown === id) return;
  heroShown = id;
  const back = heroFront === 'a' ? 'b' : 'a';
  const layer = $(`hero-${back}`);
  if (id === RANDOM) {
    layer.replaceChildren(el('div', { className: 'random-art' }, el('span', { textContent: '?' })));
  } else {
    const preview = el('img', { className: 'preview', alt: '' });
    preview.dataset.art = `${ART.stagePreview}:${id}:0`;
    const emblem = el('div', { className: 'stage-placeholder-emblem mask' });
    emblem.dataset.art = `${ART.stageEmblem}:${id}:0`;
    layer.replaceChildren(stagePlaceholder(id, emblem), preview);
  }
  art.hydrate(layer);
  layer.classList.add('shown');
  $(`hero-${heroFront}`).classList.remove('shown');
  heroFront = back;
  const name = $('stage-hero-name');
  name.dataset.art = id === RANDOM ? '' : `${ART.stageName}:${id}:0`;
  name.setAttribute('aria-label', id === RANDOM ? 'Random' : cat.stages[id].name);
  $('stage-hero-text').textContent = id === RANDOM ? 'Random' : cat.stages[id].name;
  $('stage-hero-emblem').dataset.art = id === RANDOM ? '' : `${ART.stageEmblem}:${id}:0`;
  art.apply(name);
  art.apply($('stage-hero-emblem'));
}
function updateStages(entered) {
  const selection = app.selection();
  const rules = app.rules();
  const matchup = $('matchup');
  const signature = JSON.stringify([selection, rules]);
  if (matchup.dataset.signature !== signature) {
    matchup.dataset.signature = signature;
    const players = activePlayers(selection);
    const side = (p, order) => {
      const icon = el('img', { alt: '' });
      icon.dataset.art = `${ART.stock}:${p.character}:${p.costume}`;
      const name = players.length > 2 ? '' : characterName(p.character);
      return [...(order ? [el('span', { className: 'vs-small', textContent: 'vs' })] : []), icon,
        el('span', { className: `p${p.player + 1}-text`, textContent: `${name}${p.controller === CONTROLLER.cpu ? ` CPU ${p.cpuLevel}` : ''}` })];
    };
    matchup.replaceChildren(...players.flatMap(side),
      el('span', { className: 'stocks-note', textContent: rulesSummary(rules) }));
    for (const tile of stageTiles) {
      const id = tile.dataset.stage;
      if (id === undefined || id === RANDOM) continue;
      const reason = app.stage_unavailable(Number(id));
      tile.classList.toggle('unavailable', !!reason);
      tile.title = reason ? `Not with these players and rules: ${reason}` : '';
    }
  }
  if (entered) {
    heroShown = null;
    stageTiles[stageCursor]?.focus({ preventScroll: true });
    showHero(stageTiles[stageCursor].dataset.stage);
  }
  art.hydrate($('screen-stages'));
  pollPreviews();
}
/** Stage previews are rendered by the core (newer builds) and not ready yet. */
const previewsPending = () => typeof app.stage_previews_ready === 'function' && !app.stage_previews_ready();
/** Stage previews render in the core after the art loads; look again until they are all in. */
function pollPreviews() {
  if (previewPoll || typeof app.stage_previews_ready !== 'function') return;
  previewPoll = setTimeout(() => {
    previewPoll = 0;
    const screen = devView ?? SCREENS[app.screen()];
    if (screen !== 'stages' && screen !== 'loading') return;
    const done = app.art_ready() && app.stage_previews_ready()
      && cat.stages.every((s) => art.get(ART.stagePreview, s.id) !== null);
    art.refresh();
    if (!done) pollPreviews();
  }, 400);
}

// ---- Loading.

let loadingSignature = '';
function updateLoading(entered) {
  if (entered || loadingSignature === '') {
    const selection = app.selection();
    loadingSignature = JSON.stringify(selection);
    const players = activePlayers(selection);
    const versus = $('versus');
    versus.classList.toggle('many', players.length > 2);
    versus.replaceChildren(...players.flatMap((p, order) => {
      const portrait = el('img', { alt: '' });
      portrait.dataset.art = `${ART.portrait}:${p.character}:${p.costume}`;
      const side = el('div', { className: `versus-side ${order % 2 ? 'mirrored' : ''}` }, portrait,
        el('p', { className: 'vs-name' },
          el('span', { className: 'vs-tag', textContent: p.controller === CONTROLLER.cpu ? 'CPU' : `P${p.player + 1}` }),
          characterName(p.character)));
      side.dataset.side = p.player;
      return order ? [el('div', { className: 'vs display', ariaHidden: 'true' }, el('span', { textContent: 'VS' })), side] : [side];
    }));
    const background = $('loading-bg');
    if (chosenStage !== null) {
      const preview = el('img', { alt: '' });
      preview.dataset.art = `${ART.stagePreview}:${chosenStage}:0`;
      background.replaceChildren(stagePlaceholder(chosenStage), preview);
    }
    art.hydrate($('screen-loading'));
  }
  const p = devView === 'loading' ? { filesDone: 5, filesTotal: 9, bytesDone: 9e6, bytesTotal: 15e6 } : app.load_progress();
  const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;
  const fraction = p.bytesTotal ? p.bytesDone / p.bytesTotal : 1;
  $('load-fill').style.width = `${fraction * 100}%`;
  $('screen-loading').querySelector('.load-track').setAttribute('aria-valuenow', Math.round(fraction * 100));
  $('load-detail').textContent = p.filesTotal
    ? `${p.filesDone} of ${p.filesTotal} files · ${mb(p.bytesDone)} of ${mb(p.bytesTotal)}`
    : 'Starting the match…';
}

// ---- Match HUD.

/** Melee's damage colour: white at 0%, orange at 100%, deep red from 200%. */
function percentColour(percent) {
  const stops = [[0, [255, 255, 255]], [100, [255, 160, 60]], [200, [208, 24, 40]]];
  const p = Math.min(200, Math.max(0, percent));
  const [a, b] = p <= 100 ? [stops[0], stops[1]] : [stops[1], stops[2]];
  const t = (p - a[0]) / (b[0] - a[0]);
  const c = a[1].map((v, i) => Math.round(v + (b[1][i] - v) * t));
  return `rgb(${c[0]} ${c[1]} ${c[2]})`;
}
function faceCrop(player, className) {
  const face = el('div', { className });
  const crop = el('div', { className: 'art' });
  crop.dataset.art = `${ART.portrait}:${player.character}:${player.costume}`;
  crop.dataset.crop = 'face';
  const stock = el('img', { className: 'stock-big', alt: '' });
  stock.dataset.art = `${ART.stock}:${player.character}:${player.costume}`;
  face.append(crop, stock);
  return face;
}
function stockIcons(container, player, max) {
  const icons = [];
  const shown = player.stocks > max ? 1 : player.stocks;
  for (let i = 0; i < shown; i++) {
    const icon = el('img', { alt: '' });
    icon.dataset.art = `${ART.stock}:${player.character}:${player.costume}`;
    icons.push(icon);
  }
  if (player.stocks > max) icons.push(el('span', { className: 'more', textContent: `×${player.stocks}` }));
  container.replaceChildren(...icons);
  container.setAttribute('aria-label', `${player.stocks} stock${player.stocks === 1 ? '' : 's'}`);
}

let hudPlayers = '', hudValues = [];
function updateMatch() {
  const hud = app.hud();
  const players = hud?.players ?? [];
  const container = $('hud');
  const signature = players.map((p) => `${p.port}:${p.character}:${p.costume}:${p.team}`).join();
  if (signature !== hudPlayers) {
    hudPlayers = signature;
    hudValues = [];
    container.classList.toggle('many', players.length > 2);
    container.replaceChildren(...players.map((p) => {
      const plate = el('div', { className: `plate ${portClass(p.port)} ${p.team >= 0 ? `team-${p.team}` : ''}`, role: 'group',
        ariaLabel: `${p.cpu ? 'CPU' : `P${p.port + 1}`} ${characterName(p.character)}` });
      plate.append(el('div', { className: 'plate-stocks' }),
        el('div', { className: 'plate-body' }, faceCrop(p, 'plate-face'),
          el('span', { className: 'plate-tag', textContent: p.cpu ? 'CPU' : `P${p.port + 1}` }),
          el('div', { className: 'plate-percent', ariaLive: 'off' })));
      return plate;
    }));
  }
  const timer = $('match-timer');
  const clock = hud?.secondsLeft == null ? ''
    : `${Math.floor(hud.secondsLeft / 60)}:${String(hud.secondsLeft % 60).padStart(2, '0')}`;
  const timerSignature = `${clock}/${hud?.suddenDeath}`;
  if (timer.dataset.signature !== timerSignature) {
    timer.dataset.signature = timerSignature;
    timer.replaceChildren(...(clock ? [el('span', { className: hud.secondsLeft <= 10 ? 'low' : '', textContent: clock })] : []),
      ...(hud?.suddenDeath ? [el('span', { className: 'sudden', textContent: 'Sudden Death' })] : []));
  }
  let changed = false;
  players.forEach((p, i) => {
    const percent = Math.floor(p.percent);
    const before = hudValues[i];
    if (before?.percent === percent && before?.stocks === p.stocks && before?.score === p.score) return;
    hudValues[i] = { percent, stocks: p.stocks, score: p.score };
    changed = true;
    const plate = container.children[i];
    const text = plate.querySelector('.plate-percent');
    text.replaceChildren(String(percent), el('small', { textContent: '%' }));
    text.style.setProperty('--pct', percentColour(percent));
    if (hud.timedScores) {
      if (before?.score !== p.score) {
        plate.querySelector('.plate-stocks').replaceChildren(
          el('span', { className: 'plate-score', textContent: p.score > 0 ? `+${p.score}` : String(p.score) }));
      }
    } else if (before?.stocks !== p.stocks) stockIcons(plate.querySelector('.plate-stocks'), p, 5);
    if (before && percent > before.percent && !reducedMotion.matches) {
      const kick = Math.min(6, 2 + (percent - before.percent) / 4);
      text.animate([{ transform: 'translate(0,0)' }, { transform: `translate(${kick}px,${-kick}px)` },
        { transform: `translate(${-kick}px,${kick / 2}px)` }, { transform: 'translate(0,0)' }],
      { duration: 160, easing: 'ease-out' });
    }
  });
  if (changed) art.hydrate(container);
  const pause = $('pause');
  const paused = app.is_paused() && !app.faulted() && (document.hasFocus() || devView !== null);
  if (pause.hidden === paused) {
    pause.hidden = !paused;
    if (paused) pause.querySelector('.btn').focus({ preventScroll: true });
  }
}

// ---- Results.

function updateResults(entered) {
  const results = app.results() ?? devResults;
  if (!results) return;
  if (!entered) return;
  $('hud').replaceChildren();
  hudPlayers = '';
  const players = results.hud.players;
  const team = results.winningTeam ?? -1;
  const winner = players[results.winner] ?? (team >= 0 ? players.find((p) => p.team === team) : undefined);
  const view = $('screen-results').querySelector('.results');
  view.classList.remove('port-0', 'port-1', 'port-2', 'port-3', 'draw');
  view.classList.add(winner ? portClass(winner.port) : 'draw');
  $('results-kicker').textContent = team >= 0 ? `${TEAMS[team]} team` : winner ? (winner.cpu ? 'CPU' : `Player ${winner.port + 1}`) : 'Draw';
  $('results-title').textContent = winner ? 'Winner' : 'No contest';
  $('results-name').replaceChildren(...(winner && team < 0 ? [el('span', { textContent: characterName(winner.character) })] : []));
  const portrait = $('results-portrait');
  portrait.dataset.art = winner ? `${ART.portrait}:${winner.character}:${winner.costume}` : '';
  $('results-emblem').dataset.art = winner ? `${ART.characterEmblem}:${winner.character}:0` : '';
  $('results-table').replaceChildren(...players.map((p, i) => {
    const icons = el('div', { className: 'stock-icons' });
    if (results.hud.timedScores) icons.append(el('span', { textContent: p.score > 0 ? `+${p.score}` : String(p.score) }));
    else if (p.stocks > 0) stockIcons(icons, p, 4);
    else icons.append(el('span', { className: 'none', textContent: 'out' }));
    return el('div', { className: `result-row ${portClass(p.port)} ${i === results.winner ? 'winner' : ''}`, role: 'row' },
      el('span', { className: 'tag', textContent: `P${p.port + 1}` }),
      faceCrop(p, 'face'),
      el('span', { className: 'who', textContent: characterName(p.character) }),
      icons,
      el('span', { className: 'pct', textContent: `${Math.floor(p.percent)}%`, style: `color: ${percentColour(p.percent)}` }));
  }));
  art.hydrate($('screen-results'));
  if (entered) $('screen-results').querySelector('.btn.accent').focus({ preventScroll: true });
}

function saveReplay() {
  try {
    const bytes = app.replay_bytes();
    const link = el('a', { href: URL.createObjectURL(new Blob([bytes], { type: 'application/json' })),
      download: `melee-replay-${new Date().toISOString().replaceAll(':', '-')}.json` });
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 10000);
  } catch (e) {
    message('Could not save the replay', e.message ?? String(e), [], 'danger');
  }
}

// ---- Buttons with data-action.

/** The top bar's stepper: stocks, or a Time match's minutes. */
function stepStocks(step) {
  const rules = app.rules();
  if (rules.mode === 1) app.set_rules({ timeMinutes: Math.min(99, Math.max(1, rules.timeMinutes + step)) });
  else app.set_rules({ stocks: Math.min(99, Math.max(1, rules.stocks + step)) });
}
const ACTIONS_BY_NAME = {
  back: () => { loadToken++; app.back(); },
  confirm: () => app.confirm_characters(),
  'stocks-down': () => stepStocks(-1),
  'stocks-up': () => stepStocks(1),
  rules: () => { buildRules(); $('rules').showModal(); },
  'rules-done': () => { $('rules').close(); updateCharacters(); },
  resume: () => { app.set_paused(false); lastFrame = null; },
  restart: () => { app.restart(); lastFrame = null; },
  quit: () => app.quit_to_menu(),
  rematch: () => app.rematch(randomSeed()),
  'stage-select': () => { app.quit_to_menu(); app.confirm_characters(); },
  'save-replay': saveReplay,
  controllers: () => openControllers(),
  'controllers-done': () => closeDialog('controllers'),
  'app-menu': () => openAppMenu(),
  'menu-close': () => closeDialog('app-menu'),
  'menu-fullscreen': () => toggleFullscreen(),
  'menu-help': () => { closeDialog('app-menu'); showHelp(); },
};
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action]')?.dataset.action;
  if (!action) return;
  if (devView && action !== 'save-replay') { devView = null; devResults = null; }
  attempt(ACTIONS_BY_NAME[action]);
});

// ---- Gamepads: menu events to actions (docs/APP.md "Pad parity"). A pad
// does everything the keyboard does: the D-pad or stick moves focus among
// the controls of the top layer (dialogs first; the core's focus_next picks
// the neighbour, as on macOS), A presses, B backs out, Start readies,
// resumes and rematches, X/Y change costume, L/R change player (or stage,
// or the pad being set up), Z or Select/Home open the app menu.

const DIALOGS = ['message', 'controllers', 'app-menu', 'rules'];
const anyDialogOpen = () => DIALOGS.some((id) => $(id).open);
const currentScreen = () => devView ?? SCREENS[app.screen()];
let toastTimer = 0;
function toast(text) {
  const node = $('toast');
  node.textContent = text;
  node.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { node.hidden = true; }, 2200);
}
function closeDialog(id) {
  if ($(id).open) $(id).close();
  updatePadHints();
}
function openAppMenu() {
  if (currentScreen() === 'match' && !app.is_paused() && !app.faulted()) { app.set_paused(true); lastFrame = null; render(); }
  $('fullscreen-label').textContent = document.fullscreenElement ? 'Exit Full Screen' : 'Full Screen';
  if (!$('app-menu').open) $('app-menu').showModal();
  $('app-menu').querySelector('.btn').focus({ preventScroll: true });
  updatePadHints();
}
let lastPad = null;
function openControllers() {
  closeDialog('app-menu');
  const body = $('controllers-body');
  body.dataset.handle = lastPad ?? '';
  if (!$('controllers').open) $('controllers').showModal();
  refreshControllers();
  updatePadHints();
}
function refreshControllers() {
  if (!$('controllers').open) return;
  const body = $('controllers-body');
  renderControllers(body, pads, padLabels, body.dataset.handle === '' ? null : Number(body.dataset.handle),
    { onDone: () => closeDialog('controllers') });
}
function refreshControllersLive() {
  const body = $('controllers-body');
  updateLive(body, pads.info(Number(body.dataset.handle)));
}
function toggleFullscreen() {
  closeDialog('app-menu');
  const request = document.fullscreenElement ? document.exitFullscreen() : document.documentElement.requestFullscreen();
  request?.catch?.(() => message('Full screen', 'The browser allows full screen only right after a click or a key '
    + 'press; a gamepad button does not count. Press F11 (or use the browser\'s View menu).'));
}
function showHelp() {
  message('Playing', 'Open your Super Smash Bros. Melee NTSC-U 1.02 disc image (.iso), pick up to four fighters '
    + '(human or CPU; Rules for teams, items and modes) and a stage.\n\n'
    + 'Keyboards drive the human players in port order: 1st W A S D move, K attack, J special, Space jump, '
    + 'L shield, I grab; 2nd arrows, N, M, comma, period, slash. Esc pauses.\n\n'
    + 'Gamepads work as GameCube controllers, buttons by position: press a button on character select to join. '
    + 'D-pad or stick moves, A chooses, B backs out, X/Y change costume, L/R change player, Start pauses and '
    + 'readies, Z or Select/Home opens this menu (Controllers for mappings).');
}
function padsChanged() {
  pads.save();
  if (currentScreen() === 'characters') updateCharacters();
  refreshControllers();
  updatePadHints();
}

/** Context hints in GameCube terms while a pad is connected. */
function updatePadHints() {
  const node = $('pad-hints');
  const screen = currentScreen();
  const paused = screen === 'match' && !$('pause').hidden;
  const hints = !pads?.count ? null
    : $('message').open ? [['A', 'Choose'], ['B', 'Close']]
      : $('controllers').open ? [['A', 'Choose'], ['B', 'Done'], ['L/R', 'Controller']]
        : $('app-menu').open || $('rules').open ? [['A', 'Choose'], ['B', 'Close']]
          : { disc: [['A', 'Choose'], ['Z', 'Menu']],
            characters: [['A', 'Pick'], ['B', 'Back'], ['X/Y', 'Costume'], ['L/R', 'Player'], ['Start', 'Ready'], ['Z', 'Menu']],
            stages: [['A', 'Fight'], ['B', 'Back'], ['L/R', 'Stage'], ['Z', 'Menu']],
            loading: [['B', 'Cancel']],
            match: paused ? [['A', 'Choose'], ['B', 'Resume'], ['Start', 'Resume'], ['Z', 'Menu']] : null,
            results: [['A', 'Choose'], ['Start', 'Rematch'], ['B', 'Characters'], ['Z', 'Menu']] }[screen];
  node.hidden = !hints;
  const signature = JSON.stringify(hints);
  if (!hints || node.dataset.signature === signature) return;
  node.dataset.signature = signature;
  node.replaceChildren(...hints.map(([glyph, text]) => el('span', { className: 'pad-hint' },
    el('b', { className: `glyph glyph-${glyph.toLowerCase().replace('/', '')}`, textContent: glyph }), text)));
}

/** The layer pad focus moves in: an open dialog, the pause card, or the screen. */
function topLayer() {
  const dialog = DIALOGS.find((id) => $(id).open);
  if (dialog) return $(dialog);
  if (!$('screen-match').hidden && !$('pause').hidden) return $('pause');
  return $(`screen-${currentScreen()}`);
}
const visible = (node) => node.getClientRects().length > 0 && getComputedStyle(node).visibility !== 'hidden';
function focusables(layer) {
  return [...layer.querySelectorAll('button, a.btn')].filter((b) => !b.disabled && visible(b));
}
/** Where focus lands in a layer: the grid or stage cursor, else the first accent button, else the first. */
function focusDefault(layer) {
  const items = focusables(layer);
  const preferred = layer.id === 'screen-characters' ? (app.selection().ready ? $('ready-banner') : tiles[characterCursor])
    : layer.id === 'screen-stages' ? stageTiles[stageCursor] : layer.querySelector('.btn.accent');
  const target = items.includes(preferred) ? preferred : items[0];
  target?.focus({ preventScroll: true });
}
const DIRECTIONS = { up: 0, down: 1, left: 2, right: 3 };
function moveFocus(layer, direction) {
  const items = focusables(layer);
  const from = items.indexOf(document.activeElement);
  if (from < 0) return focusDefault(layer);
  const frames = new Float32Array(items.flatMap((item) => {
    const box = item.getBoundingClientRect();
    return [box.left, box.top, box.width, box.height];
  }));
  const next = focus_next(frames, from, direction);
  if (next >= 0) items[next].focus({ preventScroll: true });
}
function activateFocus(layer) {
  const focused = document.activeElement;
  if (!layer.contains(focused) || !focusables(layer).includes(focused)) return focusDefault(layer);
  // Ready on the grid: A goes to stage select, as Enter does.
  if (focused.classList.contains('tile') && app.selection().ready) return attempt(ACTIONS_BY_NAME.confirm);
  focused.click();
}
/** B: close the top card, else Melee's back (a pick comes off before leaving). */
function padBack(layer, screen) {
  if (layer.id === 'message') return closeDialog('message');
  if (layer.id === 'rules') return attempt(ACTIONS_BY_NAME['rules-done']);
  if (layer.id === 'controllers' || layer.id === 'app-menu') return closeDialog(layer.id);
  if (layer.id === 'pause') return attempt(ACTIONS_BY_NAME.resume);
  if (screen === 'characters') {
    if (app.selection().players[picking].character >= 0) return attempt(() => app.choose_character(picking, -1));
    return attempt(ACTIONS_BY_NAME.back);
  }
  if (screen === 'stages' || screen === 'loading') return attempt(ACTIONS_BY_NAME.back);
  if (screen === 'results') { devView = null; return attempt(ACTIONS_BY_NAME.quit); }
}
/** Start: ready, resume, rematch; a card closes; elsewhere it presses the focused control. */
function padStart(layer, screen) {
  if (DIALOGS.includes(layer.id)) return padBack(layer, screen);
  if (layer.id === 'pause') return attempt(ACTIONS_BY_NAME.resume);
  if (screen === 'characters') return app.selection().ready ? attempt(ACTIONS_BY_NAME.confirm) : undefined;
  if (screen === 'results') { devView = null; return attempt(ACTIONS_BY_NAME.rematch); }
  if (screen !== 'loading') activateFocus(layer);
}
/** L and R: the player picking, the stage, or the pad being set up. */
function padShoulder(layer, screen, step) {
  if (layer.id === 'controllers') {
    const list = pads.list();
    const body = $('controllers-body');
    const index = list.findIndex((p) => p.handle === Number(body.dataset.handle));
    if (list.length) body.dataset.handle = list[(index + step + list.length) % list.length].handle;
    return refreshControllers();
  }
  if (layer.id === 'screen-characters') {
    picking = (picking + step + SLOTS) % SLOTS;
    const mine = app.selection().players[picking].character;
    if (mine >= 0 && document.activeElement?.classList.contains('tile')) tiles[mine].focus({ preventScroll: true });
    return updateCharacters();
  }
  if (layer.id === 'screen-stages' && !shuffling) {
    stageTiles[(stageCursor + step + stageTiles.length) % stageTiles.length].focus({ preventScroll: true });
  }
}
function padKey(key) {
  const layer = topLayer();
  const screen = currentScreen();
  if (key in DIRECTIONS) return moveFocus(layer, DIRECTIONS[key]);
  switch (key) {
    case 'confirm': return activateFocus(layer);
    case 'back': return padBack(layer, screen);
    case 'start': return padStart(layer, screen);
    case 'x': case 'y':
      if (layer.id === 'screen-characters' && app.selection().players[picking].character >= 0) {
        attempt(() => app.cycle_costume(picking, key === 'x' ? 1 : -1));
      }
      return;
    case 'l': case 'r': return padShoulder(layer, screen, key === 'r' ? 1 : -1);
    case 'menu': return layer.id === 'app-menu' ? closeDialog('app-menu') : openAppMenu();
  }
}
const PAD_KEYS = [['up', MENU.up], ['down', MENU.down], ['left', MENU.left], ['right', MENU.right],
  ['confirm', MENU.confirm], ['back', MENU.back], ['start', MENU.start], ['x', MENU.x], ['y', MENU.y],
  ['l', MENU.l], ['r', MENU.r], ['menu', MENU.z | MENU.menu]];
function padEvents(events, handle) {
  lastPad = handle;
  if (events & MENU.joined) {
    const pad = pads.info(handle);
    if (pad) toast(`P${pad.port + 1}: ${pad.name} joined`);
    return padsChanged();
  }
  if (events & MENU.bound) return padsChanged();
  if (document.hidden || window.meleeCrashed) return;
  const screen = currentScreen();
  if (screen === 'match' && $('pause').hidden && !anyDialogOpen()) {
    // In play the pad is a fighter's: only Start and Select/Home are the menus'.
    if ((events & MENU.start) && !app.faulted()) { app.set_paused(!app.is_paused()); lastFrame = null; render(); }
    if (events & MENU.menu) openAppMenu();
    return;
  }
  document.body.classList.add('pad-nav');
  for (const [key, bits] of PAD_KEYS) if (events & bits) padKey(key);
  updatePadHints();
}
// The pointer takes over from a pad: focus rings return to the browser's rules.
document.addEventListener('pointermove', () => document.body.classList.remove('pad-nav'));
$('controllers').addEventListener('close', () => {
  const handle = Number($('controllers-body').dataset.handle);
  if ($('controllers-body').dataset.handle !== '' && !Number.isNaN(handle)) app.pad_listen(handle, -1);
  updatePadHints();
});
for (const id of ['app-menu', 'rules', 'message']) $(id).addEventListener('close', () => updatePadHints());

/** ?pads=a,right,...: a pad-only walk with scripted pads (pads.js FakePads), logged to the console. */
function startPadWalk() {
  const params = new URLSearchParams(location.search);
  const spec = params.get('pads');
  if (!spec) return;
  const steps = spec.split(',');
  const bad = steps.find((step) => !FakePads.valid(step));
  if (bad) return console.log(`pads: unknown step ${bad}`);
  pads.fake = new FakePads();
  const interval = Number(params.get('padInterval')) || 400;
  pads.fake.run(steps, interval, (step) => console.log(`pad ${step}: ${describeState()}`), () => {
    console.log(`pads done: ${describeState()}`);
    window.meleePadWalkDone = true;
  });
}
function describeState() {
  const focused = document.activeElement;
  const label = !focused || focused === document.body ? '-'
    : focused.id || focused.dataset.focusId || focused.dataset.action || focused.getAttribute('aria-label')
      || focused.textContent.trim().slice(0, 24);
  const selection = app.selection();
  const slots = selection.players.map((p, i) => `P${i + 1} ${['off', 'human', 'cpu'][p.controller]}`
    + `${p.controller === CONTROLLER.cpu ? p.cpuLevel : ''} ${characterName(p.character) || '-'}/${p.costume}`).join(', ');
  const dialogs = DIALOGS.filter((id) => $(id).open).join(' ');
  const hud = app.hud();
  const editing = $('controllers').open ? pads.info(Number($('controllers-body').dataset.handle)) : null;
  const settings = editing ? ` editing ${editing.name} listening ${padLabels.targets[editing.listening] ?? '-'} `
    + `deadzone ${editing.deadzone.toFixed(2)} click ${editing.triggerClick.toFixed(2)} [${editing.bindings
      .map((t, i) => `${(padLabels.inputs[editing.family] ?? [])[i]}=${padLabels.targets[t]}`).join(' ')}]` : '';
  return `screen ${currentScreen()}${app.is_paused() ? ' paused' : ''} focus ${label} picking P${picking + 1} `
    + `[${dialogs}${$('toast').hidden ? '' : ` toast(${$('toast').textContent})`}] rules [${rulesSummary(app.rules())}] `
    + `slots [${slots}] pads [${pads.list().map((p) => `${p.name}=${p.port < 0 ? '-' : `P${p.port + 1}`}`).join(' ')}]`
    + `${hud ? ` tick ${hud.tick}` : ''}${settings}`;
}

// ---- Keyboard.

const focusIn = (selector) => {
  const items = [...document.querySelectorAll(selector)].filter((b) => b.offsetParent && !b.disabled);
  return { items, index: items.indexOf(document.activeElement) };
};
/** Up/down (or left/right) through a column of buttons. */
function stepFocus(selector, step) {
  const { items, index } = focusIn(selector);
  if (!items.length) return;
  items[(index + step + items.length) % items.length].focus();
}
function menuKey(e, screen) {
  const onButton = document.activeElement?.matches?.('button, a');
  switch (screen) {
    case 'characters': {
      const selection = app.selection();
      const onBanner = document.activeElement === $('ready-banner');
      const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
      if (arrows[e.code]) {
        if (!onBanner && !document.activeElement?.classList.contains('tile')) {
          tiles[characterCursor].focus({ preventScroll: true });
        } else if (onBanner) {
          if (e.code === 'ArrowUp') tiles[characterCursor].focus({ preventScroll: true });
        } else {
          if (e.code === 'ArrowDown' && selection.ready
              && tiles[characterCursor].parentElement === tiles[characterCursor].parentElement.parentElement.lastChild) {
            $('ready-banner').focus({ preventScroll: true });
          } else moveCursor(...arrows[e.code]);
        }
        return true;
      }
      if (e.code === 'Tab') { picking = (picking + (e.shiftKey ? SLOTS - 1 : 1)) % SLOTS; updateCharacters(); return true; }
      if (e.code === 'Enter') {
        // Ready: Enter starts from the grid too (like Start); other buttons keep their own Enter.
        const onGrid = document.activeElement?.classList.contains('tile') || onBanner;
        if (selection.ready && (onGrid || !onButton)) { attempt(ACTIONS_BY_NAME.confirm); return true; }
        if (!onButton) { choose(characterCursor); return true; }
        return false;
      }
      if (['KeyQ', 'KeyE', 'BracketLeft', 'BracketRight'].includes(e.code)) {
        if (selection.players[picking].character >= 0) {
          attempt(() => app.cycle_costume(picking, e.code === 'KeyQ' || e.code === 'BracketLeft' ? -1 : 1));
        }
        return true;
      }
      if (e.code === 'Backspace' || e.code === 'Delete') {
        if (selection.players[picking].character >= 0) attempt(() => app.choose_character(picking, -1));
        return true;
      }
      if (e.code === 'Minus' || e.code === 'Equal') {
        attempt(ACTIONS_BY_NAME[e.code === 'Minus' ? 'stocks-down' : 'stocks-up']);
        return true;
      }
      if (e.code === 'Escape') { attempt(ACTIONS_BY_NAME.back); return true; }
      return false;
    }
    case 'stages': {
      if (shuffling) return true;
      if (e.code === 'ArrowLeft' || e.code === 'ArrowRight') {
        const step = e.code === 'ArrowLeft' ? -1 : 1;
        stageTiles[(stageCursor + step + stageTiles.length) % stageTiles.length].focus({ preventScroll: true });
        return true;
      }
      if (e.code === 'Enter' && !onButton) { stageTiles[stageCursor].click(); return true; }
      if (e.code === 'Escape') { attempt(ACTIONS_BY_NAME.back); return true; }
      return false;
    }
    case 'loading':
      if (e.code === 'Escape') { attempt(ACTIONS_BY_NAME.back); return true; }
      return false;
    case 'results':
      if (e.code === 'ArrowLeft' || e.code === 'ArrowUp') { stepFocus('#screen-results .btn', -1); return true; }
      if (e.code === 'ArrowRight' || e.code === 'ArrowDown') { stepFocus('#screen-results .btn', 1); return true; }
      if (e.code === 'Enter' && !onButton) { devView = null; attempt(ACTIONS_BY_NAME.rematch); return true; }
      if (e.code === 'Escape') { devView = null; attempt(ACTIONS_BY_NAME.quit); return true; }
      return false;
    default:
      return false;
  }
}
document.addEventListener('keydown', (e) => {
  if (anyDialogOpen() || !app || e.metaKey || e.ctrlKey || e.altKey) return;
  const screen = devView ?? SCREENS[app.screen()];
  if (screen === 'match') {
    if (e.code === 'Escape') {
      e.preventDefault();
      if (!e.repeat && !app.faulted()) { app.set_paused(!app.is_paused()); lastFrame = null; render(); }
      return;
    }
    if (!$('pause').hidden) {
      if (e.code === 'ArrowUp' || e.code === 'ArrowDown') {
        e.preventDefault();
        stepFocus('#pause .btn', e.code === 'ArrowUp' ? -1 : 1);
      }
      return; // Enter and Space reach the focused button
    }
    const binding = KEYS[e.code];
    if (binding) {
      e.preventDefault();
      if (!e.repeat) app.action(binding[0], ACTIONS[binding[1]], true);
    }
    return;
  }
  if (menuKey(e, screen)) e.preventDefault();
});
// Releases always reach the core, so a key held into the pause is not stuck.
document.addEventListener('keyup', (e) => {
  const binding = KEYS[e.code];
  if (binding && app?.screen() === 4) app.action(binding[0], ACTIONS[binding[1]], false);
});
window.addEventListener('blur', () => { app?.set_focused(false); lastFrame = null; if (app) render(); });
window.addEventListener('focus', () => { app?.set_focused(true); lastFrame = null; if (app) render(); });

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
  pads.poll(now);
  if ($('controllers').open) refreshControllersLive();
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
    else if (screen === 4 && !devView) updateMatch();
  } else {
    lastFrame = null;
  }
}

// ---- Development: ?disc=<url> reads an image served with HTTP Range
// (tools/run-web.sh --disc); ?autostart=P1:P2[:Stage] walks the menus;
// ?screen=stages|loading|pause|results|draw|error|disc-error shows that
// screen over the reached state, for screenshots.

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
  const view = params.get('screen');
  if (view === 'disc-error') return showDiscError('Melee (PAL).iso', 'this is the PAL release (GALM01); Melee NTSC-U 1.02 is GALE01 revision 2');
  if (!params.get('disc')) return;
  await openDisc(await RemoteFile.open(params.get('disc')));
  const [p1, p2, stage] = (params.get('autostart') ?? '').split(':');
  const key = (list, name) => list.find((x) => x.key === name)?.id;
  if (p1) attempt(() => { app.choose_character(0, key(cat.characters, p1)); picking = 1; });
  if (p2) attempt(() => { app.choose_character(1, key(cat.characters, p2)); picking = 1; });
  if (p1 && params.get('costumes')) {
    const [c1, c2] = params.get('costumes').split(':').map(Number);
    attempt(() => { app.set_costume(0, c1 || 0); if (p2) app.set_costume(1, c2 || 0); });
  }
  if (stage) attempt(() => { app.confirm_characters(); startStage(key(cat.stages, stage)); });
  else if (view === 'stages' || view === 'loading') attempt(() => app.confirm_characters());
  if (!view) return;
  await new Promise((resolve) => setTimeout(resolve, 600));
  if (view === 'loading') { chosenStage ??= 1; devView = 'loading'; }
  if (view === 'pause') { app.set_paused(true); devView = 'match'; }
  if (view === 'results' || view === 'draw') {
    // Let the match run a moment for a believable HUD.
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const hud = app.hud();
    if (hud) {
      app.set_paused(true);
      devResults = { winner: view === 'draw' ? -1 : 1, hud: { ...hud,
        players: hud.players.map((p, i) => ({ ...p, percent: i ? 84.3 : 162.9, stocks: i ? 2 : 0 })) } };
      devView = 'results';
    }
  }
  if (view === 'error') {
    message('The match stopped on a fault', "panicked at crates/ft-fox/src/special.rs:412:9:\nnot implemented: ftFox_SpecialHi: wall bounce (ftfox_specialhi.c:688)",
      [['Save Replay', saveReplay], ['Quit to Character Select', () => {}]], 'fault');
  }
  if (view === 'notice') message('Melee', 'Yoshi\'s Story cannot be presented yet: its background renderer is not ported. Choose another stage.', [], 'danger');
  render();
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
  padLabels = pad_labels();
  pads = new Pads(app, { onEvents: padEvents, onChange: padsChanged });
  buildCharacters();
  buildStages();
  fitCanvas();
  try {
    app.set_surface(await create_surface(canvas));
  } catch (e) {
    return window.meleeFatal('WebGPU is unavailable', e.message ?? String(e));
  }
  render();
  requestAnimationFrame(frame);
  await developmentStart();
  startPadWalk();
}
main();
