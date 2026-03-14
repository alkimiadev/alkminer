# Implementation Plan

## Overview

alkminer is being built in phases, starting with core infrastructure and progressing toward the full stratified nonce sampling pipeline.

## Dependencies

- **Rust**: 1.94.0+
- **wgpu**: v24.0.5 (pinned, repo at `/workspace/wgpu` tag `wgpu-v24.0.5`)
- **handlebars**: v6 (repo at `/workspace/handlebars-rust`)
- **Reference implementation**: `/workspace/webgpu` (TypeScript/Deno prototype)

## Testing Strategy

See `docs/TESTING.md` for comprehensive testing patterns and utilities.

### No GPU Required
- CPU crypto primitives (SHA-256, RNG, merkle tree)
- Multi-GPU orchestration logic (using `DeviceRegistry::mock()`)
- Unit tests for individual components

### Software Fallback
wgpu works without physical GPU via software renderer. This enables:
- Development on machines without GPUs
- CI/CD pipelines
- Testing single-device GPU code

Use `force_fallback_adapter: true` in `RequestAdapterOptions` to explicitly request software implementation.

### Real GPU Required
- Multi-GPU enumeration and execution
- Performance benchmarks
- Actual mining tests

**vast.ai Integration**: Use vast.ai CLI to spawn GPU instances for multi-GPU testing. Requirements:
- Docker container with Rust toolchain
- Vulkan drivers
- NVIDIA GPU support

## Phases

### Phase 0: Testing Infrastructure

**Goal**: Establish testing patterns and mock infrastructure

**Status**: Complete

**Completed**:
- [x] Implement `DeviceRegistry::mock(count: usize)` for testing
- [x] Create test utilities in `tests/common/mod.rs`
- [x] Add GPU buffer read/write tests
- [x] Add kernel creation/execution tests

**Testing**: No GPU required. See `docs/TESTING.md` for patterns.

**Benchmarking**: Deferred to Phase 4 (Mining Pipeline) where performance matters.

---

### Phase 1: Core Compute Framework

**Goal**: Complete GPU compute abstraction layer

**Status**: Complete

**Completed**:
- [x] `DeviceRegistry` with multi-GPU enumeration
- [x] `DeviceHandle` with pytorch-style naming
- [x] `GpuBuffer` wrapper
- [x] `Kernel` and `ShaderBuilder`
- [x] `ComputeModule` trait
- [x] Test GPU buffer read/write operations
- [x] Test kernel creation and execution
- [x] Implement concrete ComputeModule example (`IncrementModule`)
- [x] Integration tests with software fallback

**Testing**: Software fallback sufficient. All 39 tests pass without GPU.

---

### Phase 2: CPU Crypto Primitives

**Goal**: Implement CPU-side cryptographic operations for verification and nonce regeneration

**Status**: Complete

**Completed**:
- [x] SHA-256 (scalar implementation)
- [x] xoshiro128+ RNG
- [x] double_sha256 helper
- [x] Merkle tree computation
- [x] Block header hashing
- [x] Verification against known test vectors (genesis block, block 1)

**Deferred**:
- SIMD optimization (later, using portable_simd or std::arch)

**Testing**: CPU unit tests only

---

### Phase 3a: CoinbaseMerkleModule

**Goal**: Port coinbase merkle generation to GPU ComputeModule

**Status**: Complete

**Completed**:
- [x] Update `rng.hbs` partial with `generate_uniform_64` function
- [x] Create `coinbase_merkle_packed.hbs` shader template
- [x] Implement `CoinbaseMerkleModule` Rust struct
- [x] Add CPU verification utilities for GPU results
- [x] Verify GPU results match CPU implementation

**Implementation**:
- Input: coinbase template, nonce byte offset, merkle branches, batch size, seed
- GPU-side: generate uniform 64-bit nonce → modify coinbase → double SHA-256 → build merkle root
- Output: batch of merkle roots (32 bytes each)
- No intermediate host-GPU transfers
- Deterministic RNG for reproducibility

**Testing**: GL backend required for llvmpipe (Vulkan has issues with fixed-size array function parameters). Real GPU should work with Vulkan.

**Known Issue**: The Vulkan backend (llvmpipe) has issues with WGSL fixed-size array function parameters (e.g., `fn foo(data: array<u32, 16>)`). Use GL backend for CPU testing. This may be resolved in future Mesa/wgpu versions.

---

### Phase 3b: HeaderHashModule

**Goal**: Port header hashing to GPU ComputeModule

**Status**: Pending

**Design**:
- Input: block header template, merkle roots buffer, target difficulty
- GPU-side: for each merkle root, enumerate 2^32 header nonces, check against target
- Output: valid (merkle_index, header_nonce) pairs found
- Exhaustive search (no early abandonment)

**Tasks**:
- [ ] Create `header_hash.hbs` shader template
- [ ] Implement `HeaderHashModule` Rust struct
- [ ] Add seeding utilities for multi-node/multi-GPU coordination
- [ ] Verify GPU results match CPU implementation
- [ ] Benchmark single-batch throughput

**Seeding Strategy** (for reproducibility across multi-node/multi-GPU):

```
global_seed: u64  // user-provided or random
node_id: u16      // coordinator-assigned
gpu_id: u16       // device index within node
batch_index: u32  // incrementing counter

per_batch_seed = global_seed ^ ((node_id as u64) << 48) ^ ((gpu_id as u64) << 32) ^ (batch_index as u64)
```

Any winning nonce can be regenerated from: `(global_seed, node_id, gpu_id, batch_index, in_batch_index)`

**RNG**:
- Coinbase nonce: uniform 64-bit random (`high=random_u32(), low=random_u32()`)
- Header nonce: enumerate 0 → 2^32-1 (deterministic, no seed needed)

---

### Phase 4: Mining Pipeline

**Goal**: Implement batched mining with exhaustive search per batch

**Components**:

1. **BatchCoordinator**
   - Manages batch lifecycle
   - Coordinates: generate merkle roots → test all headers → repeat
   - Tracks results per batch

2. **HeaderTestingKernel**
   - Tests all 2^32 header nonces against all merkle roots in batch
   - Reports any valid nonces found
   - Exhaustive search (no early abandonment)

3. **MiningLoop**
   - Simple state machine: generate → test → check → repeat
   - Configurable batch size
   - State persistence for resumption across restarts

**Key Parameters** (see `docs/mining_parameters.md` for details):
- Batch size M = 1024 merkle roots
- P(success per batch) ≈ 3.0% at current difficulty (144.4T)
- Expected batches until success: ~33
- **No early abandonment** - always exhaust each batch
- Benefit from GPU parallelism: testing 1 header × N merkles simultaneously

**Tasks**:
- [ ] Build BatchCoordinator
- [ ] Implement exhaustive header testing kernel
- [ ] Wire up modules into pipeline
- [ ] Test with mock devices
- [ ] Test with real GPU
- [ ] Benchmark actual throughput (samples/sec)

**Testing**: Mock for orchestration logic, vast.ai for real execution

---

### Phase 5: Multi-GPU

**Goal**: Distribute work across multiple GPUs

**Components**:

1. **DeviceCoordinator**
   - Spawns independent batch workers per GPU
   - Aggregates results
   - Handles GPU failure/recovery

2. **WorkDistribution**
   - Independent batch generation per GPU
   - No coordination needed (stateless)
   - First-to-success wins

**Tasks**:
- [ ] Implement DeviceCoordinator
- [ ] Test with `DeviceRegistry::mock(4)` simulating 4 GPUs
- [ ] Deploy to vast.ai multi-GPU instance
- [ ] Benchmark scaling efficiency

**Testing**: Mock for logic, vast.ai required for real multi-GPU

---

## Future Work

Not in current scope:
- Networking via iroh
- WASM target support
- ASIC comparison benchmarks
- Pool protocol integration

## What Needs Testing

The theoretical analysis shows GPU parallelism should provide speedup, but actual performance needs validation:

1. **GPU throughput benchmark**
   - Measure actual samples/sec on target hardware
   - Compare batch size tradeoffs (256 vs 1024 vs 4096)
   - Identify bottlenecks (compute vs memory bandwidth)

2. **Kernel efficiency**
   - GPU kernel launch overhead
   - Memory transfer costs (merkle roots to GPU)
   - Thread synchronization costs

3. **End-to-end timing**
   - Time per batch (generate merkles + test all headers)
   - Compare to theoretical estimates
   - Measure scaling with multiple GPUs

4. **Memory behavior**
   - Cache efficiency at different batch sizes
   - Memory bandwidth utilization
   - Shared memory vs global memory performance

## Notes

### Adding New Modules

When adding a new ComputeModule:

1. Create struct implementing `ComputeModule` trait
2. Create shader template in `shaders/templates/`
3. Reuse partials from `shaders/partials/` (sha256, rng)
4. Add CPU verification in `src/crypto/`
5. Add tests in `tests/`

### Shader Development

Shaders are WGSL with Handlebars templating:

```wgsl
{{> sha256 max_size_words=16}}

@compute @workgroup_size({{workgroup_size}})
fn main() {
  // shader code
}
```

Templates compile identically in TypeScript and Rust implementations.