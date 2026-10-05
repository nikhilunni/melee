//! A host's drawable (a CAMetalLayer, a canvas) and the GPU device behind it.
//! The device lives as long as the window; each match builds its scene on it.
use crate::renderer::{self, Renderer};
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
            samples: renderer::sample_count(&adapter, view_format),
            surface,
            device,
            queue,
            config,
            view_format,
            renderer: None,
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
