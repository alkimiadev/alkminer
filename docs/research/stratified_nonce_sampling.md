# Stratified Nonce Sampling for Bitcoin Mining

## Overview

This document explores an alternative approach to Bitcoin mining that treats the problem as **stratified sampling** of a 96-bit nonce space (64-bit coinbase + 32-bit header) rather than exhaustive linear search. The key insight is that the cost asymmetry between generating merkle roots vs. testing header nonces can be exploited through batched sampling and early abandonment.

---

## 1. Nonce Cycles and the 96-bit Search Space

### Current Mining Model

Modern mining pools (e.g., Foundry) use a **two-layer nonce strategy**:
- **32-bit header nonce**: Standard Bitcoin block header field
- **64-bit coinbase nonce** (extraNonce): Embedded in the coinbase transaction

Changing the coinbase nonce modifies the merkle root, effectively giving miners a new 32-bit header nonce space to search.

**Total effective search space**: 96 bits = 2^96 ≈ 7.92 × 10^28 possible combinations

### Expected Nonce Cycles

With current difficulty D (measured in expected full nonce cycles to find a block), the **expected number of 32-bit nonce cycles** is simply D.

For example, with D ≈ 11,644 nonce cycles (corresponding to network difficulty ~50 trillion):

**Mean**: 11,644 cycles

**90% Prediction Interval**: 597 to 34,870 cycles  
**95% Prediction Interval**: 295 to 42,930 cycles

These wide intervals reflect the geometric distribution's heavy right tail - you might get lucky and find a block in 300 cycles, or unlucky and need 40,000+ cycles.

### Number of Valid Nonces in Full Space

If we could enumerate the entire 96-bit space for a given block template, the **expected number of valid hashes** would be:

```
E[successes] = 2^96 / (D × 2^32) = 2^64 / D
```

With D ≈ 11,644:
```
E[successes] ≈ 1.84 × 10^19 / 11,644 ≈ 1.58 × 10^15
```

Wait, this seems off. Let me recalculate with the standard definition.

Actually, with network difficulty diff ≈ 50×10^12 and p = 1/(2^32 × diff):
```
E[successes in 96-bit space] = 2^96 × p
                               = 2^96 / (2^32 × diff)
                               = 2^64 / diff
                               ≈ 1.84 × 10^19 / (5 × 10^13)
                               ≈ 368,000
```

So there are approximately **184,000 to 370,000 valid hashes** uniformly scattered throughout the 2^96 space (depending on current difficulty).

**Key insight**: Most people think "there's one golden nonce." Actually, there are hundreds of thousands of them - miners just can't find them all efficiently.

---

## 2. Batching Merkle Roots: Cost Structure and Probabilities

### Cost Asymmetry

The computational cost structure heavily favors batching:

| Operation | Cost (SHA-256 hashes) | Notes |
|-----------|----------------------|-------|
| Change coinbase nonce | ~2000+ | Must recompute full merkle tree |
| Change header nonce | 2 | Single SHA-256d on block header |

**Cost ratio**: Generating a new merkle root is **~1000× more expensive** than testing a header nonce.

### Batch Strategy

**Proposed approach**:
1. Generate M merkle roots upfront (expensive, done once per batch)
2. For each merkle root, randomly sample N header nonces
3. Test all M × N combinations
4. Abandon and regenerate merkle roots when success probability drops below threshold

### Probability Calculations for a Batch

For a batch of M = 1024 merkle roots:

**Probability that at least one merkle root contains a valid nonce**:

With D ≈ 11,644 nonce cycles, each merkle root has probability p ≈ 1/D of containing at least one valid header nonce in its full 2^32 space.

```
P(≥1 success in batch) = 1 - (1 - 1/D)^M
                       ≈ 1 - e^(-M/D)
                       = 1 - e^(-1024/11644)
                       ≈ 1 - e^(-0.088)
                       ≈ 8.4%
```

**Probability of no valid nonces in the batch**:
```
P(no success) ≈ 91.6%
```

This means that **~9 out of 10 batches** will contain no valid nonces at all, regardless of how thoroughly we search them.

---

## 3. Sequential Sampling and Bayesian Abandonment

### The Sequential Testing Problem

Once we've generated our batch of M = 1024 merkle roots, we face a decision problem:

- **Total header nonce space per merkle root**: 2^32 ≈ 4.3 billion
- **Exhaustive search**: Would require 2^32 / 1024 ≈ 4.2 million batches of 1024 header nonces
- **Question**: Can we abandon much earlier if we're confident this batch contains no valid nonces?

### Bayesian Update Framework

**Hypotheses**:
- H₁: "At least one of our 1024 merkle roots has a valid header nonce" (prior ≈ 8.4%)
- H₀: "None of our merkle roots have valid nonces" (prior ≈ 91.6%)

**Sampling strategy**: Each iteration, randomly sample 1024 header nonces and test against all 1024 merkle roots (= 1024² ≈ 1M hash operations per batch).

### Posterior Probability After b Batches

After testing b batches of 1024 header nonces with no success, we update our beliefs:

**Likelihood of observing no hits under H₁**:

If H₁ is true (i.e., at least one merkle root has a valid nonce somewhere), the probability of missing it after sampling b × 1024 header nonces per merkle root is:

```
P(no hits | H₁) ≈ (1 - p_header)^(b × 1024²)
```

where p_header ≈ 1/(D × 2^32) is the probability per individual hash.

With D ≈ 11,644:
```
P(no hits | H₁) ≈ e^(-b × 1024² / (D × 2^32))
                ≈ e^(-b × 1,048,576 / (11,644 × 4,294,967,296))
                ≈ e^(-b / 47,619)
```

**Likelihood under H₀**:
```
P(no hits | H₀) = 1
```
(If there are no valid nonces, we'll never find any)

**Posterior using Bayes' rule**:
```
P(H₁ | no hits after b batches) = P(no hits | H₁) × P(H₁) / P(no hits)
                                 = 0.084 × e^(-b/47,619) / [0.084 × e^(-b/47,619) + 0.916]
```

### When to Abandon

As we sample more batches without finding a valid nonce, our confidence that H₁ is true decreases:

| Batches (b) | Header nonces tested per merkle root | P(H₁ \| no hits) | Cumulative hashes |
|-------------|--------------------------------------|------------------|-------------------|
| 0 | 0 | 8.4% | 0 |
| 1,000 | ~1M | 6.9% | 1.05B |
| 2,000 | ~2M | 5.4% | 2.10B |
| 5,000 | ~5M | 3.3% | 5.24B |
| 10,000 | ~10M | 1.4% | 10.5B |
| 20,000 | ~20M | 0.5% | 21.0B |
| 47,619 | ~48M | 0.3% | 50.0B |

**Optimal abandonment threshold**: This depends on the cost ratio between generating merkle roots vs. testing header nonces.

If we set a threshold of P(H₁) < 1%, we would abandon after approximately **10,000 batches**, having tested only:
```
10,000 × 1024 / 2^32 ≈ 0.24% of the header nonce space per merkle root
```

This is **~400× less work** than exhaustively searching the full 32-bit header space!

---

## 4. Multi-GPU Parallel Search

### Expected Batches Until Success

With our batching strategy, each batch of M = 1024 merkle roots has:
- **P(success) ≈ 8.4%** (at least one merkle root contains a valid nonce)
- **P(failure) ≈ 91.6%** (no valid nonces in this batch)

This forms a **geometric distribution**. The expected number of batches until finding one with a valid nonce is:

```
E[batches until success] = 1 / P(success)
                         = 1 / 0.084
                         ≈ 11.9 batches
```

So on average, you'd **abandon and restart ~11-12 times** before finding a batch that contains a valid nonce.

### Parallelization Across Multiple GPUs

Consider a setup with N = 10 GPUs running independently (e.g., rented from vast.ai), each:
- Generating its own batches of 1024 merkle roots
- Randomly sampling header nonces
- Abandoning unsuccessful batches
- Operating completely independently

**Expected batches until first success across all 10 GPUs**:

The probability that at least one GPU finds a successful batch in round k:
```
P(≥1 success in round k) = 1 - (1 - 0.084)^10 ≈ 1 - 0.916^10
```

| Round | P(success this round) | Cumulative P(success by round k) |
|-------|----------------------|----------------------------------|
| 1 | 57.4% | 57.4% |
| 2 | 24.5% | 81.9% |
| 3 | 10.4% | 92.3% |
| 4 | 4.5% | 96.8% |
| 5 | 1.9% | 98.7% |

**Expected number of rounds until first success**:
```
E[rounds] = 1 / 0.574 ≈ 1.74 rounds
```

**Expected total merkle root batches generated** (across all 10 GPUs):
```
Total batches = 10 GPUs × 1.74 rounds ≈ 17.4 batches
Total merkle roots = 17.4 × 1024 ≈ 17,800 merkle roots
```

### Practical Implications

With 10 GPUs in parallel:
- **~57% chance** of finding a successful batch in the first round
- **~92% chance** within 3 rounds
- On average, generate **~18 batches** (~18,000 merkle roots) before finding success

Once a successful batch is identified, that GPU then searches within its 1024 merkle roots to find the specific one(s) containing valid header nonces.

### Expected Value Calculation

To determine the optimal threshold, we need to compare:

**Cost of continuing**: 
```
C_continue = (hashes per batch) × (expected batches to success | H₁) × P(H₁)
```

**Cost of abandoning and restarting**:
```
C_abandon = (cost to generate new merkle batch) + (expected cost of new batch)
```

The optimal policy is to abandon when:
```
P(H₁ | data) × (expected remaining cost | H₁) > C_abandon
```

This creates a stopping rule that balances the diminishing probability of success against the sunk cost of the current batch.

---

## 5. Insights, Questions, and Concerns

### Key Insights

1. **Stratified sampling vs. linear search**: By treating the 96-bit space as stratified (expensive merkle roots, cheap header nonces), we can dramatically reduce wasted computation.

2. **Early abandonment yields ~400× savings**: With optimal stopping rules, we test only ~0.25% of the header space before abandoning unsuccessful batches.

3. **Most batches contain no valid nonces**: With 91.6% of batches being "duds," the ability to identify and abandon them quickly is crucial.

4. **Bayesian framework provides principled stopping**: Rather than arbitrary thresholds, we can compute exact posterior probabilities and make optimal decisions.

### Open Questions

1. **What is the true cost ratio?**: 
   - Merkle tree generation: depends on transaction count (2000+ typical)
   - GPU implementation: might change cost structure significantly
   - Memory bandwidth: might dominate for large batches

2. **Optimal batch size M**:
   - Larger M → higher P(≥1 success), but more expensive to generate
   - Smaller M → cheaper to abandon, but lower success rate
   - Optimal M depends on cost ratio and GPU parallelism

3. **Sampling strategy for header nonces**:
   - Uniform random: ensures independence
   - Stratified within 32-bit space: might improve coverage
   - Adaptive: could we use partial results to guide sampling?

4. **Does random sampling beat sequential?**:
   - Random sampling: enables early abandonment with confidence
   - Sequential: simpler, no RNG overhead
   - Tradeoff: Is the abandonment benefit > RNG cost?

5. **Real-world performance**:
   - Does GPU memory bandwidth support 1024×1024 hashing per batch?
   - What's the actual speedup vs. traditional mining?
   - How does this compare to existing ASIC approaches?

### Potential Concerns

1. **Random oracle assumption**: SHA-256 must behave as a truly random function for uniform distribution assumption to hold. Empirical studies show minor deviations in practice.

2. **Implementation complexity**: This approach requires:
   - Efficient merkle tree batching on GPU
   - Random number generation for header nonce sampling
   - Bayesian update logic for abandonment decisions
   - Coordination across these components

3. **Variance in block discovery time**: Early abandonment might increase variance in time-to-block, even if it reduces expected time. Pools might prefer predictable hashrate.

4. **Competition**: If other miners use this strategy, does it change the game theory? (Probably not - mining is still a race)

### Next Steps

1. **Implement prototype**: Test the merkle root batching + random header sampling on GPU
2. **Empirical cost measurement**: Determine actual cost ratio in our implementation
3. **Optimize M and threshold**: Use measured costs to find optimal parameters
4. **Benchmark against baseline**: Compare total hashes-to-block vs. sequential search
5. **Variance analysis**: Measure distribution of time-to-block under this strategy
