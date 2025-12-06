use midnight_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Constraints, Error, Instance, Selector},
    poly::Rotation,
};
use midnight_curves::Fq;

/// Minimal test circuit with 2 trashcans
///
/// This circuit demonstrates trashcan (additive selector) usage:
/// - Public inputs: p1, p2, p3 (to match test structure, but not used in constraints)
/// - Trashcan 1: Enforces a == b when enabled
/// - Trashcan 2: Enforces c == d when enabled
///
/// Trashcans use additive selectors instead of multiplicative, allowing
/// constraints to be "thrown away" when the selector is off, using a
/// trash polynomial to absorb the values.
#[derive(Clone, Debug, Default)]
pub struct TrashcanTestCircuit {
    pub a: Fq,
    pub b: Fq,
    pub c: Fq,
    pub d: Fq,
    pub p1: Fq, // Public input 1 (not used, for test compatibility)
    pub p2: Fq, // Public input 2 (not used, for test compatibility)
    pub p3: Fq, // Public input 3 (not used, for test compatibility)
}

#[derive(Clone, Debug)]
pub struct TrashcanTestConfig {
    advice_a: Column<Advice>,
    advice_b: Column<Advice>,
    advice_c: Column<Advice>,
    advice_d: Column<Advice>,
    instance: Column<Instance>, // Public inputs column

    selector_trash1: Selector,
    selector_trash2: Selector,
}

impl Circuit<Fq> for TrashcanTestCircuit {
    type Config = TrashcanTestConfig;
    type FloorPlanner = SimpleFloorPlanner;
    type Params = ();

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fq>) -> Self::Config {
        let advice_a = meta.advice_column();
        let advice_b = meta.advice_column();
        let advice_c = meta.advice_column();
        let advice_d = meta.advice_column();
        let instance = meta.instance_column();

        // Enable equality constraints for public inputs
        meta.enable_equality(instance);
        meta.enable_equality(advice_a); // Need this for constrain_instance

        // Trashcan 1: a == b when selector enabled
        let selector_trash1 = meta.complex_selector();
        meta.create_gate("trashcan_equality_ab", |meta| {
            let a = meta.query_advice(advice_a, Rotation::cur());
            let b = meta.query_advice(advice_b, Rotation::cur());

            // Using additive selector - constraint is "thrown in trash" when s=0
            // Use named constraint for better debugging
            Constraints::with_additive_selector(selector_trash1, vec![("a == b", a - b)])
        });

        // Trashcan 2: c == d when selector enabled
        let selector_trash2 = meta.complex_selector();
        meta.create_gate("trashcan_equality_cd", |meta| {
            let c = meta.query_advice(advice_c, Rotation::cur());
            let d = meta.query_advice(advice_d, Rotation::cur());

            // Using additive selector - constraint is "thrown in trash" when s=0
            // Use named constraint for better debugging
            Constraints::with_additive_selector(selector_trash2, vec![("c == d", c - d)])
        });

        TrashcanTestConfig {
            advice_a,
            advice_b,
            advice_c,
            advice_d,
            instance,
            selector_trash1,
            selector_trash2,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fq>,
    ) -> Result<(), Error> {
        // Assign and constrain public inputs (3 inputs to match test structure)
        let p1_cell = layouter.assign_region(
            || "public input p1",
            |mut region| {
                region.assign_advice(|| "p1", config.advice_a, 0, || Value::known(self.p1))
            },
        )?;
        layouter.constrain_instance(p1_cell.cell(), config.instance, 0)?;

        let p2_cell = layouter.assign_region(
            || "public input p2",
            |mut region| {
                region.assign_advice(|| "p2", config.advice_a, 0, || Value::known(self.p2))
            },
        )?;
        layouter.constrain_instance(p2_cell.cell(), config.instance, 1)?;

        let p3_cell = layouter.assign_region(
            || "public input p3",
            |mut region| {
                region.assign_advice(|| "p3", config.advice_a, 0, || Value::known(self.p3))
            },
        )?;
        layouter.constrain_instance(p3_cell.cell(), config.instance, 2)?;

        // Main circuit constraints
        layouter.assign_region(
            || "trashcan test region",
            |mut region| {
                // Enable trashcan selectors
                config.selector_trash1.enable(&mut region, 0)?;
                config.selector_trash2.enable(&mut region, 0)?;

                // Assign values - constraints will be checked on row 0
                region.assign_advice(|| "a", config.advice_a, 0, || Value::known(self.a))?;
                region.assign_advice(|| "b", config.advice_b, 0, || Value::known(self.b))?;
                region.assign_advice(|| "c", config.advice_c, 0, || Value::known(self.c))?;
                region.assign_advice(|| "d", config.advice_d, 0, || Value::known(self.d))?;

                // Add a second row with selectors disabled (constraints ignored via trash)
                region.assign_advice(|| "a2", config.advice_a, 1, || Value::known(Fq::from(999)))?;
                region.assign_advice(|| "b2", config.advice_b, 1, || Value::known(Fq::from(111)))?;
                region.assign_advice(|| "c2", config.advice_c, 1, || Value::known(Fq::from(888)))?;
                region.assign_advice(|| "d2", config.advice_d, 1, || Value::known(Fq::from(222)))?;

                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use midnight_proofs::dev::MockProver;

    #[test]
    fn test_trashcan_circuit_valid() {
        // NOTE: MockProver does NOT check trashcan (additive selector) constraints.
        // Trashcans are only enforced during actual proving/verification.
        // This test verifies the circuit synthesizes without errors.
        let circuit = TrashcanTestCircuit {
            a: Fq::from(42),
            b: Fq::from(42), // Equal to a
            c: Fq::from(100),
            d: Fq::from(100), // Equal to c
            p1: Fq::from(1),  // Public input 1
            p2: Fq::from(2),  // Public input 2
            p3: Fq::from(3),  // Public input 3
        };

        let k = 4;
        let public_inputs = vec![vec![Fq::from(1), Fq::from(2), Fq::from(3)]];
        let prover = MockProver::run(k, &circuit, public_inputs).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_trashcan_circuit_structure() {
        // Verify the circuit has the expected structure
        use midnight_proofs::plonk::{ConstraintSystem, k_from_circuit};

        let circuit = TrashcanTestCircuit::default();
        let k = k_from_circuit(&circuit);
        assert_eq!(k, 4);

        // Verify constraint system has 2 trashcans
        let mut cs = ConstraintSystem::default();
        let _config = <TrashcanTestCircuit as midnight_proofs::plonk::Circuit<Fq>>::configure(&mut cs);
        assert_eq!(cs.trashcans().len(), 2, "Circuit should have 2 trashcans");

        // Verify trashcan names (names come from constraint names, not gate names)
        let trashcan_names: Vec<_> = cs.trashcans().iter().map(|t| t.name()).collect();
        assert_eq!(trashcan_names[0], "a == b");
        assert_eq!(trashcan_names[1], "c == d");
    }
}
