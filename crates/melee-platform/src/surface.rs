//! Window-surface ownership, shared by native hosts after surface construction.
use crate::{renderer::Renderer, session::Session};
pub struct WindowRenderer {
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    config: wgpu::SurfaceConfiguration,
}
impl WindowRenderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        session: &Session,
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
        let mut config = surface
            .get_default_config(&adapter, size[0].max(1), size[1].max(1))
            .ok_or("surface has no supported format")?;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = 2;
        let renderer = Renderer::new(&adapter, config.format, session.presentation(), size).await?;
        surface.configure(&renderer.device, &config);
        Ok(Self {
            surface,
            renderer,
            config,
        })
    }
    pub fn resize(&mut self, size: [u32; 2]) {
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        if size != [self.config.width, self.config.height] {
            self.config.width = size[0];
            self.config.height = size[1];
            self.renderer.resize(size);
            self.surface.configure(&self.renderer.device, &self.config);
        }
    }
    pub fn draw(&mut self, session: &Session) -> Result<(), String> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.renderer.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            _ => return Err("render surface lost or invalid; reopen the window".into()),
        };
        self.renderer.draw(
            &frame.texture.create_view(&Default::default()),
            session.presentation(),
        );
        self.renderer.queue.present(frame);
        Ok(())
    }
}
