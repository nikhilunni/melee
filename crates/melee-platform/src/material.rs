//! GPU material resources, cached by immutable image/material identity.
//! HSD's common color/alpha operations run in one fragment shader. These are
//! display calculations, independent of the retail simulation math kernels.
use melee_lib::presentation::{Material, PixelState};
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

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
    image: [u32; 4],
    addressing: [u32; 4],
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
                view_dimension: wgpu::TextureViewDimension::D2Array,
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
    views: BTreeMap<Vec<usize>, wgpu::TextureView>,
    samplers: BTreeMap<(u32, u32, bool), wgpu::Sampler>,
}
impl Images {
    pub fn bind(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        material: &Material,
    ) -> Result<GpuMaterial, String> {
        if material.textures.len() > MAX_LAYERS {
            return Err("material exceeds eight GX texture maps".into());
        }
        let uniform = capture(material);
        let mut keys = Vec::with_capacity(MAX_LAYERS);
        for i in 0..MAX_LAYERS {
            let layer = material.textures.get(i);
            let bank = material
                .texture_banks
                .get(i)
                .map(AsRef::as_ref)
                .unwrap_or_else(|| layer.map_or(&[], |t| std::slice::from_ref(&t.image)));
            let image_key: Vec<_> = bank
                .iter()
                .map(|image| Arc::as_ptr(image) as usize)
                .collect();
            let sampler_key = layer.map_or((0, 0, false), |t| (t.wrap_s, t.wrap_t, t.nearest));
            if bank.len() > device.limits().max_texture_array_layers as usize {
                return Err("animated texture exceeds device array layer limit".into());
            }
            self.views.entry(image_key.clone()).or_insert_with(|| {
                let w = bank.iter().map(|t| u32::from(t.width)).max().unwrap_or(1);
                let h = bank.iter().map(|t| u32::from(t.height)).max().unwrap_or(1);
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Prepared animated texture"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: bank.len().max(1) as u32,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                for index in 0..bank.len().max(1) {
                    let (width, height, bytes) =
                        bank.get(index).map_or((1, 1, &[255u8; 4][..]), |t| {
                            (u32::from(t.width), u32::from(t.height), t.rgba.as_slice())
                        });
                    let mut destination = texture.as_image_copy();
                    destination.origin.z = index as u32;
                    queue.write_texture(
                        destination,
                        bytes,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(width * 4),
                            rows_per_image: Some(height),
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            });
            let (s, t, nearest) = sampler_key;
            self.samplers.entry(sampler_key).or_insert_with(|| {
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
            keys.push((image_key, sampler_key));
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Authored material"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material"),
            layout,
            entries: &entries,
        });
        Ok(GpuMaterial {
            bind,
            buffer,
            uniform,
        })
    }
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
    let mut lightmaps = 0;
    for i in 0..MAX_LAYERS {
        let layer = material.textures.get(i);
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
            uniform.layers[i].image = [
                material
                    .texture_banks
                    .get(i)
                    .and_then(|bank| bank.iter().position(|image| Arc::ptr_eq(image, &t.image)))
                    .unwrap_or(0) as u32,
                u32::from(t.image.width),
                u32::from(t.image.height),
                0,
            ];
            uniform.layers[i].addressing = [t.wrap_s, t.wrap_t, u32::from(t.nearest), 0];
            lightmaps |= lightmap;
        }
    }
    uniform
}
pub struct GpuMaterial {
    pub bind: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    uniform: Uniform,
}
impl GpuMaterial {
    pub fn update(&mut self, queue: &wgpu::Queue, material: &Material) {
        let uniform = capture(material);
        if uniform != self.uniform {
            queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&uniform));
            self.uniform = uniform;
        }
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
        bindings.push_str(&format!("@group(1) @binding({}) var image{i}: texture_2d_array<f32>;\n@group(1) @binding({}) var sampler{i}: sampler;\n",1+i*2,2+i*2));
        samples.push_str(&format!("if material.config.y > {i}u {{ let layer=material.layers[{i}]; let tex=sample_image(image{i},coordinates(in,layer),layer); color=combine(color,custom_texture(tex,layer),layer); }}\n"));
    }
    include_str!("render.wgsl")
        .replace("// CAMERA", include_str!("camera.wgsl"))
        .replace("// CUSTOM_COMBINERS", include_str!("tev.wgsl"))
        .replace("// TEXTURE_BINDINGS", &bindings)
        .replace("// TEXTURE_SAMPLES", &samples)
}
