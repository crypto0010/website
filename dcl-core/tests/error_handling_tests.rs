// Integration tests for error handling

use dcl_core::dcl::DclSequence;
use dcl_core::error::DclError;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;

#[test]
fn test_invalid_labeling_zero() {
    let result = Labeling::try_new(vec![2, 0, 5, 7]);
    assert!(result.is_err());
    match result {
        Err(DclError::InvalidLabeling(msg)) => {
            assert!(msg.contains("zero"));
            assert!(msg.contains("index 1"));
        }
        _ => panic!("Expected InvalidLabeling error"),
    }
}

#[test]
fn test_invalid_labeling_set_zero() {
    let mut labeling = Labeling::new(vec![2, 3, 5]);
    let result = labeling.try_set(0, 0);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DclError::InvalidLabeling(_)));
}

#[test]
fn test_labeling_index_out_of_bounds() {
    let mut labeling = Labeling::new(vec![2, 3, 5]);
    let result = labeling.try_set(10, 7);
    assert!(result.is_err());
    match result {
        Err(DclError::IndexOutOfBounds { index, len }) => {
            assert_eq!(index, 10);
            assert_eq!(len, 3);
        }
        _ => panic!("Expected IndexOutOfBounds error"),
    }
}

#[test]
fn test_graph_edge_out_of_bounds() {
    let mut graph = Graph::new(5);
    let result = graph.try_add_edge(0, 10);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DclError::IndexOutOfBounds { .. }
    ));
}

#[test]
fn test_dcl_dimension_mismatch() {
    let graph = Graph::path(5);
    let labeling = Labeling::new(vec![2, 3, 5]); // Only 3 labels for 5 vertices
    let transform = PowerMap::new(2);

    let result = DclSequence::try_new(&graph, &transform, labeling);
    assert!(result.is_err());
    match result {
        Err(DclError::DimensionMismatch { expected, actual }) => {
            assert_eq!(expected, 5);
            assert_eq!(actual, 3);
        }
        _ => panic!("Expected DimensionMismatch error"),
    }
}

#[test]
fn test_dcl_coprimality_violation_initial() {
    let graph = Graph::path(3);
    let labeling = Labeling::new(vec![6, 9, 5]); // 6 and 9 are not coprime (gcd = 3)
    let transform = PowerMap::new(2);

    let result = DclSequence::try_new(&graph, &transform, labeling);
    assert!(result.is_err());
    match result {
        Err(DclError::CoprimalityViolation {
            u,
            v,
            label_u,
            label_v,
            gcd,
        }) => {
            assert_eq!(u, 0);
            assert_eq!(v, 1);
            assert_eq!(label_u, 6);
            assert_eq!(label_v, 9);
            assert_eq!(gcd, 3);
        }
        _ => panic!("Expected CoprimalityViolation error"),
    }
}

#[test]
fn test_dcl_valid_initialization() {
    let graph = Graph::path(3);
    let labeling = Labeling::new(vec![2, 3, 5]); // All coprime
    let transform = PowerMap::new(2);

    let result = DclSequence::try_new(&graph, &transform, labeling);
    assert!(result.is_ok());
    let seq = result.unwrap();
    assert_eq!(seq.step, 0);
}

#[test]
fn test_error_display_messages() {
    let err = DclError::coprimality_violation(0, 1, 6, 9);
    let msg = format!("{}", err);
    assert!(msg.contains("edge (0, 1)"));
    assert!(msg.contains("gcd(6, 9) = 3"));

    let err = DclError::overflow(5, "u64 overflow detected");
    let msg = format!("{}", err);
    assert!(msg.contains("step 5"));

    let err = DclError::dimension_mismatch(5, 3);
    let msg = format!("{}", err);
    assert!(msg.contains("expected 5"));
    assert!(msg.contains("got 3"));
}

#[test]
fn test_dcl_verify_steps_with_error() {
    let graph = Graph::path(3);
    let labeling = Labeling::new(vec![2, 3, 5]);
    let transform = PowerMap::new(2);

    let mut seq = DclSequence::new(&graph, &transform, labeling);
    // Should work for valid labelings
    let result = seq.verify_steps(2);
    assert!(result.is_ok());
}

#[test]
fn test_backward_compatibility() {
    // Test that old panic-based API still works for valid inputs
    let labeling = Labeling::new(vec![2, 3, 5]);
    assert_eq!(labeling.len(), 3);

    let mut graph = Graph::new(3);
    graph.add_edge(0, 1);
    graph.add_edge(1, 2);
    assert_eq!(graph.edges().len(), 2);

    let transform = PowerMap::new(2);
    let _seq = DclSequence::new(&graph, &transform, labeling);
}

#[test]
#[should_panic(expected = "All labels must be positive")]
fn test_labeling_new_panics_on_zero() {
    Labeling::new(vec![2, 0, 5]);
}

#[test]
#[should_panic(expected = "Initial labeling must be valid")]
fn test_dcl_new_panics_on_invalid() {
    let graph = Graph::path(3);
    let labeling = Labeling::new(vec![6, 9, 5]); // Not coprime
    let transform = PowerMap::new(2);
    DclSequence::new(&graph, &transform, labeling);
}
