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

**Status**: Partially complete

**Completed**:
- [x] SHA-256 (scalar implementation)
- [x] xoshiro128+ RNG
- [x] double_sha256 helper

**Remaining**:
- [ ] Merkle tree computation
- [ ] Block header hashing
- [ ] SIMD optimization (later, using portable_simd or std::arch)
- [ ] Verification against known test vectors

**Testing**: CPU unit tests only

---

### Phase 3: GPU Compute Modules

**Goal**: Port TypeScript modules to Rust ComputeModules

**Modules to implement**:

1. **MerkleRootModule**
   - Input: Coinbase hash + merkle branches
   - Output: Batch of merkle roots
   - Shader: `shaders/templates/merkle_root_packed.hbs` ✓

2. **CoinbaseModule**
   - Input: Coinbase transaction template, RNG seed
   - Output: Batch of modified coinbase transactions with random nonces
   - Needs: RNG in WGSL ✓

3. **HeaderHashModule**
   - Input: Block header template + merkle roots + header nonces
   - Output: Hash results + difficulty check
   - Purpose: Test header nonces against target

**Tasks**:
- [ ] Port `MerkleRootModulePacked` from TypeScript
- [ ] Implement `CoinbaseModule`
- [ ] Implement `HeaderHashModule`
- [ ] Verify GPU results match CPU implementation

**Testing**: Software fallback sufficient for correctness, GPU for performance

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