# AGENTS.md

Context for AI agents working on alkminer.

## Project Overview

alkminer is a Rust implementation of a GPU-accelerated Bitcoin mining framework using stratified nonce sampling. It's being rewritten from a TypeScript/Deno prototype located at `/workspace/webgpu`.

**Implementation Plan**: See `docs/IMPLEMENTATION_PLAN.md` for phases, tasks, and testing strategy.

### Core Concept: Stratified Nonce Sampling

See `docs/research/stratified_nonce_sampling.md` for the full theory. Key points:

- Bitcoin mining has a 96-bit effective nonce space (64-bit coinbase nonce + 32-bit header nonce)
- Changing coinbase nonce requires recomputing merkle root (~2000 SHA-256 ops)
- Changing header nonce is cheap (2 SHA-256 ops)
- Cost ratio: ~1000x

**Strategy**: Generate batches of merkle roots upfront, randomly sample header nonces, abandon batches early using Bayesian probability when no valid nonces are found.

### Key Probabilities (at difficulty ~50T)

- ~8.4% chance a batch of 1024 merkle roots contains at least one valid header nonce
- ~91.6% of batches are "duds" - can abandon after testing ~0.25% of header space
- This yields ~400x savings vs exhaustive search

## Architecture

### Two Parallel Tracks

1. **GPU Compute Framework** (`src/compute/`)
   - wgpu-based device abstraction
   - Multi-GPU support with pytorch-style naming (e.g., "RTX A6000:0", "RTX A6000:1")
   - ComputeModule trait for reusable GPU computations
   - Handlebars-based shader templating (templates work across TS/Rust implementations)

2. **CPU Crypto Primitives** (`src/crypto/`)
   - SHA-256 (scalar first, SIMD optimization later)
   - xoshiro128+ RNG
   - Merkle tree computation
   - Used for verification and regenerating winning nonces from metadata

### Directory Structure

```
alkminer/
├── Cargo.toml
├── AGENTS.md
├── src/
│   ├── lib.rs
│   ├── compute/
│   │   ├── mod.rs
│   │   ├── device.rs       # DeviceRegistry, DeviceHandle
│   │   ├── buffer.rs       # GpuBuffer wrapper
│   │   ├── kernel.rs       # ComputePipeline wrapper
│   │   └── module.rs       # ComputeModule trait
│   └── crypto/
│       ├── mod.rs
│       ├── sha256.rs       # SHA-256 implementation
│       └── rng.rs          # xoshiro128+ RNG
├── shaders/
│   ├── partials/           # Reusable WGSL fragments
│   │   ├── sha256.hbs
│   │   └── rng.hbs
│   └── templates/          # Complete shader programs
│       └── merkle_root.hbs
└── tests/
```

## Dependencies

- **wgpu** (v24.0.5): GPU compute API. Reference repo at `/workspace/wgpu` checked out at tag `wgpu-v24.0.5`
- **handlebars** (v6): Shader templating. Reference repo at `/workspace/handlebars-rust`

### wgpu Notes

- `AdapterInfo` provides `name`, `vendor`, `device`, `device_type`
- **Critical for multi-GPU**: Same-model GPUs have IDENTICAL `name`, `vendor`, `device` values
- Cannot deduplicate by `(name, vendor, device)` tuple - must use enumeration order
- `Instance::enumerate_adapters()` returns `Vec<Adapter>` directly (synchronous in v24)
- Device naming: use enumeration index for uniqueness - `"RTX A6000:0"`, `"RTX A6000:1"`, etc.
- Known issue: `WGPU_ADAPTER_NAME` env var only selects first matching GPU (see `/workspace/wgpu/wgpu/src/util/init.rs`)
- Enumeration order typically follows PCI bus order, should be consistent across runs

### handlebars Notes

- Templates use `{{variable}}` syntax
- Partials: `{{> partial_name}}`
- Conditional: `{{#if condition}}...{{/if}}`
- Context passed as serde_json::Value or any serde::Serialize type
- API is nearly identical to JS handlebars - templates should port directly

## Reference Implementation

TypeScript/Deno prototype at `/workspace/webgpu`:

- `src/compute.ts` - ComputeEngine class
- `src/compute_module.ts` - ComputeModule base class
- `src/shaders/partials/sha256.hbs` - SHA-256 WGSL implementation
- `src/shaders/partials/rng.hbs` - xoshiro128+ RNG
- `src/modules/merkle_root_module_packed.ts` - Example module

### Key Patterns to Preserve

1. **Named tensors**: Store GPU buffers by name for easy reference
2. **Template composition**: Build shaders from partials
3. **Module lifecycle**: init() → setup() → run() → destroy()

## Development Guidelines

### Code Style

- No comments unless explicitly requested
- Follow existing patterns in the codebase
- Use `thiserror` for error types

### Testing

- Run `cargo test` after making changes
- Add tests for new functionality

### SIMD Strategy

Start with correct scalar implementations. SIMD optimization comes later using either:
- `portable_simd` (nightly)
- `std::arch` intrinsics (stable, platform-specific)

### Multi-GPU Pattern

```rust
// Device naming: name:index (pytorch style)
let registry = DeviceRegistry::enumerate()?;
let gpu = registry.get("NVIDIA RTX A6000:0")?;

// Each ComputeModule bound to single device
let mut module = MerkleRootModule::new();
module.setup(&gpu.device, &gpu.queue)?;
```

## Testing Without GPU

wgpu works without a physical GPU (software fallback). This is useful for:
- Development on machines without GPUs
- CI/CD pipelines
- Testing orchestration logic

For multi-GPU testing without hardware, use `DeviceRegistry::mock()` (to be implemented) or rent GPU instances from vast.ai.