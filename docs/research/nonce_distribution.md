# Nonce Distribution Analysis

## Overview

Analysis of 4,163 Foundry pool blocks to determine if winning nonces cluster in specific bit ranges.

## Key Findings

### Header Nonces (32-bit)

| Metric | Observed | Expected (Uniform) |
|--------|----------|-------------------|
| Mean bit length | 30.94 | ~31.0 |
| Distribution | Uniform | Uniform |

**Conclusion**: Header nonces follow uniform distribution. Exhaustive or random sampling both valid.

### Coinbase Nonces (64-bit)

| Metric | Observed | Expected (Uniform) |
|--------|----------|-------------------|
| Mean bit length | 56.69 | ~63.0 |
| >= 2^48 | 81% | 99.998% |
| >= 2^56 | 81% | 99.6% |
| >= 2^60 | 76% | 93.75% |

**At first glance**: Appears biased toward "high values"

**Reality**: Winners are actually *underrepresented* in the highest ranges:
- [2^60, 2^64): 76% of winners vs 93.75% of space
- [2^44, 2^48): 10% of winners vs 0.001% of space → **7000x overrepresented**

## Interpretation

1. **SHA-256 is uniform** - the apparent "bias toward high values" was a mirage caused by volume (most of 64-bit space is high values)

2. **Miner behavior detected** - the 44-48 bit cluster (10% of winners in 0.001% of space) indicates miners likely start/reset their coinbase counter around 2^44-2^48

3. **Recommendation**: Use uniform random sampling for coinbase nonces. No bit-length tricks needed.

## Data Source

- File: `docs/research/data/foundry_blocks.csv`
- Count: 4,163 Foundry blocks
- Fields: height, header_nonce, cb_nonce_1, cb_nonce_2, cb_nonce_3

## Analysis Code

See `src/bin/analyze_foundry.rs` for the analysis tool.