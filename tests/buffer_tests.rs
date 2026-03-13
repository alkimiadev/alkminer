use alkminer::compute::{GpuBuffer, BufferManager};
use wgpu::BufferUsages;

mod common;

#[tokio::test]
async fn test_buffer_create_input() {
    let (device, _) = common::create_test_device().await;
    let buffer = GpuBuffer::create_input(&device, "test_input", 256);
    
    assert_eq!(buffer.size(), 256);
}

#[tokio::test]
async fn test_buffer_create_output() {
    let (device, _) = common::create_test_device().await;
    let buffer = GpuBuffer::create_output(&device, "test_output", 512);
    
    assert_eq!(buffer.size(), 512);
}

#[tokio::test]
async fn test_buffer_create_uniform() {
    let (device, _) = common::create_test_device().await;
    let buffer = GpuBuffer::create_uniform(&device, "test_uniform", 64);
    
    assert_eq!(buffer.size(), 64);
}

#[tokio::test]
async fn test_buffer_write_and_read() {
    let (device, queue) = common::create_test_device().await;
    
    let buffer = GpuBuffer::new(
        &device,
        "test",
        64,
        BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    );
    
    let input_data: Vec<u8> = (0..64).collect();
    buffer.write(&queue, &input_data);
    
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
    
    let read_data = buffer.read(&device).await.expect("Failed to read buffer");
    
    assert_eq!(read_data.len(), 64);
    assert_eq!(read_data, input_data);
}

#[tokio::test]
async fn test_buffer_write_u32_values() {
    let (device, queue) = common::create_test_device().await;
    
    let buffer = GpuBuffer::new(
        &device,
        "test_u32",
        16,
        BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    );
    
    let input_data: Vec<u8> = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0];
    buffer.write(&queue, &input_data);
    
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
    
    let read_data = buffer.read(&device).await.expect("Failed to read buffer");
    
    assert_eq!(read_data, input_data);
}

#[test]
fn test_buffer_manager_create_and_get() {
    let manager = BufferManager::new();
    assert!(manager.get("nonexistent").is_err());
}

#[tokio::test]
async fn test_buffer_manager_store_and_retrieve() {
    let (device, _) = common::create_test_device().await;
    let mut manager = BufferManager::new();
    
    let buffer = GpuBuffer::create_input(&device, "my_buffer", 128);
    manager.create(buffer);
    
    let retrieved = manager.get("my_buffer");
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap().size(), 128);
}

#[tokio::test]
async fn test_buffer_manager_remove() {
    let (device, _) = common::create_test_device().await;
    let mut manager = BufferManager::new();
    
    let buffer = GpuBuffer::create_input(&device, "to_remove", 64);
    manager.create(buffer);
    
    let removed = manager.remove("to_remove");
    assert!(removed.is_some());
    
    assert!(manager.get("to_remove").is_err());
}

#[tokio::test]
async fn test_buffer_manager_clear() {
    let (device, _) = common::create_test_device().await;
    let mut manager = BufferManager::new();
    
    manager.create(GpuBuffer::create_input(&device, "buf1", 64));
    manager.create(GpuBuffer::create_input(&device, "buf2", 64));
    
    manager.clear();
    
    assert!(manager.get("buf1").is_err());
    assert!(manager.get("buf2").is_err());
}
