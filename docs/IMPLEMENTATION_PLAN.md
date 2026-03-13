# Implementation Plan

## Overview

alkminer is being built in phases, starting with core infrastructure and progressing toward the full stratified nonce sampling pipeline.

## Dependencies

- **Rust**: 1.94.0+
- **wgpu**: v24.0.5 (pinned, repo at `/workspace/wgpu` tag `wgpu-v24.0.5`)
- **handlebars**: v6 (repo at `/workspace/handlebars-rust`)
- **Reference implementation**: `/workspace/webgpu` (TypeScript/Deno prototype)

## Testing Strategy

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

### Phase 0: Research & Infrastructure

**Goal**: Establish testing patterns and mock infrastructure

**Tasks**:
- [ ] Implement `DeviceRegistry::mock(count: usize)` for testing
- [ ] Research wgpu testing patterns from `/workspace/wgpu/tests/`
- [ ] Document Bayesian abandonment parameters from research doc
- [ ] Set up benchmarking infrastructure

**Testing**: No GPU required

---

### Phase 1: Core Compute Framework

**Goal**: Complete GPU compute abstraction layer

**Status**: Partially complete

**Completed**:
- [x] `DeviceRegistry` with multi-GPU enumeration
- [x] `DeviceHandle` with pytorch-style naming
- [x] `GpuBuffer` wrapper
- [x] `Kernel` and `ShaderBuilder`
- [x] `ComputeModule` trait

**Remaining**:
- [ ] Test GPU buffer read/write operations
- [ ] Test kernel creation and execution
- [ ] Implement concrete ComputeModule example
- [ ] Integration tests with software fallback

**Testing**: Software fallback sufficient

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

**Goal**: Implement stratified nonce sampling with Bayesian abandonment

**Components**:

1. **BatchCoordinator**
   - Manages batch lifecycle
   - Coordinates merkle root generation → header nonce sampling
   - Tracks results per batch

2. **BayesianAbandonmentController**
   - Calculates P(H₁ | no hits after b iterations)
   - Determines when to abandon batch
   - Configurable thresholds

3. **SamplingStrategy**
   - Random header nonce sampling
   - Configurable sample size per iteration
   - State persistence for resumption

**Key Parameters** (see `docs/mining_parameters.md` for details):
- Batch size M = 1024 merkle roots
- P(success per batch) ≈ 3.0% at current difficulty (144.4T)
- Expected batches until success: ~33
- Abandonment yields ~400-800x savings vs exhaustive search

**Tasks**:
- [ ] Implement Bayesian probability calculations
- [ ] Build BatchCoordinator
- [ ] Wire up modules into pipeline
- [ ] Test with mock devices
- [ ] Test with real GPU

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