use alkminer::compute::{GpuBuffer, Kernel, KernelConfig, ShaderBuilder};
use wgpu::BufferUsages;

mod common;

#[tokio::test]
async fn test_shader_builder_register_template() {
    let mut builder = ShaderBuilder::new();
    
    let result = builder.register_template("test", common::INCREMENT_SHADER);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shader_builder_render() {
    let mut builder = ShaderBuilder::new();
    builder.register_template("test", common::INCREMENT_SHADER).unwrap();
    
    let rendered = builder.render("test", &());
    assert!(rendered.is_ok());
    assert_eq!(rendered.unwrap(), common::INCREMENT_SHADER);
}

#[tokio::test]
async fn test_shader_builder_with_variable() {
    let mut builder = ShaderBuilder::new();
    builder.register_template("test", "Workgroup size: {{size}}").unwrap();
    
    #[derive(serde::Serialize)]
    struct Context {
        size: u32,
    }
    
    let rendered = builder.render("test", &Context { size: 64 }).unwrap();
    assert_eq!(rendered, "Workgroup size: 64");
}

#[tokio::test]
async fn test_shader_builder_register_partial() {
    let mut builder = ShaderBuilder::new();
    
    let result = builder.register_partial("sha256", "// sha256 code here");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_kernel_create_simple() {
    let (device, _) = common::create_test_device().await;
    
    let config = KernelConfig {
        entry_point: "main".to_string(),
        bind_group_layouts: vec![vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]],
        workgroup_size: [64, 1, 1],
    };
    
    let result = Kernel::create(&device, common::INCREMENT_SHADER, config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_kernel_execute_increment() {
    let (device, queue) = common::create_test_device().await;
    
    let config = KernelConfig {
        entry_point: "main".to_string(),
        bind_group_layouts: vec![vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]],
        workgroup_size: [64, 1, 1],
    };
    
    let kernel = Kernel::create(&device, common::INCREMENT_SHADER, config).unwrap();
    
    let data: Vec<u8> = (0u32..64).flat_map(|v| v.to_ne_bytes()).collect();
    let storage_buffer = GpuBuffer::new(
        &device,
        "storage",
        256,
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    );
    storage_buffer.write(&queue, &data);
    
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: 256,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind_group"),
        layout: kernel.bind_group_layout(0).unwrap(),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.buffer().as_entire_binding(),
        }],
    });
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("encoder"),
    });
    
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(kernel.pipeline());
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    
    encoder.copy_buffer_to_buffer(storage_buffer.buffer(), 0, &staging_buffer, 0, 256);
    
    queue.submit(Some(encoder.finish()));
    
    let (tx, rx) = futures::channel::oneshot::channel();
    staging_buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
    device.poll(wgpu::Maintain::Wait);
    rx.await.expect("channel closed").expect("map failed");
    
    let result: Vec<u8> = staging_buffer.slice(..).get_mapped_range().to_vec();
    staging_buffer.unmap();
    
    let result_u32: Vec<u32> = result.chunks_exact(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect();
    
    for i in 0..64 {
        assert_eq!(result_u32[i], i as u32 + 1);
    }
}

#[tokio::test]
async fn test_kernel_copy_buffers() {
    let (device, queue) = common::create_test_device().await;
    
    let config = KernelConfig {
        entry_point: "main".to_string(),
        bind_group_layouts: vec![vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]],
        workgroup_size: [64, 1, 1],
    };
    
    let kernel = Kernel::create(&device, common::COPY_SHADER, config).unwrap();
    
    let input_data: Vec<u8> = (100u32..164).flat_map(|v| v.to_ne_bytes()).collect();
    let input_buffer = GpuBuffer::create_input(&device, "input", 256);
    input_buffer.write(&queue, &input_data);
    
    let output_buffer = GpuBuffer::new(
        &device,
        "output",
        256,
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    );
    
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: 256,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind_group"),
        layout: kernel.bind_group_layout(0).unwrap(),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.buffer().as_entire_binding(),
            },
        ],
    });
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("encoder"),
    });
    
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(kernel.pipeline());
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    
    encoder.copy_buffer_to_buffer(output_buffer.buffer(), 0, &staging_buffer, 0, 256);
    
    queue.submit(Some(encoder.finish()));
    
    let (tx, rx) = futures::channel::oneshot::channel();
    staging_buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
    device.poll(wgpu::Maintain::Wait);
    rx.await.expect("channel closed").expect("map failed");
    
    let result: Vec<u8> = staging_buffer.slice(..).get_mapped_range().to_vec();
    staging_buffer.unmap();
    
    let result_u32: Vec<u32> = result.chunks_exact(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect();
    
    for i in 0..64 {
        assert_eq!(result_u32[i], 100 + i as u32);
    }
}
