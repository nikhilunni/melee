//! A host's drawable (a CAMetalLayer, a canvas) and the GPU device behind it.
//! The device lives as long as the window; each match builds its scene on it.
use crate::{
    app::App,
    renderer::{self, Renderer},
};
use melee_lib::presentation::Presentation;

pub struct WindowRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// The sRGB format the shaders write; the surface's own format or a view of it.
    view_format: wgpu::TextureFormat,
    samples: u32,
    renderer: Option<Renderer>,
    /// The app session generation the scene was built for.
    generation: Option<u64>,
}
/// Natively 4x MSAA where supported. In the browser none: Chrome's WebGPU
/// (Dawn on Metal) cannot keep multisampled targets in tile memory, and 4x
/// cost 112 ms per 3456x1814 frame against 12.9 ms without (Battlefield,
/// M-series, 2026-10-04); at Retina density aliasing is barely visible.
fn window_samples(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> u32 {
    if cfg!(target_arch = "wasm32") {
        1
    } else {
        renderer::sample_count(adapter, format)
    }
}
impl WindowRenderer {
    /// Request an adapter and device for `surface`. On the web this awaits
    /// the browser; natively hosts block on it.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: [u32; 2],
    ) -> Result<Self, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let (device, queue) = adapter
            .request_device(&renderer::device_descriptor())
            .await
            .map_err(|e| e.to_string())?;
        let mut config = surface
            .get_default_config(&adapter, size[0].max(1), size[1].max(1))
            .ok_or("surface has no supported format")?;
        // Shaders output linear colour and rely on an sRGB target. Metal
        // offers one directly; a WebGPU canvas only through an sRGB view.
        let capabilities = surface.get_capabilities(&adapter);
        if let Some(&srgb) = capabilities.formats.iter().find(|f| f.is_srgb()) {
            config.format = srgb;
        } else {
            config.view_formats.push(config.format.add_srgb_suffix());
        }
        let view_format = config.format.add_srgb_suffix();
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = 2;
        surface.configure(&device, &config);
        Ok(Self {
            samples: window_samples(&adapter, view_format),
            surface,
            device,
            queue,
            config,
            view_format,
            renderer: None,
            generation: None,
        })
    }
    /// Upload a match's scene; replaces the previous match's.
    pub fn set_scene(&mut self, scene: &Presentation) -> Result<(), String> {
        self.renderer = None;
        self.renderer = Some(Renderer::with_device(
            self.device.clone(),
            self.queue.clone(),
            self.samples,
            self.view_format,
            scene,
            [self.config.width, self.config.height],
        )?);
        Ok(())
    }
    pub fn clear_scene(&mut self) {
        self.renderer = None;
        self.generation = None;
    }
    /// Upload or drop the scene so it shows the app's current session. A
    /// scene the renderer cannot build fails the match start (the app
    /// returns to stage select with the error).
    pub fn follow(&mut self, app: &mut App) {
        let generation = app.session().map(|_| app.generation());
        if generation == self.generation {
            return;
        }
        let built = match app.session() {
            Some(session) => self.set_scene(session.presentation()),
            None => Ok(()),
        };
        match built {
            Ok(()) if generation.is_some() => self.generation = generation,
            Ok(()) => self.clear_scene(),
            Err(error) => {
                self.clear_scene();
                app.fail_match_start(error);
            }
        }
    }
    pub fn has_scene(&self) -> bool {
        self.renderer.is_some()
    }
    pub fn resize(&mut self, size: [u32; 2]) {
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        if size != [self.config.width, self.config.height] {
            self.config.width = size[0];
            self.config.height = size[1];
            if let Some(renderer) = &mut self.renderer {
                renderer.resize(size);
            }
            self.surface.configure(&self.device, &self.config);
        }
    }
    /// Draw the scene, or clear to black when no match is loaded.
    pub fn draw(&mut self, scene: Option<&Presentation>) -> Result<(), String> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            _ => return Err("render surface lost or invalid; reopen the window".into()),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.view_format),
            ..Default::default()
        });
        match (&mut self.renderer, scene) {
            (Some(renderer), Some(scene)) => renderer.draw(&view, scene),
            _ => self.clear(&view),
        }
        self.queue.present(frame);
        Ok(())
    }
    /// Benchmarking: draw the scene `frames` times into an offscreen target
    /// the size and format of the surface, without presenting. Await
    /// [`WindowRenderer::queue`]'s submitted work to time the GPU.
    pub fn draw_offscreen(&mut self, scene: &Presentation, frames: u32) -> Result<(), String> {
        let renderer = self.renderer.as_mut().ok_or("no match scene to draw")?;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Benchmark target"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.view_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        for _ in 0..frames {
            renderer.draw(&view, scene);
        }
        Ok(())
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    /// The window's device for the stage previews: the same device, never
    /// a second one.
    pub fn preview_gpu(&self) -> crate::preview::Gpu {
        crate::preview::Gpu {
            device: self.device.clone(),
            queue: self.queue.clone(),
        }
    }
    fn clear(&self, view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.queue.submit([encoder.finish()]);
    }
}
