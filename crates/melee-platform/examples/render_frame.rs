//! Render an offscreen frame through the same renderer used by native windows.
//! Writes tightly packed RGBA8; intended for local visual inspection, not assets.
use melee_platform::{renderer::Renderer, session::Session};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let files = args
        .get(1)
        .ok_or("usage: render_frame <asset-directory> <output.rgba>")?;
    let output = args.get(2).ok_or("missing output path")?;
    let mut session = Session::new(files)?;
    for _ in 0..240 {
        session.advance(std::time::Duration::from_nanos(16_666_667))?;
    }
    if args.iter().any(|a| a == "--laser") {
        session.set_action(0, melee_platform::session::Action::Special, true);
        let mut visible = 0;
        for _ in 0..90 {
            session.advance(std::time::Duration::from_nanos(16_666_667))?;
            if session
                .game()
                .observe()?
                .items()
                .any(|item| item.velocity().x != 0.0)
            {
                visible += 1;
            }
            if visible == 5 {
                break;
            }
        }
        if visible == 0 {
            return Err("laser preview did not produce a projectile".into());
        }
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        format,
        session.presentation(),
        [1280, 720],
    ))?;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Offscreen preview"),
        size: wgpu::Extent3d {
            width: 1280,
            height: 720,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.draw(
        &texture.create_view(&Default::default()),
        session.presentation(),
    );
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Preview readback"),
        size: 1280 * 720 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1280 * 4),
                rows_per_image: Some(720),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = buffer.slice(..).get_mapped_range()?;
    std::fs::write(output, &*bytes)?;
    println!(
        "Rendered tick {} using {:?}: 1280x720 RGBA",
        session.game().tick().0,
        adapter.get_info().backend
    );
    Ok(())
}
