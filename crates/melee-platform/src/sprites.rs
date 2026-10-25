//! Shared instanced sprite drawing. Atlas pixels upload once; prepared buffers
//! carry only live particle/shield values each frame.
use melee_lib::presentation::{Presentation, SpriteShape, Texture};
use std::{collections::BTreeMap, sync::Arc};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    previous: [f32; 4],
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
        samples: u32,
        camera: &wgpu::Buffer,
        scene: &Presentation,
    ) -> Result<Self, String> {
        let Atlas {
            pixels,
            coordinates,
            size: atlas_size,
        } = atlas(scene.sprite_textures())?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Particle atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
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
                bytes_per_row: Some(atlas_size * 4),
                rows_per_image: Some(atlas_size),
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
            source: wgpu::ShaderSource::Wgsl(
                include_str!("sprites.wgsl")
                    .replace("// CAMERA", include_str!("camera.wgsl"))
                    .replace("// PIXEL", include_str!("pixel.wgsl"))
                    .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = (0..8)
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
                        format: crate::renderer::DEPTH_FORMAT,
                        depth_write_enabled: Some(mode & 4 != 0),
                        depth_compare: Some(if mode & 2 != 0 {
                            wgpu::CompareFunction::Always
                        } else {
                            wgpu::CompareFunction::LessEqual
                        }),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
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
                | (usize::from(sprite.flags & (1 << 28) != 0) << 1)
                | (usize::from(sprite.flags & (1 << 3) != 0) << 2);
            let index = self.instances.len() as u32;
            match self.batches.last_mut() {
                Some((previous, range)) if *previous == mode => range.end += 1,
                _ => self.batches.push((mode, index..index + 1)),
            }
            self.instances.push(Instance {
                previous: [
                    sprite.previous_position[0],
                    sprite.previous_position[1],
                    sprite.previous_position[2],
                    sprite.trail_alpha,
                ],
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
                flags: [
                    sprite.flags,
                    u32::from(sprite.alpha_mode),
                    u32::from(sprite.alpha_compare[0]),
                    u32::from(sprite.alpha_compare[1]),
                ],
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
/// Atlas sides tried in turn: the smallest square that holds every
/// distinct image. 8192 is WebGPU's default `maxTextureDimension2D`; stages
/// with many effect textures (Dream Land, Pokemon Stadium, Brinstar Depths)
/// outgrow 2048.
const ATLAS_SIZES: [u32; 3] = [2048, 4096, 8192];
struct Atlas {
    pixels: Vec<u8>,
    /// Per input texture: its rectangle in normalized atlas coordinates.
    coordinates: Vec<[f32; 4]>,
    size: u32,
}
/// Shelf packing, tallest images first, each with a one-texel extruded
/// border so linear filtering never samples a neighbour.
fn atlas(textures: &[Arc<Texture>]) -> Result<Atlas, String> {
    let mut distinct: Vec<&Arc<Texture>> = Vec::new();
    let mut index_of = BTreeMap::new();
    let slots: Vec<usize> = textures
        .iter()
        .map(|image| {
            *index_of.entry(Arc::as_ptr(image) as usize).or_insert_with(|| {
                distinct.push(image);
                distinct.len() - 1
            })
        })
        .collect();
    let mut order: Vec<usize> = (0..distinct.len()).collect();
    // Stable: equal heights keep their first-use order.
    order.sort_by_key(|&i| std::cmp::Reverse(distinct[i].height));
    for size in ATLAS_SIZES {
        let Some(places) = pack(&distinct, &order, size) else {
            continue;
        };
        let mut pixels = vec![0; (size * size * 4) as usize];
        let mut rects = vec![[0.0; 4]; distinct.len()];
        for (&i, &(x, y)) in order.iter().zip(&places) {
            let image = distinct[i];
            let (w, h) = (u32::from(image.width), u32::from(image.height));
            for dy in 0..h + 2 {
                for dx in 0..w + 2 {
                    let sx = dx.saturating_sub(1).min(w - 1);
                    let sy = dy.saturating_sub(1).min(h - 1);
                    let src = ((sy * w + sx) * 4) as usize;
                    let dst = (((y + dy) * size + x + dx) * 4) as usize;
                    pixels[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
                }
            }
            let side = size as f32;
            rects[i] = [
                (x + 1) as f32 / side,
                (y + 1) as f32 / side,
                (x + 1 + w) as f32 / side,
                (y + 1 + h) as f32 / side,
            ];
        }
        return Ok(Atlas {
            pixels,
            coordinates: slots.iter().map(|&i| rects[i]).collect(),
            size,
        });
    }
    Err("particle atlas capacity exceeded".into())
}
/// Top-left corners for `order`'s images on shelves `size` wide, or `None`
/// when they do not fit.
fn pack(images: &[&Arc<Texture>], order: &[usize], size: u32) -> Option<Vec<(u32, u32)>> {
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    let mut places = Vec::with_capacity(order.len());
    for &i in order {
        let (w, h) = (u32::from(images[i].width) + 2, u32::from(images[i].height) + 2);
        if w > size || h > size {
            return None;
        }
        if x + w > size {
            x = 0;
            y += row_height;
            row_height = 0;
        }
        if y + h > size {
            return None;
        }
        places.push((x, y));
        x += w;
        row_height = row_height.max(h);
    }
    Some(places)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_extrudes_edges_and_reuses_shared_images() {
        let red = Arc::new(Texture {
            mipmaps: Vec::new(),
            lod_range: [0.0; 2],
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        });
        let blue = Arc::new(Texture {
            mipmaps: Vec::new(),
            lod_range: [0.0; 2],
            width: 1,
            height: 1,
            rgba: vec![0, 0, 255, 255],
        });
        let Atlas {
            pixels,
            coordinates: uv,
            size,
        } = atlas(&[red.clone(), red, blue]).unwrap();
        assert_eq!(size, 2048);
        assert_eq!(uv[0], uv[1]);
        assert_ne!(uv[0], uv[2]);
        assert_eq!(
            &pixels[..12],
            &[255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
        );
        assert_eq!(&pixels[12..16], &[0, 0, 255, 255]);
    }
    #[test]
    fn a_full_atlas_grows_to_the_next_size() {
        let square = |side: u16, value: u8| {
            Arc::new(Texture {
                mipmaps: Vec::new(),
                lod_range: [0.0; 2],
                width: side,
                height: side,
                rgba: vec![value; usize::from(side) * usize::from(side) * 4],
            })
        };
        // Five 1000-texel images need more than one 2048 square.
        let images: Vec<_> = (0..5).map(|i| square(1000, i)).collect();
        let atlas = atlas(&images).unwrap();
        assert_eq!(atlas.size, 4096);
        let distinct: std::collections::BTreeSet<_> =
            atlas.coordinates.iter().map(|r| r.map(f32::to_bits)).collect();
        assert_eq!(distinct.len(), 5);
    }
}
