#![no_main]

use libfuzzer_sys::fuzz_target;
use dcl_core::labeling::Labeling;
use dcl_core::graph::Graph;
use dcl_core::dcl::DclSequence;
use dcl_core::transform::IdentityMap;

fuzz_target!(|data: &[u8]| {
    // Convert fuzzer data to labeling input
    if data.len() < 2 {
        return;
    }

    // Extract graph size (1-20 vertices)
    let n = ((data[0] as usize) % 20) + 1;

    // Generate labels from remaining bytes
    let mut labels = Vec::with_capacity(n);
    for i in 0..n {
        let idx = (i + 1) % data.len();
        let label = ((data[idx] as u64) % 1000) + 1; // Labels in [1, 1000]
        labels.push(label);
    }

    // Test labeling creation
    match Labeling::try_new(labels.clone()) {
        Ok(labeling) => {
            // Verify basic properties
            assert_eq!(labeling.len(), n);
            for i in 0..n {
                assert_eq!(labeling.get(i), labels[i]);
            }

            // Test set operations
            for i in 0..n {
                let new_val = labels[i] + 1;
                let mut lab = labeling.clone();
                if lab.try_set(i, new_val).is_ok() {
                    assert_eq!(lab.get(i), new_val);
                }
            }

            // Test with DCL sequence if enough data
            if data.len() >= n + 2 {
                let graph = Graph::complete(n);
                let transform = IdentityMap;

                if let Ok(mut dcl) = DclSequence::try_new(&graph, &transform, labeling.clone()) {
                    // Verify initial labeling
                    assert_eq!(dcl.current.len(), n);

                    // Try evolving a few steps
                    let steps = ((data[n + 1] as usize) % 5) + 1;
                    let _ = dcl.verify_steps(steps);
                }
            }
        }
        Err(_) => {
            // Invalid labeling is expected for some inputs
        }
    }
});
