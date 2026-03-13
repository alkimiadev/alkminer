use std::collections::HashMap;
use thiserror::Error;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Device, MapMode, Queue};

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("Buffer not found: {0}")]
    NotFound(String),
    #[error("Failed to map buffer")]
    MapError,
}

pub struct GpuBuffer {
    buffer: Buffer,
    size: u64,
    label: String,
}

impl GpuBuffer {
    pub fn new(device: &Device, label: &str, size: u64, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            size,
            label: label.to_string(),
        }
    }

    pub fn create_input(device: &Device, label: &str, size: u64) -> Self {
        Self::new(device, label, size, BufferUsages::STORAGE | BufferUsages::COPY_DST)
    }

    pub fn create_output(device: &Device, label: &str, size: u64) -> Self {
        Self::new(device, label, size, BufferUsages::COPY_DST | BufferUsages::MAP_READ)
    }

    pub fn create_uniform(device: &Device, label: &str, size: u64) -> Self {
        Self::new(device, label, size, BufferUsages::UNIFORM | BufferUsages::COPY_DST)
    }

    pub fn write(&self, queue: &Queue, data: &[u8]) {
        queue.write_buffer(&self.buffer, 0, data);
    }

    pub async fn read(&self, device: &Device) -> Result<Vec<u8>, BufferError> {
        let buffer_slice = self.buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        
        buffer_slice.map_async(MapMode::Read, move |result| {
            tx.send(result).ok();
        });
        
        device.poll(wgpu::Maintain::Wait);
        
        let map_result = rx.await.map_err(|_| BufferError::MapError)?;
        map_result.map_err(|_| BufferError::MapError)?;
        
        let data = buffer_slice.get_mapped_range().to_vec();
        self.buffer.unmap();
        
        Ok(data)
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

pub struct BufferManager {
    buffers: HashMap<String, GpuBuffer>,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    pub fn create(&mut self, buffer: GpuBuffer) {
        self.buffers.insert(buffer.label.clone(), buffer);
    }

    pub fn get(&self, name: &str) -> Result<&GpuBuffer, BufferError> {
        self.buffers.get(name).ok_or_else(|| BufferError::NotFound(name.to_string()))
    }

    pub fn remove(&mut self, name: &str) -> Option<GpuBuffer> {
        self.buffers.remove(name)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}