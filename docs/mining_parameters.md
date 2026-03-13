# Mining Probability Parameters

## Overview

This document summarizes the key parameters for stratified nonce sampling based on Bitcoin mining mathematics.

## The "Needles in Haystack" Insight

**Traditional view**: One golden nonce per block

**Reality**: At current difficulty, there are ~127,000 valid nonces per block template, uniformly distributed across the 96-bit nonce space. Miners stop at the first one found, but could keep finding more.

```
Valid nonces in 96-bit space = 2^64 / difficulty
                               = 1.84 × 10^19 / 144.4 × 10^12
                               ≈ 127,000
```

## Key Parameters

### At Difficulty 50T (research estimate)

| Parameter | Value |
|-----------|-------|
| Expected nonce cycles | ~11,600 |
| P(batch has valid nonce) | ~8.4% |
| Expected batches until success | ~12 |

### At Difficulty 144.4T (current as of Mar 2026)

| Parameter | Value |
|-----------|-------|
| Expected nonce cycles | ~33,600 |
| P(batch has valid nonce) | ~3.0% |
| Expected batches until success | ~33 |

### Derivation

```
Expected nonce cycles = difficulty / 2^32

P(batch has valid nonce) = 1 - exp(-1024 / expected_cycles)
                         ≈ 1024 / expected_cycles (for small p)
```

## Multi-GPU Expected Batches

With N independent GPUs:

| GPUs | P(success round 1) | Expected rounds | Total batches |
|------|--------------------|-----------------|---------------|
| 1 | 3.0% | 33 | 33 |
| 10 | 26% | 3.8 | 38 |
| 100 | 95% | 1.05 | 105 |

Formula: `P(any success) = 1 - (1 - p)^N`

## Bayesian Abandonment

### The Strategy

1. Generate batch of 1024 merkle roots (expensive: ~2000 hashes each)
2. Sample random header nonces against all 1024 roots
3. Update probability that batch contains a valid nonce
4. Abandon if P(valid exists) drops below threshold

### Key Insight

With P(batch success) = 3%, most batches are "duds". The Bayesian update tells us when we've tested enough to be confident a batch is a dud.

### Formula (needs verification)

The posterior probability after testing n header nonces:

```
P(H1 | no hits) = P(miss | H1) × P(H1) / P(miss)
```

Where:
- H1 = "batch has at least one valid nonce"
- P(H1) = ~3% (prior)
- P(miss | H1) = probability of missing the valid nonce if it exists

**Note**: There's an inconsistency in the research document between the stated formula and the table values. This needs investigation. The table suggests abandoning after testing ~0.1-0.3% of header space, giving ~400x savings vs exhaustive search.

## Configuration Parameters

For implementation, these should be configurable:

```rust
struct MiningConfig {
    batch_size: usize,           // 1024 merkle roots
    header_nonces_per_iter: usize, // nonces to test per iteration
    abandonment_threshold: f64,  // e.g., 0.01 = 1%
    max_iterations: usize,       // safety limit
}
```

## Savings Estimate

Even with uncertain abandonment formula, the strategy yields significant savings:

- **Without abandonment**: Search 100% of 2^32 header space × 1024 merkle roots = 4.4T hashes per batch
- **With abandonment**: Search ~0.1-0.3% before abandoning duds
- **Savings**: ~400-800x reduction in wasted work on unsuccessful batches

## Open Questions

1. **Exact abandonment threshold**: The Bayesian formula in the research doc doesn't match the table values. Needs verification.

2. **Optimal batch size**: M=1024 is used, but could be tuned based on:
   - GPU memory capacity
   - Merkle generation cost
   - Network difficulty

3. **Header nonces per iteration**: Trade-off between:
   - More iterations → finer Bayesian updates
   - Fewer iterations → more GPU batch efficiency

## References

- `docs/research/stratified_nonce_sampling.md` - Original theory
- `/workspace/webgpu/mining-calc/mining_calc.py` - Python calculations
- `/workspace/webgpu/README.md` - TypeScript implementation