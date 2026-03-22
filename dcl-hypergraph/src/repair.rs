// dcl-hypergraph/src/repair.rs
//! DCL repair procedure after label rejection.
//! When a candidate prime fails, a pivot vertex label is incremented
//! until the DCL constraint is restored on all incident hyperedges.

use crate::hypergraph::Hypergraph;
use dcl_core::gcd::setwise_gcd;
use dcl_core::labeling::Labeling;

/// Repair the DCL constraint after incrementing the pivot vertex label.
/// Increments labels[pivot] by delta, then keeps incrementing until
/// all hyperedges incident to pivot satisfy setwise gcd = 1.
///
/// Returns the number of increments applied.
pub fn repair_dcl_constraint(
    h: &Hypergraph,
    labels: &mut Labeling,
    pivot: usize,
    delta: u64,
) -> u64 {
    let mut increments = 0u64;
    labels.set(pivot, labels.get(pivot).saturating_add(delta));
    increments += 1;

    // Keep incrementing until all incident hyperedges are satisfied
    loop {
        let all_ok = h
            .edges
            .iter()
            .filter(|edge| edge.contains(&pivot))
            .all(|edge| {
                let vals: Vec<u64> = edge.iter().map(|&v| labels.get(v)).collect();
                setwise_gcd(&vals) == 1
            });
        if all_ok {
            break;
        }
        labels.set(pivot, labels.get(pivot) + 1);
        increments += 1;
        // Safety: prevent infinite loop on degenerate inputs
        if increments > 10_000 {
            break;
        }
    }
    increments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypergraph::Hypergraph;

    #[test]
    fn repair_restores_constraint() {
        let mut h = Hypergraph::new(3);
        h.add_edge(vec![0, 1, 2]);
        // Label 0 with 6 (shares factor with 6,9 -> gcd=3)
        let lab = Labeling::new(vec![6, 9, 5]);
        // pre-repair: gcd(6,9,5)=1 -- actually ok, let's break it
        let lab2 = Labeling::new(vec![6, 3, 5]);
        // gcd(6,3,5) = 1 -> still ok since gcd(6,3)=3 but setwise_gcd([6,3,5])=1
        // Let's try a broken case: [6,3,9]
        let mut lab3 = Labeling::new(vec![6, 3, 9]);
        // gcd(6,3,9)=3 != 1
        assert_eq!(setwise_gcd(&[6, 3, 9]), 3);
        let increments = repair_dcl_constraint(&h, &mut lab3, 0, 1);
        // After repair, all incident edges should be coprime
        assert!(
            h.is_valid_labeling(&lab3),
            "After repair, labeling should be valid"
        );
        println!("Repair took {increments} increments");
        let _ = lab;
        let _ = lab2;
    }
}
