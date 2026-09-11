//! Shared wgpu renderer. Native shells supply a surface; no asset parsing or
//! match state enters this module. Geometry/images upload once per scene.
use crate::{camera::Camera, material};
use melee_lib::presentation::Presentation;
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    matrix: u32,
    normal: [f32; 3],
}
struct Draw {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    image: usize,
    pipeline: usize,
}
pub struct Renderer {
    lighting: wgpu::Buffer,
    sprites: crate::sprites::Sprites,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipelines: Vec<wgpu::RenderPipeline>,
    order: Vec<usize>,
    poses: wgpu::Buffer,
    instances: wgpu::Buffer,
    camera: wgpu::Buffer,
    scene: wgpu::BindGroup,
    images: Vec<wgpu::BindGroup>,
    draws: Vec<Draw>,
    depth: wgpu::TextureView,
    size: [u32; 2],
}
impl Renderer {
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        scene: &Presentation,
        size: [u32; 2],
    ) -> Result<Self, String> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Melee renderer"),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Melee basic materials"),
            source: wgpu::ShaderSource::Wgsl(material::shader().into()),
        });
        let material_layout = material::layout(&device);
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shared material layout"),
            bind_group_layouts: &[Some(&scene_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let mut pipelines = Vec::new();
        let mut pipeline_ids = BTreeMap::new();
        for mesh in scene.meshes() {
            let pixel = pixel_state(mesh);
            if pipeline_ids.contains_key(&pixel) {
                continue;
            }
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Melee textured geometry"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 52,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Uint32, 4 => Float32x3
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(pixel.depth_write),
                depth_compare: Some(material::compare(pixel.depth_compare)),
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
                    blend: material::blend(pixel)?,
                    write_mask: (if pixel.color_write { wgpu::ColorWrites::COLOR } else { wgpu::ColorWrites::empty() }) | (if pixel.alpha_write { wgpu::ColorWrites::ALPHA } else { wgpu::ColorWrites::empty() }),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
            pipeline_ids.insert(pixel, pipelines.len());
            pipelines.push(pipeline);
        }
        let poses = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Joint matrix palettes"),
            contents: bytemuck::cast_slice(scene.matrices()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World instances"),
            contents: bytemuck::cast_slice(scene.instances()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lighting = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stage lighting"),
            contents: bytemuck::bytes_of(&crate::lighting::Lighting::capture(scene)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene"),
            layout: &scene_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: poses.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: instances.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lighting.as_entire_binding(),
                },
            ],
        });
        let mut images = Vec::new();
        let mut image_cache = material::Images::default();
        let mut cache = BTreeMap::new();
        let mut draws = Vec::new();
        for mesh in scene.meshes() {
            let key = Arc::as_ptr(&mesh.material) as usize;
            let image = if let Some(&image) = cache.get(&key) {
                image
            } else {
                let image = images.len();
                images.push(image_cache.bind(&device, &queue, &material_layout, &mesh.material)?);
                cache.insert(key, image);
                image
            };
            let vertices: Vec<_> = mesh
                .vertices
                .iter()
                .map(|v| Vertex {
                    position: v.position,
                    uv: v.uv,
                    color: std::array::from_fn(|i| f32::from(v.color[i]) / 255.0),
                    matrix: mesh.matrix_offset + u32::from(v.matrix),
                    normal: v.normal,
                })
                .collect();
            draws.push(Draw {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                count: mesh.indices.len() as u32,
                image,
                pipeline: pipeline_ids[&pixel_state(mesh)],
            });
        }
        let sprites = crate::sprites::Sprites::new(&device, &queue, format, &camera, scene)?;
        let depth = depth(&device, size);
        Ok(Self {
            sprites,
            device,
            queue,
            pipelines,
            order: (0..draws.len()).collect(),
            poses,
            lighting,
            instances,
            camera,
            scene: scene_group,
            images,
            draws,
            depth,
            size,
        })
    }
    pub fn resize(&mut self, size: [u32; 2]) {
        if self.size != size && size[0] > 0 && size[1] > 0 {
            self.depth = depth(&self.device, size);
            self.size = size;
        }
    }
    pub fn draw(&mut self, view: &wgpu::TextureView, scene: &Presentation) {
        self.queue.write_buffer(
            &self.lighting,
            0,
            bytemuck::bytes_of(&crate::lighting::Lighting::capture(scene)),
        );
        self.queue
            .write_buffer(&self.poses, 0, bytemuck::cast_slice(scene.matrices()));
        self.queue
            .write_buffer(&self.instances, 0, bytemuck::cast_slice(scene.instances()));
        let camera = Camera::frame(scene.camera_targets(), self.size).uniform();
        // Draw opaque depth writers first, then translucent meshes back-to-front.
        // Index tie-breaking preserves authored order without a sorting allocation.
        self.order.sort_unstable_by(|&a, &b| {
            let ma = &scene.meshes()[a];
            let mb = &scene.meshes()[b];
            let transparent = |m: &melee_lib::presentation::Mesh| {
                m.material.pixel.blend[0] != 0 || !m.material.pixel.depth_write
            };
            mb.background
                .cmp(&ma.background)
                .then_with(|| transparent(ma).cmp(&transparent(mb)))
                .then_with(|| {
                    if transparent(ma) {
                        scene.matrices()[ma.matrix_offset as usize][3][2]
                            .total_cmp(&scene.matrices()[mb.matrix_offset as usize][3][2])
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then(a.cmp(&b))
        });
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera));
        self.sprites.prepare(&self.queue, scene);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Match"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.018,
                            g: 0.022,
                            b: 0.04,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.scene, &[]);
            for &i in &self.order {
                let draw = &self.draws[i];
                if !scene.visibility()[i] {
                    continue;
                }
                pass.set_pipeline(&self.pipelines[draw.pipeline]);
                pass.set_bind_group(1, &self.images[draw.image], &[]);
                pass.set_vertex_buffer(0, draw.vertices.slice(..));
                pass.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(
                    0..draw.count,
                    0,
                    scene.instance_range(scene.meshes()[i].instance_group),
                );
            }
            self.sprites.draw(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
    }
}
fn depth(device: &wgpu::Device, size: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth"),
            size: wgpu::Extent3d {
                width: size[0].max(1),
                height: size[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn pixel_state(mesh: &melee_lib::presentation::Mesh) -> melee_lib::presentation::PixelState {
    let mut pixel = mesh.material.pixel;
    if mesh.background {
        pixel.depth_write = false;
        pixel.depth_compare = 7;
    }
    pixel
}
