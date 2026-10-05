// The web app runtime: HTML menus, file access, keyboard and the frame
// loop. Every rule lives in libmelee (WebApp, from crates/melee-web); the
// look follows docs/DESIGN.md.
//
// The menus are built once and updated in place (classes, text, image
// sources), so hovers, focus and animations survive state changes.
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
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const SPRING = 'cubic-bezier(0.2, 0.9, 0.25, 1.15)';
let app, cat, discFile = null, picking = 0, lastFrame = null;
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

function updateCharacters(entered) {
  const selection = app.selection();
  const ready = selection.ready;
  $('stocks').textContent = selection.stocks;
  const grid = $('character-grid');
  grid.dataset.active = picking;
  for (const tile of tiles) {
    const id = Number(tile.dataset.id);
    const by = [0, 1].filter((p) => selection.players[p].character === id);
    tile.classList.toggle('taken-p1', by.includes(0));
    tile.classList.toggle('taken-p2', by.includes(1));
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
    const other = selection.players[1 - player];
    const id = slot.character;
    const character = cat.characters[id];
    panel.classList.toggle('active', picking === player && !ready);
    panel.setAttribute('aria-label', `Player ${player + 1}${character ? `: ${character.name}` : ''}`);
    panel.querySelector('.panel-portrait .main').dataset.art = character ? `${ART.portrait}:${id}:${slot.costume}` : '';
    panel.querySelector('.stock-hero').dataset.art = character ? `${ART.stock}:${id}:${slot.costume}` : '';
    panel.querySelector('.panel-emblem').dataset.art = character ? `${ART.characterEmblem}:${id}:0` : '';
    panel.querySelector('.clear-pick').hidden = !character;
    const name = panel.querySelector('.panel-name');
    name.textContent = character?.name ?? 'Choose a fighter';
    name.classList.toggle('empty', !character);
    panel.querySelector('.panel-sub').textContent = character
      ? `Costume ${slot.costume + 1} of ${character.costumes}`
      : picking === player ? 'Click a fighter or press Enter' : 'Waiting…';
    const chips = panel.querySelector('.chips');
    const signature = character ? `${id}/${slot.costume}/${other.character === id ? other.costume : -1}` : '';
    if (chips.dataset.signature !== signature) {
      chips.dataset.signature = signature;
      chips.replaceChildren(...Array.from({ length: character?.costumes ?? 0 }, (_, costume) => {
        const taken = other.character === id && other.costume === costume;
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
  $('pick-hint').innerHTML = ready ? '' : `${tag} &middot; choose your fighter`;
  if (ready && !wasReady) $('ready-banner').focus({ preventScroll: true });
  if (!ready && document.activeElement === $('ready-banner')) tiles[characterCursor]?.focus({ preventScroll: true });
  if (entered) {
    const mine = selection.players[picking].character;
    if (mine >= 0) characterCursor = mine;
    (ready ? $('ready-banner') : tiles[characterCursor])?.focus({ preventScroll: true });
  }
  updateArtStatus();
  art.hydrate($('screen-characters'));
}
function choose(id) {
  attempt(() => app.choose_character(picking, id));
  if (app.selection().players[1 - picking].character < 0) picking = 1 - picking;
  updateCharacters();
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
  chosenStage = id;
  attempt(() => app.choose_stage(id, randomSeed()));
}
/** RANDOM: a quick shuffle across the stages, then the pick. */
function shuffleStage() {
  if (shuffling) return;
  const pick = Math.floor(Math.random() * cat.stages.length);
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
  const matchup = $('matchup');
  const signature = JSON.stringify(selection);
  if (matchup.dataset.signature !== signature) {
    matchup.dataset.signature = signature;
    const side = (p, i) => {
      const icon = el('img', { alt: '' });
      icon.dataset.art = `${ART.stock}:${p.character}:${p.costume}`;
      return [icon, el('span', { className: `p${i + 1}-text`, textContent: characterName(p.character) })];
    };
    matchup.replaceChildren(...side(selection.players[0], 0), el('span', { className: 'vs-small', textContent: 'vs' }),
      ...side(selection.players[1], 1),
      el('span', { className: 'stocks-note', textContent: `${selection.stocks} stock${selection.stocks === 1 ? '' : 's'}` }));
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
    for (const side of document.querySelectorAll('.versus-side')) {
      const i = Number(side.dataset.side);
      const p = selection.players[i];
      const portrait = el('img', { alt: '' });
      portrait.dataset.art = `${ART.portrait}:${p.character}:${p.costume}`;
      side.replaceChildren(portrait, el('p', { className: 'vs-name' },
        el('span', { className: 'vs-tag', textContent: `P${i + 1}` }), characterName(p.character)));
    }
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
  const signature = players.map((p) => `${p.port}:${p.character}:${p.costume}`).join();
  if (signature !== hudPlayers) {
    hudPlayers = signature;
    hudValues = [];
    container.replaceChildren(...players.map((p) => {
      const plate = el('div', { className: `plate ${portClass(p.port)}`, role: 'group',
        ariaLabel: `P${p.port + 1} ${characterName(p.character)}` });
      plate.append(el('div', { className: 'plate-stocks' }),
        el('div', { className: 'plate-body' }, faceCrop(p, 'plate-face'),
          el('span', { className: 'plate-tag', textContent: `P${p.port + 1}` }),
          el('div', { className: 'plate-percent', ariaLive: 'off' })));
      return plate;
    }));
  }
  let changed = false;
  players.forEach((p, i) => {
    const percent = Math.floor(p.percent);
    const before = hudValues[i];
    if (before?.percent === percent && before?.stocks === p.stocks) return;
    hudValues[i] = { percent, stocks: p.stocks };
    changed = true;
    const plate = container.children[i];
    const text = plate.querySelector('.plate-percent');
    text.replaceChildren(String(percent), el('small', { textContent: '%' }));
    text.style.setProperty('--pct', percentColour(percent));
    if (before?.stocks !== p.stocks) stockIcons(plate.querySelector('.plate-stocks'), p, 5);
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
  const winner = players[results.winner];
  const view = $('screen-results').querySelector('.results');
  view.classList.remove('port-0', 'port-1', 'port-2', 'port-3', 'draw');
  view.classList.add(winner ? portClass(winner.port) : 'draw');
  $('results-kicker').textContent = winner ? `Player ${winner.port + 1}` : 'Draw';
  $('results-title').textContent = winner ? 'Winner' : 'No contest';
  $('results-name').replaceChildren(...(winner ? [el('span', { textContent: characterName(winner.character) })] : []));
  const portrait = $('results-portrait');
  portrait.dataset.art = winner ? `${ART.portrait}:${winner.character}:${winner.costume}` : '';
  $('results-emblem').dataset.art = winner ? `${ART.characterEmblem}:${winner.character}:0` : '';
  $('results-table').replaceChildren(...players.map((p, i) => {
    const icons = el('div', { className: 'stock-icons' });
    if (p.stocks > 0) stockIcons(icons, p, 4);
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

const ACTIONS_BY_NAME = {
  back: () => { loadToken++; app.back(); },
  confirm: () => app.confirm_characters(),
  'stocks-down': () => app.set_stocks(Math.max(1, app.selection().stocks - 1)),
  'stocks-up': () => app.set_stocks(Math.min(99, app.selection().stocks + 1)),
  resume: () => { app.set_paused(false); lastFrame = null; },
  restart: () => { app.restart(); lastFrame = null; },
  quit: () => app.quit_to_menu(),
  rematch: () => app.rematch(randomSeed()),
  'stage-select': () => { app.quit_to_menu(); app.confirm_characters(); },
  'save-replay': saveReplay,
};
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action]')?.dataset.action;
  if (!action) return;
  if (devView && action !== 'save-replay') { devView = null; devResults = null; }
  attempt(ACTIONS_BY_NAME[action]);
});

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
      if (e.code === 'Tab') { picking = 1 - picking; updateCharacters(); return true; }
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
        else if (selection.players[1 - picking].character >= 0) {
          picking = 1 - picking;
          attempt(() => app.choose_character(picking, -1));
        }
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
  if ($('message').open || !app || e.metaKey || e.ctrlKey || e.altKey) return;
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
}
main();
