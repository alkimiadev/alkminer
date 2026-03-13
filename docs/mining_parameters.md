# Mining Probability Parameters

## Overview

This document summarizes the key parameters for the GPU-parallelized Bitcoin mining strategy using batched merkle root generation.

## The Core Insight: GPU Parallelism

**Traditional mining**: Test each header × merkle combination individually

**Our strategy**: Test 1 header nonce against 1024 merkle roots simultaneously

```
Traditional: 1 hash → 1 combination tested
Our batching: 1 hash → 1024 combinations tested (GPU broadcast)

Savings: ~1024x
```

The savings come from GPU utilization, not from "early abandonment" or Bayesian probability tricks.

## The "Needles in Haystack" Context

At current difficulty, there are ~127,000 valid nonces per block template:

```
Valid nonces in 96-bit space = 2^64 / difficulty
                               = 1.84 × 10^19 / 144.4 × 10^12
                               ≈ 127,000
```

These are uniformly distributed across the (merkle_root, header_nonce) combinations.

## Key Parameters

### At Difficulty 144.4T (current as of Mar 2026)

| Parameter | Value |
|-----------|-------|
| Expected nonce cycles (D) | ~33,600 |
| P(batch has valid nonce) | ~3.0% |
| Expected batches until success | ~33 |
| Expected header samples | ~141B |
| Expected hash cost | ~282B hashes |
| Merkle generation cost | ~67M hashes (negligible) |

### Derivation

```
Expected nonce cycles (D) = difficulty / 2^32

P(batch has valid nonce) = 1 - exp(-1024 / D)
                         ≈ 1024 / D (for small p)
                         ≈ 3.0%

Expected header samples = D × 2^32 / 1024
                        ≈ 141 billion

Expected hash cost = samples × 2 hashes per header
                   ≈ 282 billion hashes
```

## Comparison to Traditional Mining

| Metric | Traditional | Our Batching | Improvement |
|--------|-------------|--------------|-------------|
| Combinations to test | 144T | 144T | Same |
| Hash cost | 289T hashes | 282B hashes | **1024x** |
| Merkle generations | 33,600 | 33,621 | Same |
| Time (1 GPU, 120 MH/s) | ~76 years | ~27 days | 1024x |

The key: we test the same number of combinations, but each GPU operation tests 1024 combinations instead of 1.

## Multi-GPU Scaling

With N independent GPUs operating in parallel:

| GPUs | P(success round 1) | Expected time (at 120 MH/s each) |
|------|--------------------|----------------------------------|
| 1 | 3.0% | ~27 days |
| 10 | 26% | ~2.7 days |
| 100 | 95% | ~6.5 hours |

Each GPU independently generates batches and samples. First to find a valid nonce wins.

**Economics (vast.ai pricing):**
- 12× RTX 4090: $3.20/hr
- Expected time to find block: ~2.3 days
- Expected cost: ~$176
- Block reward: ~$218,750

Note: This is expected value. Actual time follows geometric distribution with high variance.

## Cost Structure

| Operation | Hash Cost | Notes |
|-----------|-----------|-------|
| Merkle root generation | ~2000 | Per root, depends on transaction count |
| Header nonce test | 2 | Double SHA-256 |
| Batch header test | 2 | Tests 1 header × 1024 merkles (GPU parallel) |

**Cost ratio**: Merkle generation is ~1000x more expensive than header testing.

However, with batching:
- Merkle cost: 33 batches × 1024 roots × 2000 hashes = 67M hashes
- Header cost: 282B hashes
- Merkle is negligible (0.02% of total)

## Configuration Parameters

```rust
struct MiningConfig {
    batch_size: usize,              // 1024 merkle roots per batch
    header_nonces_per_sample: usize, // GPU workgroup size
    max_samples_per_batch: u64,     // Exhaust 2^32 header space
}
```

## Secondary: Bayesian Abandonment (Minor Optimization)

After testing a fraction of the header space without success, we can update our belief about whether the batch contains a valid nonce:

```
P(batch has valid | no hits after n samples) 
  = P(miss | valid) × P(valid) / P(miss)
  ≈ exp(-n × 1024 / (D × 2^32)) × 0.03 / (...)
```

However, at current difficulty:
- Prior is only 3%
- Posterior barely changes until we've tested most of the space
- Early abandonment provides minimal benefit

The 1024x savings comes from GPU parallelism, not abandonment strategy.

## Implementation Notes

### GPU Kernel Structure

```
For each header nonce in parallel:
  For each merkle root (1024, in shared memory):
    Compute header hash
    Check against target
    If valid, report success
```

The inner loop over 1024 merkle roots is where the parallelism win happens.

### Memory Requirements

- 1024 merkle roots × 32 bytes = 32 KB per batch
- Easily fits in GPU shared memory
- Header nonce can be computed on-the-fly or stored

## References

- `docs/research/stratified_nonce_sampling.md` - Original theory
  - **Note**: Contains errors in Bayesian derivation (decay units confusion)
  - The claimed 400x savings from "early abandonment" is incorrect
  - Real 1024x savings come from GPU parallelism
- `/workspace/webgpu/mining-calc/mining_calc.py` - Python calculations
- `/workspace/webgpu/README.md` - TypeScript implementation