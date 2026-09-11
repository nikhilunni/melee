//! GPU material resources, cached by immutable image/material identity.
//! HSD's common color/alpha operations run in one fragment shader. These are
//! display calculations, independent of the retail simulation math kernels.
use melee_lib::presentation::{Material, PixelState};
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

pub const MAX_LAYERS: usize = 8;
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
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
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    diffuse: [f32; 4],
    config: [u32; 4],
    alpha: [u32; 4],
    layers: [Layer; MAX_LAYERS],
}
pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
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
#[derive(Default)]
pub struct Images {
    views: BTreeMap<usize, wgpu::TextureView>,
    samplers: BTreeMap<(u32, u32, bool), wgpu::Sampler>,
}
impl Images {
    pub fn bind(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        material: &Material,
    ) -> Result<wgpu::BindGroup, String> {
        if material.textures.len() > MAX_LAYERS {
            return Err("material exceeds eight GX texture maps".into());
        }
        let mut uniform = Uniform {
            diffuse: material.diffuse,
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
        let mut keys = [(0, (0, 0, false)); MAX_LAYERS];
        let mut lightmaps = 0;
        for (i, key) in keys.iter_mut().enumerate() {
            let layer = material.textures.get(i);
            let image = layer.map(|l| &l.image);
            key.0 = image.map_or(0, |t| Arc::as_ptr(t) as usize);
            key.1 = layer.map_or((0, 0, false), |t| (t.wrap_s, t.wrap_t, t.nearest));
            if let Some(t) = layer {
                let lightmap = t.flags & 0x1f0;
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
                    operations: [
                        (t.flags >> 16) & 15,
                        (t.flags >> 20) & 15,
                        t.flags & 15,
                        u32::from(lightmaps & lightmap == 0),
                    ],
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
                lightmaps |= lightmap;
            }
            self.views.entry(key.0).or_insert_with(|| {
                let (w, h, bytes) = image.map_or((1, 1, &[255u8; 4][..]), |t| {
                    (u32::from(t.width), u32::from(t.height), t.rgba.as_slice())
                });
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Disc texture"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
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
                    bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(w * 4),
                        rows_per_image: Some(h),
                    },
                    texture.size(),
                );
                texture.create_view(&Default::default())
            });
            let (s, t, nearest) = key.1;
            self.samplers.entry(key.1).or_insert_with(|| {
                device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Authored texture addressing"),
                    address_mode_u: address(s),
                    address_mode_v: address(t),
                    mag_filter: if nearest {
                        wgpu::FilterMode::Nearest
                    } else {
                        wgpu::FilterMode::Linear
                    },
                    min_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                })
            });
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Authored material"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }];
        for (i, (image, sampler)) in keys.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: 1 + i as u32 * 2,
                resource: wgpu::BindingResource::TextureView(&self.views[image]),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 2 + i as u32 * 2,
                resource: wgpu::BindingResource::Sampler(&self.samplers[sampler]),
            });
        }
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material"),
            layout,
            entries: &entries,
        }))
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
        samples.push_str(&format!("if material.config.y > {i}u {{ let layer=material.layers[{i}]; let tex=textureSample(image{i},sampler{i},coordinates(in,layer)); color=combine(color,custom_texture(tex,layer),layer); }}\n"));
    }
    include_str!("render.wgsl")
        .replace("// CUSTOM_COMBINERS", include_str!("tev.wgsl"))
        .replace("// TEXTURE_BINDINGS", &bindings)
        .replace("// TEXTURE_SAMPLES", &samples)
}
