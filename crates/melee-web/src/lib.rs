//! The browser host's binding to libmelee. It mirrors the C API in
//! `crates/melee-platform/include/melee_platform.h`: one `WebApp` over the
//! shared [`melee_platform::app::App`], plus a WebGPU canvas surface. The
//! page (`www/`) draws the menus as HTML and does the disc reads with
//! `Blob.slice()`; this crate holds no rules of its own.
#![cfg(target_arch = "wasm32")]

use js_sys::{Array, Object, Reflect, Uint8Array, Uint8ClampedArray};
use melee_platform::{
    app::{App, Hud, LoadProgress, Outcome, Screen},
    art::{ArtKind, Piece},
    catalog,
    disc::{DiscFiles, FileRequest},
    session::{panic_replay, Action},
    surface::WindowRenderer,
};
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Defined by index.html: shows the crash overlay, with the replay of
    /// the panicking match when there is one.
    #[wasm_bindgen(js_namespace = window, js_name = meleePanic)]
    fn melee_panic(message: &str, replay: Option<Uint8Array>);
}

/// wasm32 aborts on panic. Before it does, hand the page the message and
/// the running match's recording.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let message = info.to_string();
        web_sys::console::error_1(&message.clone().into());
        let replay = panic_replay::bytes().map(|bytes| Uint8Array::from(bytes.as_slice()));
        melee_panic(&message, replay);
    }));
}

fn object(fields: &[(&str, JsValue)]) -> JsValue {
    let object = Object::new();
    for (key, value) in fields {
        Reflect::set(&object, &(*key).into(), value).expect("plain object");
    }
    object.into()
}
fn progress_object(p: LoadProgress) -> JsValue {
    object(&[
        ("filesDone", p.files_done.into()),
        ("filesTotal", p.files_total.into()),
        ("bytesDone", (p.bytes_done as f64).into()),
        ("bytesTotal", (p.bytes_total as f64).into()),
    ])
}
fn file_requests(requests: &[FileRequest]) -> Array {
    requests
        .iter()
        .map(|r| {
            object(&[
                ("name", r.name.into()),
                ("start", (r.range.start as f64).into()),
                ("end", (r.range.end as f64).into()),
            ])
        })
        .collect()
}
fn error(message: impl AsRef<str>) -> JsError {
    JsError::new(message.as_ref())
}

/// `{characters: [{id, name, key, costumes, row, column}], stages: [{id, name, key}]}`.
#[wasm_bindgen]
pub fn catalog() -> JsValue {
    let characters: Array = catalog::characters()
        .iter()
        .enumerate()
        .map(|(id, c)| {
            let (row, column) = catalog::grid_cell(id as u32).unwrap_or_default();
            object(&[
                ("id", (id as u32).into()),
                ("name", c.display_name().into()),
                ("key", format!("{c:?}").into()),
                ("costumes", c.costume_count().into()),
                ("row", row.into()),
                ("column", column.into()),
            ])
        })
        .collect();
    let stages: Array = catalog::stages()
        .iter()
        .enumerate()
        .map(|(id, s)| {
            object(&[
                ("id", (id as u32).into()),
                ("name", s.display_name().into()),
                ("key", format!("{s:?}").into()),
            ])
        })
        .collect();
    object(&[("characters", characters.into()), ("stages", stages.into())])
}

/// Bytes of the disc header to read first.
#[wasm_bindgen]
pub fn header_len() -> u32 {
    gc_disc::HEADER_LEN as u32
}

/// Validate the header and return `[start, end)` of the file table to read next.
#[wasm_bindgen]
pub fn fst_range(header: &[u8]) -> Result<Vec<f64>, JsError> {
    let header = gc_disc::DiscHeader::parse(header).map_err(|e| error(e.to_string()))?;
    header.require_melee().map_err(|e| error(e.to_string()))?;
    let range = header.fst_range().map_err(|e| error(e.to_string()))?;
    Ok(vec![range.start as f64, range.end as f64])
}

/// A WebGPU canvas with its device, made before it is handed to the app.
#[wasm_bindgen]
pub struct WebSurface(WindowRenderer);

/// Request WebGPU for `canvas`. Rejects when the browser has no WebGPU.
#[wasm_bindgen]
pub async fn create_surface(canvas: web_sys::HtmlCanvasElement) -> Result<WebSurface, JsError> {
    let size = [canvas.width().max(1), canvas.height().max(1)];
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| error(e.to_string()))?;
    let renderer = WindowRenderer::new(&instance, surface, size)
        .await
        .map_err(error)?;
    Ok(WebSurface(renderer))
}

fn hud_object(hud: &Hud) -> JsValue {
    let players: Array = hud
        .players
        .iter()
        .map(|p| {
            object(&[
                ("port", (p.port.index() as u32).into()),
                ("character", catalog::character_id(p.character).into()),
                ("costume", p.costume.into()),
                ("percent", p.percent.into()),
                ("stocks", p.stocks.into()),
            ])
        })
        .collect();
    object(&[
        ("tick", (hud.tick as f64).into()),
        ("paused", hud.paused.into()),
        ("players", players.into()),
    ])
}

#[wasm_bindgen]
pub struct WebApp {
    app: App,
    surface: Option<WindowRenderer>,
}

#[wasm_bindgen]
impl WebApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            app: App::new(),
            surface: None,
        }
    }

    /// 0 disc, 1 characters, 2 stages, 3 loading, 4 match, 5 results.
    pub fn screen(&self) -> u32 {
        self.app.screen() as u32
    }
    /// A message for the user (load failure, match fault), cleared once read.
    pub fn take_notice(&mut self) -> Option<String> {
        self.app.take_notice()
    }

    // --- Disc
    /// Open a disc from its header, file table and the image's size.
    pub fn open_disc(&mut self, header: &[u8], fst: &[u8], image_len: f64) -> Result<(), JsError> {
        let disc = DiscFiles::from_parts(header, fst, image_len as u64).map_err(error)?;
        self.app.open_disc(disc).map_err(error)
    }
    /// `{gameId, title, revision, cachedFiles, cachedBytes}` or null.
    pub fn disc_info(&self) -> JsValue {
        let Some(disc) = self.app.disc() else {
            return JsValue::NULL;
        };
        object(&[
            ("gameId", disc.header().game_id.as_str().into()),
            ("title", disc.header().title.as_str().into()),
            ("revision", disc.header().revision.into()),
            ("cachedFiles", (disc.cached_files() as u32).into()),
            ("cachedBytes", (disc.cached_bytes() as f64).into()),
        ])
    }
    pub fn resume_disc(&mut self) -> Result<(), JsError> {
        self.app.resume_disc().map_err(error)
    }

    // --- Menu art
    /// `[{name, start, end}]`: the menu archives still to read; hand each
    /// to `provide_file` (any screen) after `open_disc`.
    pub fn art_files(&self) -> Array {
        file_requests(&self.app.art_requests())
    }
    pub fn art_ready(&self) -> bool {
        self.app.art_ready()
    }
    /// `{filesDone, filesTotal, bytesDone, bytesTotal}` of the menu archives.
    pub fn art_progress(&self) -> JsValue {
        progress_object(self.app.art_progress())
    }
    /// `{width, height, data}` with `data` a `Uint8ClampedArray` of RGBA8
    /// (straight alpha, native size: `new ImageData(data, width, height)`).
    /// `kind` is `melee_art_e`'s number (0 portrait, 1 face, 2 stock,
    /// 3 character emblem, 4 stage icon, 5 stage name, 6 stage emblem); `id`
    /// a character or stage id. Throws until the art is read and where
    /// retail has no such image.
    pub fn art(&mut self, kind: u32, id: u32, costume: u8) -> Result<JsValue, JsError> {
        let kind = ArtKind::from_u32(kind).ok_or(error("no art kind with that number"))?;
        let piece = Piece::new(kind, id, costume).map_err(error)?;
        let image = self.app.art_image(piece).map_err(error)?;
        let data = Uint8ClampedArray::new_with_length(image.rgba.len() as u32);
        data.copy_from(&image.rgba);
        Ok(object(&[
            ("width", image.width.into()),
            ("height", image.height.into()),
            ("data", data.into()),
        ]))
    }

    // --- Character select
    /// `{players: [{character (id or -1), costume}], stocks, ready}`.
    pub fn selection(&self) -> JsValue {
        let players: Array = self
            .app
            .slots()
            .iter()
            .map(|slot| {
                object(&[
                    (
                        "character",
                        slot.character
                            .map_or(-1, |c| catalog::character_id(c) as i32)
                            .into(),
                    ),
                    ("costume", slot.costume.into()),
                ])
            })
            .collect();
        object(&[
            ("players", players.into()),
            ("stocks", self.app.stocks().into()),
            ("ready", self.app.characters_ready().into()),
        ])
    }
    /// `character` -1 clears the pick.
    pub fn choose_character(&mut self, player: u32, character: i32) -> Result<(), JsError> {
        if character < 0 {
            return self.app.unchoose_character(player as usize).map_err(error);
        }
        let character = catalog::character(character as u32).ok_or(error("no such character"))?;
        self.app
            .choose_character(player as usize, character)
            .map_err(error)
    }
    pub fn set_costume(&mut self, player: u32, costume: u8) -> Result<(), JsError> {
        self.app
            .set_costume(player as usize, costume)
            .map_err(error)
    }
    pub fn cycle_costume(&mut self, player: u32, step: i32) -> Result<(), JsError> {
        self.app.cycle_costume(player as usize, step).map_err(error)
    }
    pub fn set_stocks(&mut self, stocks: u8) -> Result<(), JsError> {
        self.app.set_stocks(stocks).map_err(error)
    }
    pub fn confirm_characters(&mut self) -> Result<(), JsError> {
        self.app.confirm_characters().map_err(error)
    }
    pub fn back(&mut self) {
        self.app.back();
        self.sync_scene();
    }

    // --- Stage select and loading
    pub fn choose_stage(&mut self, stage: u32, seed: u32) -> Result<(), JsError> {
        let stage = catalog::stage(stage).ok_or(error("no such stage"))?;
        self.app.choose_stage(stage, seed).map_err(error)
    }
    /// `{filesDone, filesTotal, bytesDone, bytesTotal}`.
    pub fn load_progress(&self) -> JsValue {
        progress_object(self.app.load_progress())
    }
    /// `[{name, start, end}]`: the disc byte ranges still to read.
    pub fn pending_files(&self) -> Array {
        file_requests(&self.app.pending_requests())
    }
    pub fn provide_file(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), JsError> {
        self.app.provide_file(name, bytes).map_err(error)
    }
    /// The page could not read a file: back to stage select with `message`.
    pub fn fail_loading(&mut self, message: String) {
        self.app.fail_loading(message);
    }
    /// Build the match from the read files; on failure the notice says why.
    pub fn finish_loading(&mut self) -> Result<(), JsError> {
        self.app.finish_loading().map_err(error)?;
        self.sync_scene();
        if self.app.screen() != Screen::Match {
            return Err(error(
                self.app.notice().unwrap_or("the match could not be drawn"),
            ));
        }
        Ok(())
    }

    // --- Surface
    pub fn set_surface(&mut self, surface: WebSurface) {
        self.surface = Some(surface.0);
        self.sync_scene();
    }

    // --- Match
    /// Advance by `elapsed_ms` of page time and draw at the canvas size.
    pub fn frame(&mut self, elapsed_ms: f64, width: u32, height: u32) -> Result<(), JsError> {
        // Page time can step backwards, and NaN must not reach Duration.
        let seconds = if elapsed_ms > 0.0 { elapsed_ms / 1000.0 } else { 0.0 };
        let elapsed = Duration::from_secs_f64(seconds);
        let advanced = self.app.advance(elapsed);
        self.sync_scene();
        if let Some(surface) = &mut self.surface {
            if width > 0 && height > 0 {
                surface.resize([width, height]);
                surface
                    .draw(self.app.session().map(|s| s.presentation()))
                    .map_err(error)?;
            }
        }
        advanced.map_err(error)
    }
    /// Benchmarking (`?perf`): queue `frames` offscreen draws of the current
    /// scene at the canvas size. Time it with [`WebApp::gpu_idle`]; works in
    /// background tabs, where animation frames stop.
    pub fn draw_offscreen(&mut self, frames: u32) -> Result<(), JsError> {
        let surface = self.surface.as_mut().ok_or_else(|| error("no surface"))?;
        let scene = self
            .app
            .session()
            .map(|s| s.presentation())
            .ok_or_else(|| error("no match is running"))?;
        surface.draw_offscreen(scene, frames).map_err(error)
    }
    /// Resolves once the GPU has finished all submitted work.
    pub fn gpu_idle(&self) -> Result<js_sys::Promise, JsError> {
        let queue = self
            .surface
            .as_ref()
            .ok_or_else(|| error("no surface"))?
            .queue()
            .clone();
        Ok(js_sys::Promise::new(&mut |resolve, _| {
            queue.on_submitted_work_done(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
        }))
    }
    pub fn needs_frame(&self) -> bool {
        self.app.screen() == Screen::Match && self.app.session().is_some_and(|s| s.needs_frame())
    }
    /// `action`: 0 left, 1 right, 2 up, 3 down, 4 attack, 5 special, 6 jump, 7 shield, 8 grab.
    pub fn action(&mut self, player: u32, action: u32, down: bool) {
        if let Ok(action) = Action::try_from(action) {
            self.app.set_action(player as usize, action, down);
        }
    }
    pub fn set_paused(&mut self, paused: bool) {
        if let Some(session) = self.app.session_mut() {
            session.pause(paused);
        }
    }
    pub fn set_focused(&mut self, focused: bool) {
        if let Some(session) = self.app.session_mut() {
            session.set_focused(focused);
        }
    }
    pub fn is_paused(&self) -> bool {
        self.app.session().is_some_and(|s| s.is_paused())
    }
    pub fn faulted(&self) -> bool {
        self.app.session().is_some_and(|s| s.failure().is_some())
    }
    /// `{tick, paused, players: [{port, character, costume, percent, stocks}]}` or null.
    pub fn hud(&self) -> JsValue {
        self.app.hud().map_or(JsValue::NULL, |hud| hud_object(&hud))
    }
    /// `{winner (player index or -1), hud}` or null.
    pub fn results(&self) -> JsValue {
        let Some(results) = self.app.results() else {
            return JsValue::NULL;
        };
        let winner = match results.outcome {
            Outcome::Winner(index) => index as i32,
            Outcome::Draw => -1,
        };
        object(&[("winner", winner.into()), ("hud", hud_object(&results.hud))])
    }
    pub fn restart(&mut self) -> Result<(), JsError> {
        self.app.restart().map_err(error)
    }
    pub fn rematch(&mut self, seed: u32) -> Result<(), JsError> {
        self.app.rematch(seed).map_err(error)?;
        self.sync_scene();
        Ok(())
    }
    pub fn quit_to_menu(&mut self) {
        self.app.quit_to_menu();
        self.sync_scene();
    }
    /// The match's input recording (melee-replay JSON) for a download.
    pub fn replay_bytes(&self) -> Result<Vec<u8>, JsError> {
        self.app
            .session()
            .ok_or(error("no match to save"))?
            .replay_bytes()
            .map_err(error)
    }
}

impl Default for WebApp {
    fn default() -> Self {
        Self::new()
    }
}

impl WebApp {
    fn sync_scene(&mut self) {
        if let Some(surface) = &mut self.surface {
            surface.follow(&mut self.app);
        }
    }
}
