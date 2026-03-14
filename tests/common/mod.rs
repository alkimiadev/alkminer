use std::future::Future;
use wgpu::{Device, Queue};

#[allow(dead_code)]
pub async fn create_test_device() -> (Device, Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await;

    let adapter = match adapter {
        Some(a) => a,
        None => {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                ..Default::default()
            });
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .expect("No adapter available")
        }
    };

    let info = adapter.get_info();
    eprintln!("Adapter: {:?}", info);

    adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device")
}

#[allow(dead_code)]
pub fn run_test<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    tokio::runtime::Runtime::new()
        .expect("Failed to create runtime")
        .block_on(f());
}

#[allow(dead_code)]
pub const INCREMENT_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    data[id.x] = data[id.x] + 1u;
}
"#;

#[allow(dead_code)]
pub const COPY_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    output[id.x] = input[id.x];
}
"#;
