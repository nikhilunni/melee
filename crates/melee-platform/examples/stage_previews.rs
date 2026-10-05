//! Render the six stage previews from a disc image and write each as raw
//! RGBA8 (1920x1080) for local inspection, with the time each took:
//!
//! ```sh
//! cargo run --release -p melee-platform --example stage_previews -- GALE01.iso target/previews
//! ffmpeg -f rawvideo -pix_fmt rgba -s 1920x1080 -i target/previews/Battlefield.rgba Battlefield.png
//! ```
//!
//! The images come from the user's disc: keep them under `target/`, never
//! in the repository.
use melee_platform::{app::App, disc::DiscFiles, preview};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (Some(iso), Some(out)) = (args.get(1), args.get(2)) else {
        return Err("usage: stage_previews <GALE01.iso> <output directory>".into());
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(std::path::Path::new(iso))?)?;
    let start = std::time::Instant::now();
    app.load_art()?;
    println!("read the art and preview files in {:?}", start.elapsed());
    let job = app.stage_preview_job().ok_or("the preview files are missing")?;
    println!(
        "{} files, {} bytes",
        job.files.len(),
        job.files.values().map(Vec::len).sum::<usize>()
    );
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) = pollster::block_on(
        adapter.request_device(&melee_platform::renderer::device_descriptor()),
    )?;
    let gpu = preview::Gpu {
        device,
        queue,
    };
    let clock = std::time::Instant::now();
    // The longest stretch between two pauses: how long a single-threaded
    // host (the web page) is blocked at most.
    let mut last = std::time::Instant::now();
    let mut longest = std::time::Duration::ZERO;
    pollster::block_on(preview::run(
        job,
        gpu,
        || {
            longest = longest.max(last.elapsed());
            last = std::time::Instant::now();
            std::future::ready(())
        },
        || clock.elapsed().as_secs_f64() * 1000.0,
        |stage, result, ms| println!("{}", preview::describe(stage, result, ms)),
    ));
    println!("longest step between pauses: {longest:?}");
    std::fs::create_dir_all(out)?;
    for &stage in melee_platform::catalog::stages() {
        if let Ok(image) = app.stage_previews().image(stage) {
            std::fs::write(format!("{out}/{stage:?}.rgba"), &image.rgba)?;
        }
    }
    println!("ready: {}", app.stage_previews_ready());
    Ok(())
}
