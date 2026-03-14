use futures::future::BoxFuture;
use wgpu::{
    BindGroup, BindGroupEntry, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
    ComputePipelineDescriptor, Device, PipelineLayoutDescriptor, Queue, ShaderModuleDescriptor,
};

use crate::compute::{ComputeModule, GpuBuffer, ModuleError};

const INCREMENT_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    output[id.x] = input[id.x] + 1u;
}
"#;

pub struct IncrementModule {
    count: u64,
    input_buffer: Option<GpuBuffer>,
    output_buffer: Option<GpuBuffer>,
    staging_buffer: Option<wgpu::Buffer>,
    pipeline: Option<wgpu::ComputePipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl IncrementModule {
    pub fn new(count: u64) -> Self {
        Self {
            count,
            input_buffer: None,
            output_buffer: None,
            staging_buffer: None,
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
        }
    }

    pub fn write(&self, queue: &Queue, data: &[u8]) -> Result<(), ModuleError> {
        let buffer = self.input_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
        buffer.write(queue, data);
        Ok(())
    }

    pub async fn read_output(&self, device: &Device) -> Result<Vec<u8>, ModuleError> {
        let staging = self.staging_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
        
        let (tx, rx) = futures::channel::oneshot::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        device.poll(wgpu::Maintain::Wait);
        
        rx.await
            .map_err(|_| ModuleError::ExecutionFailed("Channel closed".into()))?
            .map_err(|_| ModuleError::ExecutionFailed("Map failed".into()))?;
        
        let data = staging.slice(..).get_mapped_range().to_vec();
        staging.unmap();
        
        Ok(data)
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

impl ComputeModule for IncrementModule {
    fn setup<'a>(
        &'a mut self,
        device: &'a Device,
        _queue: &'a Queue,
    ) -> BoxFuture<'a, Result<(), ModuleError>> {
        Box::pin(async move {
            let size_bytes = self.count * 4;
            
            self.input_buffer = Some(GpuBuffer::new(
                device,
                "increment_input",
                size_bytes,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            ));
            
            self.output_buffer = Some(GpuBuffer::new(
                device,
                "increment_output",
                size_bytes,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            ));
            
            self.staging_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("increment_staging"),
                size: size_bytes,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            
            let shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("increment_shader"),
                source: wgpu::ShaderSource::Wgsl(INCREMENT_SHADER.into()),
            });
            
            self.bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("increment_bind_group_layout"),
                entries: &[
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
                ],
            }));
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("increment_pipeline_layout"),
                bind_group_layouts: &[self.bind_group_layout.as_ref().unwrap()],
                push_constant_ranges: &[],
            });
            
            self.pipeline = Some(device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("increment_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            }));
            
            self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("increment_bind_group"),
                layout: self.bind_group_layout.as_ref().unwrap(),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.input_buffer.as_ref().unwrap().buffer().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.output_buffer.as_ref().unwrap().buffer().as_entire_binding(),
                    },
                ],
            }));
            
            Ok(())
        })
    }

    fn run<'a>(
        &'a mut self,
        device: &'a Device,
        queue: &'a Queue,
    ) -> BoxFuture<'a, Result<(), ModuleError>> {
        Box::pin(async move {
            let pipeline = self.pipeline.as_ref().ok_or(ModuleError::NotInitialized)?;
            let bind_group = self.bind_group.as_ref().ok_or(ModuleError::NotInitialized)?;
            let output_buffer = self.output_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            let staging_buffer = self.staging_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("increment_encoder"),
            });
            
            {
                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("increment_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch_workgroups(((self.count + 63) / 64) as u32, 1, 1);
            }
            
            encoder.copy_buffer_to_buffer(output_buffer.buffer(), 0, staging_buffer, 0, self.count * 4);
            
            queue.submit(Some(encoder.finish()));
            
            Ok(())
        })
    }

    fn destroy(&mut self) {
        self.input_buffer = None;
        self.output_buffer = None;
        self.staging_buffer = None;
        self.pipeline = None;
        self.bind_group_layout = None;
        self.bind_group = None;
    }
}
