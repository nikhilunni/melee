//! GPU numeric fixtures for the same material shader used by native windows.
//! Run explicitly on a graphical host; the workspace test suite stays headless.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance.request_adapter(&Default::default()).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    let source = melee_platform::material::shader()
        + r#"
@group(0) @binding(7) var<storage,read_write> checks:array<vec4<f32>>;
@compute @workgroup_size(1) fn probe() {
    var layer:Layer;
    let tex=vec4(0.25,0.5,0.75,0.5);
    checks[0]=custom_texture(tex,layer);
    layer.activation.x=0xc0000000u;
    layer.constants[0]=vec4(0.0,0.0,0.0,0.25);
    layer.color_inputs=vec4<u32>(15u,8u,132u,15u);
    layer.alpha_inputs=vec4<u32>(7u,4u,67u,7u);
    layer.color_operation=vec4<u32>(0u,0u,0u,1u);
    layer.alpha_operation=layer.color_operation;
    checks[1]=custom_texture(tex,layer);
    checks[2]=vec4(tev_operation(vec3(0.25),vec3(0.75),vec3(0.5),vec3(0.25),vec4<u32>(0u,1u,3u,1u)),1.0);
    checks[3]=vec4(tev_operation(vec3(0.75),vec3(0.0),vec3(0.0),vec3(0.25),vec4<u32>(1u,0u,0u,0u)),1.0);
    checks[4]=vec4(tev_operation(vec3(1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.75),vec3(0.25),vec4<u32>(8u,0u,0u,1u)),1.0);
    checks[5]=vec4(tev_operation(vec3(1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.75),vec3(0.25),vec4<u32>(10u,0u,0u,1u)),1.0);
    checks[6]=vec4(tev_operation(vec3(0.25,0.5,0.75),vec3(0.5),vec3(0.75),vec3(0.25),vec4<u32>(14u,0u,0u,1u)),1.0);
    checks[7]=vec4(tev_operation(vec3(0.25,0.5,0.75),vec3(0.5),vec3(0.75),vec3(0.25),vec4<u32>(15u,0u,0u,1u)),1.0);
    layer.operations=vec4<u32>(4u,3u,0u,1u);
    checks[8]=combine(vec4(0.5),custom_texture(tex,layer),layer);
    layer.constants[2]=vec4(0.125,0.25,0.5,0.75);
    layer.color_inputs=vec4<u32>(15u,15u,15u,135u);
    layer.alpha_inputs=vec4<u32>(7u,7u,7u,69u);
    checks[9]=custom_texture(tex,layer);
}
"#;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Material fixtures"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Material numeric probe"),
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: Default::default(),
        cache: None,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU results"),
        size: 160,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Readback"),
        size: 160,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 7,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 160);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = readback.slice(..).get_mapped_range()?;
    let actual: &[[f32; 4]] = bytemuck::cast_slice(&bytes);
    let expected = [
        [0.25, 0.5, 0.75, 0.5],
        [0.0625, 0.125, 0.1875, 0.125],
        [0.625, 0.625, 0.625, 1.0],
        [-0.5, -0.5, -0.5, 1.0],
        [1.0, 1.0, 1.0, 1.0],
        [0.25, 0.25, 0.25, 1.0],
        [0.25, 0.25, 1.0, 1.0],
        [0.25, 1.0, 0.25, 1.0],
        [0.03125, 0.0625, 0.09375, 0.0625],
        [0.125, 0.25, 0.5, 0.75],
    ];
    for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        for channel in 0..4 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 0.000001,
                "fixture {i} channel {channel}: {} != {}",
                actual[channel],
                expected[channel]
            );
        }
    }
    println!(
        "10 material GPU fixtures passed on {:?}",
        adapter.get_info().backend
    );
    Ok(())
}
