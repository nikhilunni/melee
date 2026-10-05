//! The C API in `include/melee_platform.h`: an opaque app handle over
//! [`crate::app::App`] plus the native surface. Only plain C types cross.
use crate::{
    app::{App, Hud, Outcome, Screen},
    art::{ArtKind, Image, Piece},
    catalog,
    disc::DiscFiles,
    panic_text,
    session::Action,
    surface::WindowRenderer,
};
use std::{
    ffi::{c_char, CStr, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

/// 2: menu art (`melee_app_art` and friends).
/// 3: stage previews (`MELEE_ART_STAGE_PREVIEW`, `melee_app_stage_previews_ready`).
pub const API_VERSION: u32 = 3;

pub struct Handle {
    app: App,
    surface: Option<WindowRenderer>,
    last_frame: Option<Instant>,
    error: String,
    /// The image `melee_app_art` last returned; its pixels stay valid
    /// until the next call.
    image: Option<Arc<Image>>,
}

#[repr(C)]
pub struct ImageOut {
    width: u32,
    height: u32,
    rgba: *const u8,
    len: usize,
}

#[repr(C)]
pub struct CharacterInfo {
    name: *const c_char,
    key: *const c_char,
    costume_count: u8,
    grid_row: u8,
    grid_column: u8,
}
#[repr(C)]
pub struct StageInfo {
    name: *const c_char,
    key: *const c_char,
}
#[repr(C)]
pub struct DiscInfo {
    game_id: [c_char; 8],
    title: [c_char; 96],
    revision: u8,
    cached_files: u32,
    cached_bytes: u64,
}
#[repr(C)]
pub struct Slot {
    character: i32,
    costume: u8,
}
#[repr(C)]
pub struct Selection {
    players: [Slot; 2],
    stocks: u8,
    ready: bool,
}
#[repr(C)]
pub struct LoadProgress {
    files_done: u32,
    files_total: u32,
    bytes_done: u64,
    bytes_total: u64,
}
#[repr(C)]
#[derive(Default)]
pub struct PlayerHud {
    port: u8,
    character: u32,
    costume: u8,
    percent: f32,
    stocks: u8,
}
#[repr(C)]
#[derive(Default)]
pub struct HudOut {
    tick: u64,
    paused: bool,
    faulted: bool,
    players: [PlayerHud; 2],
}
#[repr(C)]
pub struct ResultsOut {
    winner: i32,
    hud: HudOut,
}

/// NUL-terminated copies of the catalog's names, built once.
struct Names {
    characters: Vec<(CString, CString)>,
    stages: Vec<(CString, CString)>,
}
fn names() -> &'static Names {
    static NAMES: OnceLock<Names> = OnceLock::new();
    NAMES.get_or_init(|| {
        let c = |text: &str| CString::new(text).expect("catalog names have no NUL");
        Names {
            characters: catalog::characters()
                .iter()
                .map(|ch| (c(ch.display_name()), c(&format!("{ch:?}"))))
                .collect(),
            stages: catalog::stages()
                .iter()
                .map(|st| (c(st.display_name()), c(&format!("{st:?}"))))
                .collect(),
        }
    })
}

/// Copy `text` into a C buffer, truncated and NUL-terminated; the full length.
unsafe fn copy_out(text: &str, out: *mut c_char, capacity: usize) -> usize {
    if !out.is_null() && capacity > 0 {
        let bytes = text.as_bytes();
        let n = bytes.len().min(capacity - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast(), n);
            *out.add(n) = 0;
        }
    }
    text.len()
}
fn fill<const N: usize>(text: &str) -> [c_char; N] {
    let mut out = [0; N];
    for (slot, &byte) in out.iter_mut().zip(text.as_bytes().iter().take(N - 1)) {
        *slot = byte as c_char;
    }
    out
}
unsafe fn string(text: *const c_char) -> Result<String, String> {
    if text.is_null() {
        return Err("missing string argument".into());
    }
    unsafe { CStr::from_ptr(text) }
        .to_str()
        .map(str::to_owned)
        .map_err(|e| e.to_string())
}

impl Handle {
    /// Run a call, turning errors and panics into `false` plus a message.
    fn perform(&mut self, call: impl FnOnce(&mut Self) -> Result<(), String>) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| call(self)))
            .unwrap_or_else(|panic| Err(panic_text(&*panic)));
        match result {
            Ok(()) => true,
            Err(error) => {
                self.error = error;
                false
            }
        }
    }
    /// Start the stage previews on a background thread once the disc's
    /// files are in and a surface gives a device; idempotent. A failure to
    /// start leaves the previews pending: hosts keep their placeholders.
    fn start_previews(&mut self) {
        let Some(surface) = &self.surface else {
            return;
        };
        if let Some(job) = self.app.stage_preview_job() {
            if let Err(error) = crate::preview::spawn(job, surface.preview_gpu()) {
                eprintln!("stage previews could not start: {error}");
            }
        }
    }
    /// Keep the surface's scene in step with the app's session.
    fn sync_scene(&mut self) {
        if let Some(surface) = &mut self.surface {
            surface.follow(&mut self.app);
        }
    }
    fn hud(&self) -> Option<HudOut> {
        self.app.hud().map(|hud| self.hud_out(hud))
    }
    fn hud_out(&self, hud: Hud) -> HudOut {
        HudOut {
            tick: hud.tick,
            paused: hud.paused,
            faulted: self.app.session().is_some_and(|s| s.failure().is_some()),
            players: hud.players.map(|p| PlayerHud {
                port: p.port.index() as u8,
                character: catalog::character_id(p.character),
                costume: p.costume,
                percent: p.percent,
                stocks: p.stocks,
            }),
        }
    }
}

macro_rules! handle {
    ($app:ident) => {
        match unsafe { $app.as_mut() } {
            Some(handle) => handle,
            None => return Default::default(),
        }
    };
}

#[no_mangle]
pub extern "C" fn melee_api_version() -> u32 {
    API_VERSION
}

#[no_mangle]
pub extern "C" fn melee_character_count() -> u32 {
    catalog::characters().len() as u32
}
/// # Safety
/// `out` is null or writable for one struct.
#[no_mangle]
pub unsafe extern "C" fn melee_character_info(id: u32, out: *mut CharacterInfo) -> bool {
    let (Some(character), Some(out)) = (catalog::character(id), unsafe { out.as_mut() }) else {
        return false;
    };
    let (name, key) = &names().characters[id as usize];
    let (row, column) = catalog::grid_cell(id).unwrap_or_default();
    *out = CharacterInfo {
        name: name.as_ptr(),
        key: key.as_ptr(),
        costume_count: character.costume_count(),
        grid_row: row as u8,
        grid_column: column as u8,
    };
    true
}
#[no_mangle]
pub extern "C" fn melee_stage_count() -> u32 {
    catalog::stages().len() as u32
}
/// # Safety
/// `out` is null or writable for one struct.
#[no_mangle]
pub unsafe extern "C" fn melee_stage_info(id: u32, out: *mut StageInfo) -> bool {
    let (Some(_), Some(out)) = (catalog::stage(id), unsafe { out.as_mut() }) else {
        return false;
    };
    let (name, key) = &names().stages[id as usize];
    *out = StageInfo {
        name: name.as_ptr(),
        key: key.as_ptr(),
    };
    true
}

#[no_mangle]
pub extern "C" fn melee_app_new() -> *mut Handle {
    Box::into_raw(Box::new(Handle {
        app: App::new(),
        surface: None,
        last_frame: None,
        error: String::new(),
        image: None,
    }))
}
/// # Safety
/// `app` is null or a live handle, freed exactly once. A surface's layer must
/// outlive this call.
#[no_mangle]
pub unsafe extern "C" fn melee_app_free(app: *mut Handle) {
    if !app.is_null() {
        drop(unsafe { Box::from_raw(app) });
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_screen(app: *const Handle) -> u32 {
    unsafe { app.as_ref() }.map_or(Screen::Disc, |h| h.app.screen()) as u32
}
/// # Safety
/// `app` is null or live; `buf` is null or writable for `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn melee_app_last_error(
    app: *const Handle,
    buf: *mut c_char,
    capacity: usize,
) -> usize {
    unsafe { app.as_ref() }.map_or(0, |h| unsafe { copy_out(&h.error, buf, capacity) })
}
/// # Safety
/// `app` is null or live; `buf` is null or writable for `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn melee_app_take_notice(
    app: *mut Handle,
    buf: *mut c_char,
    capacity: usize,
) -> usize {
    let h = handle!(app);
    h.app
        .take_notice()
        .map_or(0, |notice| unsafe { copy_out(&notice, buf, capacity) })
}

/// # Safety
/// `app` is null or live; `path` is a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn melee_app_open_disc(app: *mut Handle, path: *const c_char) -> bool {
    let h = handle!(app);
    h.perform(|h| {
        let path = unsafe { string(path) }?;
        let disc = DiscFiles::open_path(std::path::Path::new(&path))?;
        h.app.open_disc(disc)?;
        // Menu art is optional: a read failure leaves placeholders, and
        // melee_app_load_art retries and reports it.
        let _ = h.app.load_art();
        h.start_previews();
        Ok(())
    })
}

/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_load_art(app: *mut Handle) -> bool {
    let h = handle!(app);
    let ok = h.perform(|h| h.app.load_art());
    h.start_previews();
    ok
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_art_ready(app: *const Handle) -> bool {
    unsafe { app.as_ref() }.is_some_and(|h| h.app.art_ready())
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_stage_previews_ready(app: *const Handle) -> bool {
    unsafe { app.as_ref() }.is_some_and(|h| h.app.stage_previews_ready())
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_art_progress(app: *const Handle, out: *mut LoadProgress) {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return;
    };
    let p = h.app.art_progress();
    *out = LoadProgress {
        files_done: p.files_done,
        files_total: p.files_total,
        bytes_done: p.bytes_done,
        bytes_total: p.bytes_total,
    };
}
/// # Safety
/// `app` is null or live; `out` is null or writable. The pixels `out`
/// points at stay valid until the next `melee_app_art` call or
/// `melee_app_free`.
#[no_mangle]
pub unsafe extern "C" fn melee_app_art(
    app: *mut Handle,
    kind: u32,
    id: u32,
    costume: u8,
    out: *mut ImageOut,
) -> bool {
    let h = handle!(app);
    let Some(out) = (unsafe { out.as_mut() }) else {
        h.error = "missing image output".into();
        return false;
    };
    h.perform(|h| {
        let kind = ArtKind::from_u32(kind).ok_or("no art kind with that number")?;
        let image = h.app.art_image(Piece::new(kind, id, costume)?)?;
        *out = ImageOut {
            width: image.width,
            height: image.height,
            rgba: image.rgba.as_ptr(),
            len: image.rgba.len(),
        };
        h.image = Some(image);
        Ok(())
    })
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_disc_info(app: *const Handle, out: *mut DiscInfo) -> bool {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Some(disc) = h.app.disc() else {
        return false;
    };
    *out = DiscInfo {
        game_id: fill(&disc.header().game_id),
        title: fill(&disc.header().title),
        revision: disc.header().revision,
        cached_files: disc.cached_files() as u32,
        cached_bytes: disc.cached_bytes(),
    };
    true
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_resume_disc(app: *mut Handle) -> bool {
    handle!(app).perform(|h| h.app.resume_disc())
}

/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_selection(app: *const Handle, out: *mut Selection) {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return;
    };
    *out = Selection {
        players: h.app.slots().map(|slot| Slot {
            character: slot
                .character
                .map_or(-1, |c| catalog::character_id(c) as i32),
            costume: slot.costume,
        }),
        stocks: h.app.stocks(),
        ready: h.app.characters_ready(),
    };
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_choose_character(
    app: *mut Handle,
    player: u32,
    character: i32,
) -> bool {
    handle!(app).perform(|h| {
        if character < 0 {
            return h.app.unchoose_character(player as usize);
        }
        let character = catalog::character(character as u32).ok_or("no character with that id")?;
        h.app.choose_character(player as usize, character)
    })
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_set_costume(app: *mut Handle, player: u32, costume: u8) -> bool {
    handle!(app).perform(|h| h.app.set_costume(player as usize, costume))
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_cycle_costume(app: *mut Handle, player: u32, step: i32) -> bool {
    handle!(app).perform(|h| h.app.cycle_costume(player as usize, step))
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_set_stocks(app: *mut Handle, stocks: u8) -> bool {
    handle!(app).perform(|h| h.app.set_stocks(stocks))
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_confirm_characters(app: *mut Handle) -> bool {
    handle!(app).perform(|h| h.app.confirm_characters())
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_back(app: *mut Handle) {
    let h = handle!(app);
    h.app.back();
    h.sync_scene();
}

/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_choose_stage(app: *mut Handle, stage: u32, seed: u32) -> bool {
    handle!(app).perform(|h| {
        let stage = catalog::stage(stage).ok_or("no stage with that id")?;
        h.app.choose_stage(stage, seed)
    })
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_load_progress(app: *const Handle, out: *mut LoadProgress) {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return;
    };
    let p = h.app.load_progress();
    *out = LoadProgress {
        files_done: p.files_done,
        files_total: p.files_total,
        bytes_done: p.bytes_done,
        bytes_total: p.bytes_total,
    };
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_load_step(app: *mut Handle) -> i32 {
    let h = handle!(app);
    let mut more = false;
    if h.perform(|h| {
        more = h.app.load_next_file()?;
        Ok(())
    }) {
        i32::from(more)
    } else {
        -1
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_finish_loading(app: *mut Handle) -> bool {
    let h = handle!(app);
    let started = h.perform(|h| h.app.finish_loading());
    h.sync_scene();
    if started && h.app.screen() != Screen::Match {
        // The renderer rejected the scene; the notice says why.
        h.error = h.app.notice().unwrap_or_default().to_owned();
        return false;
    }
    started
}

/// # Safety
/// `app` is null or live. `layer` is a CAMetalLayer the host keeps alive
/// until the surface is detached or the app freed.
#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn melee_app_attach_metal_layer(
    app: *mut Handle,
    layer: *mut std::ffi::c_void,
    width: u32,
    height: u32,
) -> bool {
    let h = handle!(app);
    h.perform(|h| {
        if layer.is_null() {
            return Err("missing layer".into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))
        }
        .map_err(|e| e.to_string())?;
        h.surface = None;
        h.surface = Some(pollster::block_on(WindowRenderer::new(
            &instance,
            surface,
            [width, height],
        ))?);
        h.sync_scene();
        h.start_previews();
        Ok(())
    })
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_detach_surface(app: *mut Handle) {
    let h = handle!(app);
    h.surface = None;
}

/// # Safety
/// `app` is null or live; the attached layer is alive.
#[no_mangle]
pub unsafe extern "C" fn melee_app_frame(app: *mut Handle, width: u32, height: u32) -> bool {
    let h = handle!(app);
    h.perform(|h| {
        let now = Instant::now();
        let elapsed = h
            .last_frame
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        h.last_frame = Some(now);
        let advanced = h.app.advance(elapsed);
        h.sync_scene();
        if let Some(surface) = &mut h.surface {
            if width > 0 && height > 0 {
                surface.resize([width, height]);
                surface.draw(h.app.session().map(|s| s.presentation()))?;
            }
        }
        advanced
    })
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_needs_frame(app: *const Handle) -> bool {
    unsafe { app.as_ref() }.is_some_and(|h| {
        h.app.screen() == Screen::Match && h.app.session().is_some_and(|s| s.needs_frame())
    })
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_action(app: *mut Handle, player: u32, action: u32, down: bool) {
    let h = handle!(app);
    if let Ok(action) = Action::try_from(action) {
        h.app.set_action(player as usize, action, down);
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_set_paused(app: *mut Handle, paused: bool) {
    let h = handle!(app);
    if let Some(session) = h.app.session_mut() {
        session.pause(paused);
    }
    h.last_frame = None;
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_set_focused(app: *mut Handle, focused: bool) {
    let h = handle!(app);
    if let Some(session) = h.app.session_mut() {
        session.set_focused(focused);
    }
    h.last_frame = None;
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_hud(app: *const Handle, out: *mut HudOut) -> bool {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    h.hud().map(|hud| *out = hud).is_some()
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_results(app: *const Handle, out: *mut ResultsOut) -> bool {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Some(results) = h.app.results() else {
        return false;
    };
    *out = ResultsOut {
        winner: match results.outcome {
            Outcome::Winner(index) => index as i32,
            Outcome::Draw => -1,
        },
        hud: h.hud_out(results.hud),
    };
    true
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_restart(app: *mut Handle) -> bool {
    let h = handle!(app);
    let ok = h.perform(|h| h.app.restart());
    h.last_frame = None;
    ok
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_rematch(app: *mut Handle, seed: u32) -> bool {
    let h = handle!(app);
    let ok = h.perform(|h| h.app.rematch(seed));
    h.sync_scene();
    ok
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_quit_to_menu(app: *mut Handle) {
    let h = handle!(app);
    h.app.quit_to_menu();
    h.sync_scene();
}
/// # Safety
/// `app` is null or live; `path` is a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn melee_app_save_replay(app: *mut Handle, path: *const c_char) -> bool {
    handle!(app).perform(|h| {
        let path = unsafe { string(path) }?;
        h.app
            .session()
            .ok_or("no match to save")?
            .save_replay(std::path::Path::new(&path))
    })
}
