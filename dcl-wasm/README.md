# DCL Framework - WebAssembly Bindings

WebAssembly bindings for the DCL (Dynamic Coprime Labeling) Framework, enabling DCL functionality in web browsers.

## Prerequisites

1. **Rust** (with `wasm32-unknown-unknown` target)
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. **wasm-pack**
   ```bash
   cargo install wasm-pack
   ```

## Building

Build the WASM module:

```bash
./build.sh
```

Or manually:

```bash
wasm-pack build --target web --out-dir www/pkg
```

## Running the Demo

1. Build the WASM module (see above)

2. Start a local web server:
   ```bash
   cd www
   python -m http.server 8080
   ```
   Or use any other HTTP server (http-server, live-server, etc.)

3. Open http://localhost:8080 in your browser

## API Reference

### WasmGraph

Create and manipulate graphs:

```javascript
import { WasmGraph } from './pkg/dcl_wasm.js';

// Create a complete graph with 5 vertices
const graph = WasmGraph.complete(5);

// Create a cycle graph
const cycle = WasmGraph.cycle(6);

// Create a path graph
const path = WasmGraph.path(4);

// Get graph properties
console.log(graph.vertex_count()); // 5
console.log(graph.edge_count());   // 10
console.log(graph.has_edge(0, 1)); // true

// Get edges as JSON
const edges = JSON.parse(graph.edges_json());
console.log(edges); // [[0,1], [0,2], ...]
```

### WasmLabeling

Work with DCL labelings:

```javascript
import { WasmLabeling } from './pkg/dcl_wasm.js';

// Create labeling from array
const labels = [2, 3, 5, 7, 11];
const labeling = new WasmLabeling(labels);

// Get individual label
console.log(labeling.get(0)); // 2

// Get all labels as JSON
const allLabels = JSON.parse(labeling.labels_json());
console.log(allLabels); // [2, 3, 5, 7, 11]

// Get number of labels
console.log(labeling.len()); // 5
```

### WasmDclSequence

Evolve DCL sequences:

```javascript
import { WasmGraph, WasmLabeling, WasmDclSequence } from './pkg/dcl_wasm.js';

const graph = WasmGraph.complete(5);
const labeling = new WasmLabeling([2, 3, 5, 7, 11]);
const sequence = new WasmDclSequence(graph, labeling);

// Evolve for 3 steps
const historyJson = sequence.evolve(3);
const history = JSON.parse(historyJson);
// history = [[2,3,5,7,11], [4,9,25,49,121], ...]

// Verify coprimality
const isValid = sequence.verify_coprime();
console.log(isValid); // true or false
```

### DclUtils

Utility functions:

```javascript
import { DclUtils } from './pkg/dcl_wasm.js';

// Compute GCD
const gcd = DclUtils.gcd(48, 18);
console.log(gcd); // 6

// Check coprimality
const coprime = DclUtils.are_coprime(17, 13);
console.log(coprime); // true

// Compute LCM
const lcm = DclUtils.lcm(12, 18);
console.log(lcm); // 36

// Test primality
const isPrime = DclUtils.is_prime_simple(17);
console.log(isPrime); // true
```

## Features

The web demo includes:

- **Interactive Graph Creation**: Create complete, cycle, and path graphs
- **Custom Labelings**: Define your own initial labels
- **DCL Evolution**: Evolve sequences with the power map transformation g(x) = x²
- **Coprimality Verification**: Check if labelings satisfy coprime constraints
- **Visual Representation**: Canvas-based graph visualization
- **Utility Functions**: GCD, LCM, and primality testing
- **Evolution History**: View the complete evolution sequence

## Browser Compatibility

Requires a modern browser with WebAssembly support:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## Performance

The WASM module provides near-native performance for:
- Graph operations
- GCD/LCM computations
- DCL evolution
- Coprimality verification

## Development

To modify the WASM bindings:

1. Edit `src/lib.rs`
2. Rebuild with `./build.sh`
3. Refresh your browser

## License

Part of the DCL Framework project.
