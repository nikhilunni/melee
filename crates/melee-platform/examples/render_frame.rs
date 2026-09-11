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
    let ticks = args
        .windows(2)
        .find(|pair| pair[0] == "--ticks")
        .map(|pair| pair[1].parse::<usize>())
        .transpose()?
        .unwrap_or(240);
    for _ in 0..ticks {
        session.advance(std::time::Duration::from_nanos(16_666_667))?;
    }
    if args.iter().any(|a| a == "--illusion") {
        session.set_action(0, melee_platform::session::Action::Right, true);
        session.set_action(0, melee_platform::session::Action::Special, true);
        for _ in 0..24 {
            session.advance(std::time::Duration::from_nanos(16_666_667))?;
        }
    }
    if args.iter().any(|a| a == "--shield") {
        session.set_action(1, melee_platform::session::Action::Shield, true);
        for _ in 0..20 {
            session.advance(std::time::Duration::from_nanos(16_666_667))?;
        }
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
    eprintln!(
        "Visible background meshes: {}",
        session
            .presentation()
            .meshes()
            .iter()
            .zip(session.presentation().visibility())
            .filter(|(mesh, visible)| mesh.background && **visible)
            .count()
    );
    if args.iter().any(|a| a == "--inspect-materials") {
        eprintln!(
            "Ambient: {:?}; lights: {:?}",
            session.presentation().ambient_light(),
            session.presentation().directional_lights()
        );
        let mut seen = std::collections::BTreeSet::new();
        for (i, mesh) in session.presentation().meshes().iter().enumerate() {
            if !session.presentation().visibility()[i]
                || !seen.insert(std::sync::Arc::as_ptr(&mesh.material) as usize)
            {
                continue;
            }
            let m = &session.presentation().materials()[i];
            eprintln!("mesh {i} fighter {:?} mode {:x} ambient {:?} diffuse {:?} spec {:?} shine {} textures {:?}", mesh.shadow_owner, m.render_mode, m.ambient, m.diffuse, m.specular, m.shininess, m.textures.iter().map(|t| (t.flags, t.scale, t.translation)).collect::<Vec<_>>());
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
