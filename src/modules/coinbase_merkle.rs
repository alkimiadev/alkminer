use futures::future::BoxFuture;
use serde::Serialize;
use wgpu::{
    BindGroup, BindGroupEntry, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
    ComputePipelineDescriptor, Device, PipelineLayoutDescriptor, Queue, ShaderModuleDescriptor,
};

use crate::compute::{ComputeModule, GpuBuffer, ModuleError};

const SHADER_TEMPLATE: &str = include_str!("../../shaders/templates/coinbase_merkle_packed.hbs");
const SHA256_PARTIAL: &str = include_str!("../../shaders/partials/sha256.hbs");
const RNG_PARTIAL: &str = include_str!("../../shaders/partials/rng.hbs");

#[derive(Serialize)]
struct ShaderParams {
    workgroup_size: u32,
    max_size_words: u32,
}

pub struct CoinbaseMerkleConfig {
    pub coinbase_template: Vec<u8>,
    pub nonce_byte_offset: u32,
    pub merkle_branches: Vec<[u8; 32]>,
    pub batch_size: u32,
    pub seed: u32,
}

pub struct CoinbaseMerkleModule {
    config: CoinbaseMerkleConfig,
    template_buffer: Option<GpuBuffer>,
    params_buffer: Option<GpuBuffer>,
    merkle_root_buffer: Option<GpuBuffer>,
    branch_buffer: Option<GpuBuffer>,
    staging_buffer: Option<wgpu::Buffer>,
    pipeline: Option<wgpu::ComputePipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl CoinbaseMerkleModule {
    pub fn new(config: CoinbaseMerkleConfig) -> Self {
        Self {
            config,
            template_buffer: None,
            params_buffer: None,
            merkle_root_buffer: None,
            branch_buffer: None,
            staging_buffer: None,
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
        }
    }

    pub fn batch_size(&self) -> u32 {
        self.config.batch_size
    }

    pub async fn read_merkle_roots(&self, device: &Device) -> Result<Vec<u8>, ModuleError> {
        let staging = self.staging_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;

        let (tx, rx) = futures::channel::oneshot::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| ModuleError::ExecutionFailed("Channel closed".into()))?
            .map_err(|_| ModuleError::ExecutionFailed("Map failed".into()))?;

        let data = staging.slice(..).get_mapped_range().to_vec();
        staging.unmap();

        Ok(data)
    }

    fn pack_template(&self) -> Vec<u32> {
        let template = &self.config.coinbase_template;
        let word_count = (template.len() + 3) / 4;
        let mut packed = vec![0u32; word_count];

        for (i, &byte) in template.iter().enumerate() {
            let word_idx = i / 4;
            let byte_idx = i % 4;
            packed[word_idx] |= (byte as u32) << (byte_idx * 8);
        }

        packed
    }

    fn pack_params(&self) -> Vec<u32> {
        vec![
            self.config.batch_size,
            self.config.coinbase_template.len() as u32,
            self.config.nonce_byte_offset,
            self.config.merkle_branches.len() as u32,
            self.config.seed,
        ]
    }

    fn pack_branches(&self) -> Vec<u32> {
        let branch_count = self.config.merkle_branches.len();
        let mut packed = Vec::with_capacity(branch_count * 8);

        for branch in &self.config.merkle_branches {
            for chunk in branch.chunks(4) {
                let word = if chunk.len() == 4 {
                    u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                } else {
                    let mut bytes = [0u8; 4];
                    bytes[..chunk.len()].copy_from_slice(chunk);
                    u32::from_be_bytes(bytes)
                };
                packed.push(word);
            }
        }

        packed
    }

    fn compile_shader(&self) -> Result<String, ModuleError> {
        let mut handlebars = handlebars::Handlebars::new();
        handlebars.set_strict_mode(false);

        handlebars
            .register_partial("sha256", SHA256_PARTIAL)
            .map_err(|e| ModuleError::SetupFailed(format!("Failed to register sha256 partial: {}", e)))?;
        handlebars
            .register_partial("rng", RNG_PARTIAL)
            .map_err(|e| ModuleError::SetupFailed(format!("Failed to register rng partial: {}", e)))?;
        handlebars
            .register_template_string("coinbase_merkle", SHADER_TEMPLATE)
            .map_err(|e| ModuleError::SetupFailed(format!("Failed to register template: {}", e)))?;

        let params = ShaderParams {
            workgroup_size: 256,
            max_size_words: 16,
        };

        handlebars
            .render("coinbase_merkle", &params)
            .map_err(|e| ModuleError::SetupFailed(format!("Failed to render shader: {}", e)))
    }
}

impl ComputeModule for CoinbaseMerkleModule {
    fn setup<'a>(
        &'a mut self,
        device: &'a Device,
        _queue: &'a Queue,
    ) -> BoxFuture<'a, Result<(), ModuleError>> {
        Box::pin(async move {
            let template_data = self.pack_template();
            let params_data = self.pack_params();
            let branch_data = self.pack_branches();

            let template_size = (template_data.len() * 4) as u64;
            let params_size = (params_data.len() * 4) as u64;
            let merkle_size = (self.config.batch_size as u64) * 32;
            let branch_size = (branch_data.len() * 4) as u64;

            self.template_buffer = Some(GpuBuffer::new(
                device,
                "template_buffer",
                template_size,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            ));

            self.params_buffer = Some(GpuBuffer::new(
                device,
                "params_buffer",
                params_size,
                BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            ));

            self.merkle_root_buffer = Some(GpuBuffer::new(
                device,
                "merkle_root_buffer",
                merkle_size,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            ));

            self.branch_buffer = Some(GpuBuffer::new(
                device,
                "branch_buffer",
                branch_size,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            ));

            self.staging_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("merkle_staging"),
                size: merkle_size,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));

            let shader_source = self.compile_shader()?;
            let shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("coinbase_merkle_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

            self.bind_group_layout = Some(device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("coinbase_merkle_bind_group_layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                },
            ));

            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("coinbase_merkle_pipeline_layout"),
                bind_group_layouts: &[self.bind_group_layout.as_ref().unwrap()],
                push_constant_ranges: &[],
            });

            self.pipeline = Some(device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("coinbase_merkle_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("coinbase_merkle_batch"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            }));

            self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("coinbase_merkle_bind_group"),
                layout: self.bind_group_layout.as_ref().unwrap(),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.template_buffer.as_ref().unwrap().buffer().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.params_buffer.as_ref().unwrap().buffer().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: self.merkle_root_buffer.as_ref().unwrap().buffer().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: self.branch_buffer.as_ref().unwrap().buffer().as_entire_binding(),
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
            let template_buffer = self.template_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            let params_buffer = self.params_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            let branch_buffer = self.branch_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            let merkle_root_buffer = self.merkle_root_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;
            let staging_buffer = self.staging_buffer.as_ref().ok_or(ModuleError::NotInitialized)?;

            let template_data = self.pack_template();
            let params_data = self.pack_params();
            let branch_data = self.pack_branches();

            template_buffer.write(queue, bytemuck::cast_slice(&template_data));
            params_buffer.write(queue, bytemuck::cast_slice(&params_data));
            branch_buffer.write(queue, bytemuck::cast_slice(&branch_data));

            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("coinbase_merkle_encoder"),
            });

            {
                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("coinbase_merkle_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);

                let workgroup_count = (self.config.batch_size + 255) / 256;
                pass.dispatch_workgroups(workgroup_count, 1, 1);
            }

            let output_size = (self.config.batch_size as u64) * 32;
            encoder.copy_buffer_to_buffer(
                merkle_root_buffer.buffer(),
                0,
                staging_buffer,
                0,
                output_size,
            );

            queue.submit(Some(encoder.finish()));

            Ok(())
        })
    }

    fn destroy(&mut self) {
        self.template_buffer = None;
        self.params_buffer = None;
        self.merkle_root_buffer = None;
        self.branch_buffer = None;
        self.staging_buffer = None;
        self.pipeline = None;
        self.bind_group_layout = None;
        self.bind_group = None;
    }
}