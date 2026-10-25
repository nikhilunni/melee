//! Every character and every stage the menus offer loads from the real disc
//! and draws: a short match per pick, then one frame through the match
//! renderer with wgpu validation on. Nothing rendered is kept or compared;
//! this catches "cannot be presented" failures (renderer limits, missing
//! scene data) before a player does. Skips without the disc, and without a
//! GPU adapter.
use melee_lib::{Character, Stage};
use melee_platform::{app::App, catalog, disc::DiscFiles, renderer};
use std::{path::PathBuf, time::Duration};

fn iso() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let iso = root.join("harness/roms/GALE01.iso");
    melee_test_support::require_files([&iso]).then_some(iso)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}
fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("skipping: no GPU adapter");
        return None;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&renderer::device_descriptor())).ok()?;
    Some(Gpu { device, queue })
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SIZE: [u32; 2] = [320, 180];
/// Past the start banner, so fighters, effects and the HUD-free scene move.
const TICKS: u32 = 200;

/// Start `p1` vs `p2` on `stage`, play a few seconds and draw one frame.
fn play_and_draw(app: &mut App, gpu: &Gpu, p1: Character, p2: Character, stage: Stage) -> Result<(), String> {
    app.quit_to_menu();
    app.choose_character(0, p1)?;
    app.choose_character(1, p2)?;
    app.confirm_characters()?;
    app.choose_stage(stage, 7)?;
    while app.load_next_file()? {}
    app.finish_loading()?;
    for _ in 0..TICKS {
        app.advance(Duration::from_nanos(16_666_667))?;
    }
    let scene = app.session().ok_or("no session")?.presentation();
    let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer =
        renderer::Renderer::with_device(gpu.device.clone(), gpu.queue.clone(), 1, FORMAT, scene, SIZE)?;
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("smoke target"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    renderer.draw(&target.create_view(&Default::default()), scene);
    let error = pollster::block_on(scope.pop());
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    match error {
        Some(error) => Err(error.to_string()),
        None => Ok(()),
    }
}

#[test]
fn every_character_and_stage_loads_and_draws() {
    let (Some(iso), Some(gpu)) = (iso(), gpu()) else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    let mut failures = Vec::new();
    for &character in catalog::characters() {
        if let Err(error) = play_and_draw(&mut app, &gpu, character, Character::Fox, Stage::Battlefield) {
            failures.push(format!("{character:?}: {error}"));
        }
    }
    for &stage in catalog::stages() {
        if let Err(error) = play_and_draw(&mut app, &gpu, Character::Fox, Character::Marth, stage) {
            failures.push(format!("{stage:?}: {error}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
