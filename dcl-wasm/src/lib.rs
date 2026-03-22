// dcl-wasm/src/lib.rs
//! WebAssembly bindings for DCL Framework
//! Exposes core DCL functionality to JavaScript

use wasm_bindgen::prelude::*;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use dcl_core::gcd;

/// Initialize panic hook for better error messages in browser
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// DCL Graph wrapper for JavaScript
#[wasm_bindgen]
pub struct WasmGraph {
    graph: Graph,
}

#[wasm_bindgen]
impl WasmGraph {
    /// Create a complete graph with n vertices
    #[wasm_bindgen(constructor)]
    pub fn complete(n: usize) -> WasmGraph {
        WasmGraph {
            graph: Graph::complete(n),
        }
    }

    /// Create a cycle graph with n vertices
    pub fn cycle(n: usize) -> WasmGraph {
        WasmGraph {
            graph: Graph::cycle(n),
        }
    }

    /// Create a path graph with n vertices
    pub fn path(n: usize) -> WasmGraph {
        WasmGraph {
            graph: Graph::path(n),
        }
    }

    /// Get number of vertices
    pub fn vertex_count(&self) -> usize {
        self.graph.n
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.graph.edges().len()
    }

    /// Check if edge exists
    pub fn has_edge(&self, u: usize, v: usize) -> bool {
        self.graph.edges().contains(&(u, v)) || self.graph.edges().contains(&(v, u))
    }

    /// Get edges as JSON
    pub fn edges_json(&self) -> Result<String, JsValue> {
        let edges = &self.graph.edges();
        serde_json::to_string(edges)
            .map_err(|e: serde_json::Error| JsValue::from_str(&e.to_string()))
    }
}

/// DCL Labeling wrapper for JavaScript
#[wasm_bindgen]
pub struct WasmLabeling {
    labeling: Labeling,
}

#[wasm_bindgen]
impl WasmLabeling {
    /// Create a labeling from a JavaScript array
    #[wasm_bindgen(constructor)]
    pub fn new(labels_js: JsValue) -> Result<WasmLabeling, JsValue> {
        let labels: Vec<u64> = serde_wasm_bindgen::from_value(labels_js)?;

        Ok(WasmLabeling {
            labeling: Labeling::new(labels),
        })
    }

    /// Get label at index
    pub fn get(&self, i: usize) -> u64 {
        self.labeling.get(i)
    }

    /// Get all labels as JSON
    pub fn labels_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.labeling.labels)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get number of labels
    pub fn len(&self) -> usize {
        self.labeling.len()
    }
}

/// DCL Sequence wrapper for JavaScript
#[wasm_bindgen]
pub struct WasmDclSequence {
    initial_labels: Vec<u64>,
    graph: Graph,
}

#[wasm_bindgen]
impl WasmDclSequence {
    /// Create a new DCL sequence
    #[wasm_bindgen(constructor)]
    pub fn new(graph: &WasmGraph, labeling: &WasmLabeling) -> WasmDclSequence {
        WasmDclSequence {
            initial_labels: labeling.labeling.labels.clone(),
            graph: graph.graph.clone(),
        }
    }

    /// Evolve the sequence for t steps
    pub fn evolve(&self, steps: usize) -> Result<String, JsValue> {
        let labeling = Labeling::new(self.initial_labels.clone());
        let transform = PowerMap::new(2);

        let mut history = vec![labeling.labels.clone()];

        let mut current = labeling;
        for _ in 0..steps {
            current = current.evolve(&transform);
            history.push(current.labels.clone());
        }

        serde_json::to_string(&history)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Verify coprimality constraints
    pub fn verify_coprime(&self) -> bool {
        let labeling = Labeling::new(self.initial_labels.clone());

        for (u, v) in self.graph.edges() {
            if gcd::gcd(labeling.get(u), labeling.get(v)) != 1 {
                return false;
            }
        }

        true
    }
}

/// Utility functions for JavaScript
#[wasm_bindgen]
pub struct DclUtils;

#[wasm_bindgen]
impl DclUtils {
    /// Compute GCD of two numbers
    pub fn gcd(a: u64, b: u64) -> u64 {
        gcd::gcd(a, b)
    }

    /// Check if two numbers are coprime
    pub fn are_coprime(a: u64, b: u64) -> bool {
        gcd::are_coprime(a, b)
    }

    /// Compute LCM of two numbers
    pub fn lcm(a: u64, b: u64) -> u64 {
        if a == 0 || b == 0 {
            0
        } else {
            (a / gcd::gcd(a, b)) * b
        }
    }

    /// Test if number is prime (simple trial division)
    pub fn is_prime_simple(n: u64) -> bool {
        if n <= 1 {
            return false;
        }
        if n == 2 || n == 3 {
            return true;
        }
        if n % 2 == 0 || n % 3 == 0 {
            return false;
        }

        let mut i = 5;
        while i * i <= n {
            if n % i == 0 || n % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_graph() {
        let graph = WasmGraph::complete(5);
        assert_eq!(graph.vertex_count(), 5);
        assert_eq!(graph.edge_count(), 10);
    }
}
