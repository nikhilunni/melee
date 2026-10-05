//! Stage previews rendered from the real disc image: every stage renders at
//! the preview size, shows a picture rather than a flat fill, differs from
//! the other stages and renders identically twice. Nothing rendered is
//! stored: the checks compare images made in this run with each other.
//! Skips without the disc, and without a GPU adapter.
use melee_platform::{
    app::App,
    art::{Image, Piece},
    catalog,
    disc::DiscFiles,
    preview,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

fn iso() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let iso = root.join("harness/roms/GALE01.iso");
    melee_test_support::require_files([&iso]).then_some(iso)
}

fn gpu() -> Option<preview::Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("skipping: no GPU adapter");
        return None;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&melee_platform::renderer::device_descriptor()),
    )
    .ok()?;
    Some(preview::Gpu {
        device,
        queue,
    })
}

/// Open the disc, read its files and render the six previews.
fn render_all(iso: &Path, gpu: &preview::Gpu) -> App {
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(iso).unwrap()).unwrap();
    assert!(app.stage_preview_job().is_none(), "files not read yet");
    app.load_art().unwrap();
    let stage = catalog::stages()[0];
    assert!(app.art_image(Piece::StagePreview(stage)).is_err());
    assert!(!app.stage_previews_ready());
    let job = app.stage_preview_job().expect("files are in");
    assert!(app.stage_preview_job().is_none(), "the job is handed out once");
    pollster::block_on(preview::run(
        job,
        gpu.clone(),
        || std::future::ready(()),
        || 0.0,
        |stage, result, _| {
            if let Err(error) = result {
                panic!("{stage:?}: {error}");
            }
        },
    ));
    assert!(app.stage_previews_ready());
    app
}

fn previews(app: &mut App) -> Vec<Arc<Image>> {
    catalog::stages()
        .iter()
        .map(|&stage| app.art_image(Piece::StagePreview(stage)).unwrap())
        .collect()
}

#[test]
fn every_stage_preview_renders_wide_varied_and_deterministic() {
    let Some(iso) = iso() else {
        return;
    };
    let Some(gpu) = gpu() else {
        return;
    };
    let first = previews(&mut render_all(&iso, &gpu));
    let second = previews(&mut render_all(&iso, &gpu));
    let mut seen = HashSet::new();
    for ((stage, a), b) in catalog::stages().iter().zip(&first).zip(&second) {
        assert_eq!([a.width, a.height], preview::SIZE, "{stage:?}");
        assert_eq!(a.rgba.len(), (a.width * a.height * 4) as usize, "{stage:?}");
        assert!(a.rgba.chunks_exact(4).all(|p| p[3] == 255), "{stage:?} is opaque");
        let colours: HashSet<_> = a.rgba.chunks_exact(4).map(|p| [p[0], p[1], p[2]]).collect();
        assert!(colours.len() > 1000, "{stage:?} shows {} colours", colours.len());
        assert!(a.rgba == b.rgba, "{stage:?} renders the same twice");
        assert!(seen.insert(a.rgba.clone()), "{stage:?} repeats another stage");
    }
}
