# VK Structure Investigation - Findings

**Date**: 2025-12-06
**Issue**: Haskell verification test failing despite Rust verification passing

## Executive Summary

**CONCLUSION**: The VK structures between IOG halo2_proofs and midnight-proofs are **IDENTICAL**. The earlier discrepancy in VKConstants.hs (showing only 2 fixed + 3 perm commitments) was due to an **earlier build artifact** from when we were testing with the `committed-instances` feature enabled. After rebuilding with the feature properly disabled, both versions generate identical VKs.

## Evidence Collected

### VK Structure Comparison

Added debug logging to both versions at extraction point (`src/plutus_gen/extraction/mod.rs`):

**midnight-proofs** (lookup_table example):
```
VK fixed_commitments count: 4
VK permutation commitments count: 4
CS num_fixed_columns: 4
CS num_advice_columns: 4
CS num_instance_columns: 1
CS permutation columns count: 4
CS degree: 5
chunk_len (degree-2): 3
Expected perm chunks: 2
```

**IOG halo2_proofs** (lookup_table example):
```
VK fixed_commitments count: 4
VK permutation commitments count: 4
CS num_fixed_columns: 4
CS num_advice_columns: 4
CS num_instance_columns: 1
CS permutation columns count: 4
CS degree: 5
chunk_len (degree-2): 3
Expected perm chunks: 2
```

### Generated Files Comparison

After proper rebuild (2025-12-06 15:33:08), the VKConstants.hs files differ **only** in:
- `transcriptRepr` value (expected - hash of circuit/transcript state)

All other fields are **identical**:
- 4 fixed commitments (f1, f2, f3, f4)
- 4 permutation commitments (p1, p2, p3, p4)
- Same omega, omegaInv, barycentricWeight values
- Same s_g2_val, blinding_factors

### Proof Verification

**Rust verification**: ✅ PASSES (both IOG and midnight versions)
**Haskell verification**: ❌ FAILS (midnight version)

All files from same run (15:33:08):
- `serialized_proof.json` (9103 bytes = 2544 raw bytes)
- `serialized_public_input.hex`
- `VKConstants.hs` (122 lines)
- `Verifier.hs` (18297 bytes)

## Root Cause Analysis

### NOT a VK Structure Incompatibility

The VK structures are identical. Both IOG halo2 v0.2.0 fork and midnight halo2 v0.3.0 fork generate the same verifying key structure for the same circuit.

### Previous Misleading Evidence

The earlier diff showing only 2 fixed + 3 perm commitments in midnight's VKConstants.hs was from an old build artifact created during feature flag experimentation. After clean rebuild with `committed-instances` disabled, the generated VK is correct.

## Remaining Mystery

**Question**: Why does the Haskell verifier still reject the proof even though:
1. VK structure is correct (4+4 commitments)
2. Proof was generated with matching VK
3. Rust verification passes
4. All files are from the same generation run

**Next Steps**: Need to investigate:
- Proof encoding differences between IOG and midnight (even if size is same)
- Transcript implementation differences (CardanoFriendlyState)
- Verifier.hs logic - perhaps a subtle difference in how queries are evaluated

## Key Learning

**DO NOT ASSUME**. Always gather evidence before jumping to conclusions. The VK structure difference was a red herring caused by stale build artifacts, not a fundamental library incompatibility.
