//! Shared wgpu renderer. Native shells supply a surface; no asset parsing or
//! match state enters this module. Geometry/images upload once per scene.
use crate::material;
use melee_lib::presentation::Presentation;
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    matrix: u32,
    normal: [f32; 3],
    billboard: u32,
}
/// One mesh: its range in the shared index buffer and its first vertex.
struct Draw {
    indices: std::ops::Range<u32>,
    base_vertex: i32,
    image: usize,
    pipeline: usize,
}
pub struct Renderer {
    shadows: wgpu::RenderPipeline,
    floors: wgpu::Buffer,
    lighting: wgpu::Buffer,
    sprites: crate::sprites::Sprites,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipelines: Vec<wgpu::RenderPipeline>,
    order: Vec<usize>,
    poses: wgpu::Buffer,
    pose_stride: u32,
    instances: wgpu::Buffer,
    camera: wgpu::Buffer,
    scene: wgpu::BindGroup,
    images: Vec<(usize, material::GpuMaterial)>,
    draws: Vec<Draw>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    depth: wgpu::TextureView,
    multisampled: Option<wgpu::TextureView>,
    samples: u32,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    /// Built for the stage's meshes alone ([`Renderer::stage_only`]).
    stage_only: bool,
}
impl Renderer {
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        scene: &Presentation,
        size: [u32; 2],
    ) -> Result<Self, String> {
        let samples = sample_count(adapter, format);
        let (device, queue) = adapter
            .request_device(&device_descriptor())
            .await
            .map_err(|e| e.to_string())?;
        Self::with_device(device, queue, samples, format, scene, size)
    }
    /// Build one scene's pipelines and buffers on a device the host owns, so
    /// a window keeps one device across matches. `samples` comes from
    /// [`sample_count`]; the device was requested with [`device_descriptor`].
    pub fn with_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        samples: u32,
        format: wgpu::TextureFormat,
        scene: &Presentation,
        size: [u32; 2],
    ) -> Result<Self, String> {
        Self::build(device, queue, samples, format, scene, size, false)
    }
    /// A renderer of the stage alone (the stage previews): fighters, items
    /// and effects are neither uploaded nor drawn, and neither are shadows
    /// or sprites. The scene itself is read as it is.
    pub fn stage_only(
        device: wgpu::Device,
        queue: wgpu::Queue,
        samples: u32,
        format: wgpu::TextureFormat,
        scene: &Presentation,
        size: [u32; 2],
    ) -> Result<Self, String> {
        Self::build(device, queue, samples, format, scene, size, true)
    }
    fn build(
        device: wgpu::Device,
        queue: wgpu::Queue,
        samples: u32,
        format: wgpu::TextureFormat,
        scene: &Presentation,
        size: [u32; 2],
        stage_only: bool,
    ) -> Result<Self, String> {
        let included = |mesh: usize| !stage_only || scene.is_stage_mesh(mesh);
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
                        has_dynamic_offset: true,
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
        for (_, mesh) in scene.meshes().iter().enumerate().filter(|(i, _)| included(*i)) {
            let key = pipeline_key(mesh);
            if pipeline_ids.contains_key(&key) {
                continue;
            }
            let pixel = key.pixel;
            let shape = key.shape;
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Melee textured geometry"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[Some(vertex_layout())],
                },
                primitive: wgpu::PrimitiveState {
                    // GXSetCullMode swaps the hardware face encoding. Preserve the
                    // authored GX winding under the wgpu viewport convention.
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: match mesh.culling {
                        melee_lib::presentation::FaceCulling::Front => Some(wgpu::Face::Front),
                        melee_lib::presentation::FaceCulling::Back => Some(wgpu::Face::Back),
                        _ => None,
                    },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(pixel.depth_write),
                    depth_compare: Some(material::compare(pixel.depth_compare)),
                    stencil: receiver_stencil(mesh.shadow_receiver),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[
                            (
                                "ALPHA_TEST",
                                if alpha_test_can_fail(pixel) { 1.0 } else { 0.0 },
                            ),
                            ("LAYERS", f64::from(shape.layers)),
                            ("TEV_LAYERS", f64::from(shape.custom_combiners)),
                        ],
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: material::blend(pixel)?,
                        write_mask: (if pixel.color_write {
                            wgpu::ColorWrites::COLOR
                        } else {
                            wgpu::ColorWrites::empty()
                        }) | (if pixel.alpha_write {
                            wgpu::ColorWrites::ALPHA
                        } else {
                            wgpu::ColorWrites::empty()
                        }),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            pipeline_ids.insert(key, pipelines.len());
            pipelines.push(pipeline);
        }
        let pose_bytes = std::mem::size_of_val(scene.matrices()) as u64;
        let alignment = u64::from(device.limits().min_storage_buffer_offset_alignment);
        let pose_stride = pose_bytes.div_ceil(alignment) * alignment;
        let poses = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Joint matrix palettes and active effects"),
            size: pose_stride * (1 + scene.effect_capacity()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World instances"),
            contents: bytemuck::cast_slice(scene.instances()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera"),
            size: std::mem::size_of::<crate::camera::Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lighting = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stage lighting"),
            contents: bytemuck::bytes_of(&crate::lighting::Lighting::capture(scene)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let floors = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Live shadow floors"),
            contents: bytemuck::cast_slice(scene.shadow_floors()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shadows = shadow_pipeline(&device, &scene_layout, format, samples);
        let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene"),
            layout: &scene_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &poses,
                        offset: 0,
                        size: std::num::NonZeroU64::new(pose_bytes),
                    }),
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
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: floors.as_entire_binding(),
                },
            ],
        });
        let mut images = Vec::new();
        let mut image_cache = material::Images::default();
        let mut cache = BTreeMap::new();
        let mut draws = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (mesh_index, mesh) in scene.meshes().iter().enumerate() {
            if !included(mesh_index) {
                // Never drawn: no material, no geometry.
                draws.push(Draw {
                    indices: 0..0,
                    base_vertex: 0,
                    image: usize::MAX,
                    pipeline: usize::MAX,
                });
                continue;
            }
            let key = Arc::as_ptr(&mesh.material) as usize;
            let image = if let Some(&image) = cache.get(&key) {
                image
            } else {
                let image = images.len();
                images.push((
                    mesh_index,
                    image_cache.bind(
                        &device,
                        &queue,
                        &material_layout,
                        &scene.materials()[mesh_index],
                        if scene.is_effect_mesh(mesh_index) {
                            scene.effect_capacity()
                        } else {
                            1
                        },
                    )?,
                ));
                cache.insert(key, image);
                image
            };
            let base_vertex = vertices.len() as i32;
            let first_index = indices.len() as u32;
            indices.extend_from_slice(&mesh.indices);
            vertices.extend(mesh.vertices.iter().map(|v| Vertex {
                    position: v.position,
                    uv: v.uv,
                    color: std::array::from_fn(|i| f32::from(v.color[i]) / 255.0),
                    matrix: mesh.matrix_offset + u32::from(v.matrix),
                    normal: v.normal,
                    billboard: match mesh.billboard {
                        melee_lib::presentation::Billboard::None => 0,
                        melee_lib::presentation::Billboard::ViewPlane => 1,
                        melee_lib::presentation::Billboard::ViewPoint => 2,
                    },
                }));
            draws.push(Draw {
                indices: first_index..first_index + mesh.indices.len() as u32,
                base_vertex,
                image,
                pipeline: pipeline_ids[&pipeline_key(mesh)],
            });
        }
        // Every mesh shares one vertex and one index buffer, bound once a frame.
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let sprites =
            crate::sprites::Sprites::new(&device, &queue, format, samples, &camera, scene)?;
        let depth = attachment(&device, size, DEPTH_FORMAT, samples);
        let multisampled = (samples > 1).then(|| attachment(&device, size, format, samples));
        Ok(Self {
            shadows,
            floors,
            sprites,
            device,
            queue,
            pipelines,
            order: (0..draws.len()).collect(),
            poses,
            pose_stride: pose_stride as u32,
            lighting,
            instances,
            camera,
            scene: scene_group,
            images,
            draws,
            vertices,
            indices,
            depth,
            multisampled,
            samples,
            format,
            size,
            stage_only,
        })
    }
    pub fn resize(&mut self, size: [u32; 2]) {
        if self.size != size && size[0] > 0 && size[1] > 0 {
            self.depth = attachment(&self.device, size, DEPTH_FORMAT, self.samples);
            self.multisampled = (self.samples > 1)
                .then(|| attachment(&self.device, size, self.format, self.samples));
            self.size = size;
        }
    }
    pub fn draw(&mut self, view: &wgpu::TextureView, scene: &Presentation) {
        self.draw_with(view, scene, &DrawOptions::default());
    }
    /// Draw with a framing or filter of the host's choosing (the stage
    /// previews). Options only select what is drawn; the scene is read-only.
    pub fn draw_with(
        &mut self,
        view: &wgpu::TextureView,
        scene: &Presentation,
        options: &DrawOptions,
    ) {
        for (mesh, material) in &mut self.images {
            if scene.is_effect_mesh(*mesh) {
                continue;
            }
            material.update(&self.device, &self.queue, &scene.materials()[*mesh]);
        }
        self.queue.write_buffer(
            &self.lighting,
            0,
            bytemuck::bytes_of(&crate::lighting::Lighting::capture(scene)),
        );
        self.queue
            .write_buffer(&self.floors, 0, bytemuck::cast_slice(scene.shadow_floors()));
        self.queue
            .write_buffer(&self.poses, 0, bytemuck::cast_slice(scene.matrices()));
        self.queue
            .write_buffer(&self.instances, 0, bytemuck::cast_slice(scene.instances()));
        for (slot, poses) in scene.effect_poses().enumerate() {
            self.queue.write_buffer(
                &self.poses,
                u64::from(self.pose_stride) * (slot as u64 + 1),
                bytemuck::cast_slice(poses),
            );
        }
        let effects = if self.stage_only {
            &[][..]
        } else {
            scene.effect_draws()
        };
        for effect in effects {
            let material = &mut self.images[self.draws[effect.mesh].image].1;
            material.update_slot(
                &self.device,
                &self.queue,
                scene.effect_material(*effect),
                effect.slot,
            );
        }
        let camera = crate::camera::retail(
            options.camera.as_ref().unwrap_or(scene.view_camera()),
            self.size,
        );
        // Draw opaque depth writers first, then translucent meshes back-to-front.
        // Index tie-breaking preserves authored order without a sorting allocation.
        self.order.sort_unstable_by(|&a, &b| {
            let ma = &scene.meshes()[a];
            let mb = &scene.meshes()[b];
            let transparent = |m: &melee_lib::presentation::Mesh| {
                m.material.pixel.blend[0] != 0 || !m.material.pixel.depth_write
            };
            // Background, opaque world, fighter underlays (drawn over the
            // world without depth writes), opaque fighters, translucent.
            let rank = |m: &melee_lib::presentation::Mesh| {
                if m.background {
                    0
                } else if transparent(m) && !m.underlay {
                    4
                } else if m.underlay {
                    2
                } else if m.shadow_owner.is_some() {
                    3
                } else {
                    1
                }
            };
            rank(ma)
                .cmp(&rank(mb))
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
            .write_buffer(&self.camera, 0, bytemuck::bytes_of(&camera));
        self.sprites.prepare(&self.queue, scene);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Match"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.multisampled.as_ref().unwrap_or(view),
                    depth_slice: None,
                    resolve_target: self.multisampled.as_ref().map(|_| view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(crate::srgb::clear_color(
                            scene.background_color(),
                        )),
                        // Only the resolve target is read: discarding the
                        // multisampled samples lets tiled GPUs keep them on
                        // chip instead of writing 4x the frame to memory.
                        store: if self.multisampled.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_stencil_reference(1);
            pass.set_bind_group(0, &self.scene, &[0]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            // Skip state that is already set: each call costs on the web.
            let mut pipeline = usize::MAX;
            let mut image = usize::MAX;
            for &i in &self.order {
                let draw = &self.draws[i];
                if !scene.visibility()[i] || draw.image == usize::MAX {
                    continue;
                }
                if pipeline != draw.pipeline {
                    pipeline = draw.pipeline;
                    pass.set_pipeline(&self.pipelines[pipeline]);
                }
                if image != draw.image {
                    image = draw.image;
                    pass.set_bind_group(1, self.images[image].1.bind(0), &[0]);
                }
                pass.draw_indexed(
                    draw.indices.clone(),
                    draw.base_vertex,
                    scene.instance_range(scene.meshes()[i].instance_group),
                );
            }
            // A stage-only view has no fighters: no shadows, effects or sprites.
            if !self.stage_only {
                self.draw_overlays(&mut pass, scene);
            }
        }
        self.queue.submit([encoder.finish()]);
    }
    /// Fighter shadows, effects and sprites, after the meshes.
    fn draw_overlays(&self, pass: &mut wgpu::RenderPass<'_>, scene: &Presentation) {
        // Stencil marks visible stage pixels; incrementing on the first hit
        // prevents overlapping caster triangles from darkening them twice.
        pass.set_pipeline(&self.shadows);
        for (i, mesh) in scene.meshes().iter().enumerate() {
            let Some(owner) = mesh.shadow_owner else {
                continue;
            };
            if !scene.visibility()[i] {
                continue;
            }
            let draw = &self.draws[i];
            pass.draw_indexed(
                draw.indices.clone(),
                draw.base_vertex,
                owner as u32..owner as u32 + 1,
            );
        }
        let mut pipeline = usize::MAX;
        for effect in scene.effect_draws() {
            let draw = &self.draws[effect.mesh];
            let material = &self.images[draw.image].1;
            if pipeline != draw.pipeline {
                pipeline = draw.pipeline;
                pass.set_pipeline(&self.pipelines[pipeline]);
            }
            pass.set_bind_group(
                0,
                &self.scene,
                &[self.pose_stride * (effect.slot as u32 + 1)],
            );
            pass.set_bind_group(
                1,
                material.bind(effect.slot),
                &[material.offset(effect.slot)],
            );
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, 0..1);
        }
        self.sprites.draw(pass);
    }
}
/// What [`Renderer::draw_with`] draws and from where.
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawOptions {
    /// A view in place of the retail main camera.
    pub camera: Option<melee_lib::presentation::ViewCamera>,
}
/// 4x MSAA where the adapter supports it for colour and depth, else none.
pub fn sample_count(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> u32 {
    if [format, DEPTH_FORMAT].into_iter().all(|format| {
        adapter
            .get_texture_format_features(format)
            .flags
            .sample_count_supported(4)
    }) {
        4
    } else {
        1
    }
}
/// The device the renderer needs: default features and limits.
pub fn device_descriptor() -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        label: Some("Melee renderer"),
        ..Default::default()
    }
}
fn attachment(
    device: &wgpu::Device,
    size: [u32; 2],
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth"),
            size: wgpu::Extent3d {
                width: size[0].max(1),
                height: size[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// What one mesh pipeline is specialised on: fixed-function pixel state and
/// the material's shape, which compiles unused shader code away.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PipelineKey {
    pixel: melee_lib::presentation::PixelState,
    culling: melee_lib::presentation::FaceCulling,
    shadow_receiver: bool,
    shape: material::Shape,
}
fn pipeline_key(mesh: &melee_lib::presentation::Mesh) -> PipelineKey {
    PipelineKey {
        pixel: pixel_state(mesh),
        culling: mesh.culling,
        shadow_receiver: mesh.shadow_receiver,
        shape: material::Shape::of(&mesh.material),
    }
}
fn pixel_state(mesh: &melee_lib::presentation::Mesh) -> melee_lib::presentation::PixelState {
    let mut pixel = mesh.material.pixel;
    if mesh.background {
        pixel.depth_write = false;
        pixel.depth_compare = 7;
    }
    pixel
}

/// Whether GX's two alpha compares, combined by the alpha operation, can
/// reject a fragment. Compare functions are static per material (only the
/// references animate), so a pipeline whose test always passes drops `discard`.
fn alpha_test_can_fail(pixel: melee_lib::presentation::PixelState) -> bool {
    const NEVER: u8 = 0;
    const ALWAYS: u8 = 7;
    let [first, second] = pixel.alpha_compare;
    let always_passes = match pixel.alpha_operation {
        // AND
        0 => first == ALWAYS && second == ALWAYS,
        // OR
        1 => first == ALWAYS || second == ALWAYS,
        // XNOR: both results equal.
        3 => (first == ALWAYS && second == ALWAYS) || (first == NEVER && second == NEVER),
        // XOR, or an unknown operation: keep the test.
        _ => false,
    };
    !always_passes
}

fn receiver_stencil(receiver: bool) -> wgpu::StencilState {
    if !receiver {
        return wgpu::StencilState::default();
    }
    let face = wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        pass_op: wgpu::StencilOperation::Replace,
        ..Default::default()
    };
    wgpu::StencilState {
        front: face,
        back: face,
        read_mask: 0xff,
        write_mask: 0xff,
    }
}
fn shadow_pipeline(
    device: &wgpu::Device,
    scene: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Planar fighter shadows"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("shadows.wgsl")
                .replace("// CAMERA", include_str!("camera.wgsl"))
                .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Shadow layout"),
        bind_group_layouts: &[Some(scene)],
        immediate_size: 0,
    });
    let face = wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        pass_op: wgpu::StencilOperation::IncrementClamp,
        ..Default::default()
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Planar fighter shadows"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[Some(vertex_layout())],
        },
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState {
                front: face,
                back: face,
                read_mask: 0xff,
                write_mask: 0xff,
            },
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Uint32, 4 => Float32x3, 5 => Uint32
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}
