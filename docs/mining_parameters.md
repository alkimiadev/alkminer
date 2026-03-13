# Mining Probability Parameters

## Overview

This document summarizes the key parameters for GPU-parallelized Bitcoin mining using batched merkle root generation.

## The Strategy

1. **Generate** a batch of N merkle roots (default: 1024)
2. **Test** all 2^32 header nonces against all N merkle roots exhaustively
3. **Repeat** with new batches until a valid nonce is found

**No early abandonment** - always exhaust each batch completely.

## The Benefit: GPU Parallelism

The speedup comes from testing 1 header nonce against N merkle roots in parallel:

| Approach | Per Operation | Parallelism |
|----------|---------------|-------------|
| Traditional | 1 header × 1 merkle | Header nonces only |
| Batched | 1 header × N merkles | Headers AND merkles |

Each GPU thread handles one merkle root. The header nonce is broadcast to all threads.

**Not fewer hashes - faster execution due to:**
- All N merkle roots stay in GPU memory (32KB for N=1024)
- No CPU-GPU synchronization between merkle roots
- Better GPU utilization

## Key Parameters

### At Difficulty 144.4T (current as of Mar 2026)

| Parameter | Value |
|-----------|-------|
| Expected nonce cycles (D) | ~33,600 |
| P(batch of 1024 has valid) | ~3.0% |
| Expected batches until success | ~33 |
| Total merkle roots generated | ~33,600 |
| Total header tests | ~144T combinations |

### Derivation

```
D = difficulty / 2^32                    # expected nonce cycles

P(batch has valid) = 1 - exp(-N/D)       # N = batch size
                   ≈ N/D (for small values)

Expected batches = 1 / P(batch has valid)
                 ≈ D/N
```

## Why No Early Abandonment?

The cost ratio makes early abandonment non-optimal:

| Cost | Hashes | Ratio |
|------|--------|-------|
| Generate batch of 1024 merkles | ~2M | 1 |
| Test batch exhaustively | ~8.8T | 1:4,000,000 |

Abandoning early wastes trillions of hashes to save millions. Always exhaust.

**Bayesian update doesn't help:**
- Prior P(valid) = 3%
- After testing 50% with no find: posterior = 1.5%
- Still worth continuing (remaining work < cost of new batch)

## Batch Size Selection

All batch sizes generate the same total merkle roots:

| Batch Size | P(valid) | Expected Batches | Total Merkles |
|------------|----------|------------------|---------------|
| 256 | 0.8% | 131 | 33,600 |
| 512 | 1.5% | 66 | 33,600 |
| 1024 | 3.0% | 33 | 33,600 |
| 2048 | 6.1% | 16 | 33,600 |
| 4096 | 12.2% | 8 | 33,600 |

**Batch size tradeoffs:**

| Size | GPU Memory | Parallelism | Notes |
|------|------------|-------------|-------|
| 256 | 8 KB | Lower | More batches, more overhead |
| 1024 | 32 KB | Good | Fits in L1 cache, reasonable choice |
| 4096 | 128 KB | Higher | Fewer batches, more memory |

## Time Estimates

Per batch (N=1024, testing all 2^32 headers):

| Samples/sec | Time per Batch | Total (33 batches) |
|-------------|----------------|-------------------|
| 10M (conservative) | ~7 min | ~4 hours |
| 100M (optimistic) | ~0.7 min | ~24 min |

**With multiple GPUs:**

| GPUs | Conservative | Optimistic |
|------|--------------|------------|
| 1 | 4 hours | 24 min |
| 10 | 24 min | 2.4 min |
| 80 | 3 min | 18 sec |

These are EXPECTED times. Actual follows geometric distribution with high variance.

## Sampling Math

Each sample tests 1024 × 1024 = 1M combinations:

| Samples | % of Batch | P(find \| valid exists) |
|---------|------------|------------------------|
| 1 | 0.00002% | 0.00002% |
| 1,000 | 0.02% | 0.02% |
| 1,000,000 | 24% | 21% |
| 4,194,304 | 100% | ~63% |

To find a valid nonce (if it exists): need to test ~50% of batch on average.

## Configuration

```rust
struct MiningConfig {
    batch_size: usize,              // 1024 merkle roots per batch
    exhaust_batch: bool,            // true - always test all headers
    header_nonces_per_kernel: usize, // GPU workgroup size for parallelism
}
```

## Caveats and Testing Needed

1. **Actual GPU throughput** - need benchmarks to validate samples/sec estimates
2. **Memory bandwidth** - 32KB per sample may hit bandwidth limits at high rates
3. **Kernel efficiency** - overhead from GPU kernel launch, synchronization
4. **Comparison to traditional** - need side-by-side benchmark on same hardware

## References

- `docs/research/stratified_nonce_sampling.md` - Original theory
  - **Note**: Contains errors in Bayesian derivation
  - Claimed 400x from "early abandonment" is incorrect
- `/workspace/webgpu/mining-calc/mining_calc.py` - Python calculations
- `/workspace/webgpu/README.md` - TypeScript implementation