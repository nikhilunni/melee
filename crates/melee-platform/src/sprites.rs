//! Shared instanced sprite drawing. Atlas pixels upload once; prepared buffers
//! carry only live particle/shield values each frame.
use melee_lib::presentation::{Presentation, SpriteShape, Texture};
use std::{collections::BTreeMap, sync::Arc};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    center: [f32; 4],
    extent: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    environment: [f32; 4],
    flags: [u32; 4],
}
pub struct Sprites {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipelines: Vec<wgpu::RenderPipeline>,
    coordinates: Vec<[f32; 4]>,
    instances: Vec<Instance>,
    batches: Vec<(usize, std::ops::Range<u32>)>,
}
impl Sprites {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        camera: &wgpu::Buffer,
        scene: &Presentation,
    ) -> Result<Self, String> {
        let (pixels, coordinates) = atlas(scene.sprite_textures())?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Particle atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE * 4),
                rows_per_image: Some(ATLAS_SIZE),
            },
            texture.size(),
        );
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sprite filtering"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let nearest = device.create_sampler(&Default::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Live sprites"),
            size: (scene.sprite_capacity() * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprites"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite resources"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&nearest),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particles and shields"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprites.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = (0..4)
            .map(|mode| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Sprite blending"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vertex"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(if mode & 2 != 0 {
                            wgpu::CompareFunction::Always
                        } else {
                            wgpu::CompareFunction::LessEqual
                        }),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: if mode & 1 != 0 {
                                        wgpu::BlendFactor::One
                                    } else {
                                        wgpu::BlendFactor::OneMinusSrcAlpha
                                    },
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::COLOR,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();
        Ok(Self {
            buffer,
            group,
            pipelines,
            coordinates,
            instances: Vec::with_capacity(scene.sprite_capacity()),
            batches: Vec::with_capacity(scene.sprite_capacity()),
        })
    }
    pub fn prepare(&mut self, queue: &wgpu::Queue, scene: &Presentation) {
        self.instances.clear();
        self.batches.clear();
        for sprite in scene.sprites() {
            let mode = usize::from(sprite.flags & (1 << 22) != 0)
                | (usize::from(sprite.flags & (1 << 28) != 0) << 1);
            let index = self.instances.len() as u32;
            match self.batches.last_mut() {
                Some((previous, range)) if *previous == mode => range.end += 1,
                _ => self.batches.push((mode, index..index + 1)),
            }
            self.instances.push(Instance {
                center: [
                    sprite.position[0],
                    sprite.position[1],
                    sprite.position[2],
                    sprite.rotation,
                ],
                extent: [
                    sprite.half_size[0],
                    sprite.half_size[1],
                    if matches!(sprite.shape, SpriteShape::Shield { .. }) {
                        1.0
                    } else {
                        0.0
                    },
                    0.0,
                ],
                uv: self.coordinates[sprite.texture],
                color: match sprite.shape {
                    SpriteShape::Shield { port } => {
                        let rgb = [
                            [1.0, 0.28, 0.28],
                            [0.31, 0.5, 1.0],
                            [1.0, 0.85, 0.25],
                            [0.3, 1.0, 0.45],
                        ][port.index()];
                        [rgb[0], rgb[1], rgb[2], f32::from(sprite.color[3]) / 255.0]
                    }
                    SpriteShape::Texture => sprite.color.map(|v| f32::from(v) / 255.0),
                },
                environment: sprite.environment.map(|v| f32::from(v) / 255.0),
                flags: [sprite.flags, 0, 0, 0],
            });
        }
        if !self.instances.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.instances));
        }
    }
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.group, &[]);
        for (mode, range) in &self.batches {
            pass.set_pipeline(&self.pipelines[*mode]);
            pass.draw(0..6, range.clone());
        }
    }
}
const ATLAS_SIZE: u32 = 2048;
type Atlas = (Vec<u8>, Vec<[f32; 4]>);
fn atlas(textures: &[Arc<Texture>]) -> Result<Atlas, String> {
    let mut pixels = vec![0; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
    let mut coordinates = Vec::with_capacity(textures.len());
    let mut cache = BTreeMap::new();
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    for image in textures {
        let key = Arc::as_ptr(image) as usize;
        if let Some(&uv) = cache.get(&key) {
            coordinates.push(uv);
            continue;
        }
        let (w, h) = (u32::from(image.width), u32::from(image.height));
        if w + 2 > ATLAS_SIZE || h + 2 > ATLAS_SIZE {
            return Err("particle image exceeds atlas size".into());
        }
        if x + w + 2 > ATLAS_SIZE {
            x = 0;
            y += row_height;
            row_height = 0;
        }
        if y + h + 2 > ATLAS_SIZE {
            return Err("particle atlas capacity exceeded".into());
        }
        // Extrude one texel so linear filtering never samples a neighbor image.
        for dy in 0..h + 2 {
            for dx in 0..w + 2 {
                let sx = dx.saturating_sub(1).min(w - 1);
                let sy = dy.saturating_sub(1).min(h - 1);
                let src = ((sy * w + sx) * 4) as usize;
                let dst = (((y + dy) * ATLAS_SIZE + x + dx) * 4) as usize;
                pixels[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
            }
        }
        let uv = [
            (x + 1) as f32 / ATLAS_SIZE as f32,
            (y + 1) as f32 / ATLAS_SIZE as f32,
            (x + 1 + w) as f32 / ATLAS_SIZE as f32,
            (y + 1 + h) as f32 / ATLAS_SIZE as f32,
        ];
        coordinates.push(uv);
        cache.insert(key, uv);
        x += w + 2;
        row_height = row_height.max(h + 2);
    }
    Ok((pixels, coordinates))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_extrudes_edges_and_reuses_shared_images() {
        let red = Arc::new(Texture {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        });
        let blue = Arc::new(Texture {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 255, 255],
        });
        let (pixels, uv) = atlas(&[red.clone(), red, blue]).unwrap();
        assert_eq!(uv[0], uv[1]);
        assert_ne!(uv[0], uv[2]);
        assert_eq!(
            &pixels[..12],
            &[255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
        );
        assert_eq!(&pixels[12..16], &[0, 0, 255, 255]);
    }
}
