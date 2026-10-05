//! Stage previews: a wide picture of each stage for the stage select menu,
//! rendered at runtime from the user's disc. Retail's stage select shows
//! small 3D models, so the disc holds no 2D preview to decode.
//!
//! Each preview runs a short match on the stage with the cheapest roster
//! (Jigglypuff twice), lets the stage settle, and draws only the stage's
//! meshes through a framing of its whole camera range with the match
//! renderer, supersampled, into an offscreen target that is read back. The match
//! is untouched by the framing and filter (they only choose what is drawn),
//! and it and its renderer are dropped once the image is read.
//!
//! The work is split so a host can keep its menus responsive:
//! [`build`] (CPU: load, tick, pose) and [`render`] (GPU: submit and copy)
//! return quickly; a [`Readback`] completes on a later poll. [`run`] drives
//! all six with a host-supplied pause between steps; natively a thread
//! runs it ([`spawn`]), on the web the page's event loop.
use crate::{art::Image, catalog, renderer};
use melee_lib::{
    presentation::{Presentation, ViewCamera},
    Character, Costume, FileSource, GameAssets, Inputs, Match, MatchConfig, PlayerConfig, Port,
    Seed, Stage,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
};

/// Preview size in pixels: 16:9, the width of a menu hero on a Retina
/// screen, so hosts show it without upscaling.
pub const SIZE: [u32; 2] = [1920, 1080];
/// Supersampling: each preview is drawn at this multiple of [`SIZE`] in
/// each direction and averaged down in linear light, four samples per
/// pixel for edges and textures alike (cheaper in memory than 4x MSAA at
/// the same size, which it replaces).
const SUPERSAMPLE: u32 = 2;
/// RGBA8 with sRGB encoding, so the readback is the image hosts show.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Ticks run before the picture: platforms, water and backgrounds move into
/// a representative state (five seconds; Pokemon Stadium's first
/// transformation comes much later, so it stays in its neutral form).
const SETTLE_TICKS: u32 = 300;
/// Any fixed seed: the previews are deterministic.
const SEED: u32 = 0x5354_4147;

/// The match a preview runs: Jigglypuff (the smallest fighter files) twice,
/// in two costumes so the pair is a legal Versus setup. Neither is drawn.
pub fn config(stage: Stage) -> MatchConfig {
    let player = |port, costume| {
        let mut player = PlayerConfig::new(port, Character::Jigglypuff);
        player.costume = Costume(costume);
        player
    };
    MatchConfig::versus(stage, [player(Port::P1, 0), player(Port::P2, 1)]).with_seed(Seed(SEED))
}

/// Every disc file the six previews read, sorted.
pub fn files() -> Vec<&'static str> {
    let mut names: Vec<_> = catalog::stages()
        .iter()
        .flat_map(|&stage| GameAssets::files(&config(stage)).unwrap_or_default())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// The GPU previews draw with: the host window's device and queue (one
/// device per window; previews never create their own).
#[derive(Clone)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Settling ticks run between two pauses.
const TICKS_PER_STEP: u32 = 60;

/// The CPU half of a preview: the settled stage, posed and ready to draw.
/// It pauses between its parts (loading, starting, settling in steps of
/// [`TICKS_PER_STEP`], posing) so a single-threaded host stays responsive.
pub async fn build<P: Future<Output = ()>>(
    source: &dyn FileSource,
    stage: Stage,
    pause: &mut impl FnMut() -> P,
) -> Result<Presentation, String> {
    let config = config(stage);
    let assets = catch_panic(|| GameAssets::load_from(source, &config).map_err(|e| e.to_string()))?;
    pause().await;
    let mut game = catch_panic(|| Match::new(&assets, config).map_err(|e| e.to_string()))?;
    drop(assets);
    let neutral = Inputs::default();
    for _ in 0..SETTLE_TICKS / TICKS_PER_STEP {
        pause().await;
        catch_panic(|| {
            for _ in 0..TICKS_PER_STEP {
                game.step(&neutral).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?;
    }
    pause().await;
    catch_panic(|| Presentation::new(&game).map_err(|e| e.to_string()))
}

/// How a preview frames its stage, chosen by eye for each stage: the
/// visible width at the interest point and the interest's height, in world
/// units, and how far the view looks down.
struct Framing {
    width: f32,
    height: f32,
    pitch_degrees: f32,
}
fn framing(stage: Stage) -> Framing {
    let (width, height, pitch_degrees) = match stage {
        Stage::Battlefield => (185.0, 18.0, 18.0),
        Stage::FinalDestination => (235.0, -15.0, 12.0),
        Stage::DreamLand => (240.0, 35.0, 8.0),
        Stage::FountainOfDreams => (200.0, 0.0, 8.0),
        Stage::PokemonStadium => (240.0, 15.0, 8.0),
        Stage::YoshisStory => (200.0, 15.0, 12.0),
    };
    Framing {
        width,
        height,
        pitch_degrees,
    }
}

/// The view a preview uses: from slightly above, centred on the stage's
/// camera range, through the stage's own field of view.
pub fn camera(stage: Stage, scene: &Presentation) -> ViewCamera {
    use gekko_math::msl::{cosf, sinf, tanf};
    let frame = scene.stage_frame();
    let retail = scene.view_camera();
    let framing = framing(stage);
    let [left, right, ..] = frame.camera;
    let aspect = SIZE[0] as f32 / SIZE[1] as f32;
    let fov = frame.fov;
    let distance = framing.width * 0.5 / (tanf(fov.to_radians() * 0.5) * aspect);
    let pitch = framing.pitch_degrees.to_radians();
    let interest = [(left + right) * 0.5, framing.height, 0.0];
    let toward_eye = [0.0, sinf(pitch), cosf(pitch)];
    ViewCamera {
        eye: std::array::from_fn(|i| interest[i] + toward_eye[i] * distance),
        right: [1.0, 0.0, 0.0],
        up: [0.0, cosf(pitch), -sinf(pitch)],
        toward_eye,
        fov,
        near: retail.near,
        far: retail.far,
        aspect,
    }
}

/// A preview drawn and being copied back from the GPU.
pub struct Readback {
    buffer: wgpu::Buffer,
    row_bytes: u32,
    mapped: Arc<Mutex<Option<Result<(), String>>>>,
}

/// The GPU half: draw the stage alone into an offscreen target and start
/// copying it back. The renderer is dropped on return.
pub fn render(gpu: &Gpu, stage: Stage, scene: &Presentation) -> Result<Readback, String> {
    let [width, height] = SIZE.map(|n| n * SUPERSAMPLE);
    let mut renderer = renderer::Renderer::stage_only(
        gpu.device.clone(),
        gpu.queue.clone(),
        1,
        FORMAT,
        scene,
        [width, height],
    )?;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Stage preview"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let options = renderer::DrawOptions {
        camera: Some(camera(stage, scene)),
    };
    renderer.draw_with(&view, scene, &options);
    let row_bytes = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Stage preview readback"),
        size: u64::from(row_bytes * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    let mapped = Arc::new(Mutex::new(None));
    let done = Arc::clone(&mapped);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            *done.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(result.map_err(|e| e.to_string()));
        });
    Ok(Readback {
        buffer,
        row_bytes,
        mapped,
    })
}

impl Readback {
    /// The image once the copy has landed; `None` while it is in flight.
    /// Natively the device must be polled for that to happen; in the
    /// browser it completes on its own.
    pub fn finish(&self) -> Option<Result<Image, String>> {
        let state = self.mapped.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        Some(state.and_then(|()| {
            let view = self
                .buffer
                .slice(..)
                .get_mapped_range()
                .map_err(|e| e.to_string())?;
            let rgba = downsample(&view, self.row_bytes as usize);
            drop(view);
            self.buffer.unmap();
            Ok(Image {
                width: SIZE[0],
                height: SIZE[1],
                rgba,
            })
        }))
    }
}

/// Average each [`SUPERSAMPLE`]-square block of the supersampled sRGB
/// frame (rows `row_bytes` apart) in linear light, into an opaque
/// [`SIZE`] image.
fn downsample(frame: &[u8], row_bytes: usize) -> Vec<u8> {
    let [width, height] = SIZE.map(|n| n as usize);
    let n = SUPERSAMPLE as usize;
    let count = (n * n) as u32;
    let linear = crate::srgb::Linear16::get();
    let mut out = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 3];
            for sy in 0..n {
                let row = &frame[(y * n + sy) * row_bytes..];
                for sx in 0..n {
                    let pixel = &row[(x * n + sx) * 4..];
                    for (total, &code) in sum.iter_mut().zip(pixel) {
                        *total += linear.decode(code);
                    }
                }
            }
            out.extend(sum.map(|total| linear.encode((total + count / 2) / count)));
            // The target is opaque; say so for hosts.
            out.push(255);
        }
    }
    out
}

/// One preview's state.
#[derive(Clone, Debug)]
pub enum Slot {
    Pending,
    Ready(Arc<Image>),
    /// It could not be rendered; hosts keep their placeholder.
    Failed(String),
}

/// The six previews, shared between the app and whatever renders them.
#[derive(Clone, Default)]
pub struct Previews(Arc<Mutex<BTreeMap<u32, Slot>>>);

impl Previews {
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<u32, Slot>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// The preview of `stage`; fails while it is being rendered and when it
    /// could not be.
    pub fn image(&self, stage: Stage) -> Result<Arc<Image>, String> {
        match self.lock().get(&catalog::stage_id(stage)) {
            Some(Slot::Ready(image)) => Ok(Arc::clone(image)),
            Some(Slot::Failed(error)) => Err(format!("no preview of {stage:?}: {error}")),
            Some(Slot::Pending) | None => Err("the stage previews are not rendered yet".into()),
        }
    }
    /// Whether every preview is settled: rendered, or failed.
    pub fn ready(&self) -> bool {
        let slots = self.lock();
        catalog::stages().iter().all(|&stage| {
            matches!(
                slots.get(&catalog::stage_id(stage)),
                Some(Slot::Ready(_) | Slot::Failed(_))
            )
        })
    }
    pub fn slot(&self, stage: Stage) -> Slot {
        self.lock()
            .get(&catalog::stage_id(stage))
            .cloned()
            .unwrap_or(Slot::Pending)
    }
    fn set(&self, stage: Stage, slot: Slot) {
        self.lock().insert(catalog::stage_id(stage), slot);
    }
}

/// What a renderer needs to draw the previews: the files and where to put
/// the results. [`crate::app::App::stage_preview_job`] hands it out once.
pub struct Job {
    pub files: BTreeMap<String, Vec<u8>>,
    pub previews: Previews,
}

/// Render every preview in catalog order. `pause` runs between steps (the
/// web yields to the page there); `now` is a millisecond clock for the
/// timings `report` receives with each stage's result.
pub async fn run<P: Future<Output = ()>>(
    job: Job,
    gpu: Gpu,
    mut pause: impl FnMut() -> P,
    now: impl Fn() -> f64,
    mut report: impl FnMut(Stage, &Result<Arc<Image>, String>, f64),
) {
    for &stage in catalog::stages() {
        pause().await;
        let start = now();
        let result = render_one(&job.files, &gpu, stage, &mut pause).await;
        report(stage, &result, now() - start);
        job.previews.set(
            stage,
            match result {
                Ok(image) => Slot::Ready(image),
                Err(error) => Slot::Failed(error),
            },
        );
    }
}

async fn render_one<P: Future<Output = ()>>(
    files: &BTreeMap<String, Vec<u8>>,
    gpu: &Gpu,
    stage: Stage,
    pause: &mut impl FnMut() -> P,
) -> Result<Arc<Image>, String> {
    let scene = build(files, stage, pause).await?;
    pause().await;
    let readback = catch_panic(|| render(gpu, stage, &scene))?;
    drop(scene);
    pause().await;
    loop {
        // Natively the map completes inside a device poll; the browser
        // completes it from its own event loop while we pause.
        #[cfg(not(target_arch = "wasm32"))]
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        if let Some(image) = readback.finish() {
            return image.map(Arc::new);
        }
        pause().await;
    }
}

/// A panic (an unported branch in the settling ticks) fails this preview,
/// not the menus. On the web a panic aborts the module regardless.
fn catch_panic<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|panic| Err(crate::panic_text(&*panic)))
}

/// Natively: render on a background thread, so the menus never wait. The
/// thread shares the window's device and ends when the six are done.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn(job: Job, gpu: Gpu) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("stage previews".into())
        .spawn(move || {
            let start = std::time::Instant::now();
            let clock = move || start.elapsed().as_secs_f64() * 1000.0;
            let previews = job.previews.clone();
            pollster::block_on(run(job, gpu, || std::future::ready(()), clock, log));
            eprintln!(
                "stage previews: {} in {:.0} ms",
                if previews.ready() { "ready" } else { "incomplete" },
                clock()
            );
        })
        .map(drop)
}

/// The line a host logs per preview.
pub fn describe(stage: Stage, result: &Result<Arc<Image>, String>, ms: f64) -> String {
    match result {
        Ok(image) => format!(
            "stage preview {stage:?}: {}x{} in {ms:.0} ms",
            image.width, image.height
        ),
        Err(error) => format!("stage preview {stage:?} failed after {ms:.0} ms: {error}"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn log(stage: Stage, result: &Result<Arc<Image>, String>, ms: f64) {
    eprintln!("{}", describe(stage, result, ms));
}
