# Plutus Midnight Verifier Generator

A Rust tool that generates Plutus verifiers for Halo2 circuits using midnight-proofs, enabling verification of proofs on the Cardano blockchain.

**Fork of**: [input-output-hk/plutus-halo2-verifier-gen](https://github.com/input-output-hk/plutus-halo2-verifier-gen)
**Original author**: [adamsmo](https://github.com/adamsmo) (Input Output Global)
**Modifications**: midnight-proofs compatibility (PSE halo2 v0.3.0 fork)

> ### ⚠️ Important Disclaimer & Acceptance of Risk
>
> **This repository contains proof-of-concept implementations** intended to evaluate the feasibility of verifying Halo2
> proofs in Plutus smart contracts. This code is provided "as is" for research and educational purposes only. It has not
> been thoroughly tested and audited and is not intended for production use. By using this code, you acknowledge and
> accept all associated risks, and our company disclaims any liability for damages or losses.

## Overview

This project bridges Rust-based Halo2 implementations with Plutus smart contracts on Cardano. It extracts verification keys and circuit structures from Halo2 circuits implemented with **midnight-proofs** and generates corresponding Plinth verifier code that can validate proofs on-chain.

### Changes from Original

This fork replaces IOG's `halo2_proofs` (v0.2.0 fork) with `midnight-proofs` (PSE halo2 v0.3.0 fork). Key changes:

1. **Library migration**: All dependencies migrated to midnight-proofs types (`midnight_curves::Fq`, `midnight-proofs::plonk::VerifyingKey`)
2. **Transcript compatibility**: Added `trash_challenge` support to match midnight-proofs Fiat-Shamir sequence
3. **Trashcan support**: Full implementation of trashcan (additive selector) constraint evaluation and query generation


## Features

- **Circuit-Agnostic Generation**: Automatically generates Plinth verifiers for various Halo2 circuits
- **Template-Based Code Generation**: Uses Handlebars templates for flexible verifier generation
- **Multiple Circuit Types**: Supports basic Halo2 circuits, lookup tables, custom gates, and trashcan constraints
- **midnight-proofs Native**: Extracts circuit structure directly from midnight-proofs VK without type conversion

## Architecture

### Core Components

1. **Halo2 proof generation in Rust** (`src/`)
    - Circuit definitions and implementations
    - Proof generation and verification

2. **Plutus Generation Pipeline** (`src/plutus_gen/`)
    - `extraction/`: Extracts circuit data from Halo2 structures
    - `code_emitters.rs`: Generates Plinth code from templates that is optimized to verify a particular circuit

3. **Plutus Verifier** (`plutus-verifier/`)
    - Common Plinth code for Halo2 verification
    - Template files for circuit-tailored code generation

### Workflow

1. Define Halo2 circuit in Rust
2. Generate proving/verifying keys
3. Extract circuit structure and constraints
4. Generate Plinth verifier code using templates
5. Integrate verifier into Plinth smart-contract to be deployed on Cardano

## Build prerequisites

The prototype consists of two main parts:

1. The Rust component generates a Halo2 proof and produces the corresponding Plinth verifier code.
    - it can be build using the standard `cargo` tooling from the root of the repository.
2. The Plinth component contains template files and serves as the target location for inserting the generated Plinth
   verifier.
    - Plinth smart contract can be build using `cabal` in `nix` environment.

#### How to install and use nix

1. Install `nix` - the package manager

```
sh <(curl -L https://nixos.org/nix/install)
```

2. Modify/Create the conf file `/etc/nix/nix.conf` by adding

```
substituters = https://cache.nixos.org https://cache.iog.io
trusted-public-keys = hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ= cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
experimental-features = nix-command flakes
allow-import-from-derivation = true
```

3. The contract can be build from the relevant templates folder using the nix shell:

```bash
nix develop github:input-output-hk/devx#ghc96-iog
cd plutus-verifier
cabal update
cabal build -j all
cabal test all
```

If you have build errors due to missing package descriptions like this:

```bash
.....
Error: cabal: No cabal file found.
Please create a package description file <pkgname>.cabal
Failed to build random-shuffle-0.0.4. The failure occurred during the
configure step.
.....
```

just try to re-run the build (may require several re-runs). (cabal update helps as well)

## Running Examples

### Rust part

The repository includes several example circuits:

* `simple_mul` - Simple multiplication circuit with standard PLONK gates
* `lookup_table` - Circuit with lookup arguments (4 lookups, 0 trashcans)
* `trashcan_test` - Circuit with trashcan constraints (2 trashcans for testing)

**Note**: GWC19 support removed in midnight-proofs fork. Only Halo2 KZG multi-open protocol supported.

```bash
# Simple multiplication circuit
cargo run --example simple_mul

# Lookup table circuit (4 lookups, 0 trashcans)
cargo run --example lookup_table

# Trashcan test circuit (2 trashcans)
cargo run --example trashcan_test

# With detailed logging
RUST_LOG=debug cargo run --example simple_mul
```

Running an example will generate the verification and proving keys for the circuit, create a proof using test public
inputs, and produce the corresponding Plinth verifier code. The generated files will be saved in their respective
locations within the plutus-verifier folder:

* The generated proof is saved in `./plutus-verifier/plutus-halo2/test/Generic/serialized_proof.json`.
* The public inputs are saved in `./plutus-verifier/plutus-halo2/test/Generic/serialized_public_inputs.hex`.
* The generated Plinth code is saved in:

```
./plutus-verifier/plutus-halo2/src/Plutus/Crypto/Halo2/Generic/Verifier.hs
./plutus-verifier/plutus-halo2/src/Plutus/Crypto/Halo2/Generic/VKConstants.hs
```

### Plutus part

After the Rust part is executed you can test Plutus verifier as follows:

```bash
nix develop github:input-output-hk/devx#ghc96-iog
cd plutus-verifier
cabal build -j all
cabal test all
```

## Technical Details

### Trashcan Support

Trashcans (additive selectors) allow constraints to be disabled when `selector = 0`. Implementation:

- **Expression compilation**: `Horner(constraints, trash_challenge) - (1 - selector) * trash_eval`
- **Query generation**: 1 query per trashcan at current rotation
- **Vanishing polynomial**: Trashcan expressions added after lookups in Horner evaluation

Circuits with 0 trashcans generate no trashcan code (backward compatible).

### midnight-proofs Compatibility

| Component | IOG halo2_proofs | midnight-proofs | Status |
|-----------|------------------|-----------------|--------|
| Base version | PSE v0.2.0 fork | PSE v0.3.0 fork | Different |
| Transcript | No trash_challenge | Always squeezes trash_challenge | Handled |
| Trashcans | Not supported | Supported | Fully implemented |
| Curve | BLS12-381 | BLS12-381 | Identical |

### Fiat-Shamir Sequence

midnight-proofs transcript order (matches generated Plutus code):
1. VK hash, instances, advice commitments
2. theta → lookup permuted commitments
3. beta, gamma → permutation/lookup products
4. **trash_challenge** → trashcan commitments
5. vanishing commitments → y challenge
6. evaluations → x challenge
7. multipoint opening (x1, x2, f_commitment, x3, q_evals, x4)

## License

Copyright 2025 Input Output Global (original work)
Copyright 2025 [sbcdn](https://github.com/sbcdn) (modifications)

Licensed under the Apache License, Version 2.0 (the "License"). You may not use this repository except in compliance
with the License. You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "
AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific
language governing permissions and limitations under the License
