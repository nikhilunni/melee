//! GPU material resources, cached by immutable image/material identity.
//! HSD's common color/alpha operations run in one fragment shader. These are
//! display calculations, independent of the retail simulation math kernels.
//!
//! Each decoded image is its own texture with its authored mip chain, sampled
//! by hardware: GX wrap modes become address modes, GX filters and LOD range
//! become sampler state. An animated texture bank selects its current image
//! by bind group: one per combination of bank images in use, made on first
//! use and kept.
use melee_lib::presentation::{Material, PixelState, Texture, TextureLayer};
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_LAYERS: usize = 8;
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Layer {
    scale: [f32; 4],
    translation: [f32; 4],
    rotation: [f32; 4],
    operations: [u32; 4],
    color_operation: [u32; 4],
    alpha_operation: [u32; 4],
    color_inputs: [u32; 4],
    alpha_inputs: [u32; 4],
    constants: [[f32; 4]; 3],
    active: [u32; 4],
    /// x: GX LOD bias; the rest is padding.
    lod: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    overlay: [f32; 4],
    diffuse: [f32; 4],
    ambient: [f32; 4],
    specular: [f32; 4],
    config: [u32; 4],
    alpha: [u32; 4],
    layers: [Layer; MAX_LAYERS],
}
pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: None,
        },
        count: None,
    }];
    for i in 0..MAX_LAYERS as u32 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 1 + i * 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2 + i * 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("HSD materials"),
        entries: &entries,
    })
}
/// Hardware sampler state for one GX texture layer and image.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SamplerKey {
    wrap: [u32; 2],
    magnify_nearest: bool,
    min_filter: u32,
    /// `lod_min_clamp` and `lod_max_clamp` as f32 bits.
    lod: [u32; 2],
    anisotropy: u16,
}
impl SamplerKey {
    /// GX_NEAR, GX_LINEAR, GX_NEAR_MIP_NEAR, GX_LIN_MIP_NEAR, GX_NEAR_MIP_LIN
    /// and GX_LIN_MIP_LIN (`min_filter` 0..=5) as hardware filter modes; the
    /// image's LOD range, limited to its decoded levels, as the LOD clamp.
    fn new(layer: &TextureLayer, image: &Texture) -> Self {
        let min_filter = layer.lod.min_filter;
        let lod = if min_filter < 2 {
            // No mip filter: the base level only.
            [0.0, 0.0]
        } else {
            let levels = image.mipmaps.len() as f32;
            let max = gekko_math::cmp::min(image.lod_range[1], levels);
            [gekko_math::cmp::min(image.lod_range[0], max), max]
        };
        // GX anisotropy 0/1/2 takes 1, 2 or 4 taps. Hardware anisotropy
        // needs every filter linear; other layers sample isotropically.
        let all_linear = !layer.nearest && min_filter == 5;
        Self {
            wrap: [layer.wrap_s, layer.wrap_t],
            magnify_nearest: layer.nearest,
            min_filter,
            lod: lod.map(f32::to_bits),
            anisotropy: if all_linear {
                1 << layer.lod.anisotropy.min(2)
            } else {
                1
            },
        }
    }
    fn descriptor(self) -> wgpu::SamplerDescriptor<'static> {
        let filter = |nearest: bool| {
            if nearest {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            }
        };
        wgpu::SamplerDescriptor {
            label: Some("Authored texture sampling"),
            address_mode_u: address(self.wrap[0]),
            address_mode_v: address(self.wrap[1]),
            mag_filter: filter(self.magnify_nearest),
            min_filter: filter(matches!(self.min_filter, 0 | 2 | 4)),
            mipmap_filter: if self.min_filter >= 4 {
                wgpu::MipmapFilterMode::Linear
            } else {
                wgpu::MipmapFilterMode::Nearest
            },
            lod_min_clamp: f32::from_bits(self.lod[0]),
            lod_max_clamp: f32::from_bits(self.lod[1]),
            anisotropy_clamp: self.anisotropy,
            ..Default::default()
        }
    }
}
/// Scene-wide image and sampler cache: each decoded image uploads once.
#[derive(Default)]
pub struct Images {
    views: BTreeMap<usize, wgpu::TextureView>,
    samplers: BTreeMap<SamplerKey, wgpu::Sampler>,
    blank: Option<Binding>,
}
type Binding = (wgpu::TextureView, wgpu::Sampler);
impl Images {
    fn view(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Arc<Texture>,
    ) -> wgpu::TextureView {
        self.views
            .entry(Arc::as_ptr(image) as usize)
            .or_insert_with(|| upload(device, queue, image))
            .clone()
    }
    fn sampler(&mut self, device: &wgpu::Device, key: SamplerKey) -> wgpu::Sampler {
        self.samplers
            .entry(key)
            .or_insert_with(|| device.create_sampler(&key.descriptor()))
            .clone()
    }
    /// An opaque white texel for unused layer bindings.
    fn blank(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Binding {
        self.blank
            .get_or_insert_with(|| {
                let white = Texture {
                    mipmaps: Vec::new(),
                    lod_range: [0.0; 2],
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                };
                (
                    upload(device, queue, &white),
                    device.create_sampler(&Default::default()),
                )
            })
            .clone()
    }
    pub fn bind(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        material: &Material,
        slots: usize,
    ) -> Result<GpuMaterial, String> {
        if material.textures.len() > MAX_LAYERS {
            return Err("material exceeds eight GX texture maps".into());
        }
        let uniform = capture(material);
        let mut layers = Vec::with_capacity(MAX_LAYERS);
        for i in 0..MAX_LAYERS {
            let mut images = LayerImages::default();
            if let Some(layer) = material.textures.get(i) {
                let bank = material
                    .texture_banks
                    .get(i)
                    .map(AsRef::as_ref)
                    .unwrap_or_else(|| std::slice::from_ref(&layer.image));
                if bank.len() > usize::from(u16::MAX) {
                    return Err("animated texture bank exceeds 65535 images".into());
                }
                for image in bank {
                    images.identities.push(Arc::as_ptr(image) as usize);
                    images.bindings.push((
                        self.view(device, queue, image),
                        self.sampler(device, SamplerKey::new(layer, image)),
                    ));
                }
            }
            if images.bindings.is_empty() {
                images.bindings.push(self.blank(device, queue));
            }
            layers.push(images);
        }
        let size = std::mem::size_of::<Uniform>() as u64;
        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = size.div_ceil(alignment) * alignment;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Authored material instances"),
            size: stride * slots as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for slot in 0..slots {
            queue.write_buffer(&buffer, stride * slot as u64, bytemuck::bytes_of(&uniform));
        }
        let mut gpu = GpuMaterial {
            layout: layout.clone(),
            buffer,
            uniforms: vec![uniform; slots],
            stride: stride as u32,
            layers,
            groups: Vec::new(),
            selected: vec![0; slots],
        };
        let initial = gpu.group(device, gpu.selection(material));
        gpu.selected.fill(initial);
        Ok(gpu)
    }
}
/// One texture layer's bank: image identities and bindings, in bank order.
#[derive(Default)]
struct LayerImages {
    identities: Vec<usize>,
    bindings: Vec<Binding>,
}
fn upload(device: &wgpu::Device, queue: &wgpu::Queue, image: &Texture) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Authored texture"),
        size: wgpu::Extent3d {
            width: u32::from(image.width),
            height: u32::from(image.height),
            depth_or_array_layers: 1,
        },
        mip_level_count: image.mipmaps.len() as u32 + 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let levels = std::iter::once((image.width, image.height, image.rgba.as_slice())).chain(
        image
            .mipmaps
            .iter()
            .map(|mip| (mip.width, mip.height, mip.rgba.as_slice())),
    );
    for (level, (width, height, rgba)) in levels.enumerate() {
        let mut destination = texture.as_image_copy();
        destination.mip_level = level as u32;
        queue.write_texture(
            destination,
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(u32::from(width) * 4),
                rows_per_image: Some(u32::from(height)),
            },
            wgpu::Extent3d {
                width: u32::from(width),
                height: u32::from(height),
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&Default::default())
}
fn capture(material: &Material) -> Uniform {
    let mut uniform = Uniform {
        overlay: material.overlay,
        diffuse: material.diffuse,
        ambient: [
            material.ambient[0],
            material.ambient[1],
            material.ambient[2],
            0.0,
        ],
        specular: [
            material.specular[0],
            material.specular[1],
            material.specular[2],
            material.shininess,
        ],
        config: [
            material.render_mode,
            material.textures.len() as u32,
            u32::from(material.pixel.alpha_operation),
            0,
        ],
        alpha: [
            u32::from(material.pixel.alpha_compare[0]),
            u32::from(material.pixel.alpha_reference[0]),
            u32::from(material.pixel.alpha_compare[1]),
            u32::from(material.pixel.alpha_reference[1]),
        ],
        layers: [Layer::default(); MAX_LAYERS],
    };
    for i in 0..MAX_LAYERS {
        let layer = material.textures.get(i);
        if let Some(t) = layer {
            uniform.layers[i] = Layer {
                scale: [
                    t.scale[0],
                    t.scale[1],
                    f32::from(t.repeat[0]),
                    f32::from(t.repeat[1]),
                ],
                translation: [
                    t.translation[0],
                    t.translation[1],
                    t.blending,
                    if t.wrap_t == 2 { 1.0 } else { 0.0 },
                ],
                rotation: [t.rotation[0], t.rotation[1], t.rotation[2], 0.0],
                operations: [(t.flags >> 16) & 15, (t.flags >> 20) & 15, t.flags & 15, 1],
                ..Layer::default()
            };
            if let Some(tev) = t.combiner {
                let layer = &mut uniform.layers[i];
                let operation = |op: melee_lib::presentation::TextureCombiner, color: bool| {
                    let op = if color { op.color } else { op.alpha };
                    [
                        u32::from(op.function),
                        u32::from(op.bias),
                        u32::from(op.scale),
                        u32::from(op.clamp),
                    ]
                };
                layer.color_operation = operation(tev, true);
                layer.alpha_operation = operation(tev, false);
                layer.color_inputs = tev.color.inputs.map(u32::from);
                layer.alpha_inputs = tev.alpha.inputs.map(u32::from);
                layer.constants = tev
                    .constants
                    .map(|color| color.map(|v| f32::from(v) / 255.0));
                layer.active[0] = tev.active;
            }
            uniform.layers[i].active[1] = t.flags & 0x1f0;
            uniform.layers[i].lod = [t.lod.bias, 0.0, 0.0, 0.0];
        }
    }
    uniform
}
/// The bank image each layer shows, as indices into the layers' banks.
type Selection = [u16; MAX_LAYERS];
pub struct GpuMaterial {
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    uniforms: Vec<Uniform>,
    stride: u32,
    layers: Vec<LayerImages>,
    /// Bind groups for the bank selections seen so far.
    groups: Vec<(Selection, wgpu::BindGroup)>,
    /// Per slot: the index in `groups` of its current selection.
    selected: Vec<usize>,
}
impl GpuMaterial {
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, material: &Material) {
        self.update_slot(device, queue, material, 0);
    }
    pub fn offset(&self, slot: usize) -> u32 {
        self.stride * slot as u32
    }
    /// The bind group for a slot's current images.
    pub fn bind(&self, slot: usize) -> &wgpu::BindGroup {
        &self.groups[self.selected[slot]].1
    }
    pub fn update_slot(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material: &Material,
        slot: usize,
    ) {
        let uniform = capture(material);
        if uniform != self.uniforms[slot] {
            queue.write_buffer(
                &self.buffer,
                u64::from(self.offset(slot)),
                bytemuck::bytes_of(&uniform),
            );
            self.uniforms[slot] = uniform;
        }
        let selection = self.selection(material);
        if self.groups[self.selected[slot]].0 != selection {
            self.selected[slot] = self.group(device, selection);
        }
    }
    /// Each layer's current image, by identity within its bank (the first
    /// image when the layer shows one outside it).
    fn selection(&self, material: &Material) -> Selection {
        std::array::from_fn(|i| {
            material.textures.get(i).map_or(0, |layer| {
                let image = Arc::as_ptr(&layer.image) as usize;
                self.layers[i]
                    .identities
                    .iter()
                    .position(|&identity| identity == image)
                    .unwrap_or(0) as u16
            })
        })
    }
    /// The bind group for a selection, created on its first use.
    fn group(&mut self, device: &wgpu::Device, selection: Selection) -> usize {
        if let Some(index) = self.groups.iter().position(|(s, _)| *s == selection) {
            return index;
        }
        let size = std::mem::size_of::<Uniform>() as u64;
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.buffer,
                offset: 0,
                size: std::num::NonZeroU64::new(size),
            }),
        }];
        for (i, layer) in self.layers.iter().enumerate() {
            let (view, sampler) = &layer.bindings[usize::from(selection[i])];
            entries.push(wgpu::BindGroupEntry {
                binding: 1 + i as u32 * 2,
                resource: wgpu::BindingResource::TextureView(view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 2 + i as u32 * 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material"),
            layout: &self.layout,
            entries: &entries,
        });
        self.groups.push((selection, group));
        self.groups.len() - 1
    }
}
fn address(value: u32) -> wgpu::AddressMode {
    match value {
        1 => wgpu::AddressMode::Repeat,
        2 => wgpu::AddressMode::MirrorRepeat,
        _ => wgpu::AddressMode::ClampToEdge,
    }
}
pub fn compare(value: u8) -> wgpu::CompareFunction {
    use wgpu::CompareFunction::*;
    match value {
        0 => Never,
        1 => Less,
        2 => Equal,
        3 => LessEqual,
        4 => Greater,
        5 => NotEqual,
        6 => GreaterEqual,
        _ => Always,
    }
}
pub fn blend(state: PixelState) -> Result<Option<wgpu::BlendState>, String> {
    let component = match state.blend[0] {
        0 => return Ok(None),
        1 => wgpu::BlendComponent {
            src_factor: factor(state.blend[1])?,
            dst_factor: factor(state.blend[2])?,
            operation: wgpu::BlendOperation::Add,
        },
        3 => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::ReverseSubtract,
        },
        _ => return Err("GX logic blend operations are not implemented".into()),
    };
    Ok(Some(wgpu::BlendState {
        color: component,
        alpha: component,
    }))
}
fn factor(value: u8) -> Result<wgpu::BlendFactor, String> {
    use wgpu::BlendFactor::*;
    Ok(match value {
        0 => Zero,
        1 => One,
        2 => Src,
        3 => OneMinusSrc,
        4 => SrcAlpha,
        5 => OneMinusSrcAlpha,
        6 => DstAlpha,
        7 => OneMinusDstAlpha,
        _ => return Err("invalid GX blend factor".into()),
    })
}
/// Explicit bindings avoid non-portable texture binding-array features.
pub fn shader() -> String {
    let mut bindings = String::new();
    let mut samples = String::new();
    for i in 0..MAX_LAYERS {
        bindings.push_str(&format!("@group(1) @binding({}) var image{i}: texture_2d<f32>;\n@group(1) @binding({}) var sampler{i}: sampler;\n",1+i*2,2+i*2));
        samples.push_str(&format!("if material.config.y > {i}u {{ let layer=material.layers[{i}]; let tex=textureSampleBias(image{i},sampler{i},coordinates(in,layer),layer.lod.x); texels[{i}]=custom_texture(tex,layer); composition.operations[{i}]=layer.operations; composition.parameters[{i}]=vec2(layer.translation.z,f32(layer.activation.y)); }}\n"));
    }
    include_str!("render.wgsl")
        .replace("// CAMERA", include_str!("camera.wgsl"))
        .replace("// PIXEL", include_str!("pixel.wgsl"))
        .replace("// CUSTOM_COMBINERS", include_str!("tev.wgsl"))
        .replace("// TEXTURE_BINDINGS", &bindings)
        .replace("// TEXTURE_SAMPLES", &samples)
}
