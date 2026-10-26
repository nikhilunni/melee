// Gamepads on the page (docs/APP.md "Gamepads", "Pad parity"): reading
// the Gamepad API every frame, saved settings, the controllers dialog and
// the scripted pads of ?pads=. The core (WebApp.pad_*) does the mapping,
// joining and menu repeat; app.js turns menu events into actions.

/** melee_menu_event_e bits. */
export const MENU = {
  up: 1 << 0, down: 1 << 1, left: 1 << 2, right: 1 << 3, confirm: 1 << 4, back: 1 << 5, x: 1 << 6, y: 1 << 7,
  l: 1 << 8, r: 1 << 9, z: 1 << 10, start: 1 << 11, menu: 1 << 12, joined: 1 << 16, bound: 1 << 17,
};
/** W3C standard gamepad controls (melee_pad_input_e). */
const INPUTS = 17;
const PREFS_KEY = 'melee.padPrefs';

const el = (tag, props = {}, ...children) => {
  const node = Object.assign(document.createElement(tag), props);
  for (const child of children) if (child != null) node.append(child);
  return node;
};

/**
 * Connected pads: `poll(now)` reports each to the core every animation
 * frame on every screen and hands its menu events to `onEvents(events,
 * handle)`; `onChange()` runs when pads come, go or change settings.
 */
export class Pads {
  constructor(app, { onEvents, onChange }) {
    Object.assign(this, { app, onEvents, onChange });
    this.connected = new Map(); // Gamepad.index -> Gamepad.id
    this.fake = null;
    this.buttons = new Float32Array(INPUTS);
    this.axes = new Float32Array(4);
    try {
      const saved = localStorage.getItem(PREFS_KEY);
      if (saved) app.pad_load_prefs(saved);
    } catch { /* storage unavailable: defaults */ }
  }
  get count() { return this.connected.size; }
  /** Every connected pad's state (WebApp.pads()). */
  list() { return this.app.pads(); }
  info(handle) { return this.list().find((p) => p.handle === handle); }
  save() {
    try { localStorage.setItem(PREFS_KEY, this.app.pad_prefs()); } catch { /* private mode: not kept */ }
  }
  gamepads() {
    const real = [...(navigator.getGamepads?.() ?? [])].filter((p) => p && p.connected);
    return this.fake ? real.concat(this.fake.gamepads()) : real;
  }
  poll(now) {
    const seen = new Set();
    let changed = false;
    for (const pad of this.gamepads()) {
      seen.add(pad.index);
      if (!this.connected.has(pad.index)) {
        this.connected.set(pad.index, pad.id);
        this.app.pad_connect(pad.index, pad.id);
        changed = true;
      }
      this.buttons.fill(0);
      pad.buttons.forEach((b, i) => { if (i < INPUTS) this.buttons[i] = b.value || (b.pressed ? 1 : 0); });
      this.axes.fill(0);
      pad.axes.forEach((v, i) => { if (i < 4) this.axes[i] = v; });
      const events = this.app.pad_update(pad.index, this.buttons, this.axes, now);
      if (events) this.onEvents(events, pad.index);
    }
    for (const index of [...this.connected.keys()]) {
      if (!seen.has(index)) {
        this.connected.delete(index);
        this.app.pad_disconnect(index);
        changed = true;
      }
    }
    this.fake?.afterPoll();
    if (changed) this.onChange();
  }
}

// ---- The controllers dialog: one pad at a time, everything focusable.

const TARGET_GLYPHS = { 9: 'D ↑', 10: 'D ↓', 11: 'D ←', 12: 'D →' };
const LIVE_BUTTONS = ['A', 'B', 'X', 'Y', 'Z', 'L', 'R', '←', '→', '↓', '↑'];

/**
 * Fill `root` with the settings of pad `handle` (null: the first). Each
 * control has a data-focus-id so focus survives a rebuild. Returns the
 * handle shown.
 */
export function renderControllers(root, pads, labels, handle, { onDone }) {
  const list = pads.list();
  const pad = list.find((p) => p.handle === handle) ?? list[0];
  const app = pads.app;
  const focused = document.activeElement?.dataset?.focusId;
  const edit = (change) => () => { change(pad.handle); pads.save(); pads.onChange(); };
  const button = (label, id, run, className = 'pad-button') => {
    const node = el('button', { className, textContent: label, onclick: run });
    node.dataset.focusId = id;
    return node;
  };
  const row = (title, ...controls) => el('div', { className: 'pad-row' },
    el('span', { className: 'label', textContent: title }), el('div', { className: 'pad-controls' }, ...controls));
  const children = [el('h1', { className: 'display title', textContent: 'Controllers' })];
  if (!pad) {
    children.push(el('p', { className: 'pad-note', textContent: 'No controllers connected. Connect a controller over USB '
      + 'or Bluetooth (Switch Pro, Joy-Con, DualSense, DualShock 4, Xbox, 8BitDo) or a GameCube adapter and press a '
      + 'button on it; it appears here. Browsers show a pad only after one of its buttons is pressed.' }));
  } else {
    const index = list.indexOf(pad);
    const step = (by) => () => {
      const next = list[(index + by + list.length) % list.length];
      root.dataset.handle = next.handle;
      pads.onChange();
    };
    children.push(el('div', { className: 'pad-header' },
      ...(list.length > 1 ? [button('◀', 'pads.prev', step(-1))] : []),
      el('div', {}, el('p', { className: 'pad-name', textContent: pad.name }),
        el('p', { className: 'pad-note', textContent: `${pad.familyName}${list.length > 1 ? ` · ${index + 1} of ${list.length}` : ''}` })),
      ...(list.length > 1 ? [button('▶', 'pads.next', step(1))] : [])));
    children.push(el('p', { className: 'pad-note', textContent: 'Buttons map by position, as on a GameCube controller: '
      + 'the bottom face button is A, the left one B, the right X, the top Y (on a Nintendo pad the button labelled B '
      + 'is the GameCube\'s A).' }));
    const port = (label, value) => {
      const node = button(label, `pads.port.${value}`, edit((h) => app.pad_assign(h, value)));
      node.classList.toggle('on', pad.port === value);
      return node;
    };
    children.push(row('Player', port('None', -1), ...[0, 1, 2, 3].map((p) => port(`P${p + 1}`, p))));
    const percent = (v) => el('output', { textContent: `${Math.round(v * 100)}%` });
    children.push(row('Stick deadzone',
      button('−', 'pads.deadzone.down', edit((h) => app.pad_set_deadzone(h, pad.deadzone - 0.02))),
      percent(pad.deadzone),
      button('+', 'pads.deadzone.up', edit((h) => app.pad_set_deadzone(h, pad.deadzone + 0.02)))));
    children.push(row('Trigger click',
      button('−', 'pads.click.down', edit((h) => app.pad_set_trigger_click(h, pad.triggerClick - 0.05))),
      percent(pad.triggerClick),
      button('+', 'pads.click.up', edit((h) => app.pad_set_trigger_click(h, pad.triggerClick + 0.05))),
      button('Swap A and B', 'pads.swap', edit((h) => app.pad_swap_ab(h))),
      button('Reset to defaults', 'pads.reset', edit((h) => app.pad_reset(h)))));
    const family = labels.inputs[pad.family] ?? labels.inputs[labels.inputs.length - 1];
    const bindings = el('div', { className: 'pad-bindings' });
    for (let target = 1; target < labels.targets.length; target++) {
      const controls = pad.bindings.map((t, input) => (t === target ? family[input] : null)).filter(Boolean);
      const waiting = pad.listening === target;
      bindings.append(el('div', { className: 'pad-binding' },
        el('span', { className: 'glyph', textContent: TARGET_GLYPHS[target] ?? labels.targets[target] }),
        el('span', { className: `bound ${waiting ? 'waiting' : ''}`,
          textContent: waiting ? 'Press a control…' : controls.length ? controls.join(', ') : 'Unbound' }),
        button('Change', `pads.bind.${target}`, edit((h) => app.pad_listen(h, target)))));
    }
    children.push(bindings);
    children.push(el('p', { className: 'pad-live', ariaLabel: 'Live input' }));
  }
  const done = button('Done', 'pads.done', onDone, 'btn accent');
  done.replaceChildren(el('span', { textContent: 'Done' }));
  children.push(el('div', { className: 'rules-buttons' }, done));
  root.replaceChildren(...children);
  root.dataset.handle = pad?.handle ?? '';
  updateLive(root, pad);
  const again = focused && root.querySelector(`[data-focus-id="${focused}"]`);
  (again || (root.contains(document.activeElement) ? null : root.querySelector('button')))?.focus({ preventScroll: true });
  return pad?.handle ?? null;
}

/** The live readout: sticks, triggers and held GameCube buttons. */
export function updateLive(root, pad) {
  const live = root.querySelector('.pad-live');
  if (!live || !pad) return;
  const held = LIVE_BUTTONS.filter((_, bit) => pad.buttons & (1 << bit));
  const text = `Stick ${pad.stick[0]}, ${pad.stick[1]}   C ${pad.cstick[0]}, ${pad.cstick[1]}   `
    + `L ${pad.triggers[0]}  R ${pad.triggers[1]}   ${held.join(' ')}`;
  if (live.textContent !== text) live.textContent = text;
}

// ---- ?pads=a,right,...: scripted pads for a pad-only walk.

/** Steps by position (as macOS's MELEE_APP_PADS): a b x y are the bottom, left, right and top face buttons. */
const STEP_BUTTONS = { a: 0, b: 2, x: 1, y: 3, z: 5, l: 6, r: 7, select: 8, start: 9, home: 16,
  up: 12, down: 13, left: 14, right: 15 };
const STEP_STICKS = { 'stick-up': [0, -1], 'stick-down': [0, 1], 'stick-left': [-1, 0], 'stick-right': [1, 0] };

/**
 * Two fake standard gamepads (a Switch Pro Controller and a DualSense).
 * Each step taps one control for one poll on pad 1 (`2:a` on pad 2);
 * `report(step)` runs after it. Returns null for an unknown step.
 */
export class FakePads {
  static ids = ['Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)',
    'DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)'];
  constructor() {
    this.pads = FakePads.ids.map((id, i) => ({ id, index: 100 + i, connected: true, mapping: 'standard',
      buttons: Array.from({ length: INPUTS }, () => ({ pressed: false, value: 0 })), axes: [0, 0, 0, 0] }));
    this.release = [];
  }
  gamepads() { return this.pads; }
  static valid(step) {
    const name = step.replace(/^[12]:/, '');
    return name in STEP_BUTTONS || name in STEP_STICKS || /^wait(:[\d.]+)?$/.test(name);
  }
  tap(step) {
    const match = /^([12]):(.*)$/.exec(step);
    const pad = this.pads[match ? Number(match[1]) - 1 : 0];
    const name = match ? match[2] : step;
    if (name in STEP_BUTTONS) {
      const button = pad.buttons[STEP_BUTTONS[name]];
      Object.assign(button, { pressed: true, value: 1 });
      this.release.push(() => Object.assign(button, { pressed: false, value: 0 }));
    } else if (name in STEP_STICKS) {
      pad.axes = [...STEP_STICKS[name], 0, 0];
      this.release.push(() => { pad.axes = [0, 0, 0, 0]; });
    }
  }
  /** The tap lasts one poll. */
  afterPoll() {
    for (const release of this.release.splice(0)) release();
  }
  /** Run `steps` one per `interval` ms, calling report(step) just before the next. */
  run(steps, interval, report, done) {
    let at = 1000;
    for (const step of steps) {
      const wait = /^wait(?::([\d.]+))?$/.exec(step);
      if (wait) { at += wait[1] ? Number(wait[1]) * 1000 : interval; continue; }
      setTimeout(() => this.tap(step), at);
      setTimeout(() => report(step), at + interval * 0.9);
      at += interval;
    }
    setTimeout(done, at + 500);
  }
}
