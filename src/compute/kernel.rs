use handlebars::Handlebars;
use serde::Serialize;
use thiserror::Error;
use wgpu::{
    BindGroupLayout, BindGroupLayoutEntry, ComputePipeline, Device, PipelineLayoutDescriptor,
    ShaderModuleDescriptor,
};

#[derive(Error, Debug)]
pub enum KernelError {
    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(#[from] handlebars::RenderError),
    #[error("Template error: {0}")]
    TemplateError(String),
    #[error("Pipeline creation failed: {0}")]
    PipelineCreation(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
}

pub struct ShaderBuilder {
    registry: Handlebars<'static>,
}

impl ShaderBuilder {
    pub fn new() -> Self {
        let mut registry = Handlebars::new();
        registry.set_strict_mode(true);
        Self { registry }
    }

    pub fn register_partial(&mut self, name: &str, template: &str) -> Result<(), KernelError> {
        let _ = self.registry.register_partial(name, template);
        Ok(())
    }

    pub fn register_template(&mut self, name: &str, template: &str) -> Result<(), KernelError> {
        self.registry
            .register_template_string(name, template)
            .map_err(|e| KernelError::TemplateError(e.to_string()))
    }

    pub fn render<T: Serialize>(&self, name: &str, data: &T) -> Result<String, KernelError> {
        Ok(self.registry.render(name, data)?)
    }
}

impl Default for ShaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KernelConfig {
    pub entry_point: String,
    pub bind_group_layouts: Vec<Vec<BindGroupLayoutEntry>>,
    pub workgroup_size: [u32; 3],
}

pub struct Kernel {
    pipeline: ComputePipeline,
    bind_group_layouts: Vec<BindGroupLayout>,
    entry_point: String,
    workgroup_size: [u32; 3],
}

impl Kernel {
    pub fn create(
        device: &Device,
        shader_source: &str,
        config: KernelConfig,
    ) -> Result<Self, KernelError> {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layouts: Vec<BindGroupLayout> = config
            .bind_group_layouts
            .iter()
            .enumerate()
            .map(|(group_idx, entries)| {
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("bind_group_layout_{}", group_idx)),
                    entries,
                })
            })
            .collect();

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &bind_group_layouts.iter().collect::<Vec<_>>(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some(&config.entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(Self {
            pipeline,
            bind_group_layouts,
            entry_point: config.entry_point,
            workgroup_size: config.workgroup_size,
        })
    }

    pub fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }

    pub fn bind_group_layout(&self, index: usize) -> Option<&BindGroupLayout> {
        self.bind_group_layouts.get(index)
    }

    pub fn workgroup_size(&self) -> [u32; 3] {
        self.workgroup_size
    }

    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }
}
