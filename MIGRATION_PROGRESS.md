# Migration Progress: halo2_proofs → midnight-proofs

**Date Started**: 2025-12-06
**Status**: ✅ COMPLETE - All phases successful
**Completion**: 100%

---

## Quick Summary

Successfully migrated `plutus-halo2-verifier-gen` to use `midnight-proofs` instead of IOG's `halo2_proofs`. The generator now produces correct Plutus verifiers for midnight-zk circuits.

**Key Achievement**: Fixed transcript sequence mismatch by adding `trash_challenge` support.

---

## Phase 1: Library Migration ✅

### Type Replacements
- `blstrs::Scalar` → `midnight_curves::Fq`
- `blstrs::G1Affine` → `midnight_curves::G1Affine`
- `blstrs::G1Projective` → `midnight_curves::G1Projective`
- `blstrs::G2Affine` → `midnight_curves::G2Affine`
- `blstrs::Bls12` → `midnight_curves::Bls12`
- `halo2_proofs::*` → `midnight_proofs::*`

### Architectural Changes
- **Removed GWC19 Support**: midnight-proofs only has standard KZG
- **Renamed scheme**: `Halo2MultiOpenScheme` → `MidnightKZGScheme`
- **Feature flags**: Added `atms_circuits` for optional ATMS support

### Files Modified (13 core files)

**Core Infrastructure**:
- `Cargo.toml` - Updated dependencies
- `src/lib.rs` - Updated exports
- `src/plutus_gen/extraction/data.rs` - Updated type aliases
- `src/plutus_gen/extraction/mod.rs` - Updated extraction logic
- `src/plutus_gen/extraction/utils.rs` - Updated utility functions
- `src/plutus_gen/mod.rs` - Updated main generation
- `src/plutus_gen/code_emitters.rs` - Updated G2Affine import
- `src/plutus_gen/adjusted_types/mod.rs` - Updated transcript types
- `src/plutus_gen/proof_serialization.rs` - Updated proof serialization

**Circuit Files**:
- `src/circuits/simple_mul_circuit.rs`
- `src/circuits/atms_circuit.rs` (feature-gated)
- `src/circuits/atms_with_lookups_circuit.rs` (feature-gated)
- `src/circuits/lookup_table_circuit.rs`

**Examples**:
- `examples/simple_mul.rs` ✅
- `examples/lookup_table.rs` ✅
- `examples/atms.rs` (feature-gated)
- `examples/atms_with_lookups.rs` (feature-gated)

---

## Phase 2: Root Cause Investigation ✅

### Problem
Haskell verification test was failing even though:
- VK structures were identical
- Field constants were correct
- All dependencies updated

### Investigation Process

1. **Verified VK Structure** - Added debug logging, confirmed 4+4 commitments match
2. **Checked Field Constants** - DELTA, ONE, ZERO all correct
3. **Compared Proofs** - First 192 bytes identical, then 86% different (expected due to randomness)
4. **Investigated transcriptRepr** - Found it differs due to `trashcans` field in CS (expected)
5. **Analyzed Vanishing MSM** - Found mathematically identical despite different APIs
6. **Found Real Cause** - midnight-proofs always squeezes `trash_challenge` (verifier.rs:143)

### Root Cause Identified

**midnight-proofs verifier.rs line 143**:
```rust
let trash_challenge: F = transcript.squeeze_challenge();  // ALWAYS executed!

let trashcans_committed = (0..num_proofs)
    .map(|_| -> Result<Vec<_>, _> {
        vk.cs.trashcans.iter()  // Empty for lookup_table, but challenge already squeezed!
        // ...
    })
```

**IOG verifier.rs**:
```bash
$ grep -n "trash" verifier.rs
# (no results - NO trash_challenge!)
```

This extra challenge advanced the transcript state, causing **all subsequent Fiat-Shamir challenges to differ**.

---

## Phase 3: Generator Fix ✅

### Changes Made (3 files, 27 lines)

**1. extraction/data.rs** (+5 lines):
```rust
pub enum ProofExtractionSteps {
    // ...
    LookupEval,

    TrashChallenge,      // ← NEW
    TrashcanCommitment,  // ← NEW

    VanishingRand,
    // ...
}

pub struct InstantiationSpecificData {
    // ...
    pub num_trashcans: usize,  // ← NEW
}
```

**2. extraction/mod.rs** (+14 lines):
```rust
// Extract num_trashcans
circuit_description.instantiation_data.num_trashcans = vk.cs().trashcans().len();

// Add to proof extraction sequence (between lookup and vanishing)
(0..num_lookups_permuted).for_each(|_| {
    circuit_description.proof_extraction_steps.push(ProofExtractionSteps::LookupCommitment)
});

// Always squeeze trash_challenge (even if num_trashcans=0)
circuit_description.proof_extraction_steps.push(ProofExtractionSteps::TrashChallenge);

// Read trashcan commitments (empty loop for num_trashcans=0)
let num_trashcans = vk.cs().trashcans().len();
(0..num_trashcans).for_each(|_| {
    circuit_description.proof_extraction_steps.push(ProofExtractionSteps::TrashcanCommitment)
});

circuit_description.proof_extraction_steps.push(ProofExtractionSteps::VanishingRand);
```

**3. code_emitters.rs** (+8 lines):
```rust
ProofExtractionSteps::TrashChallenge => "  !trash_challenge <- M.squeezeChallange\n".to_string(),
ProofExtractionSteps::TrashcanCommitment => section
    .enumerate()
    .map(|(number, _trashcan_commitment)| {
        format!("  !trashcanCommitment{} <- M.readPoint\n", number + 1)
    })
    .join(""),
```

### Result

**Generated Verifier.hs now has correct sequence**:
```haskell
-- Line 163-166: Lookup commitments
!lookupCommitment1 <- M.readPoint
!lookupCommitment2 <- M.readPoint
!lookupCommitment3 <- M.readPoint
!lookupCommitment4 <- M.readPoint
!trash_challenge <- M.squeezeChallange  -- Line 167: NEW!
!vanishingRand <- M.readPoint            -- Line 168
!y <- M.squeezeChallange                 -- Line 169
```

---

## Phase 4: Verification ✅

### Build & Test Status
- ✅ `cargo check` - PASSES
- ✅ `cargo build --release` - PASSES
- ✅ `cargo test` - ALL TESTS PASS (2/2)
- ✅ `simple_mul` example - WORKS END-TO-END
- ✅ `lookup_table` example - WORKS END-TO-END
- ✅ **Haskell verification test - PASSES** 🎉

### Proof Sizes
- `simple_mul`: 1120 bytes
- `lookup_table`: 2544 bytes (has 4 lookup arguments)

---

## API Changes Fixed (Phase 1)

During migration, we had to adapt to midnight-proofs API changes:

1. **Circuit trait**: Added `type Params = ();`
2. **create_gate()**: Use `Constraints::with_selector()` instead of `vec![]`
3. **Error::Synthesis**: Takes `String` instead of `&str`
4. **create_proof()**: Added `nb_committed_instances: usize` parameter (use `0`)
5. **prepare()**: Added `committed_instances: &[&[CS::Commitment]]` parameter (use `&[&[]]`)
6. **Instance types**: Use `Fq::from()` instead of `Base::from()`

---

## Feature Flags

### New: `atms_circuits`

**Why needed**: ATMS circuits depend on IOG's `halo2_proofs`, creating type conflicts with `midnight_proofs`.

**Usage**:
```bash
# Default build (without ATMS)
cargo build

# With ATMS support (has type conflicts)
cargo build --features atms_circuits
```

**Future**: Migrate ATMS to midnight-proofs to remove this feature gate.

---

## Critical Success Factors

### What Made This Work

1. ✅ **Evidence-based investigation** - No premature conclusions
2. ✅ **Source code comparison** - Read actual midnight-proofs code
3. ✅ **Minimal changes** - Only modified what was necessary
4. ✅ **Security focus** - Preserved transcript/monad verification logic
5. ✅ **Thorough testing** - Verified each step

### What We Learned

- **transcriptRepr differences are expected** (due to CS.trashcans field)
- **Vanishing MSM is mathematically identical** (despite different APIs)
- **Transcript sequence is critical** (one missing challenge breaks everything)
- **Template doesn't need changes** (uses `{{{PES}}}` placeholder system)

---

## Future Work

### Phase 2: Full Trashcan Support (When Needed)

**Scope**: Support circuits that actually USE trashcans
- Add trashcan evaluation extraction
- Generate trashcan vanishing polynomial expressions
- Add trashcan queries to multipoint opening
- Test with circuit that has `num_trashcans > 0`

**Effort**: 4-8 hours
**Complexity**: Medium (follow lookup pattern)

### Optional Improvements
- [ ] Migrate ATMS circuits to midnight-proofs
- [ ] Add CI/CD testing
- [ ] Performance benchmarking
- [ ] Update README.md

---

## Commands to Use

```bash
cd /home/tp/iog_dev/plutus-midnight-verifier-gen

# Run examples
cargo run --example simple_mul
cargo run --example lookup_table

# Run tests
cargo test

# Build release
cargo build --release

# Run Haskell verification
cd plutus-verifier/plutus-halo2
cabal test
```

---

## References

- **Original plan**: `PLUTUS_VERIFIER_GENERATOR_FORK_PLAN.md`
- **Root cause details**: `ROOT_CAUSE_IDENTIFIED.md`
- **Current status**: `STATUS.md`
- **midnight-zk**: `/home/tp/iog_dev/midnight-zk`
- **IOG version**: `/home/tp/iog_dev/plutus-halo2-verifier-gen`

---

**Migration Status**: ✅ COMPLETE - Production-ready for circuits with `num_trashcans=0`
