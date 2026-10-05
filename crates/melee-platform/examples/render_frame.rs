//! Render an offscreen frame through the same renderer used by native windows.
//! Writes tightly packed RGBA8; intended for local visual inspection, not assets.
//!
//! Inspection flags: `--size WxH` (default 1280x720); `--costume1 N` and
//! `--costume2 N`; `--then P:Action[+Action]:ticks` (repeatable, in order)
//! holds those actions for player P, then releases them; `--crop P` writes
//! only a square around player P's fighter and prints its size.
use melee_lib::{Character, Costume, PlayerConfig, Port, Stage};
use melee_platform::{
    renderer::Renderer,
    session::{Action, Session},
};
#[path = "../src/camera.rs"]
#[allow(dead_code)]
mod camera;

const TICK: std::time::Duration = std::time::Duration::from_nanos(16_666_667);

/// `--stage Battlefield --p1 Peach --p2 Fox` (enum names) replace the default match.
fn config(args: &[String]) -> Result<melee_lib::MatchConfig, String> {
    let flag = |name: &str| {
        args.windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    };
    let mut config = Session::default_config();
    if let Some(name) = flag("--stage") {
        config.stage = *Stage::ALL
            .iter()
            .find(|s| format!("{s:?}") == name)
            .ok_or_else(|| format!("unknown stage {name}"))?;
    }
    for (index, (port, name)) in [(Port::P1, "--p1"), (Port::P2, "--p2")]
        .into_iter()
        .enumerate()
    {
        if let Some(name) = flag(name) {
            let character = *Character::ALL
                .iter()
                .find(|c| format!("{c:?}") == name)
                .ok_or_else(|| format!("unknown character {name}"))?;
            config.players[index] = PlayerConfig::new(port, character);
        }
    }
    for (index, name) in ["--costume1", "--costume2"].into_iter().enumerate() {
        if let Some(value) = flag(name) {
            config.players[index].costume =
                Costume(value.parse().map_err(|_| format!("bad costume {value}"))?);
        }
    }
    Ok(config)
}

/// `P:Action[+Action]:ticks`: hold the actions for that many ticks, then release.
fn run_step(session: &mut Session, step: &str) -> Result<(), Box<dyn std::error::Error>> {
    let fields: Vec<_> = step.split(':').collect();
    let [player, actions, count] = fields[..] else {
        return Err(format!("bad --then {step}").into());
    };
    let player: usize = player.parse()?;
    let actions = actions
        .split('+')
        .filter(|a| !a.is_empty())
        .map(|name| {
            (0..9)
                .filter_map(|i| Action::try_from(i).ok())
                .find(|a| format!("{a:?}") == name)
                .ok_or_else(|| format!("unknown action {name}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for &action in &actions {
        session.set_action(player, action, true);
    }
    for _ in 0..count.parse::<usize>()? {
        session.advance(TICK)?;
    }
    for &action in &actions {
        session.set_action(player, action, false);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let files = args.get(1).ok_or(
        "usage: render_frame <asset-directory> <output.rgba> [--stage S] [--p1 C] [--p2 C]",
    )?;
    let output = args.get(2).ok_or("missing output path")?;
    let mut session = Session::new(&std::path::Path::new(files), config(&args)?)?;
    let ticks = args
        .windows(2)
        .find(|pair| pair[0] == "--ticks")
        .map(|pair| pair[1].parse::<usize>())
        .transpose()?
        .unwrap_or(240);
    for _ in 0..ticks {
        session.advance(TICK)?;
    }
    for pair in args.windows(2).filter(|pair| pair[0] == "--then") {
        run_step(&mut session, &pair[1])?;
    }
    if args.iter().any(|a| a == "--illusion") {
        session.set_action(0, Action::Right, true);
        session.set_action(0, Action::Special, true);
        for _ in 0..24 {
            session.advance(TICK)?;
        }
    }
    if args.iter().any(|a| a == "--shield") {
        session.set_action(1, Action::Shield, true);
        for _ in 0..20 {
            session.advance(TICK)?;
        }
    }
    if args.iter().any(|a| a == "--laser") {
        session.set_action(0, Action::Special, true);
        let mut visible = 0;
        for _ in 0..90 {
            session.advance(TICK)?;
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
    let [width, height] = match args.windows(2).find(|pair| pair[0] == "--size") {
        Some(pair) => {
            let (w, h) = pair[1].split_once('x').ok_or("--size WxH")?;
            [w.parse::<u32>()?, h.parse::<u32>()?]
        }
        None => [1280, 720],
    };
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        format,
        session.presentation(),
        [width, height],
    ))?;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Offscreen preview"),
        size: wgpu::Extent3d {
            width,
            height,
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
        size: u64::from(width * height * 4),
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
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
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
    let crop = args
        .windows(2)
        .find(|pair| pair[0] == "--crop")
        .map(|pair| pair[1].parse::<usize>())
        .transpose()?;
    let (pixels, out_width, out_height) = match crop {
        Some(player) => crop_fighter(&session, player, &bytes, [width, height])?,
        None => (bytes.to_vec(), width, height),
    };
    std::fs::write(output, &pixels)?;
    println!(
        "Rendered tick {} using {:?}: {out_width}x{out_height} RGBA",
        session.game().tick().0,
        adapter.get_info().backend
    );
    Ok(())
}

/// Cut a square around one player's fighter using the renderer's camera.
fn crop_fighter(
    session: &Session,
    player: usize,
    bytes: &[u8],
    size: [u32; 2],
) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let targets = session.presentation().camera_targets();
    let target = targets[player].ok_or("cropped fighter has no camera target")?;
    let uniform = camera::retail(session.presentation().view_camera(), size);
    let project = |x: f32, y: f32| {
        let p = [x - uniform.eye[0], y - uniform.eye[1], -uniform.eye[2], 1.0];
        let clip: [f32; 4] =
            std::array::from_fn(|row| (0..4).map(|col| uniform.projection[col][row] * p[col]).sum());
        [
            (clip[0] / clip[3] + 1.0) * 0.5 * size[0] as f32,
            (1.0 - clip[1] / clip[3]) * 0.5 * size[1] as f32,
        ]
    };
    let center = project(target[0], target[1] + 10.0);
    let top = project(target[0], target[1] + 30.0);
    let half = (center[1] - top[1]).abs().max(8.0);
    let x0 = (center[0] - half).max(0.0) as u32;
    let y0 = (center[1] - half).max(0.0) as u32;
    let x1 = ((center[0] + half) as u32).min(size[0]);
    let y1 = ((center[1] + half) as u32).min(size[1]);
    let mut out = Vec::new();
    for y in y0..y1 {
        let row = (y * size[0]) as usize * 4;
        out.extend_from_slice(&bytes[row + x0 as usize * 4..row + x1 as usize * 4]);
    }
    Ok((out, x1 - x0, y1 - y0))
}
