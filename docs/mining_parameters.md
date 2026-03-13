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
| Abandonment threshold (hashes) | ~48M |

### At Difficulty 144.4T (current as of Mar 2026)

| Parameter | Value |
|-----------|-------|
| Expected nonce cycles | ~33,600 |
| P(batch has valid nonce) | ~3.0% |
| Expected batches until success | ~33 |
| Abandonment threshold (hashes) | ~138M |

### Derivation

```
Expected nonce cycles (D) = difficulty / 2^32

P(batch has valid nonce) = 1 - exp(-1024 / D)
                         ≈ 1024 / D (for small p)
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

### Correct Formula

The posterior probability after testing H hashes:

```
P(H1 | no hits) = P(miss | H1) × P(H1) / P(miss)
```

Where:
- H1 = "batch has at least one valid nonce"
- P(H1) = prior probability (~3% at 144T)
- P(miss | H1) = exp(-H / (D × 2^32))

**Decay constant** (in hashes):

```
decay_hashes = D × 2^32

At 50T:  decay ≈ 11,600 × 4.3B ≈ 48M hashes
At 144T: decay ≈ 33,600 × 4.3B ≈ 138M hashes
```

### Abandonment Threshold

To reach P(H1|miss) < 1%:

| Difficulty | Hashes before abandon | % of 2^32 space |
|------------|----------------------|-----------------|
| 50T | ~48M | ~1.1% |
| 144T | ~138M | ~3.2% |

Wait, this still seems wrong. Let me recalculate...

Actually, the decay tells us how quickly the posterior drops, but abandonment happens when we've tested enough to be confident the batch is a dud. With P(H1) = 3%:

```
After testing 48M hashes at 50T:
  P(miss | H1) = exp(-48M / 48M) = exp(-1) ≈ 37%
  P(H1 | miss) = 0.37 × 0.084 / (0.37 × 0.084 + 0.916) ≈ 3.3%

After testing 138M hashes at 144T:
  P(miss | H1) = exp(-138M / 138M) = exp(-1) ≈ 37%
  P(H1 | miss) = 0.37 × 0.03 / (0.37 × 0.03 + 0.97) ≈ 1.1%
```

At 144T with P(H1)=3%, testing 138M hashes (one "decay unit") gets us close to the 1% abandonment threshold.

### Conversion to Iterations

The number of iterations depends on sampling rate:

```
hashes_per_iteration = header_nonces_per_iter × merkle_roots

If header_nonces_per_iter = 1024 and merkle_roots = 1024:
  hashes_per_iteration = 1M
  
decay_iterations = decay_hashes / hashes_per_iteration

At 50T:  decay ≈ 48M / 1M = 48 iterations
At 144T: decay ≈ 138M / 1M = 138 iterations
```

### Note: Research Doc Error

The original research document (`docs/research/stratified_nonce_sampling.md`) has a units error:

- Formula claims decay = 47,619 "iterations"
- But the table values imply decay ≈ 5,000 "iterations"
- The discrepancy is ~9x, matching 1024² vs 1024 confusion

The table was calculated correctly but used a different (unstated) sampling rate than the formula assumed.

## Configuration Parameters

For implementation, these should be configurable:

```rust
struct MiningConfig {
    batch_size: usize,              // 1024 merkle roots
    hashes_per_iteration: usize,    // e.g., 1M = 1024 × 1024
    abandonment_threshold: f64,     // e.g., 0.01 = 1%
    decay_hashes: u64,              // D × 2^32, computed from difficulty
}
```

## Savings Estimate

The strategy yields significant savings on dud batches:

| Difficulty | Hashes before abandon | Exhaustive would need | Savings |
|------------|----------------------|----------------------|---------|
| 50T | ~48M | 4.4T (1024 × 2^32) | ~90x |
| 144T | ~138M | 4.4T | ~32x |

Wait, this is much less than the claimed 400x. Let me reconsider...

The savings depend on:
1. Cost of generating new batch (~2M hashes for merkle roots)
2. Cost of additional sampling iterations

If we abandon after ~50-150M hashes and the batch was a dud, we saved 4.4T hashes. But the 400x figure from the research doc may have used different assumptions.

### More Realistic Savings

Actually, the key comparison is:

- **Traditional mining**: Generates one merkle root, exhaustively searches 2^32 header nonces
- **Our strategy**: Generates 1024 merkle roots, samples header nonces, abandons duds early

The savings come from abandoning dud batches before exhaustive search. At 144T:
- 97% of batches are duds
- We abandon after ~3% of header space instead of 100%
- Savings: ~30x on dud batches

But we also pay the cost of generating 1024 merkle roots upfront. The net benefit requires simulation.

## Remaining Questions

1. **Optimal batch size**: M=1024 is assumed, but could be tuned.

2. **Optimal sampling rate**: More hashes per iteration = faster abandonment but less granularity.

3. **Net savings accounting for merkle generation**: Need to factor in the ~2M hash cost per merkle root.

4. **Comparison to traditional mining**: Traditional miners don't generate 1024 merkle roots - they generate one and exhaustively search. Our strategy trades merkle generation cost for early abandonment on duds.

## References

- `docs/research/stratified_nonce_sampling.md` - Original theory (has units error)
- `/workspace/webgpu/mining-calc/mining_calc.py` - Python calculations
- `/workspace/webgpu/README.md` - TypeScript implementation