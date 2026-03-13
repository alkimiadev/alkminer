# Testing Strategy

This document describes the testing approach for alkminer, designed to work without requiring physical GPU hardware for most tests.

## Overview

### Test Categories

| Category | GPU Required | Approach |
|----------|--------------|----------|
| CPU crypto primitives | No | Standard unit tests |
| GPU buffer operations | No (software fallback) | `force_fallback_adapter: true` |
| Kernel execution | No (software fallback) | `force_fallback_adapter: true` |
| ComputeModule lifecycle | No (software fallback) | `force_fallback_adapter: true` |
| Multi-GPU orchestration | No | `DeviceRegistry::mock(count)` |
| Performance benchmarks | Yes | vast.ai GPU instances |
| Multi-GPU execution | Yes | vast.ai multi-GPU instances |

## Test Infrastructure

### Software Fallback

wgpu provides CPU-based software rendering via `force_fallback_adapter`:

```rust
pub async fn create_test_device() -> (Device, Queue) {
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: true,  // Forces CPU/software adapter
        compatible_surface: None,
    }).await.expect("No adapter available");
    
    adapter.request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device")
}
```

This works on any machine, including CI runners without GPUs.

### Mock Device Registry

For testing multi-GPU orchestration logic without hardware:

```rust
let registry = DeviceRegistry::mock(4);  // Simulates 4 GPUs
```

Mock devices have:
- Unique IDs: `"MockGPU:0"`, `"MockGPU:1"`, etc.
- `DeviceType::Cpu`
- Cannot actually execute kernels (use software fallback for that)

### Async Test Pattern

GPU operations are async. Use `#[tokio::test]`:

```rust
#[tokio::test]
async fn test_buffer_roundtrip() {
    let (device, queue) = create_test_device().await;
    
    let buffer = GpuBuffer::create_input(&device, "test", 64);
    buffer.write(&queue, &[1u8; 64]);
    
    let output = GpuBuffer::create_output(&device, "out", 64);
    // ... execute kernel to copy buffer ...
    
    let data = output.read(&device).await.expect("read failed");
    assert_eq!(data.len(), 64);
}
```

### Buffer Readback Pattern

The standard pattern for reading GPU results:

```rust
use wgpu::MapMode;

// 1. Create staging buffer with MAP_READ | COPY_DST
let staging = device.create_buffer(&wgpu::BufferDescriptor {
    size: 64,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
    ..Default::default()
});

// 2. Copy compute output to staging
encoder.copy_buffer_to_buffer(&compute_buffer, 0, &staging, 0, 64);

// 3. Submit commands
queue.submit(Some(encoder.finish()));

// 4. Map for reading (async)
let (tx, rx) = futures::channel::oneshot::channel();
staging.slice(..).map_async(MapMode::Read, move |r| { let _ = tx.send(r); });

// 5. Poll to complete mapping
device.poll(wgpu::Maintain::Wait);

// 6. Await and read
rx.await.expect("channel closed").expect("map failed");
let data = staging.slice(..).get_mapped_range().to_vec();
staging.unmap();
```

Note: `GpuBuffer::read()` already encapsulates this pattern.

## Test Organization

```
tests/
├── common/mod.rs           # Shared test utilities (create_test_device, shaders)
├── crypto_tests.rs         # SHA-256, RNG tests
├── device_tests.rs         # DeviceRegistry enumeration & lookup
├── buffer_tests.rs         # GpuBuffer read/write operations
└── kernel_tests.rs         # Kernel creation & execution
```

### Test Utilities (`tests/common/mod.rs`)

```rust
pub async fn create_test_device() -> (wgpu::Device, wgpu::Queue) { ... }

pub const INCREMENT_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    data[id.x] = data[id.x] + 1u;
}
"#;

pub const COPY_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    output[id.x] = input[id.x];
}
"#;
```

### Key Testing Patterns

**Buffer write/read requires submit + poll:**
```rust
buffer.write(&queue, &data);
let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
queue.submit(Some(encoder.finish()));
device.poll(wgpu::Maintain::Wait);
let read_data = buffer.read(&device).await.expect("read failed");
```

**Kernel execution with staging buffer:**
```rust
// 1. Create storage buffer (GPU-side)
let storage = GpuBuffer::new(&device, "storage", size, 
    BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC);

// 2. Create staging buffer (for CPU readback)
let staging = device.create_buffer(&wgpu::BufferDescriptor {
    size, usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ, ..
});

// 3. Execute kernel
let mut encoder = device.create_command_encoder(&..);
{ let mut pass = encoder.begin_compute_pass(&..); /* dispatch */ }

// 4. Copy to staging for readback
encoder.copy_buffer_to_buffer(storage.buffer(), 0, &staging, 0, size);
queue.submit(Some(encoder.finish()));

// 5. Map and read
staging.slice(..).map_async(MapMode::Read, |r| { .. });
device.poll(wgpu::Maintain::Wait);
let data = staging.slice(..).get_mapped_range().to_vec();
staging.unmap();
```

## Running Tests

### All tests (CPU only, works everywhere)
```bash
cargo test
```

### Specific test file
```bash
cargo test --test compute_tests
```

### With output for debugging
```bash
cargo test -- --nocapture
```

### Using cargo-nextest (faster, better output)
```bash
cargo nextest run
```

Note: For GPU tests, use `--test-threads=1` to avoid state conflicts:
```bash
cargo nextest run --test-threads=1
```

## GPU Testing with vast.ai

For tests requiring real GPU hardware:

### 1. Create Instance

```bash
vastai create instance \
  --image pytorch/pytorch:latest \
  --gpu-count 1 \
  --disk 50 \
  <template_id>
```

### 2. Setup Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Install Vulkan drivers (if needed)

```bash
apt-get update && apt-get install -y vulkan-tools libvulkan1
```

### 4. Run GPU-specific tests

```bash
cargo test -- --ignored  # Tests marked #[ignore] require real GPU
```

### Test Markers

Mark GPU-required tests:
```rust
#[tokio::test]
#[ignore = "Requires real GPU"]
async fn test_gpu_performance() { ... }
```

## Reference Implementation Patterns

The TypeScript prototype at `/workspace/webgpu` has test patterns in:
- `src/compute.test.ts` - Device enumeration tests
- `src/modules/*.test.ts` - Module-level tests

Key patterns to preserve:
1. Write known input data
2. Execute kernel
3. Read back and verify output matches expected

## wgpu Testing Reference

The wgpu repository (`/workspace/wgpu`) has comprehensive test infrastructure:

- `/workspace/wgpu/tests/src/init.rs` - Device initialization utilities
- `/workspace/wgpu/tests/src/run.rs` - `TestingContext` and execution
- `/workspace/wgpu/tests/tests/buffer.rs` - Buffer operation tests
- `/workspace/wgpu/tests/tests/dispatch_workgroups_indirect.rs` - Compute dispatch patterns

Key takeaways:
1. Use `force_fallback_adapter: true` for software rendering
2. `device.poll(wgpu::Maintain::Wait)` blocks until GPU work completes
3. Buffer mapping is async; use channels to await completion
4. `cargo-nextest` provides better test isolation for GPU state

## Common Issues

### "No adapters found"
- Ensure `force_fallback_adapter: true` is set
- Check that wgpu feature flags include the software backend

### Buffer mapping timeout
- Always call `device.poll(wgpu::Maintain::Wait)` after queue submission
- Ensure staging buffer has `MAP_READ` usage

### Validation errors
- Use `device.push_error_scope(wgpu::ErrorFilter::Validation)` to capture
- Check bind group layouts match shader expectations

### Tests interfering with each other
- Use `cargo nextest run --test-threads=1`
- Or use `#[serial]` from serial_test crate
