use midnight_curves::{Bls12, Fq, G1Projective};
use midnight_proofs::{
    halo2curves::group::GroupEncoding,
    plonk::{
        ProvingKey, VerifyingKey, create_proof, k_from_circuit, keygen_pk, keygen_vk, prepare,
    },
    poly::{
        commitment::PolynomialCommitmentScheme, commitment::Guard,
        kzg::KZGCommitmentScheme, kzg::params::ParamsKZG, kzg::params::ParamsVerifierKZG,
    },
    transcript::{CircuitTranscript, Transcript},
};
use log::info;
use plutus_midnight_verifier_gen::{
    circuits::trashcan_test_circuit::TrashcanTestCircuit,
    plutus_gen::{
        adjusted_types::CardanoFriendlyState, extraction::ExtractKZG, generate_plinth_verifier,
        proof_serialization::export_public_inputs, proof_serialization::serialize_proof,
    },
};
use rand::rngs::StdRng;
use rand_core::SeedableRng;
use std::env;
use std::fs::File;

fn main() {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));
    let args: Vec<String> = env::args().collect();

    match &args[1..] {
        [] => {
            compile_trashcan_test_circuit::<KZGCommitmentScheme<Bls12>>();
        }
        _ => {
            println!("Usage:");
            println!("- to run the example: `cargo run --example trashcan_test`");
        }
    }
}

pub fn compile_trashcan_test_circuit<
    S: PolynomialCommitmentScheme<
            Fq,
            Commitment = G1Projective,
            Parameters = ParamsKZG<Bls12>,
            VerifierParameters = ParamsVerifierKZG<Bls12>,
        > + ExtractKZG,
>() {
    let mut rng = StdRng::seed_from_u64(42);

    // Create circuit with valid values (both trashcan constraints satisfied)
    let a = Fq::from(42);
    let b = Fq::from(42); // Equal to a - trashcan 1 satisfied
    let c = Fq::from(100);
    let d = Fq::from(100); // Equal to c - trashcan 2 satisfied

    // Public inputs (3 to match test structure)
    let p1 = Fq::from(1);
    let p2 = Fq::from(2);
    let p3 = Fq::from(3);

    let circuit = TrashcanTestCircuit { a, b, c, d, p1, p2, p3 };

    let k: u32 = k_from_circuit(&circuit);
    info!("Circuit k: {}", k);

    let kzg_params: ParamsKZG<Bls12> = ParamsKZG::<Bls12>::unsafe_setup(k, rng.clone());
    let vk: VerifyingKey<Fq, S> = keygen_vk(&kzg_params, &circuit).unwrap();

    // Log trashcan information
    info!("VK num_trashcans: {}", vk.cs().trashcans().len());
    for (i, trash) in vk.cs().trashcans().iter().enumerate() {
        info!(
            "Trashcan {}: {} with {} constraint expressions",
            i + 1,
            trash.name(),
            trash.constraint_expressions().len()
        );
    }

    let pk: ProvingKey<Fq, S> = keygen_pk(vk.clone(), &circuit).unwrap();

    // Public inputs (3 to match test structure)
    let public_inputs_vec = vec![p1, p2, p3];
    let instances: &[&[&[Fq]]] = &[&[&public_inputs_vec]];
    info!("Public inputs: {:?}", instances);

    let instances_file =
        "./plutus-verifier/plutus-halo2/test/Generic/serialized_public_input_trashcan.hex"
            .to_string();
    let mut output = File::create(instances_file).expect("failed to create instances file");
    export_public_inputs(instances, &mut output);

    let mut transcript: CircuitTranscript<CardanoFriendlyState> =
        CircuitTranscript::<CardanoFriendlyState>::init();

    create_proof(
        &kzg_params,
        &pk,
        &[circuit],
        instances,
        &mut rng,
        &mut transcript,
    )
    .expect("proof generation should not fail");

    let proof = transcript.finalize();

    info!("Proof size: {} bytes", proof.len());

    // Verify with Rust verifier first
    let mut transcript_verifier: CircuitTranscript<CardanoFriendlyState> =
        CircuitTranscript::<CardanoFriendlyState>::init_from_bytes(&proof);

    let verifier = prepare::<_, _, CircuitTranscript<CardanoFriendlyState>>(
        &vk,
        instances,
        &mut transcript_verifier,
    )
    .expect("prepare verification failed");

    verifier
        .verify(&kzg_params.verifier_params())
        .expect("Rust verification failed");

    info!("✅ Rust verification PASSED");

    // Serialize proof for Haskell verifier
    serialize_proof(
        "./plutus-verifier/plutus-halo2/test/Generic/serialized_proof_trashcan.json".to_string(),
        proof,
    )
    .unwrap();

    // Generate Plutus verifier
    generate_plinth_verifier(&kzg_params, &vk, instances, |a| hex::encode(a.to_bytes()))
        .expect("Plinth verifier generation failed");

    info!("✅ Generated Plutus verifier with {} trashcans", vk.cs().trashcans().len());
}
